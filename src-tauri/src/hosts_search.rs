//! Screen-center Host search popup + global shortcut.

use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{
    window::Color, AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::Shortcut;
use tauri_plugin_store::StoreExt;

use crate::global_shortcuts;
use crate::state::{AppState, DEFAULT_HOSTS_SEARCH_SHORTCUT};
use crate::windows;

const SETTINGS_STORE: &str = "settings.json";
const SHORTCUT_KEY: &str = "hostsSearchShortcut";
const PREF_KEY: &str = "hostsSearchEnabled";

pub const HOSTS_SEARCH_LABEL: &str = "hosts-search";
const HOSTS_SEARCH_WIDTH: f64 = 360.0;
const HOSTS_SEARCH_COMPACT_HEIGHT: f64 = 110.0;
const BLUR_ARM_MS: u64 = 300;
const DRAG_SUPPRESS_MS: u64 = 800;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusSchemePayload {
    pub scheme_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
}

static BLUR_ARMED_AFTER: Mutex<Option<Instant>> = Mutex::new(None);
static SUPPRESS_BLUR_CLOSE: AtomicBool = AtomicBool::new(false);
static DRAG_SUPPRESS_SEQ: AtomicU64 = AtomicU64::new(0);
static PENDING_FOCUS: Mutex<Option<FocusSchemePayload>> = Mutex::new(None);

pub fn load_shortcut(app: &AppHandle) -> String {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return DEFAULT_HOSTS_SEARCH_SHORTCUT.to_string();
    };
    store
        .get(SHORTCUT_KEY)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|raw| crate::mouse_follow::validate_shortcut(&raw).ok())
        .unwrap_or_else(|| DEFAULT_HOSTS_SEARCH_SHORTCUT.to_string())
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
        .hosts_search_pref_enabled
        .store(enabled, Ordering::Relaxed);
    save_pref(app, enabled);
    if !enabled {
        hide(app);
    }
    let _ = global_shortcuts::register_all(app);
}

pub fn handle_shortcut(app: &AppHandle, shortcut: &Shortcut) {
    let state = app.state::<AppState>();
    if !state.hosts_search_pref_enabled.load(Ordering::Relaxed) {
        return;
    }
    let expected = state
        .hosts_search_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| DEFAULT_HOSTS_SEARCH_SHORTCUT.to_string());
    let Ok(expected_shortcut) = Shortcut::from_str(&expected) else {
        return;
    };
    if shortcut.id() != expected_shortcut.id() {
        return;
    }
    show(app);
}

fn arm_blur_dismiss() {
    if let Ok(mut guard) = BLUR_ARMED_AFTER.lock() {
        *guard = Some(Instant::now() + Duration::from_millis(BLUR_ARM_MS));
    }
}

fn blur_dismiss_armed() -> bool {
    let Ok(guard) = BLUR_ARMED_AFTER.lock() else {
        return false;
    };
    match *guard {
        Some(deadline) => Instant::now() >= deadline,
        None => false,
    }
}

fn bind_blur_close(win: &WebviewWindow) {
    arm_blur_dismiss();
    let closed = win.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(false) = event {
            if SUPPRESS_BLUR_CLOSE.load(Ordering::Relaxed) {
                return;
            }
            if !blur_dismiss_armed() {
                return;
            }
            let _ = closed.close();
        }
    });
}

/// Suppress blur-close while the user drags the window title (startDragging blurs briefly).
pub fn begin_drag() {
    let seq = DRAG_SUPPRESS_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    SUPPRESS_BLUR_CLOSE.store(true, Ordering::Relaxed);
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(DRAG_SUPPRESS_MS));
        if DRAG_SUPPRESS_SEQ.load(Ordering::Relaxed) == seq {
            SUPPRESS_BLUR_CLOSE.store(false, Ordering::Relaxed);
        }
    });
}

fn center_origin(app: &AppHandle, logical_height: f64) -> (i32, i32) {
    let (cursor_x, cursor_y) = windows::cursor_pos_public().unwrap_or((0, 0));
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
        return (0, 0);
    };
    let work = monitor.work_area();
    let scale = monitor.scale_factor();
    let win_w = (HOSTS_SEARCH_WIDTH * scale).round() as i32;
    let win_h = (logical_height * scale).round() as i32;
    let max_w = work.size.width as i32;
    let max_h = work.size.height as i32;
    let win_w = win_w.min(max_w.saturating_sub(24));
    let win_h = win_h.min(max_h.saturating_sub(24));
    let x = work.position.x + (max_w - win_w) / 2;
    let y = work.position.y + (max_h - win_h) / 2;
    (x, y)
}

fn current_logical_height(win: &WebviewWindow) -> f64 {
    let scale = win.scale_factor().unwrap_or(1.0);
    win.inner_size()
        .ok()
        .map(|size| size.height as f64 / scale)
        .unwrap_or(HOSTS_SEARCH_COMPACT_HEIGHT)
}

/// Re-center the search window using its current height.
pub fn recenter(app: &AppHandle) {
    let Some(win) = app.get_webview_window(HOSTS_SEARCH_LABEL) else {
        return;
    };
    let height = current_logical_height(&win);
    let target = center_origin(app, height);
    let _ = win.set_position(PhysicalPosition::new(target.0, target.1));
}

pub fn show(app: &AppHandle) {
    let target = center_origin(app, HOSTS_SEARCH_COMPACT_HEIGHT);

    if let Some(win) = app.get_webview_window(HOSTS_SEARCH_LABEL) {
        let _ = win.set_size(tauri::LogicalSize::new(
            HOSTS_SEARCH_WIDTH,
            HOSTS_SEARCH_COMPACT_HEIGHT,
        ));
        let _ = win.set_position(PhysicalPosition::new(target.0, target.1));
        let _ = win.set_always_on_top(true);
        arm_blur_dismiss();
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.emit("hosts://search-open", ());
        crate::mouse_trail::schedule_raise_overlays(app);
        return;
    }

    let result = WebviewWindowBuilder::new(
        app,
        HOSTS_SEARCH_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("Host 检索")
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .background_color(Color(0, 0, 0, 0))
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .focused(true)
    .visible(true)
    .inner_size(HOSTS_SEARCH_WIDTH, HOSTS_SEARCH_COMPACT_HEIGHT)
    .build();

    if let Ok(win) = result {
        let _ = win.set_position(PhysicalPosition::new(target.0, target.1));
        let _ = win.set_always_on_top(true);
        bind_blur_close(&win);
        crate::mouse_trail::schedule_raise_overlays(app);
        let handle = app.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(80));
            if let Some(win) = handle.get_webview_window(HOSTS_SEARCH_LABEL) {
                let _ = win.emit("hosts://search-open", ());
            }
        });
    }
}

/// Destroy the search window (reclaim WebView process).
pub fn hide(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(HOSTS_SEARCH_LABEL) {
        let _ = win.close();
    }
}

/// Open Host 管理 and focus a scheme (optionally a content line); closes search first.
pub fn open_hosts_scheme(app: &AppHandle, scheme_id: &str, line: Option<&str>) {
    hide(app);
    let payload = FocusSchemePayload {
        scheme_id: scheme_id.to_string(),
        line: line
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
    };
    if let Ok(mut pending) = PENDING_FOCUS.lock() {
        *pending = Some(payload.clone());
    }
    windows::show_feature(app, "hosts");
    emit_focus_scheme(app, &payload);
}

pub fn take_pending_focus() -> Option<FocusSchemePayload> {
    PENDING_FOCUS.lock().ok().and_then(|mut guard| guard.take())
}

fn emit_focus_scheme(app: &AppHandle, payload: &FocusSchemePayload) {
    let _ = app.emit("hosts://focus-scheme", payload);
    let handle = app.clone();
    let first = payload.clone();
    let second = payload.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(400));
        let _ = handle.emit("hosts://focus-scheme", &first);
        thread::sleep(Duration::from_millis(400));
        let _ = handle.emit("hosts://focus-scheme", &second);
    });
}
