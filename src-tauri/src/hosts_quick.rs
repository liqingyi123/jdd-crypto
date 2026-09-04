//! Near-cursor Host quick-switch popup + global shortcut.

use std::str::FromStr;
use std::thread;
use std::time::Duration;

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, window::Color,
};
use tauri_plugin_global_shortcut::Shortcut;
use tauri_plugin_store::StoreExt;

use crate::state::{AppState, DEFAULT_HOSTS_QUICK_SHORTCUT};
use crate::windows;

const SETTINGS_STORE: &str = "settings.json";
const SHORTCUT_KEY: &str = "hostsQuickShortcut";

pub const HOSTS_QUICK_LABEL: &str = "hosts-quick";
const HOSTS_QUICK_WIDTH: f64 = 300.0;
const HOSTS_QUICK_HEIGHT: f64 = 380.0;
const CURSOR_OFFSET: i32 = 16;

pub fn load_shortcut(app: &AppHandle) -> String {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return DEFAULT_HOSTS_QUICK_SHORTCUT.to_string();
    };
    store
        .get(SHORTCUT_KEY)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|raw| crate::mouse_follow::validate_shortcut(&raw).ok())
        .unwrap_or_else(|| DEFAULT_HOSTS_QUICK_SHORTCUT.to_string())
}

pub fn save_shortcut(app: &AppHandle, shortcut: &str) {
    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(SHORTCUT_KEY, serde_json::json!(shortcut));
        let _ = store.save();
    }
}

pub fn handle_shortcut(app: &AppHandle, shortcut: &Shortcut) {
    let state = app.state::<AppState>();
    let expected = state
        .hosts_quick_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| DEFAULT_HOSTS_QUICK_SHORTCUT.to_string());
    let Ok(expected_shortcut) = Shortcut::from_str(&expected) else {
        return;
    };
    if shortcut.id() != expected_shortcut.id() {
        return;
    }
    show(app);
}

pub fn show(app: &AppHandle) {
    let (cursor_x, cursor_y) = windows::cursor_pos_public().unwrap_or((0, 0));
    let target = windows::clamp_popup_origin_public(
        app,
        cursor_x,
        cursor_y,
        HOSTS_QUICK_WIDTH,
        HOSTS_QUICK_HEIGHT,
        CURSOR_OFFSET,
    );

    if let Some(win) = app.get_webview_window(HOSTS_QUICK_LABEL) {
        let _ = win.set_size(tauri::LogicalSize::new(
            HOSTS_QUICK_WIDTH,
            HOSTS_QUICK_HEIGHT,
        ));
        let _ = win.set_position(PhysicalPosition::new(target.0, target.1));
        let _ = win.set_always_on_top(true);
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.emit("hosts://quick-open", ());
        return;
    }

    let result = WebviewWindowBuilder::new(
        app,
        HOSTS_QUICK_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("Host 快速切换")
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
    .inner_size(HOSTS_QUICK_WIDTH, HOSTS_QUICK_HEIGHT)
    .build();

    if let Ok(win) = result {
        let _ = win.set_position(PhysicalPosition::new(target.0, target.1));
        let _ = win.set_always_on_top(true);
        windows::bind_close_to_hide(&win);
        let handle = app.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(80));
            if let Some(win) = handle.get_webview_window(HOSTS_QUICK_LABEL) {
                let _ = win.emit("hosts://quick-open", ());
            }
        });
    }
}

pub fn hide(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(HOSTS_QUICK_LABEL) {
        let _ = win.hide();
    }
}
