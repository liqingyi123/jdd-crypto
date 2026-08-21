use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    window::Color,
};

#[derive(Clone, Serialize)]
pub struct CryptoHint {
    pub text: String,
    pub mode: String,
}

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
        "feedback" => Some(FeatureSpec {
            label: "feedback",
            title: "意见反馈",
            width: 720.0,
            height: 560.0,
            min_width: 560.0,
            min_height: 420.0,
        }),
        "plugins" => Some(FeatureSpec {
            label: "plugins",
            title: "插件管理",
            width: 720.0,
            height: 560.0,
            min_width: 560.0,
            min_height: 420.0,
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

fn focus_window(win: &WebviewWindow) {
    let _ = win.unminimize();
    let _ = win.show();
    let _ = win.set_focus();
}

pub fn show_main(app: &AppHandle, hint: Option<CryptoHint>) {
    if let Some(win) = app.get_webview_window("main") {
        focus_window(&win);
        if let Some(hint) = hint {
            let _ = win.emit("app://crypto-payload", hint);
        }
    }
}

pub fn show_feature(app: &AppHandle, label: &str) {
    // Feature windows are created lazily (not declared in tauri.conf.json) so
    // cold start only pays for badge + main. Re-open reuses the existing webview.
    if let Some(win) = app.get_webview_window(label) {
        focus_window(&win);
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
        .visible(true)
        .build();

    if let Ok(win) = result {
        bind_close_to_hide(&win);
    }
}

/// Map the window close button to hide instead of destroy.
/// Keeps tray/badge workflows snappy and avoids recreating webviews on every open.
pub fn bind_close_to_hide(win: &WebviewWindow) {
    let hidden = win.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = hidden.hide();
        }
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
/// Logical size of the near-cursor clipboard prompt card window.
const CLIPBOARD_PROMPT_WIDTH: f64 = 244.0;
const CLIPBOARD_PROMPT_HEIGHT: f64 = 116.0;
#[cfg(windows)]
const CLIPBOARD_PROMPT_CURSOR_OFFSET: i32 = 16;

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

    let work = monitor.work_area();
    let scale = monitor.scale_factor();
    let Ok(outer_size) = win.outer_size() else {
        return;
    };

    let margin = (BADGE_EDGE_MARGIN * scale).round() as i32;
    let x = work.position.x + work.size.width as i32 - outer_size.width as i32 - margin;
    let y = work.position.y + work.size.height as i32 - outer_size.height as i32 - margin;
    set_badge_position(&win, x, y);
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
        // Clipboard prompt is a separate window; badge stays collapsed.
        let _ = expanded;
        let _ = win.set_size(tauri::LogicalSize::new(f64::from(size), f64::from(size)));
        let _ = win.emit("app://badge-size", size);
    }
}

/// Show (or create) the clipboard prompt near the cursor. Must run on the main thread.
pub fn show_clipboard_prompt(app: &AppHandle) {
    use crate::state::AppState;

    let (cursor_x, cursor_y) = cursor_pos().unwrap_or((0, 0));
    let target = clamp_prompt_origin(app, cursor_x, cursor_y);

    let ensure_emit = |win: &WebviewWindow| {
        if let Ok(guard) = app.state::<AppState>().last_candidate.lock() {
            if let Some(payload) = guard.as_ref() {
                let _ = win.emit("clipboard://candidate", payload);
            }
        }
    };

    if let Some(win) = app.get_webview_window(CLIPBOARD_PROMPT_LABEL) {
        let _ = win.set_size(tauri::LogicalSize::new(
            CLIPBOARD_PROMPT_WIDTH,
            CLIPBOARD_PROMPT_HEIGHT,
        ));
        let _ = win.set_position(PhysicalPosition::new(target.0, target.1));
        let _ = win.set_always_on_top(true);
        // Do not focus — keep the user's original input focused.
        let _ = win.show();
        ensure_emit(&win);
        return;
    }

    let result = WebviewWindowBuilder::new(
        app,
        CLIPBOARD_PROMPT_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("剪贴板询问")
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
    .visible(true)
    .inner_size(CLIPBOARD_PROMPT_WIDTH, CLIPBOARD_PROMPT_HEIGHT)
    .build();

    if let Ok(win) = result {
        let _ = win.set_position(PhysicalPosition::new(target.0, target.1));
        let _ = win.set_always_on_top(true);
        if let Some(badge) = app.get_webview_window("badge") {
            let _ = badge.set_always_on_top(true);
        }
        let _ = win.set_always_on_top(true);
        ensure_emit(&win);
    }
}

pub fn hide_clipboard_prompt(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(CLIPBOARD_PROMPT_LABEL) {
        let _ = win.hide();
    }
}

pub fn schedule_show_clipboard_prompt(app: &AppHandle) {
    let handle = app.clone();
    let _ = handle.clone().run_on_main_thread(move || {
        show_clipboard_prompt(&handle);
    });
}

pub fn schedule_hide_clipboard_prompt(app: &AppHandle) {
    let handle = app.clone();
    let _ = handle.clone().run_on_main_thread(move || {
        hide_clipboard_prompt(&handle);
    });
}

fn clamp_prompt_origin(app: &AppHandle, cursor_x: i32, cursor_y: i32) -> (i32, i32) {
    #[cfg(windows)]
    let (mut x, mut y) = (
        cursor_x + CLIPBOARD_PROMPT_CURSOR_OFFSET,
        cursor_y + CLIPBOARD_PROMPT_CURSOR_OFFSET,
    );
    #[cfg(not(windows))]
    let (mut x, mut y) = (cursor_x + 16, cursor_y + 16);

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
    let prompt_w = (CLIPBOARD_PROMPT_WIDTH * scale).round() as i32;
    let prompt_h = (CLIPBOARD_PROMPT_HEIGHT * scale).round() as i32;
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

#[cfg(not(windows))]
fn cursor_pos() -> Option<(i32, i32)> {
    None
}
