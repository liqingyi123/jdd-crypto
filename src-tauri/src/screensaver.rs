//! Multi-monitor fullscreen screensaver + global shortcut.

use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, EventTarget, LogicalSize, Manager, PhysicalPosition, WebviewUrl,
    WebviewWindowBuilder, window::Color,
};
use tauri_plugin_global_shortcut::Shortcut;
use tauri_plugin_store::StoreExt;

use crate::global_shortcuts;
use crate::state::{AppState, DEFAULT_SCREENSAVER_SHORTCUT};

const MAX_OVERLAYS: usize = 8;
const SETTINGS_STORE: &str = "settings.json";
const SHORTCUT_KEY: &str = "screensaverShortcut";
const PREF_KEY: &str = "screensaverEnabled";
const EFFECT_STORE_KEY: &str = "screensaver";
const DEFAULT_BACKGROUND: &str = "parallax";
const DEFAULT_CLOCK: &str = "lcd3d";

static ACTIVE: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreensaverPref {
    #[serde(default = "default_background")]
    pub background: String,
    #[serde(default = "default_clock")]
    pub clock: String,
}

fn default_background() -> String {
    DEFAULT_BACKGROUND.to_string()
}

fn default_clock() -> String {
    DEFAULT_CLOCK.to_string()
}

impl Default for ScreensaverPref {
    fn default() -> Self {
        Self {
            background: DEFAULT_BACKGROUND.to_string(),
            clock: DEFAULT_CLOCK.to_string(),
        }
    }
}

struct MonitorLayout {
    physical_pos: PhysicalPosition<i32>,
    logical_pos: (f64, f64),
    logical_size: LogicalSize<f64>,
}

fn monitor_layout(monitor: &tauri::Monitor) -> MonitorLayout {
    let pos = *monitor.position();
    let size = *monitor.size();
    let scale = monitor.scale_factor();
    MonitorLayout {
        physical_pos: PhysicalPosition::new(pos.x, pos.y),
        logical_pos: (pos.x as f64 / scale, pos.y as f64 / scale),
        logical_size: LogicalSize::new(size.width as f64 / scale, size.height as f64 / scale),
    }
}

fn overlay_label(index: usize) -> String {
    format!("screensaver-{index}")
}

fn is_overlay_target(target: &EventTarget) -> bool {
    match target {
        EventTarget::WebviewWindow { label }
        | EventTarget::Webview { label }
        | EventTarget::Window { label }
        | EventTarget::AnyLabel { label } => label.starts_with("screensaver-"),
        _ => false,
    }
}

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

pub fn load_shortcut(app: &AppHandle) -> String {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return DEFAULT_SCREENSAVER_SHORTCUT.to_string();
    };
    store
        .get(SHORTCUT_KEY)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|raw| crate::mouse_follow::validate_shortcut(&raw).ok())
        .unwrap_or_else(|| DEFAULT_SCREENSAVER_SHORTCUT.to_string())
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
        .screensaver_pref_enabled
        .store(enabled, Ordering::Relaxed);
    save_pref(app, enabled);
    if !enabled {
        hide(app);
    }
    let _ = global_shortcuts::register_all(app);
}

fn normalize_background(raw: &str) -> String {
    match raw {
        "corona" | "parallax" | "snow" => raw.to_string(),
        _ => DEFAULT_BACKGROUND.to_string(),
    }
}

fn normalize_clock(raw: &str) -> String {
    match raw {
        "lcd3d" | "analog" | "compass" | "off" => raw.to_string(),
        _ => DEFAULT_CLOCK.to_string(),
    }
}

fn pref_from_store_value(value: serde_json::Value) -> ScreensaverPref {
    // Legacy: { "effect": "parallax" }
    if let Some(effect) = value.get("effect").and_then(|v| v.as_str()) {
        if value.get("background").is_none() {
            return ScreensaverPref {
                background: normalize_background(effect),
                clock: DEFAULT_CLOCK.to_string(),
            };
        }
    }
    let background = value
        .get("background")
        .and_then(|v| v.as_str())
        .map(normalize_background)
        .unwrap_or_else(default_background);
    let clock = value
        .get("clock")
        .and_then(|v| v.as_str())
        .map(normalize_clock)
        .unwrap_or_else(default_clock);
    ScreensaverPref { background, clock }
}

pub fn load_effect_pref(app: &AppHandle) -> ScreensaverPref {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return ScreensaverPref::default();
    };
    store
        .get(EFFECT_STORE_KEY)
        .map(pref_from_store_value)
        .unwrap_or_default()
}

pub fn save_effect_pref(app: &AppHandle, pref: &ScreensaverPref) {
    if let Ok(store) = app.store(SETTINGS_STORE) {
        if let Ok(value) = serde_json::to_value(pref) {
            store.set(EFFECT_STORE_KEY, value);
            let _ = store.save();
        }
    }
}

fn emit_pref(app: &AppHandle, pref: &ScreensaverPref) {
    let _ = app.emit_filter("app://screensaver-pref", pref, is_overlay_target);
    let _ = app.emit("app://screensaver-pref", pref);
}

pub fn set_background(app: &AppHandle, background: String) -> ScreensaverPref {
    let mut pref = load_effect_pref(app);
    pref.background = normalize_background(&background);
    save_effect_pref(app, &pref);
    emit_pref(app, &pref);
    pref
}

pub fn set_clock(app: &AppHandle, clock: String) -> ScreensaverPref {
    let mut pref = load_effect_pref(app);
    pref.clock = normalize_clock(&clock);
    save_effect_pref(app, &pref);
    emit_pref(app, &pref);
    pref
}

pub fn handle_shortcut(app: &AppHandle, shortcut: &Shortcut) {
    let state = app.state::<AppState>();
    if !state.screensaver_pref_enabled.load(Ordering::Relaxed) {
        return;
    }
    let expected = state
        .screensaver_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| DEFAULT_SCREENSAVER_SHORTCUT.to_string());
    let Ok(expected_shortcut) = Shortcut::from_str(&expected) else {
        return;
    };
    if shortcut.id() != expected_shortcut.id() {
        return;
    }
    toggle(app);
}

pub fn toggle(app: &AppHandle) {
    if is_active() {
        hide(app);
    } else {
        show(app);
    }
}

pub fn show(app: &AppHandle) {
    if crate::spotlight::is_active() {
        crate::spotlight::hide(app);
    }
    crate::mouse_trail::pause_for_screensaver(app);
    ACTIVE.store(true, Ordering::Relaxed);
    let state = app.state::<AppState>();
    state.screensaver_active.store(true, Ordering::Relaxed);
    sync_overlays(app, true);
    let pref = load_effect_pref(app);
    let handle = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(80));
        emit_pref(&handle, &pref);
        if let Some(win) = handle.get_webview_window(&overlay_label(0)) {
            let _ = win.set_focus();
        }
    });
}

pub fn hide(app: &AppHandle) {
    ACTIVE.store(false, Ordering::Relaxed);
    let state = app.state::<AppState>();
    state.screensaver_active.store(false, Ordering::Relaxed);
    sync_overlays(app, false);
    crate::mouse_trail::resume_after_screensaver(app);
}

fn sync_overlays(app: &AppHandle, visible: bool) {
    let monitors = app.available_monitors().unwrap_or_default();
    for index in 0..MAX_OVERLAYS {
        let label = overlay_label(index);
        if index >= monitors.len() {
            if let Some(win) = app.get_webview_window(&label) {
                let _ = win.close();
            }
            continue;
        }

        let layout = monitor_layout(&monitors[index]);

        if let Some(win) = app.get_webview_window(&label) {
            let _ = win.set_position(layout.physical_pos);
            let _ = win.set_size(layout.logical_size);
            if visible {
                let _ = win.set_always_on_top(true);
                let _ = win.show();
            } else {
                let _ = win.hide();
            }
            continue;
        }

        if !visible {
            continue;
        }

        let (logical_x, logical_y) = layout.logical_pos;
        let result = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
            .title("屏幕保护")
            .decorations(false)
            .transparent(false)
            .shadow(false)
            .background_color(Color(0, 0, 0, 255))
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .visible(false)
            .focused(index == 0)
            .position(logical_x, logical_y)
            .inner_size(layout.logical_size.width, layout.logical_size.height)
            .build();

        if let Ok(win) = result {
            let _ = win.set_always_on_top(true);
            let _ = win.show();
            if index == 0 {
                let _ = win.set_focus();
            }
        }
    }
}
