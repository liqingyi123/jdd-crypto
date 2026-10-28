use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tauri::{
    window::Color, AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

#[derive(Clone, Serialize)]
pub struct CryptoHint {
    pub text: String,
    pub mode: String,
}

static PENDING_CRYPTO_BUBBLE: Mutex<Option<CryptoHint>> = Mutex::new(None);
/// Center `main` only on the first show in this process; later reopen keeps user position.
static MAIN_CENTERED_ONCE: AtomicBool = AtomicBool::new(false);

struct FeatureSpec {
    label: &'static str,
    title: &'static str,
    width: f64,
    height: f64,
    min_width: f64,
    min_height: f64,
}

fn feature_spec(label: &str) -> Option<FeatureSpec> {
    match label {
        "settings" => Some(FeatureSpec {
            label: "settings",
            title: "功能设置",
            width: 720.0,
            height: 560.0,
            min_width: 560.0,
            min_height: 420.0,
        }),
        "hosts" => Some(FeatureSpec {
            label: "hosts",
            title: "Host管理",
            width: 880.0,
            height: 640.0,
            min_width: 720.0,
            min_height: 480.0,
        }),
        "ai" => Some(FeatureSpec {
            label: "ai",
            title: "AI相关",
            width: 720.0,
            height: 640.0,
            min_width: 560.0,
            min_height: 480.0,
        }),
        "essentials" => Some(FeatureSpec {
            label: "essentials",
            title: "装机必备",
            width: 880.0,
            height: 720.0,
            min_width: 720.0,
            min_height: 560.0,
        }),
        "about" => Some(FeatureSpec {
            label: "about",
            title: "关于",
            width: 480.0,
            height: 420.0,
            min_width: 400.0,
            min_height: 360.0,
        }),
        _ => None,
    }
}

fn focus_window(app: &AppHandle, win: &WebviewWindow) {
    let _ = win.unminimize();
    let _ = win.show();
    let _ = win.set_focus();
    // Re-raise trail after focus so overlays stay above feature/main windows.
    crate::mouse_trail::schedule_raise_overlays(app);
}

/// Center a window on the monitor under the cursor (fallback: primary).
pub fn center_on_cursor_monitor(app: &AppHandle, win: &WebviewWindow) {
    let (cursor_x, cursor_y) = cursor_pos().unwrap_or((0, 0));
    let monitors = app.available_monitors().unwrap_or_default();
    let monitor = monitors
        .iter()
        .find(|m| {
            let work = m.work_area();
            let px = work.position.x;
            let py = work.position.y;
            let pw = work.size.width as i32;
            let ph = work.size.height as i32;
            cursor_x >= px && cursor_x < px + pw && cursor_y >= py && cursor_y < py + ph
        })
        .or_else(|| monitors.first());
    let Some(monitor) = monitor else {
        return;
    };
    let work = monitor.work_area();
    let outer = win.outer_size().ok();
    let (win_w, win_h) = match outer {
        Some(size) => (size.width as i32, size.height as i32),
        None => {
            let scale = monitor.scale_factor();
            let inner = win.inner_size().ok();
            match inner {
                Some(size) => (size.width as i32, size.height as i32),
                None => (
                    (720.0 * scale).round() as i32,
                    (560.0 * scale).round() as i32,
                ),
            }
        }
    };
    let max_w = work.size.width as i32;
    let max_h = work.size.height as i32;
    let win_w = win_w.min(max_w.saturating_sub(24));
    let win_h = win_h.min(max_h.saturating_sub(24));
    let x = work.position.x + (max_w - win_w) / 2;
    let y = work.position.y + (max_h - win_h) / 2;
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

fn should_recenter_on_show(label: &str) -> bool {
    feature_spec(label).is_some()
}

pub fn show_main(app: &AppHandle, hint: Option<CryptoHint>) {
    if let Some(win) = app.get_webview_window("main") {
        if !MAIN_CENTERED_ONCE.swap(true, Ordering::Relaxed) {
            center_on_cursor_monitor(app, &win);
        }
        focus_window(app, &win);
        if let Some(hint) = hint {
            let _ = win.emit("app://crypto-payload", &hint);
            let handle = app.clone();
            let delayed = hint.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(250));
                if let Some(win) = handle.get_webview_window("main") {
                    let _ = win.emit("app://crypto-payload", &delayed);
                }
            });
        }
    }
}

pub fn show_feature(app: &AppHandle, label: &str) {
    // Feature windows are created lazily (not declared in tauri.conf.json) so
    // cold start only pays for badge + main. Closing destroys the webview to
    // reclaim the process; re-open creates a fresh one.
    if let Some(win) = app.get_webview_window(label) {
        if should_recenter_on_show(label) {
            center_on_cursor_monitor(app, &win);
        }
        focus_window(app, &win);
        return;
    }

    if feature_spec(label).is_none() {
        return;
    }

    // Windows WebView2 deadlocks if WebviewWindowBuilder::build runs inside a
    // sync command or tray/menu event handler. Escape via a worker thread first
    // (same pattern as overlay_toast::schedule_show).
    let handle = app.clone();
    let label = label.to_string();
    thread::spawn(move || {
        let app = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            create_feature_window(&app, &label);
        });
    });
}

fn create_feature_window(app: &AppHandle, label: &str) {
    if let Some(win) = app.get_webview_window(label) {
        if should_recenter_on_show(label) {
            center_on_cursor_monitor(app, &win);
        }
        focus_window(app, &win);
        return;
    }

    let Some(spec) = feature_spec(label) else {
        return;
    };

    let result = WebviewWindowBuilder::new(app, spec.label, WebviewUrl::App("index.html".into()))
        .title(spec.title)
        .inner_size(spec.width, spec.height)
        .min_inner_size(spec.min_width, spec.min_height)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .visible(false)
        .build();

    if let Ok(win) = result {
        if should_recenter_on_show(label) {
            center_on_cursor_monitor(app, &win);
        }
        let _ = win.show();
        let _ = win.set_focus();
        // Allow real close() so WebView process is reclaimed; only raise trail on blur.
        bind_raise_on_blur(&win);
        crate::mouse_trail::schedule_raise_overlays(app);
    }
}

/// Map the window close button to hide instead of destroy.
/// Used for prebuilt windows (crypto-bubble / compare-*) that must stay warm.
pub fn bind_close_to_hide(win: &WebviewWindow) {
    let hidden = win.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = hidden.hide();
        }
    });
}

/// Re-raise mouse-trail overlays when the window loses focus.
/// Does not intercept close — the webview is destroyed and the process reclaimed.
pub fn bind_raise_on_blur(win: &WebviewWindow) {
    let app = win.app_handle().clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(false) = event {
            crate::mouse_trail::schedule_raise_overlays(&app);
        }
    });
}

/// Close-to-hide plus re-raise mouse-trail when the window loses focus.
/// Kept for `main` so tray / single-instance reopen stays snappy.
pub fn bind_interactive_window(win: &WebviewWindow) {
    let hidden = win.clone();
    let app = win.app_handle().clone();
    win.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = hidden.hide();
        }
        tauri::WindowEvent::Focused(false) => {
            crate::mouse_trail::schedule_raise_overlays(&app);
        }
        _ => {}
    });
}

const SETTINGS_STORE: &str = "settings.json";
const BADGE_SIZE_KEY: &str = "badgeSize";
const BADGE_POS_X_KEY: &str = "badgePosX";
const BADGE_POS_Y_KEY: &str = "badgePosY";
const THEME_KEY: &str = "themePreference";
const DEFAULT_THEME_PREF: &str = "system";
// Keep in sync with `src/constants/badge.ts` (`BADGE_EDGE_MARGIN`).
const BADGE_EDGE_MARGIN: f64 = 50.0;

pub const CLIPBOARD_PROMPT_LABEL: &str = "clipboard-prompt";
pub const CRYPTO_BUBBLE_LABEL: &str = "crypto-bubble";
const CRYPTO_BUBBLE_WIDTH: f64 = 480.0;
const CRYPTO_BUBBLE_HEIGHT: f64 = 360.0;
const SHORT_TEXT_BUBBLE_LIMIT: usize = 1100;
#[cfg(windows)]
const POPUP_CURSOR_OFFSET: i32 = 16;

static SUPPRESS_BADGE_POS_SAVE: AtomicBool = AtomicBool::new(false);
static BADGE_MOVE_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn load_badge_size(app: &AppHandle) -> u32 {
    use crate::state::normalize_badge_size;
    use crate::state::DEFAULT_BADGE_SIZE;
    use tauri_plugin_store::StoreExt;

    let Ok(store) = app.store(SETTINGS_STORE) else {
        return DEFAULT_BADGE_SIZE;
    };
    store
        .get(BADGE_SIZE_KEY)
        .and_then(|value| value.as_u64())
        .map(|value| normalize_badge_size(value as u32))
        .unwrap_or(DEFAULT_BADGE_SIZE)
}

pub fn save_badge_size(app: &AppHandle, size: u32) {
    use tauri_plugin_store::StoreExt;

    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(BADGE_SIZE_KEY, serde_json::json!(size));
        let _ = store.save();
    }
}

pub fn load_badge_position(app: &AppHandle) -> Option<(i32, i32)> {
    use tauri_plugin_store::StoreExt;

    let store = app.store(SETTINGS_STORE).ok()?;
    let x = store.get(BADGE_POS_X_KEY)?.as_i64()? as i32;
    let y = store.get(BADGE_POS_Y_KEY)?.as_i64()? as i32;
    Some((x, y))
}

pub fn save_badge_position(app: &AppHandle, x: i32, y: i32) {
    use tauri_plugin_store::StoreExt;

    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(BADGE_POS_X_KEY, serde_json::json!(x));
        store.set(BADGE_POS_Y_KEY, serde_json::json!(y));
        let _ = store.save();
    }
}

fn set_badge_position(win: &WebviewWindow, x: i32, y: i32) {
    SUPPRESS_BADGE_POS_SAVE.store(true, Ordering::Relaxed);
    let _ = win.set_position(PhysicalPosition::new(x, y));
    thread::spawn(|| {
        thread::sleep(Duration::from_millis(100));
        SUPPRESS_BADGE_POS_SAVE.store(false, Ordering::Relaxed);
    });
}

pub fn position_badge_on_startup(app: &AppHandle) {
    let Some(win) = app.get_webview_window("badge") else {
        return;
    };

    if let Some((x, y)) = load_badge_position(app) {
        set_badge_position(&win, x, y);
        ensure_badge_origin_in_work_area(app);
        return;
    }

    let monitor = win
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        return;
    };
    let area = monitor.work_area();
    let work = WorkArea {
        x: area.position.x,
        y: area.position.y,
        width: area.size.width as i32,
        height: area.size.height as i32,
    };
    place_badge_bottom_right(app, &win, &work, monitor.scale_factor());
}

pub fn watch_badge_position(app: &AppHandle) {
    let Some(win) = app.get_webview_window("badge") else {
        return;
    };

    let app_handle = app.clone();
    win.on_window_event(move |event| {
        if !matches!(event, tauri::WindowEvent::Moved(_)) {
            return;
        }
        if SUPPRESS_BADGE_POS_SAVE.swap(false, Ordering::Relaxed) {
            return;
        }

        let seq = BADGE_MOVE_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
        let app = app_handle.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            if BADGE_MOVE_SEQ.load(Ordering::Relaxed) != seq {
                return;
            }
            let Some(win) = app.get_webview_window("badge") else {
                return;
            };
            if let Ok(pos) = win.outer_position() {
                save_badge_position(&app, pos.x, pos.y);
            }
        });
    });
}

/// Move app windows that no longer intersect any monitor onto the nearest work area.
/// Skips mouse-trail / screensaver / spotlight overlays (managed by their own sync).
pub fn relocate_windows_to_visible_monitors(app: &AppHandle) {
    let work_areas = collect_work_areas(app);
    if work_areas.is_empty() {
        return;
    }

    for (label, win) in app.webview_windows() {
        if label.starts_with("mouse-trail-")
            || label.starts_with("screensaver-")
            || label.starts_with("spotlight-")
        {
            continue;
        }
        let Ok(pos) = win.outer_position() else {
            continue;
        };
        let Ok(size) = win.outer_size() else {
            continue;
        };
        let width = size.width as i32;
        let height = size.height as i32;
        if width <= 0 || height <= 0 {
            continue;
        }

        if work_areas.iter().any(|work| {
            rects_intersect(
                pos.x,
                pos.y,
                width,
                height,
                work.x,
                work.y,
                work.width,
                work.height,
            )
        }) {
            continue;
        }

        let cx = pos.x + width / 2;
        let cy = pos.y + height / 2;
        let Some(work) = nearest_work_area(cx, cy, &work_areas) else {
            continue;
        };
        let (nx, ny) = clamp_origin_to_work_area(pos.x, pos.y, width, height, work);
        if label == "badge" {
            set_badge_position(&win, nx, ny);
            save_badge_position(app, nx, ny);
        } else {
            let _ = win.set_position(PhysicalPosition::new(nx, ny));
        }
    }
}

#[derive(Clone, Copy)]
struct WorkArea {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

fn rects_intersect(ax: i32, ay: i32, aw: i32, ah: i32, bx: i32, by: i32, bw: i32, bh: i32) -> bool {
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

fn nearest_work_area(cx: i32, cy: i32, work_areas: &[WorkArea]) -> Option<&WorkArea> {
    let mut best: Option<&WorkArea> = None;
    let mut best_dist = i64::MAX;
    for work in work_areas {
        let left = work.x;
        let top = work.y;
        let right = left + work.width;
        let bottom = top + work.height;
        let nearest_x = cx.clamp(left, right.saturating_sub(1).max(left));
        let nearest_y = cy.clamp(top, bottom.saturating_sub(1).max(top));
        let dx = i64::from(cx - nearest_x);
        let dy = i64::from(cy - nearest_y);
        let dist = dx * dx + dy * dy;
        if dist < best_dist {
            best_dist = dist;
            best = Some(work);
        }
    }
    best
}

fn clamp_origin_to_work_area(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    work: &WorkArea,
) -> (i32, i32) {
    let left = work.x;
    let top = work.y;
    let right = left + work.width;
    let bottom = top + work.height;
    let max_x = (right - width).max(left);
    let max_y = (bottom - height).max(top);
    (x.clamp(left, max_x), y.clamp(top, max_y))
}

pub fn normalize_theme_pref(raw: &str) -> String {
    match raw {
        "system" | "light" | "dark" => raw.to_string(),
        _ => DEFAULT_THEME_PREF.to_string(),
    }
}

pub fn load_theme_pref(app: &AppHandle) -> String {
    use tauri_plugin_store::StoreExt;

    let Ok(store) = app.store(SETTINGS_STORE) else {
        return DEFAULT_THEME_PREF.to_string();
    };
    store
        .get(THEME_KEY)
        .and_then(|value| value.as_str().map(str::to_string))
        .map(|raw| normalize_theme_pref(&raw))
        .unwrap_or_else(|| DEFAULT_THEME_PREF.to_string())
}

pub fn save_theme_pref(app: &AppHandle, preference: &str) -> String {
    use tauri_plugin_store::StoreExt;

    let normalized = normalize_theme_pref(preference);
    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(THEME_KEY, serde_json::json!(normalized));
        let _ = store.save();
    }
    let _ = app.emit("app://theme-preference", &normalized);
    normalized
}

pub fn set_badge_size(app: &AppHandle, expanded: bool) {
    use std::sync::atomic::Ordering;

    use crate::state::AppState;

    let size = app.state::<AppState>().badge_size.load(Ordering::Relaxed);
    apply_badge_window_size(app, size, expanded);
}

pub fn apply_badge_window_size(app: &AppHandle, size: u32, expanded: bool) {
    if let Some(win) = app.get_webview_window("badge") {
        let _ = expanded;
        if size == crate::state::BADGE_HIDDEN_SIZE {
            let _ = win.hide();
            return;
        }
        let _ = win.show();
        let _ = win.set_size(tauri::LogicalSize::new(f64::from(size), f64::from(size)));
        let _ = win.emit("app://badge-size", size);
        let skip_ensure = app
            .try_state::<crate::state::AppState>()
            .is_some_and(|s| s.follow_blocks_resize());
        if !skip_ensure {
            ensure_badge_origin_in_work_area(app);
        }
    }
}

/// If badge is not fully inside any work area, reset to that screen's bottom-right (50×50 margin).
fn ensure_badge_origin_in_work_area(app: &AppHandle) {
    let Some(win) = app.get_webview_window("badge") else {
        return;
    };
    let Ok(pos) = win.outer_position() else {
        return;
    };
    let Ok(size) = win.outer_size() else {
        return;
    };
    let width = size.width as i32;
    let height = size.height as i32;
    if width <= 0 || height <= 0 {
        return;
    }

    let work_areas = collect_work_areas(app);
    if work_areas.is_empty() {
        return;
    }

    if work_areas
        .iter()
        .any(|work| rect_fully_inside(pos.x, pos.y, width, height, work))
    {
        return;
    }

    let work = work_areas
        .iter()
        .find(|work| {
            rects_intersect(
                pos.x,
                pos.y,
                width,
                height,
                work.x,
                work.y,
                work.width,
                work.height,
            )
        })
        .or_else(|| nearest_work_area(pos.x + width / 2, pos.y + height / 2, &work_areas));
    let Some(work) = work else {
        return;
    };
    let scale = scale_for_work_area(app, work);
    place_badge_bottom_right(app, &win, work, scale);
}

/// Place badge at work-area bottom-right with [`BADGE_EDGE_MARGIN`] (logical px × scale).
fn place_badge_bottom_right(app: &AppHandle, win: &WebviewWindow, work: &WorkArea, scale: f64) {
    let Ok(outer_size) = win.outer_size() else {
        return;
    };
    let margin = (BADGE_EDGE_MARGIN * scale).round() as i32;
    let x = work.x + work.width - outer_size.width as i32 - margin;
    let y = work.y + work.height - outer_size.height as i32 - margin;
    set_badge_position(win, x, y);
    save_badge_position(app, x, y);
}

fn rect_fully_inside(x: i32, y: i32, w: i32, h: i32, work: &WorkArea) -> bool {
    x >= work.x
        && y >= work.y
        && x + w <= work.x + work.width
        && y + h <= work.y + work.height
}

fn scale_for_work_area(app: &AppHandle, work: &WorkArea) -> f64 {
    for monitor in app.available_monitors().unwrap_or_default() {
        let area = monitor.work_area();
        if area.position.x == work.x
            && area.position.y == work.y
            && area.size.width as i32 == work.width
            && area.size.height as i32 == work.height
        {
            return monitor.scale_factor();
        }
    }
    1.0
}

fn collect_work_areas(app: &AppHandle) -> Vec<WorkArea> {
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let work = m.work_area();
            WorkArea {
                x: work.position.x,
                y: work.position.y,
                width: work.size.width as i32,
                height: work.size.height as i32,
            }
        })
        .collect()
}

pub fn hide_clipboard_prompt(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(CLIPBOARD_PROMPT_LABEL) {
        let _ = win.hide();
    }
}

pub fn is_short_bubble_text(text: &str) -> bool {
    // 不超过 1100 字用气泡；超出打开主窗口
    text.chars().count() <= SHORT_TEXT_BUBBLE_LIMIT
}

fn emit_crypto_bubble_hint(app: &AppHandle, hint: &CryptoHint) {
    // Single emit; cold-start uses PENDING + get_crypto_bubble_payload on mount.
    let _ = app.emit("app://crypto-bubble", hint);
}

/// Show (or create) a near-cursor bubble that displays short crypto results.
/// `silent_json` only emits the payload; the frontend reveals after decrypt+JSON succeed.
pub fn show_crypto_bubble(app: &AppHandle, hint: CryptoHint) {
    if let Ok(mut pending) = PENDING_CRYPTO_BUBBLE.lock() {
        *pending = Some(hint.clone());
    }

    let silent = hint.mode == "silent_json";
    let (cursor_x, cursor_y) = cursor_pos().unwrap_or((0, 0));
    let target = clamp_popup_origin(
        app,
        cursor_x,
        cursor_y,
        CRYPTO_BUBBLE_WIDTH,
        CRYPTO_BUBBLE_HEIGHT,
    );

    // Prefer the startup window from tauri.conf; create lazily if missing.
    if let Some(win) = app.get_webview_window(CRYPTO_BUBBLE_LABEL) {
        let _ = win.set_size(tauri::LogicalSize::new(
            CRYPTO_BUBBLE_WIDTH,
            CRYPTO_BUBBLE_HEIGHT,
        ));
        let _ = win.set_position(PhysicalPosition::new(target.0, target.1));
        let _ = win.set_always_on_top(true);
        if !silent {
            let _ = win.show();
            crate::mouse_trail::schedule_raise_overlays(app);
        }
        emit_crypto_bubble_hint(app, &hint);
        hide_clipboard_prompt(app);
        return;
    }

    // Silent path expects a prebuilt hidden window; do not create+show (would steal focus).
    if silent {
        let _ = app.emit("app://crypto-bubble", &hint);
        hide_clipboard_prompt(app);
        return;
    }

    let result = WebviewWindowBuilder::new(
        app,
        CRYPTO_BUBBLE_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("加解密结果")
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .background_color(Color(0, 0, 0, 0))
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .focused(false)
    .visible(false)
    .inner_size(CRYPTO_BUBBLE_WIDTH, CRYPTO_BUBBLE_HEIGHT)
    .build();

    match result {
        Ok(win) => {
            let _ = win.set_position(PhysicalPosition::new(target.0, target.1));
            let _ = win.set_always_on_top(true);
            if let Some(badge) = app.get_webview_window("badge") {
                let _ = badge.set_always_on_top(true);
            }
            let _ = win.set_always_on_top(true);
            let _ = win.show();
            emit_crypto_bubble_hint(app, &hint);
            hide_clipboard_prompt(app);
            crate::mouse_trail::schedule_raise_overlays(app);
        }
        Err(err) => {
            eprintln!("[crypto-bubble] create failed: {err}");
            if let Ok(mut pending) = PENDING_CRYPTO_BUBBLE.lock() {
                *pending = None;
            }
            hide_clipboard_prompt(app);
            show_main(app, Some(hint));
        }
    }
}

/// Show crypto-bubble near cursor without taking keyboard focus (macOS-safe).
/// Repositions only when revealing from hidden; already-visible windows stay put.
pub fn reveal_crypto_bubble(app: &AppHandle) {
    let Some(win) = app.get_webview_window(CRYPTO_BUBBLE_LABEL) else {
        return;
    };
    let was_visible = win.is_visible().unwrap_or(false);
    if !was_visible {
        let (cursor_x, cursor_y) = cursor_pos().unwrap_or((0, 0));
        let target = clamp_popup_origin(
            app,
            cursor_x,
            cursor_y,
            CRYPTO_BUBBLE_WIDTH,
            CRYPTO_BUBBLE_HEIGHT,
        );
        let _ = win.set_size(tauri::LogicalSize::new(
            CRYPTO_BUBBLE_WIDTH,
            CRYPTO_BUBBLE_HEIGHT,
        ));
        let _ = win.set_position(PhysicalPosition::new(target.0, target.1));
    }
    let _ = win.set_always_on_top(true);
    let _ = win.show();
    crate::mouse_trail::schedule_raise_overlays(app);
}

/// Hide bubble window only; keep pending payload so a concurrent show can still read it.
pub fn hide_crypto_bubble_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(CRYPTO_BUBBLE_LABEL) {
        let _ = win.hide();
    }
}

pub fn get_pending_crypto_bubble() -> Option<CryptoHint> {
    PENDING_CRYPTO_BUBBLE
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
}

pub fn hide_crypto_bubble(app: &AppHandle) {
    if let Ok(mut pending) = PENDING_CRYPTO_BUBBLE.lock() {
        *pending = None;
    }
    hide_crypto_bubble_window(app);
}

pub fn schedule_show_crypto_bubble(app: &AppHandle, hint: CryptoHint) {
    let handle = app.clone();
    let _ = handle.clone().run_on_main_thread(move || {
        show_crypto_bubble(&handle, hint);
    });
}

#[allow(dead_code)]
pub fn schedule_hide_crypto_bubble(app: &AppHandle) {
    let handle = app.clone();
    let _ = handle.clone().run_on_main_thread(move || {
        hide_crypto_bubble(&handle);
    });
}

fn clamp_popup_origin(
    app: &AppHandle,
    cursor_x: i32,
    cursor_y: i32,
    logical_width: f64,
    logical_height: f64,
) -> (i32, i32) {
    #[cfg(windows)]
    let offset = POPUP_CURSOR_OFFSET;
    #[cfg(not(windows))]
    let offset = 16;
    clamp_popup_origin_with_offset(
        app,
        cursor_x,
        cursor_y,
        logical_width,
        logical_height,
        offset,
    )
}

pub fn clamp_popup_origin_public(
    app: &AppHandle,
    cursor_x: i32,
    cursor_y: i32,
    logical_width: f64,
    logical_height: f64,
    offset: i32,
) -> (i32, i32) {
    clamp_popup_origin_with_offset(
        app,
        cursor_x,
        cursor_y,
        logical_width,
        logical_height,
        offset,
    )
}

fn clamp_popup_origin_with_offset(
    app: &AppHandle,
    cursor_x: i32,
    cursor_y: i32,
    logical_width: f64,
    logical_height: f64,
    offset: i32,
) -> (i32, i32) {
    let (mut x, mut y) = (cursor_x + offset, cursor_y + offset);

    let monitors = app.available_monitors().unwrap_or_default();
    let area = monitors
        .iter()
        .find(|m| {
            let work = m.work_area();
            let px = work.position.x;
            let py = work.position.y;
            let pw = work.size.width as i32;
            let ph = work.size.height as i32;
            cursor_x >= px && cursor_x < px + pw && cursor_y >= py && cursor_y < py + ph
        })
        .or_else(|| monitors.first())
        .map(|m| m.work_area());

    let Some(work) = area else {
        return (x, y);
    };

    let scale = monitors
        .iter()
        .find(|m| {
            let w = m.work_area();
            w.position.x == work.position.x && w.position.y == work.position.y
        })
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);
    let prompt_w = (logical_width * scale).round() as i32;
    let prompt_h = (logical_height * scale).round() as i32;
    let left = work.position.x;
    let top = work.position.y;
    let right = left + work.size.width as i32;
    let bottom = top + work.size.height as i32;

    if x + prompt_w > right {
        x = right - prompt_w;
    }
    if y + prompt_h > bottom {
        y = bottom - prompt_h;
    }
    x = x.max(left);
    y = y.max(top);
    (x, y)
}

pub fn cursor_pos_public() -> Option<(i32, i32)> {
    cursor_pos()
}

/// Point shell shortcuts at `desktop-icon.ico` so Windows does not keep showing a
/// cached older icon embedded in the same exe path after upgrades.
#[cfg(windows)]
pub fn sync_shell_shortcut_icons() {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(dir) = exe.parent() else {
        return;
    };
    let ico = dir.join("desktop-icon.ico");
    if !ico.is_file() {
        return;
    }

    let exe_s = exe.to_string_lossy().replace('\'', "''");
    let ico_s = ico.to_string_lossy().replace('\'', "''");
    let script = format!(
        r#"
$ErrorActionPreference = 'SilentlyContinue'
$sh = New-Object -ComObject WScript.Shell
$want = '{exe_s}'
$ico = '{ico_s},0'
$targets = @(
  [IO.Path]::Combine($env:USERPROFILE, 'Desktop', '多多工具箱.lnk'),
  [IO.Path]::Combine($env:PUBLIC, 'Desktop', '多多工具箱.lnk'),
  [IO.Path]::Combine($env:APPDATA, 'Microsoft\Windows\Start Menu\Programs', '多多工具箱.lnk')
)
foreach ($p in $targets) {{
  if (-not (Test-Path -LiteralPath $p)) {{ continue }}
  $lnk = $sh.CreateShortcut($p)
  if ($lnk.TargetPath -ne $want) {{ continue }}
  if ($lnk.IconLocation -eq $ico) {{ continue }}
  $lnk.IconLocation = $ico
  $lnk.Save()
}}
Add-Type -Namespace Jdd -Name ShellNotify -MemberDefinition '[DllImport("shell32.dll")] public static extern void SHChangeNotify(int e, uint f, System.IntPtr a, System.IntPtr b);' -ErrorAction SilentlyContinue
[Jdd.ShellNotify]::SHChangeNotify(0x08000000, 0, [IntPtr]::Zero, [IntPtr]::Zero)
"#
    );
    let _ = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .status();
}

#[cfg(not(windows))]
pub fn sync_shell_shortcut_icons() {}

#[cfg(windows)]
fn cursor_pos() -> Option<(i32, i32)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT { x: 0, y: 0 };
    // SAFETY: `point` is a valid stack POINT; GetCursorPos only writes into it.
    let ok = unsafe { GetCursorPos(&mut point) };
    if ok == 0 {
        return None;
    }
    Some((point.x, point.y))
}

#[cfg(target_os = "macos")]
fn cursor_pos() -> Option<(i32, i32)> {
    use core_graphics::display::CGDisplay;
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok()?;
    let event = CGEvent::new(source).ok()?;
    let loc = event.location();
    let main = CGDisplay::main();
    let logical_h = main.bounds().size.height;
    let scale = if logical_h > 0.0 {
        main.pixels_high() as f64 / logical_h
    } else {
        1.0
    };
    Some((
        (loc.x * scale).round() as i32,
        (loc.y * scale).round() as i32,
    ))
}

#[cfg(target_os = "linux")]
fn cursor_pos() -> Option<(i32, i32)> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use x11rb::connection::Connection;

    static WAYLAND_CURSOR_WARNED: AtomicBool = AtomicBool::new(false);

    let Ok((conn, screen_num)) = x11rb::connect(None) else {
        if !WAYLAND_CURSOR_WARNED.swap(true, Ordering::Relaxed) {
            eprintln!(
                "[cursor] global pointer unavailable (pure Wayland?); mini-window leave-to-hide may not work"
            );
        }
        return None;
    };
    let root = conn.setup().roots.get(screen_num)?.root;
    let Ok(cookie) = conn.query_pointer(root) else {
        return None;
    };
    let Ok(reply) = cookie.reply() else {
        return None;
    };
    Some((i32::from(reply.root_x), i32::from(reply.root_y)))
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn cursor_pos() -> Option<(i32, i32)> {
    None
}
