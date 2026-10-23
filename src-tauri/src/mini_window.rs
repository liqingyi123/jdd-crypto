//! Frameless mini browser window with custom chrome + child webview.

use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{
    window::Color, AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, PhysicalPosition, Url,
    WebviewBuilder, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::Shortcut;
use tauri_plugin_store::StoreExt;

use crate::global_shortcuts;
use crate::state::{AppState, DEFAULT_MINI_WINDOW_SHORTCUT};
use crate::windows;

const SETTINGS_STORE: &str = "settings.json";
const SHORTCUT_KEY: &str = "miniWindowShortcut";
const PREF_KEY: &str = "miniWindowEnabled";
const AUTO_HIDE_KEY: &str = "miniWindowAutoHide";
const ALWAYS_ON_TOP_KEY: &str = "miniWindowAlwaysOnTop";
const CONTENT_OPACITY_KEY: &str = "miniWindowContentOpacity";
const BACKGROUND_OPACITY_KEY: &str = "miniWindowBackgroundOpacity";
const WIDTH_KEY: &str = "miniWindowWidth";
const HEIGHT_KEY: &str = "miniWindowHeight";

pub const MINI_WINDOW_LABEL: &str = "mini-window";
pub const MINI_CONTENT_LABEL: &str = "mini-content";
pub const MINI_OPACITY_LABEL: &str = "mini-opacity";

const DEFAULT_WIDTH: f64 = 300.0;
const DEFAULT_HEIGHT: f64 = 420.0;
const MIN_WIDTH: f64 = 250.0;
const MIN_HEIGHT: f64 = 380.0;
const TOOLBAR_HEIGHT: f64 = 44.0;
const DEFAULT_CONTENT_OPACITY: f64 = 1.0;
const DEFAULT_BACKGROUND_OPACITY: f64 = 1.0;
const SIZE_SAVE_DEBOUNCE: Duration = Duration::from_millis(300);
/// Debounce recreating mini-content when the background-opacity slider moves.
#[cfg(windows)]
const LWA_REBIND_DEBOUNCE: Duration = Duration::from_millis(280);
const OPACITY_BUBBLE_WIDTH: f64 = 220.0;
const OPACITY_BUBBLE_HEIGHT: f64 = 130.0;

/// After a move event, briefly suppress auto-hide as a drag fallback.
const DRAG_SUPPRESS: Duration = Duration::from_millis(1000);
/// Clear DRAGGING after this quiet period with no Moved events.
const DRAG_IDLE_CLEAR: Duration = Duration::from_millis(400);

static AUTO_HIDE_LOOP_STARTED: AtomicBool = AtomicBool::new(false);
static CONTENT_VISIBLE: AtomicBool = AtomicBool::new(false);
/// Explicit drag session from the title-bar chrome (cross-monitor safe).
static DRAGGING: AtomicBool = AtomicBool::new(false);
/// Millis since UNIX_EPOCH of last WindowEvent::Moved (or 0).
static LAST_MOVED_MS: AtomicU64 = AtomicU64::new(0);
static LAST_BOUNDS: Mutex<Option<(i32, i32, i32, i32)>> = Mutex::new(None);
/// Bumps on each Resized so debounced size save only persists the latest size.
static SIZE_SAVE_GEN: AtomicU64 = AtomicU64::new(0);
/// Bumps on each Moved so idle-clear only runs after drag motion stops.
static DRAG_MOVE_GEN: AtomicU64 = AtomicU64::new(0);
/// Bumps on each background-opacity change so only the latest rebind runs.
#[cfg(windows)]
static LWA_REBIND_GEN: AtomicU64 = AtomicU64::new(0);
/// True only when minimized by the auto-hide loop (hover-restore eligible).
static AUTO_HIDE_MINIMIZED: AtomicBool = AtomicBool::new(false);
/// Guards Focused handlers while collapse is tearing down WebView2.
static COLLAPSING: AtomicBool = AtomicBool::new(false);
/// URL to recreate in `mini-content` after collapse (browsing restore).
static PENDING_RESTORE_URL: Mutex<Option<Url>> = Mutex::new(None);
/// Cached top-level HWND — Tauri may drop `WebviewWindow` lookup while the OS window remains.
static MINI_HWND: AtomicIsize = AtomicIsize::new(0);
/// Rate-limit "window missing" logs in the auto-hide loop.
static AUTO_HIDE_MISSING_LOG_MS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MiniWindowPrefs {
    pub auto_hide: bool,
    pub always_on_top: bool,
    pub content_opacity: f64,
    pub background_opacity: f64,
    pub browsing: bool,
}

fn clamp_opacity(value: f64) -> f64 {
    value.clamp(0.2, 1.0)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn mark_moved() {
    LAST_MOVED_MS.store(now_ms(), Ordering::Relaxed);
}

fn drag_suppressed() -> bool {
    if DRAGGING.load(Ordering::Relaxed) {
        return true;
    }
    let last = LAST_MOVED_MS.load(Ordering::Relaxed);
    if last == 0 {
        return false;
    }
    now_ms().saturating_sub(last) < DRAG_SUPPRESS.as_millis() as u64
}

pub fn set_dragging(dragging: bool) {
    DRAGGING.store(dragging, Ordering::Relaxed);
    if dragging {
        mark_moved();
    }
}

/// Keep auto-hide suppressed while the window is being dragged (Moved stream).
fn note_window_moved() {
    mark_moved();
    DRAGGING.store(true, Ordering::Relaxed);
    let gen = DRAG_MOVE_GEN.fetch_add(1, Ordering::Relaxed) + 1;
    thread::spawn(move || {
        thread::sleep(DRAG_IDLE_CLEAR);
        if DRAG_MOVE_GEN.load(Ordering::Relaxed) != gen {
            return;
        }
        DRAGGING.store(false, Ordering::Relaxed);
    });
}

fn save_last_bounds(win: &WebviewWindow) {
    let Ok(pos) = win.outer_position() else {
        return;
    };
    let Ok(size) = win.outer_size() else {
        return;
    };
    if let Ok(mut guard) = LAST_BOUNDS.lock() {
        *guard = Some((pos.x, pos.y, size.width as i32, size.height as i32));
    }
}

fn clear_last_bounds() {
    if let Ok(mut guard) = LAST_BOUNDS.lock() {
        *guard = None;
    }
}

fn cursor_in_rect(x: i32, y: i32, w: i32, h: i32) -> bool {
    let Some((cx, cy)) = windows::cursor_pos_public() else {
        return false;
    };
    cx >= x && cx < x + w && cy >= y && cy < y + h
}

fn cursor_in_last_bounds() -> bool {
    let Ok(guard) = LAST_BOUNDS.lock() else {
        return false;
    };
    let Some((x, y, w, h)) = *guard else {
        return false;
    };
    cursor_in_rect(x, y, w, h)
}

pub fn load_shortcut(app: &AppHandle) -> String {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return DEFAULT_MINI_WINDOW_SHORTCUT.to_string();
    };
    store
        .get(SHORTCUT_KEY)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|raw| crate::mouse_follow::validate_shortcut(&raw).ok())
        .unwrap_or_else(|| DEFAULT_MINI_WINDOW_SHORTCUT.to_string())
}

pub fn save_shortcut(app: &AppHandle, shortcut: &str) {
    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(SHORTCUT_KEY, serde_json::json!(shortcut));
        let _ = store.save();
    }
}

pub fn load_pref(app: &AppHandle) -> bool {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return true;
    };
    store
        .get(PREF_KEY)
        .and_then(|value| value.as_bool())
        .unwrap_or(true)
}

pub fn save_pref(app: &AppHandle, enabled: bool) {
    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(PREF_KEY, serde_json::json!(enabled));
        let _ = store.save();
    }
}

pub fn apply_pref(app: &AppHandle, enabled: bool) {
    let state = app.state::<AppState>();
    state
        .mini_window_pref_enabled
        .store(enabled, Ordering::Relaxed);
    save_pref(app, enabled);
    if !enabled {
        close(app);
    }
    let _ = global_shortcuts::register_all(app);
}

fn load_bool(app: &AppHandle, key: &str, default: bool) -> bool {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return default;
    };
    store
        .get(key)
        .and_then(|value| value.as_bool())
        .unwrap_or(default)
}

fn save_bool(app: &AppHandle, key: &str, value: bool) {
    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(key, serde_json::json!(value));
        let _ = store.save();
    }
}

fn load_opacity(app: &AppHandle, key: &str, default: f64) -> f64 {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return default;
    };
    store
        .get(key)
        .and_then(|value| value.as_f64())
        .map(clamp_opacity)
        .unwrap_or(default)
}

fn save_opacity(app: &AppHandle, key: &str, value: f64) {
    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(key, serde_json::json!(clamp_opacity(value)));
        let _ = store.save();
    }
}

pub fn load_auto_hide(app: &AppHandle) -> bool {
    load_bool(app, AUTO_HIDE_KEY, false)
}

pub fn load_always_on_top(app: &AppHandle) -> bool {
    load_bool(app, ALWAYS_ON_TOP_KEY, true)
}

pub fn load_content_opacity(app: &AppHandle) -> f64 {
    load_opacity(app, CONTENT_OPACITY_KEY, DEFAULT_CONTENT_OPACITY)
}

pub fn load_background_opacity(app: &AppHandle) -> f64 {
    load_opacity(app, BACKGROUND_OPACITY_KEY, DEFAULT_BACKGROUND_OPACITY)
}

fn clamp_size(width: f64, height: f64) -> (f64, f64) {
    (width.max(MIN_WIDTH), height.max(MIN_HEIGHT))
}

fn load_saved_size(app: &AppHandle) -> (f64, f64) {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return (DEFAULT_WIDTH, DEFAULT_HEIGHT);
    };
    let width = store
        .get(WIDTH_KEY)
        .and_then(|value| value.as_f64())
        .unwrap_or(DEFAULT_WIDTH);
    let height = store
        .get(HEIGHT_KEY)
        .and_then(|value| value.as_f64())
        .unwrap_or(DEFAULT_HEIGHT);
    clamp_size(width, height)
}

fn save_size(app: &AppHandle, width: f64, height: f64) {
    let (width, height) = clamp_size(width, height);
    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(WIDTH_KEY, serde_json::json!(width));
        store.set(HEIGHT_KEY, serde_json::json!(height));
        let _ = store.save();
    }
}

fn schedule_save_size(app: &AppHandle, win: &WebviewWindow) {
    let gen = SIZE_SAVE_GEN.fetch_add(1, Ordering::Relaxed) + 1;
    let handle = app.clone();
    let win = win.clone();
    thread::spawn(move || {
        thread::sleep(SIZE_SAVE_DEBOUNCE);
        if SIZE_SAVE_GEN.load(Ordering::Relaxed) != gen {
            return;
        }
        let Ok(size) = win.inner_size() else {
            return;
        };
        let scale = win.scale_factor().unwrap_or(1.0);
        let width = size.width as f64 / scale;
        let height = size.height as f64 / scale;
        save_size(&handle, width, height);
    });
}

pub fn current_prefs(app: &AppHandle) -> MiniWindowPrefs {
    MiniWindowPrefs {
        auto_hide: load_auto_hide(app),
        always_on_top: load_always_on_top(app),
        content_opacity: load_content_opacity(app),
        background_opacity: load_background_opacity(app),
        browsing: CONTENT_VISIBLE.load(Ordering::Relaxed) || has_pending_restore_url(),
    }
}

pub fn handle_shortcut(app: &AppHandle, shortcut: &Shortcut) {
    let state = app.state::<AppState>();
    if !state.mini_window_pref_enabled.load(Ordering::Relaxed) {
        return;
    }
    let expected = state
        .mini_window_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| DEFAULT_MINI_WINDOW_SHORTCUT.to_string());
    let Ok(expected_shortcut) = Shortcut::from_str(&expected) else {
        return;
    };
    if shortcut.id() != expected_shortcut.id() {
        return;
    }
    toggle(app);
}

pub fn toggle(app: &AppHandle) {
    if let Some(win) = resolve_mini_window(app) {
        remember_mini_hwnd(&win);
        let minimized = win.is_minimized().unwrap_or(false);
        let visible = win.is_visible().unwrap_or(false);
        if minimized || !visible {
            focus_existing(app, &win);
        } else {
            schedule_collapse(app, win);
        }
        return;
    }
    if show_cached_hwnd() {
        schedule_restore_pending_content(app);
        return;
    }
    #[cfg(not(windows))]
    if restore_mini_window_native(app) {
        return;
    }
    show(app);
}

pub fn show(app: &AppHandle) {
    if let Some(win) = resolve_mini_window(app) {
        focus_existing(app, &win);
        return;
    }
    if show_cached_hwnd() {
        schedule_restore_pending_content(app);
        return;
    }
    #[cfg(not(windows))]
    if restore_mini_window_native(app) {
        return;
    }

    let handle = app.clone();
    thread::spawn(move || {
        let app = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            create_window(&app);
        });
    });
}

/// Non-Windows: retry resolve + unminimize when HWND cache is unavailable.
#[cfg(not(windows))]
fn restore_mini_window_native(app: &AppHandle) -> bool {
    let Some(win) = resolve_mini_window(app) else {
        return false;
    };
    focus_existing(app, &win);
    true
}

fn focus_existing(app: &AppHandle, win: &WebviewWindow) {
    COLLAPSING.store(false, Ordering::Relaxed);
    AUTO_HIDE_MINIMIZED.store(false, Ordering::Relaxed);
    clear_last_bounds();
    remember_mini_hwnd(win);
    let _ = win.unminimize();
    let _ = win.show();
    show_cached_hwnd();
    let always_on_top = load_always_on_top(app);
    let _ = win.set_always_on_top(always_on_top);
    let _ = win.set_focus();
    apply_background_opacity(app, load_background_opacity(app));
    apply_content_opacity(app, load_content_opacity(app));
    let _ = win.emit("mini-window://prefs", current_prefs(app));
    crate::mouse_trail::schedule_raise_overlays(app);
    // Recreate or reshown content off the UI/invoke thread (add_child can deadlock).
    schedule_restore_pending_content(app);
}

fn create_window(app: &AppHandle) {
    if app.get_webview_window(MINI_WINDOW_LABEL).is_some() {
        return;
    }

    let always_on_top = load_always_on_top(app);
    let (width, height) = load_saved_size(app);
    // Not Tauri-transparent: parent LWA_ALPHA can fade chrome + WebView2 together.
    // (transparent(true) uses per-pixel alpha and conflicts with LWA → solid black.)
    let result =
        WebviewWindowBuilder::new(app, MINI_WINDOW_LABEL, WebviewUrl::App("index.html".into()))
            .title("迷你窗")
            .inner_size(width, height)
            .min_inner_size(MIN_WIDTH, MIN_HEIGHT)
            .resizable(true)
            .maximizable(false)
            .minimizable(true)
            .decorations(false)
            .transparent(false)
            .shadow(true)
            .background_color(Color(0x2a, 0x24, 0x1f, 0xff))
            .always_on_top(always_on_top)
            .skip_taskbar(false)
            .focused(true)
            .visible(false)
            .build();

    let Ok(win) = result else {
        return;
    };

    CONTENT_VISIBLE.store(false, Ordering::Relaxed);
    clear_last_bounds();
    clear_pending_restore_url();
    remember_mini_hwnd(&win);
    windows::center_on_cursor_monitor(app, &win);
    let _ = win.show();
    let _ = win.set_focus();
    bind_window_events(app, &win);
    #[cfg(windows)]
    {
        apply_rounded_corners(&win);
    }
    apply_background_opacity(app, load_background_opacity(app));
    ensure_auto_hide_loop(app);
    crate::mouse_trail::schedule_raise_overlays(app);

    let handle = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(120));
        if let Some(win) = handle.get_webview_window(MINI_WINDOW_LABEL) {
            let _ = win.emit("mini-window://prefs", current_prefs(&handle));
        }
    });
}

fn bind_window_events(app: &AppHandle, win: &WebviewWindow) {
    let app_resize = app.clone();
    let win_resize = win.clone();
    win.on_window_event(move |event| match event {
        tauri::WindowEvent::Resized(size) => {
            if win_resize.is_minimized().unwrap_or(false)
                || size.width == 0
                || size.height == 0
            {
                return;
            }
            layout_content(&app_resize, &win_resize);
            schedule_save_size(&app_resize, &win_resize);
        }
        tauri::WindowEvent::ScaleFactorChanged { .. } => {
            if win_resize.is_minimized().unwrap_or(false) {
                return;
            }
            layout_content(&app_resize, &win_resize);
        }
        tauri::WindowEvent::Moved(_) => {
            note_window_moved();
            hide_opacity_bubble(&app_resize);
        }
        tauri::WindowEvent::Focused(true) => {
            // Clicking the mini window should dismiss the opacity bubble.
            hide_opacity_bubble(&app_resize);
            if COLLAPSING.load(Ordering::Relaxed) {
                return;
            }
            // Taskbar/show may restore the shell without going through focus_existing.
            schedule_restore_pending_content(&app_resize);
        }
        tauri::WindowEvent::Destroyed => {
            CONTENT_VISIBLE.store(false, Ordering::Relaxed);
            COLLAPSING.store(false, Ordering::Relaxed);
            DRAGGING.store(false, Ordering::Relaxed);
            AUTO_HIDE_MINIMIZED.store(false, Ordering::Relaxed);
            MINI_HWND.store(0, Ordering::Relaxed);
            clear_pending_restore_url();
            clear_last_bounds();
            hide_opacity_bubble(&app_resize);
        }
        _ => {}
    });
}

fn client_logical_size(win: &WebviewWindow) -> Option<(f64, f64)> {
    let size = win.inner_size().ok()?;
    let scale = win.scale_factor().unwrap_or(1.0);
    Some(((size.width as f64) / scale, (size.height as f64) / scale))
}

fn content_bounds(win: &WebviewWindow) -> Option<(LogicalPosition<f64>, LogicalSize<f64>)> {
    let (width, height) = client_logical_size(win)?;
    let content_w = width.max(80.0);
    let content_h = (height - TOOLBAR_HEIGHT).max(80.0);
    Some((
        LogicalPosition::new(0.0, TOOLBAR_HEIGHT),
        LogicalSize::new(content_w, content_h),
    ))
}

fn layout_content(app: &AppHandle, win: &WebviewWindow) {
    if win.is_minimized().unwrap_or(false) {
        return;
    }
    if !CONTENT_VISIBLE.load(Ordering::Relaxed) {
        return;
    }
    let Some(webview) = app.get_webview(MINI_CONTENT_LABEL) else {
        return;
    };
    let Some((pos, size)) = content_bounds(win) else {
        return;
    };
    let _ = webview.set_position(pos);
    let _ = webview.set_size(size);
}

fn emit_browsing(app: &AppHandle, browsing: bool) {
    if let Some(win) = app.get_webview_window(MINI_WINDOW_LABEL) {
        let _ = win.emit("mini-window://browsing", browsing);
    }
}

fn ensure_content_webview(app: &AppHandle, url: Url) -> Result<(), String> {
    let win = app
        .get_webview_window(MINI_WINDOW_LABEL)
        .ok_or_else(|| "迷你窗未打开".to_string())?;

    if let Some(webview) = app.get_webview(MINI_CONTENT_LABEL) {
        webview.navigate(url).map_err(|err| err.to_string())?;
        CONTENT_VISIBLE.store(true, Ordering::Relaxed);
        layout_content(app, &win);
        emit_browsing(app, true);
        // Document may still be unloading — delay opacity / LWA apply.
        schedule_reapply_opacities(app);
        return Ok(());
    }

    CONTENT_VISIBLE.store(true, Ordering::Relaxed);

    let (pos, size) = content_bounds(&win).ok_or_else(|| "无法获取窗口尺寸".to_string())?;
    let content_opacity = load_content_opacity(app);
    let opacity_script = content_opacity_script(content_opacity);

    let builder = WebviewBuilder::new(MINI_CONTENT_LABEL, WebviewUrl::External(url))
        .initialization_script(&opacity_script)
        .on_navigation(|nav_url| {
            matches!(nav_url.scheme(), "http" | "https" | "about" | "data" | "blob")
        });

    // WebView2 must attach under a live LWA parent; otherwise later alpha changes are ignored.
    let background_opacity = load_background_opacity(app);
    #[cfg(windows)]
    {
        apply_parent_lwa_now(app, background_opacity);
    }

    // Must run off the IPC/main thread: add_child blocks on run_on_main_thread + recv.
    let window = win.as_ref().window();
    window
        .add_child(builder, pos, size)
        .map_err(|err| err.to_string())?;

    apply_content_opacity(app, content_opacity);
    apply_background_opacity(app, background_opacity);
    // WebView2 init can race immediate LWA — reapply shortly after attach.
    schedule_reapply_opacities(app);
    emit_browsing(app, true);
    Ok(())
}

fn schedule_reapply_opacities(app: &AppHandle) {
    let handle = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(120));
        apply_content_opacity(&handle, load_content_opacity(&handle));
        apply_background_opacity(&handle, load_background_opacity(&handle));
    });
}

/// While browsing, WebView2 ignores post-attach LWA changes — recreate under current alpha.
#[cfg(windows)]
fn schedule_rebind_content_for_background_opacity(app: &AppHandle) {
    let gen = LWA_REBIND_GEN.fetch_add(1, Ordering::Relaxed) + 1;
    let handle = app.clone();
    thread::spawn(move || {
        thread::sleep(LWA_REBIND_DEBOUNCE);
        if LWA_REBIND_GEN.load(Ordering::Relaxed) != gen {
            return;
        }
        if let Err(err) = rebind_content_under_lwa(&handle) {
            eprintln!("[mini-window] rebind content for LWA failed: {err}");
        }
    });
}

fn close_content_webview(app: &AppHandle) {
    if let Some(webview) = app.get_webview(MINI_CONTENT_LABEL) {
        let _ = webview.hide();
        let _ = webview.set_size(LogicalSize::new(0.0, 0.0));
        let _ = webview.set_position(LogicalPosition::new(-10_000.0, -10_000.0));
        if let Err(err) = webview.close() {
            eprintln!("[mini-window] close content failed: {err}");
            let _ = webview.hide();
        }
    }
}

/// Tear down mini-content and recreate it after parent LWA is set (known-working path).
#[cfg(windows)]
fn rebind_content_under_lwa(app: &AppHandle) -> Result<(), String> {
    let Some(webview) = app.get_webview(MINI_CONTENT_LABEL) else {
        return Ok(());
    };
    let url = webview.url().map_err(|err| err.to_string())?;
    close_content_webview(app);
    // Wait until the label is gone so ensure_content takes the add_child path (not navigate).
    for _ in 0..40 {
        if app.get_webview(MINI_CONTENT_LABEL).is_none() {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    if app.get_webview(MINI_CONTENT_LABEL).is_some() {
        return Err("关闭网页视图超时".to_string());
    }
    apply_background_opacity(app, load_background_opacity(app));
    ensure_content_webview(app, url)
}

/// Open URL in the child webview. Always schedules work on a worker thread so
/// `add_child` does not deadlock the WebView2 main thread (invoke handlers).
pub fn navigate(app: &AppHandle, raw_url: String) -> Result<(), String> {
    let url = normalize_url(&raw_url)?;
    let need_create = app.get_webview_window(MINI_WINDOW_LABEL).is_none();
    if need_create {
        show(app);
    }

    let handle = app.clone();
    thread::spawn(move || {
        if need_create {
            for _ in 0..40 {
                thread::sleep(Duration::from_millis(50));
                if handle.get_webview_window(MINI_WINDOW_LABEL).is_some() {
                    break;
                }
            }
        } else {
            // Brief yield so the invoke returns before we touch WebView2.
            thread::sleep(Duration::from_millis(10));
        }

        if let Err(err) = ensure_content_webview(&handle, url) {
            eprintln!("[mini-window] navigate failed: {err}");
            CONTENT_VISIBLE.store(false, Ordering::Relaxed);
            emit_browsing(&handle, false);
        }
    });
    Ok(())
}

pub fn show_home(app: &AppHandle) -> Result<(), String> {
    let handle = app.clone();
    thread::spawn(move || {
        // Tear down child first so a zombie HWND cannot block home UI hits.
        clear_pending_restore_url();
        close_content_webview(&handle);
        CONTENT_VISIBLE.store(false, Ordering::Relaxed);
        emit_browsing(&handle, false);
    });
    Ok(())
}

pub fn go_back(app: &AppHandle) -> Result<(), String> {
    let Some(webview) = app.get_webview(MINI_CONTENT_LABEL) else {
        return Ok(());
    };
    webview
        .eval("window.history.back();")
        .map_err(|err| err.to_string())
}

pub fn go_forward(app: &AppHandle) -> Result<(), String> {
    let Some(webview) = app.get_webview(MINI_CONTENT_LABEL) else {
        return Ok(());
    };
    webview
        .eval("window.history.forward();")
        .map_err(|err| err.to_string())
}

fn clear_pending_restore_url() {
    if let Ok(mut guard) = PENDING_RESTORE_URL.lock() {
        *guard = None;
    }
}

fn take_pending_restore_url() -> Option<Url> {
    PENDING_RESTORE_URL.lock().ok().and_then(|mut guard| guard.take())
}

fn set_pending_restore_url(url: Url) {
    if let Ok(mut guard) = PENDING_RESTORE_URL.lock() {
        *guard = Some(url);
    }
}

fn has_pending_restore_url() -> bool {
    PENDING_RESTORE_URL
        .lock()
        .ok()
        .is_some_and(|guard| guard.is_some())
}

fn wait_content_closed(app: &AppHandle) -> bool {
    for _ in 0..40 {
        if app.get_webview(MINI_CONTENT_LABEL).is_none() {
            return true;
        }
        thread::sleep(Duration::from_millis(25));
    }
    app.get_webview(MINI_CONTENT_LABEL).is_none()
}

#[cfg(windows)]
fn remember_mini_hwnd(win: &WebviewWindow) {
    if let Ok(hwnd) = win.hwnd() {
        MINI_HWND.store(hwnd.0 as isize, Ordering::Relaxed);
    }
}

#[cfg(not(windows))]
fn remember_mini_hwnd(_win: &WebviewWindow) {}

fn log_webview_window_labels(app: &AppHandle, context: &str) {
    let mut labels: Vec<String> = app.webview_windows().into_keys().collect();
    labels.sort();
    eprintln!("[mini-window] {context} labels={labels:?} hwnd={}", MINI_HWND.load(Ordering::Relaxed));
}

/// Prefer live lookup; retry briefly (Tauri may lag after show/hide on Windows).
fn resolve_mini_window(app: &AppHandle) -> Option<WebviewWindow> {
    if let Some(win) = app.get_webview_window(MINI_WINDOW_LABEL) {
        remember_mini_hwnd(&win);
        return Some(win);
    }
    for _ in 0..10 {
        thread::sleep(Duration::from_millis(20));
        if let Some(win) = app.get_webview_window(MINI_WINDOW_LABEL) {
            remember_mini_hwnd(&win);
            return Some(win);
        }
    }
    None
}

#[cfg(windows)]
fn show_cached_hwnd() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IsWindow, IsWindowVisible, ShowWindow, SW_RESTORE, SW_SHOW,
    };

    let hwnd = MINI_HWND.load(Ordering::Relaxed);
    if hwnd == 0 {
        return false;
    }
    let hwnd = hwnd as windows_sys::Win32::Foundation::HWND;
    // SAFETY: hwnd was captured from a live mini window; validate before use.
    unsafe {
        if IsWindow(hwnd) == 0 {
            MINI_HWND.store(0, Ordering::Relaxed);
            return false;
        }
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = ShowWindow(hwnd, SW_SHOW);
        IsWindowVisible(hwnd) != 0
    }
}

#[cfg(not(windows))]
fn show_cached_hwnd() -> bool {
    false
}

/// Prefer live Tauri window; Windows-only HWND minimize when lookup fails.
/// Never SW_HIDE — that removes the taskbar button.
fn collapse_via_cached_hwnd(app: &AppHandle) {
    if let Some(win) = resolve_mini_window(app) {
        collapse_mini_window(app, &win);
        return;
    }

    eprintln!(
        "[mini-window] collapse: resolve failed hwnd={}",
        MINI_HWND.load(Ordering::Relaxed)
    );

    COLLAPSING.store(true, Ordering::SeqCst);

    if let Some(webview) = app.get_webview(MINI_CONTENT_LABEL) {
        match webview.url() {
            Ok(url) => {
                eprintln!("[mini-window] collapse save url={url}");
                set_pending_restore_url(url);
            }
            Err(err) => eprintln!("[mini-window] collapse url read failed: {err}"),
        }
        close_content_webview(app);
        if !wait_content_closed(app) {
            eprintln!("[mini-window] collapse: content close timed out");
        }
        CONTENT_VISIBLE.store(false, Ordering::Relaxed);
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            IsIconic, IsWindow, ShowWindow, SW_FORCEMINIMIZE, SW_MINIMIZE,
        };

        let hwnd_val = MINI_HWND.load(Ordering::Relaxed);
        if hwnd_val == 0 {
            COLLAPSING.store(false, Ordering::SeqCst);
            return;
        }
        let hwnd = hwnd_val as windows_sys::Win32::Foundation::HWND;
        // SAFETY: hwnd cached from mini window; IsWindow checked below.
        unsafe {
            if IsWindow(hwnd) == 0 {
                eprintln!("[mini-window] cached hwnd invalid");
                MINI_HWND.store(0, Ordering::Relaxed);
                COLLAPSING.store(false, Ordering::SeqCst);
                return;
            }
            let _ = ShowWindow(hwnd, SW_MINIMIZE);
            thread::sleep(Duration::from_millis(40));
            if IsIconic(hwnd) == 0 {
                let _ = ShowWindow(hwnd, SW_FORCEMINIMIZE);
                thread::sleep(Duration::from_millis(40));
            }
            eprintln!(
                "[mini-window] cached hwnd after collapse iconic={}",
                IsIconic(hwnd) != 0
            );
        }
    }

    #[cfg(not(windows))]
    {
        if let Some(win) = resolve_mini_window(app) {
            if let Err(err) = win.minimize() {
                eprintln!("[mini-window] collapse retry minimize failed: {err}");
            }
        } else {
            eprintln!("[mini-window] collapse: still no window after retry (no HWND fallback)");
        }
    }

    COLLAPSING.store(false, Ordering::SeqCst);
}

/// Run collapse off the invoke/UI thread (close + add_child paths can deadlock).
/// `win` must be resolved on the caller thread — lookup after spawn often returns None.
fn schedule_collapse(app: &AppHandle, win: WebviewWindow) {
    remember_mini_hwnd(&win);
    let handle = app.clone();
    thread::spawn(move || {
        collapse_mini_window(&handle, &win);
    });
}

fn schedule_restore_pending_content(app: &AppHandle) {
    if COLLAPSING.load(Ordering::Relaxed) {
        return;
    }
    let handle = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(30));
        if COLLAPSING.load(Ordering::Relaxed) {
            return;
        }
        restore_pending_or_existing_content(&handle);
    });
}

fn restore_pending_or_existing_content(app: &AppHandle) {
    let Some(win) = app.get_webview_window(MINI_WINDOW_LABEL) else {
        return;
    };
    if win.is_minimized().unwrap_or(false) || !win.is_visible().unwrap_or(true) {
        return;
    }

    if let Some(url) = take_pending_restore_url() {
        eprintln!("[mini-window] restoring content url={url}");
        if let Err(err) = ensure_content_webview(app, url.clone()) {
            eprintln!("[mini-window] restore content failed: {err}");
            set_pending_restore_url(url);
        }
        return;
    }

    if let Some(webview) = app.get_webview(MINI_CONTENT_LABEL) {
        let _ = webview.show();
        layout_content(app, &win);
    }
}

/// Minimize via Win32 only — never SW_HIDE (keeps the taskbar button).
#[cfg(windows)]
fn minimize_parent_hwnd(win: &WebviewWindow) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IsIconic, IsWindow, ShowWindow, SW_FORCEMINIMIZE, SW_MINIMIZE,
    };

    let Ok(hwnd) = win.hwnd() else {
        return false;
    };
    let hwnd = hwnd.0 as windows_sys::Win32::Foundation::HWND;
    // SAFETY: hwnd belongs to the live mini window.
    unsafe {
        if IsWindow(hwnd) == 0 {
            return false;
        }
        let _ = ShowWindow(hwnd, SW_MINIMIZE);
        if IsIconic(hwnd) == 0 {
            let _ = ShowWindow(hwnd, SW_FORCEMINIMIZE);
        }
        IsIconic(hwnd) != 0
    }
}

#[cfg(not(windows))]
fn minimize_parent_hwnd(_win: &WebviewWindow) -> bool {
    false
}

/// Tear down WebView2 first (it blocks parent minimize), then minimize only — no hide.
fn collapse_mini_window(app: &AppHandle, win: &WebviewWindow) {
    remember_mini_hwnd(win);
    eprintln!(
        "[mini-window] collapse begin content_visible={} has_webview={} minimized={} visible={}",
        CONTENT_VISIBLE.load(Ordering::Relaxed),
        app.get_webview(MINI_CONTENT_LABEL).is_some(),
        win.is_minimized().unwrap_or(false),
        win.is_visible().unwrap_or(false),
    );

    COLLAPSING.store(true, Ordering::SeqCst);

    if let Some(webview) = app.get_webview(MINI_CONTENT_LABEL) {
        match webview.url() {
            Ok(url) => {
                eprintln!("[mini-window] collapse save url={url}");
                set_pending_restore_url(url);
            }
            Err(err) => eprintln!("[mini-window] collapse url read failed: {err}"),
        }
        close_content_webview(app);
        if !wait_content_closed(app) {
            eprintln!("[mini-window] collapse: content close timed out");
        }
        CONTENT_VISIBLE.store(false, Ordering::Relaxed);
        // Keep frontend chrome in browsing mode until restore recreates content.
    }

    // Without the child, minimize usually works (same as home). Keep taskbar button.
    if let Err(err) = win.minimize() {
        eprintln!("[mini-window] minimize after content close: {err}");
    }
    thread::sleep(Duration::from_millis(40));
    let mut minimized = win.is_minimized().unwrap_or(false);
    if !minimized {
        minimized = minimize_parent_hwnd(win);
        thread::sleep(Duration::from_millis(40));
        if !minimized {
            minimized = win.is_minimized().unwrap_or(false);
        }
    }
    eprintln!("[mini-window] after minimize minimized={minimized}");

    COLLAPSING.store(false, Ordering::SeqCst);
    eprintln!("[mini-window] collapse end");
}

pub fn minimize(app: &AppHandle) {
    eprintln!("[mini-window] minimize command");
    hide_opacity_bubble(app);
    // Manual collapse must not be restored by the auto-hide hover loop.
    AUTO_HIDE_MINIMIZED.store(false, Ordering::Relaxed);
    clear_last_bounds();
    if let Some(win) = resolve_mini_window(app) {
        schedule_collapse(app, win);
        return;
    }
    log_webview_window_labels(app, "minimize: window missing on command thread");
    collapse_via_cached_hwnd(app);
}

pub fn close(app: &AppHandle) {
    hide_opacity_bubble(app);
    clear_last_bounds();
    DRAGGING.store(false, Ordering::Relaxed);
    AUTO_HIDE_MINIMIZED.store(false, Ordering::Relaxed);
    COLLAPSING.store(false, Ordering::Relaxed);
    clear_pending_restore_url();
    if let Some(webview) = app.get_webview(MINI_CONTENT_LABEL) {
        let _ = webview.close();
    }
    if let Some(opacity) = app.get_webview_window(MINI_OPACITY_LABEL) {
        let _ = opacity.close();
    }
    if let Some(win) = app.get_webview_window(MINI_WINDOW_LABEL) {
        let _ = win.close();
    }
    CONTENT_VISIBLE.store(false, Ordering::Relaxed);
    MINI_HWND.store(0, Ordering::Relaxed);
}

/// Non-blocking from invoke handlers (Windows deadlocks if build runs on the command thread).
pub fn show_opacity_bubble(app: &AppHandle, x: i32, y: i32) {
    let handle = app.clone();
    thread::spawn(move || {
        let app = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            show_opacity_bubble_on_main(&app, x, y);
        });
    });
}

fn show_opacity_bubble_on_main(app: &AppHandle, x: i32, y: i32) {
    let target = windows::clamp_popup_origin_public(
        app,
        x,
        y,
        OPACITY_BUBBLE_WIDTH,
        OPACITY_BUBBLE_HEIGHT,
        0,
    );
    let prefs = current_prefs(app);

    if let Some(win) = app.get_webview_window(MINI_OPACITY_LABEL) {
        let _ = win.set_size(LogicalSize::new(
            OPACITY_BUBBLE_WIDTH,
            OPACITY_BUBBLE_HEIGHT,
        ));
        let _ = win.set_position(PhysicalPosition::new(target.0, target.1));
        let _ = win.set_always_on_top(true);
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.emit("mini-window://prefs", &prefs);
        emit_opacity_opened(app);
        crate::mouse_trail::schedule_raise_overlays(app);
        raise_opacity_bubble(app);
        return;
    }

    let result = WebviewWindowBuilder::new(
        app,
        MINI_OPACITY_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("透明度")
    .inner_size(OPACITY_BUBBLE_WIDTH, OPACITY_BUBBLE_HEIGHT)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .background_color(Color(0, 0, 0, 0))
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(true)
    .visible(false)
    .build();

    match result {
        Ok(win) => {
            let _ = win.set_position(PhysicalPosition::new(target.0, target.1));
            let _ = win.set_always_on_top(true);
            let _ = win.show();
            let handle = app.clone();
            let prefs = prefs.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(80));
                if let Some(win) = handle.get_webview_window(MINI_OPACITY_LABEL) {
                    let _ = win.emit("mini-window://prefs", &prefs);
                }
            });
            emit_opacity_opened(app);
            crate::mouse_trail::schedule_raise_overlays(app);
            raise_opacity_bubble(app);
        }
        Err(err) => {
            eprintln!("[mini-opacity] create failed: {err}");
            emit_opacity_failed(app);
        }
    }
}

/// Keep the opacity bubble above the mini window without stealing focus.
fn raise_opacity_bubble(app: &AppHandle) {
    let Some(win) = app.get_webview_window(MINI_OPACITY_LABEL) else {
        return;
    };
    if !win.is_visible().unwrap_or(false) {
        return;
    }
    let _ = win.set_always_on_top(true);

    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        };
        if let Ok(hwnd) = win.hwnd() {
            let hwnd = hwnd.0 as windows_sys::Win32::Foundation::HWND;
            // SAFETY: HWND belongs to the live opacity bubble window.
            unsafe {
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }
}

fn emit_opacity_opened(app: &AppHandle) {
    if let Some(mini) = app.get_webview_window(MINI_WINDOW_LABEL) {
        let _ = mini.emit("mini-window://opacity-opened", ());
    }
}

fn emit_opacity_failed(app: &AppHandle) {
    if let Some(mini) = app.get_webview_window(MINI_WINDOW_LABEL) {
        let _ = mini.emit("mini-window://opacity-failed", ());
    }
}

pub fn hide_opacity_bubble(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(MINI_OPACITY_LABEL) {
        let _ = win.hide();
    }
    if let Some(mini) = app.get_webview_window(MINI_WINDOW_LABEL) {
        let _ = mini.emit("mini-window://opacity-closed", ());
    }
}

pub fn set_auto_hide(app: &AppHandle, enabled: bool) {
    save_bool(app, AUTO_HIDE_KEY, enabled);
    let state = app.state::<AppState>();
    state
        .mini_window_auto_hide
        .store(enabled, Ordering::Relaxed);
    if !enabled {
        clear_last_bounds();
        DRAGGING.store(false, Ordering::Relaxed);
    }
    ensure_auto_hide_loop(app);
    emit_prefs(app);
}

pub fn set_always_on_top(app: &AppHandle, enabled: bool) {
    save_bool(app, ALWAYS_ON_TOP_KEY, enabled);
    if let Some(win) = app.get_webview_window(MINI_WINDOW_LABEL) {
        let _ = win.set_always_on_top(enabled);
    }
    // Keep mouse-trail overlays above the mini window when it claims TOPMOST.
    if enabled {
        crate::mouse_trail::schedule_raise_overlays(app);
    }
    emit_prefs(app);
}

pub fn set_content_opacity(app: &AppHandle, opacity: f64) {
    let opacity = clamp_opacity(opacity);
    save_opacity(app, CONTENT_OPACITY_KEY, opacity);
    apply_content_opacity(app, opacity);
    emit_prefs(app);
}

pub fn set_background_opacity(app: &AppHandle, opacity: f64) {
    let opacity = clamp_opacity(opacity);
    save_opacity(app, BACKGROUND_OPACITY_KEY, opacity);
    apply_background_opacity(app, opacity);
    // Post-attach LWA is ignored by WebView2 — recreate content under the new alpha.
    // macOS/Linux only adjust native alpha; do not tear down mini-content.
    #[cfg(windows)]
    if CONTENT_VISIBLE.load(Ordering::Relaxed) {
        schedule_rebind_content_for_background_opacity(app);
    }
    emit_prefs(app);
}

fn emit_prefs(app: &AppHandle) {
    let prefs = current_prefs(app);
    if let Some(win) = app.get_webview_window(MINI_WINDOW_LABEL) {
        let _ = win.emit("mini-window://prefs", &prefs);
    }
    if let Some(win) = app.get_webview_window(MINI_OPACITY_LABEL) {
        let _ = win.emit("mini-window://prefs", &prefs);
    }
}

/// Content-slider fade only (CSS). Do not multiply with background.
fn content_opacity_script(opacity: f64) -> String {
    let o = clamp_opacity(opacity);
    format!(
        r#"(function(){{try{{document.documentElement.style.opacity='{o}';if(document.body)document.body.style.opacity='{o}';}}catch(e){{}}}})();"#,
        o = o
    )
}

fn apply_content_opacity(app: &AppHandle, content_opacity: f64) {
    let content_opacity = clamp_opacity(content_opacity);
    if let Some(webview) = app.get_webview(MINI_CONTENT_LABEL) {
        let _ = webview.eval(content_opacity_script(content_opacity));
    }
    if let Some(win) = app.get_webview_window(MINI_WINDOW_LABEL) {
        let _ = win.emit("mini-window://content-opacity", content_opacity);
    }
}

/// Overall window fade: Windows LWA_ALPHA, macOS NSWindow alpha, Linux GTK opacity.
fn apply_background_opacity(app: &AppHandle, opacity: f64) {
    let opacity = clamp_opacity(opacity);
    if let Some(win) = app.get_webview_window(MINI_WINDOW_LABEL) {
        let _ = win.emit("mini-window://background-opacity", opacity);
    }
    #[cfg(windows)]
    {
        // Immediate apply (safe from create_window on the UI thread — no blocking wait).
        apply_parent_lwa_now(app, opacity);
        // Second pass on the UI thread when invoked from a worker / IPC thread.
        let handle = app.clone();
        let _ = app.clone().run_on_main_thread(move || {
            apply_parent_lwa_now(&handle, opacity);
        });
    }
    #[cfg(not(windows))]
    {
        // GTK / AppKit must be touched on the UI thread.
        let handle = app.clone();
        let _ = app.clone().run_on_main_thread(move || {
            apply_native_window_opacity_now(&handle, opacity);
        });
    }
}

#[cfg(not(windows))]
fn apply_native_window_opacity_now(app: &AppHandle, opacity: f64) {
    let Some(win) = app.get_webview_window(MINI_WINDOW_LABEL) else {
        return;
    };
    if let Err(err) = apply_native_window_opacity(&win, opacity) {
        eprintln!("[mini-window] native window opacity failed: {err}");
    }
}

#[cfg(target_os = "macos")]
fn apply_native_window_opacity(win: &WebviewWindow, opacity: f64) -> Result<(), String> {
    use objc2_app_kit::NSWindow;

    let opacity = clamp_opacity(opacity);
    let ptr = win.ns_window().map_err(|err| err.to_string())?;
    // SAFETY: ns_window is the live NSWindow for this WebviewWindow.
    let ns_window = unsafe { &*ptr.cast::<NSWindow>() };
    ns_window.setAlphaValue(opacity as f32);
    Ok(())
}

#[cfg(target_os = "linux")]
fn apply_native_window_opacity(win: &WebviewWindow, opacity: f64) -> Result<(), String> {
    use gtk::prelude::WidgetExt;

    let opacity = clamp_opacity(opacity);
    let gtk_win = win.gtk_window().map_err(|err| err.to_string())?;
    // Needs a compositor (Wayland or compositing X11); silent no-op otherwise.
    gtk_win.set_opacity(opacity);
    Ok(())
}

#[cfg(all(not(windows), not(target_os = "macos"), not(target_os = "linux")))]
fn apply_native_window_opacity(_win: &WebviewWindow, _opacity: f64) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
fn apply_parent_lwa_now(app: &AppHandle, opacity: f64) {
    let Some(win) = app.get_webview_window(MINI_WINDOW_LABEL) else {
        return;
    };
    if let Err(err) = set_parent_hwnd_alpha(&win, opacity) {
        eprintln!("[mini-window] parent LWA failed: {err}");
    }
}

/// Win11 rounded corners for frameless non-transparent mini window.
#[cfg(windows)]
fn apply_rounded_corners(win: &WebviewWindow) {
    use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;

    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    const DWMWCP_ROUND: u32 = 2;

    let Ok(hwnd) = win.hwnd() else {
        return;
    };
    let hwnd = hwnd.0 as windows_sys::Win32::Foundation::HWND;
    let mut pref = DWMWCP_ROUND;
    // SAFETY: hwnd belongs to the live mini window.
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &mut pref as *mut u32 as *mut _,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

#[cfg(windows)]
fn set_parent_hwnd_alpha(win: &WebviewWindow, opacity: f64) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW, GWL_EXSTYLE, LWA_ALPHA,
        WS_EX_LAYERED,
    };

    let hwnd = win.hwnd().map_err(|err| err.to_string())?;
    let hwnd = hwnd.0 as windows_sys::Win32::Foundation::HWND;
    let opacity = clamp_opacity(opacity);
    // Cap at 254 so WebView2 attaches under a live layered parent (255 often sticks opaque).
    let alpha = if opacity >= 0.99 {
        254u8
    } else {
        (opacity * 255.0).round().clamp(51.0, 254.0) as u8
    };

    // SAFETY: hwnd is the live mini-window top-level HWND (non-Tauri-transparent).
    unsafe {
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex | WS_EX_LAYERED as i32);
        let ok = SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA);
        if ok == 0 {
            return Err("设置窗口透明度失败".to_string());
        }
    }
    Ok(())
}

fn input_host_token(raw: &str) -> &str {
    let without_scheme = raw
        .strip_prefix("http://")
        .or_else(|| raw.strip_prefix("https://"))
        .or_else(|| raw.strip_prefix("HTTP://"))
        .or_else(|| raw.strip_prefix("HTTPS://"))
        .unwrap_or(raw);
    without_scheme
        .split(['/', ':', '?', '#'])
        .next()
        .unwrap_or(without_scheme)
}

fn host_looks_like_web(host: &str, input_host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") || host.to_ascii_lowercase().ends_with(".localhost")
    {
        return true;
    }
    // Chromium/url map bare decimals to IPv4 (15646 → 0.0.61.14). Only allow
    // dotted IPv4 that the user actually typed.
    let dotted_v4 = |s: &str| -> bool {
        s.parse::<std::net::Ipv4Addr>().is_ok() && s.contains('.')
    };
    if dotted_v4(host) {
        return dotted_v4(input_host);
    }
    if host.parse::<std::net::Ipv6Addr>().is_ok() {
        return true;
    }
    // Require a dot so bare tokens like `4532543` are rejected.
    host.contains('.') && !host.starts_with('.') && !host.ends_with('.')
}

fn normalize_url(raw: &str) -> Result<Url, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("请输入网址".to_string());
    }
    let input_host = input_host_token(trimmed);
    // Reject integer-form hosts before Url::parse rewrites them to dotted IPv4.
    if !input_host.is_empty() && input_host.bytes().all(|b| b.is_ascii_digit()) {
        return Err("请输入有效网址".to_string());
    }
    let candidate = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    let url = Url::parse(&candidate).map_err(|_| "请输入有效网址".to_string())?;
    match url.scheme() {
        "http" | "https" => {}
        _ => return Err("请输入有效网址".to_string()),
    }
    let Some(host) = url.host_str() else {
        return Err("请输入有效网址".to_string());
    };
    if !host_looks_like_web(host, input_host) {
        return Err("请输入有效网址".to_string());
    }
    Ok(url)
}

fn ensure_auto_hide_loop(app: &AppHandle) {
    let state = app.state::<AppState>();
    state
        .mini_window_auto_hide
        .store(load_auto_hide(app), Ordering::Relaxed);

    if AUTO_HIDE_LOOP_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    let handle = app.clone();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(50));
        let state = handle.state::<AppState>();
        if !state.mini_window_auto_hide.load(Ordering::Relaxed) {
            continue;
        }

        let win = handle.get_webview_window(MINI_WINDOW_LABEL);
        if let Some(ref win) = win {
            remember_mini_hwnd(win);
            // Minimized outer_position is often off-screen (~-32000); never overwrite
            // the pre-collapse rect used for hover-restore.
            if !win.is_minimized().unwrap_or(false) && win.is_visible().unwrap_or(false) {
                save_last_bounds(win);
            }
        } else {
            // Tauri lookup failed — still try last-bounds + cached HWND path.
            let now = now_ms();
            let last = AUTO_HIDE_MISSING_LOG_MS.load(Ordering::Relaxed);
            if now.saturating_sub(last) > 2000 {
                AUTO_HIDE_MISSING_LOG_MS.store(now, Ordering::Relaxed);
                log_webview_window_labels(&handle, "auto-hide: window missing");
            }
            if COLLAPSING.load(Ordering::Relaxed) || drag_suppressed() {
                continue;
            }
            if windows::cursor_pos_public().is_none() {
                continue;
            }
            if cursor_in_last_bounds() {
                if AUTO_HIDE_MINIMIZED.load(Ordering::Relaxed) {
                    let _ = show_cached_hwnd();
                    AUTO_HIDE_MINIMIZED.store(false, Ordering::Relaxed);
                    schedule_restore_pending_content(&handle);
                }
                continue;
            }
            // Cursor left last known bounds while Tauri has no window record.
            if !AUTO_HIDE_MINIMIZED.load(Ordering::Relaxed) {
                hide_opacity_bubble(&handle);
                AUTO_HIDE_MINIMIZED.store(true, Ordering::Relaxed);
                collapse_via_cached_hwnd(&handle);
            }
            continue;
        }
        let win = win.expect("checked Some above");

        let minimized = win.is_minimized().unwrap_or(false);
        let visible = win.is_visible().unwrap_or(false);

        // Collapsed via minimize/hide: restore on hover into last bounds.
        if minimized || !visible {
            if AUTO_HIDE_MINIMIZED.load(Ordering::Relaxed) && cursor_in_last_bounds() {
                focus_existing(&handle, &win);
            }
            continue;
        }

        if COLLAPSING.load(Ordering::Relaxed) {
            continue;
        }

        // Explicit title-bar drag or brief post-Moved suppress — never hide mid-drag.
        if drag_suppressed() {
            continue;
        }

        // Unknown cursor: skip this tick (do not treat as "inside" forever).
        if windows::cursor_pos_public().is_none() {
            continue;
        }

        if cursor_inside_window(&win) {
            continue;
        }

        // Immediate hide for 摸鱼.
        hide_opacity_bubble(&handle);
        save_last_bounds(&win);
        AUTO_HIDE_MINIMIZED.store(true, Ordering::Relaxed);
        collapse_mini_window(&handle, &win);
    });
}

fn cursor_inside_window(win: &WebviewWindow) -> bool {
    let Ok(pos) = win.outer_position() else {
        return true;
    };
    let Ok(size) = win.outer_size() else {
        return true;
    };
    cursor_in_rect(pos.x, pos.y, size.width as i32, size.height as i32)
}

/// Init auto-hide flag from store at app startup.
pub fn init_from_store(app: &AppHandle) {
    let state = app.state::<AppState>();
    state
        .mini_window_auto_hide
        .store(load_auto_hide(app), Ordering::Relaxed);
}
