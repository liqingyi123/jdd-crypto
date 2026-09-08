//! Near-cursor multi-display brightness popup + global shortcut.

use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, window::Color,
};
use tauri_plugin_global_shortcut::Shortcut;
use tauri_plugin_store::StoreExt;

use crate::brightness_control;
use crate::global_shortcuts;
use crate::state::{AppState, DEFAULT_BRIGHTNESS_QUICK_SHORTCUT};
use crate::windows;

const SETTINGS_STORE: &str = "settings.json";
const SHORTCUT_KEY: &str = "brightnessQuickShortcut";
const PREF_KEY: &str = "brightnessQuickEnabled";

pub const BRIGHTNESS_BUBBLE_LABEL: &str = "brightness-bubble";
const BUBBLE_WIDTH: f64 = 320.0;
const BUBBLE_ROW_HEIGHT: f64 = 72.0;
const BUBBLE_CHROME: f64 = 56.0;
const CURSOR_OFFSET: i32 = 16;

pub fn load_shortcut(app: &AppHandle) -> String {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return DEFAULT_BRIGHTNESS_QUICK_SHORTCUT.to_string();
    };
    store
        .get(SHORTCUT_KEY)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|raw| crate::mouse_follow::validate_shortcut(&raw).ok())
        .unwrap_or_else(|| DEFAULT_BRIGHTNESS_QUICK_SHORTCUT.to_string())
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
        .brightness_quick_pref_enabled
        .store(enabled, Ordering::Relaxed);
    save_pref(app, enabled);
    if !enabled {
        hide(app);
    }
    let _ = global_shortcuts::register_all(app);
}

pub fn handle_shortcut(app: &AppHandle, shortcut: &Shortcut) {
    let state = app.state::<AppState>();
    if !state.brightness_quick_pref_enabled.load(Ordering::Relaxed) {
        return;
    }
    let expected = state
        .brightness_quick_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| DEFAULT_BRIGHTNESS_QUICK_SHORTCUT.to_string());
    let Ok(expected_shortcut) = Shortcut::from_str(&expected) else {
        return;
    };
    if shortcut.id() != expected_shortcut.id() {
        return;
    }
    show(app);
}

fn bubble_height(display_count: usize) -> f64 {
    let rows = display_count.max(1) as f64;
    (BUBBLE_CHROME + rows * BUBBLE_ROW_HEIGHT).max(120.0).min(800.0)
}

pub fn show(app: &AppHandle) {
    let displays = brightness_control::list_displays().unwrap_or_default();
    let height = bubble_height(displays.len());
    let (cursor_x, cursor_y) = windows::cursor_pos_public().unwrap_or((0, 0));
    let target = windows::clamp_popup_origin_public(
        app,
        cursor_x,
        cursor_y,
        BUBBLE_WIDTH,
        height,
        CURSOR_OFFSET,
    );

    if let Some(win) = app.get_webview_window(BRIGHTNESS_BUBBLE_LABEL) {
        let _ = win.set_size(tauri::LogicalSize::new(BUBBLE_WIDTH, height));
        let _ = win.set_position(PhysicalPosition::new(target.0, target.1));
        let _ = win.set_always_on_top(true);
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.emit("brightness://quick-open", &displays);
        crate::mouse_trail::schedule_raise_overlays(app);
        return;
    }

    let result = WebviewWindowBuilder::new(
        app,
        BRIGHTNESS_BUBBLE_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("屏幕亮度")
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
    .inner_size(BUBBLE_WIDTH, height)
    .build();

    if let Ok(win) = result {
        let _ = win.set_position(PhysicalPosition::new(target.0, target.1));
        let _ = win.set_always_on_top(true);
        windows::bind_close_to_hide(&win);
        crate::mouse_trail::schedule_raise_overlays(app);
        let handle = app.clone();
        let payload = displays;
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(80));
            if let Some(win) = handle.get_webview_window(BRIGHTNESS_BUBBLE_LABEL) {
                let _ = win.emit("brightness://quick-open", &payload);
            }
        });
    }
}

pub fn hide(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(BRIGHTNESS_BUBBLE_LABEL) {
        let _ = win.hide();
    }
}
