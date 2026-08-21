use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, EventTarget, LogicalSize, Manager, PhysicalPosition, WebviewUrl,
    WebviewWindowBuilder, window::Color,
};
use tauri_plugin_store::StoreExt;

const MAX_OVERLAYS: usize = 8;
const SETTINGS_STORE: &str = "settings.json";
const PREF_KEY: &str = "mouseTrail";
const DEFAULT_EFFECT: &str = "ribbon";

static CURSOR_LOOP_STARTED: AtomicBool = AtomicBool::new(false);
static TRAIL_ENABLED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseTrailCursor {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseTrailMonitorBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseTrailPref {
    pub enabled: bool,
    pub effect: String,
}

impl Default for MouseTrailPref {
    fn default() -> Self {
        Self {
            enabled: false,
            effect: DEFAULT_EFFECT.to_string(),
        }
    }
}

struct MonitorLayout {
    physical_pos: PhysicalPosition<i32>,
    logical_pos: (f64, f64),
    logical_size: LogicalSize<f64>,
}

fn monitor_layout(monitor: &tauri::Monitor) -> MonitorLayout {
    let work = monitor.work_area();
    let scale = monitor.scale_factor();
    MonitorLayout {
        physical_pos: PhysicalPosition::new(work.position.x, work.position.y),
        logical_pos: (
            work.position.x as f64 / scale,
            work.position.y as f64 / scale,
        ),
        logical_size: LogicalSize::new(
            work.size.width as f64 / scale,
            work.size.height as f64 / scale,
        ),
    }
}

pub fn overlay_label(index: usize) -> String {
    format!("mouse-trail-{index}")
}

pub fn parse_overlay_index(label: &str) -> Option<usize> {
    label
        .strip_prefix("mouse-trail-")
        .and_then(|rest| rest.parse().ok())
}

fn is_overlay_target(target: &EventTarget) -> bool {
    match target {
        EventTarget::WebviewWindow { label }
        | EventTarget::Webview { label }
        | EventTarget::Window { label }
        | EventTarget::AnyLabel { label } => label.starts_with("mouse-trail-"),
        _ => false,
    }
}

fn normalize_effect(raw: &str) -> String {
    match raw {
        "meteor" => "meteor".to_string(),
        "graffiti" => "graffiti".to_string(),
        "dots" => "dots".to_string(),
        "heart" => "heart".to_string(),
        _ => DEFAULT_EFFECT.to_string(),
    }
}

pub fn load_pref(app: &AppHandle) -> MouseTrailPref {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return MouseTrailPref::default();
    };
    let Some(value) = store.get(PREF_KEY) else {
        return MouseTrailPref::default();
    };
    let Ok(mut pref) = serde_json::from_value::<MouseTrailPref>(value) else {
        return MouseTrailPref::default();
    };
    pref.effect = normalize_effect(&pref.effect);
    pref
}

fn save_pref(app: &AppHandle, pref: &MouseTrailPref) -> Result<(), String> {
    let store = app.store(SETTINGS_STORE).map_err(|e| e.to_string())?;
    store.set(
        PREF_KEY,
        serde_json::to_value(pref).map_err(|e| e.to_string())?,
    );
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

fn emit_pref(app: &AppHandle, pref: &MouseTrailPref) {
    let _ = app.emit("app://mouse-trail-pref", pref);
}

pub fn get_pref(app: AppHandle) -> MouseTrailPref {
    load_pref(&app)
}

pub fn set_enabled_pref(app: AppHandle, enabled: bool) -> Result<MouseTrailPref, String> {
    let mut pref = load_pref(&app);
    pref.enabled = enabled;
    save_pref(&app, &pref)?;
    set_enabled(&app, enabled);
    emit_pref(&app, &pref);
    Ok(pref)
}

pub fn set_effect_pref(app: AppHandle, effect: String) -> Result<MouseTrailPref, String> {
    let mut pref = load_pref(&app);
    pref.effect = normalize_effect(&effect);
    save_pref(&app, &pref)?;
    emit_pref(&app, &pref);
    Ok(pref)
}

pub fn reset_pref(app: AppHandle) -> Result<MouseTrailPref, String> {
    let pref = MouseTrailPref::default();
    save_pref(&app, &pref)?;
    set_enabled(&app, false);
    emit_pref(&app, &pref);
    Ok(pref)
}

pub fn init_from_store(app: &AppHandle) {
    let pref = load_pref(app);
    set_enabled(app, pref.enabled);
}

/// Flip the enabled flag and schedule overlay sync off the invoke path
/// so settings UI does not wait on WebView creation.
pub fn set_enabled(app: &AppHandle, enabled: bool) {
    TRAIL_ENABLED.store(enabled, Ordering::Relaxed);
    ensure_cursor_loop(app);
    schedule_sync_overlays(app, enabled);
}

fn schedule_sync_overlays(app: &AppHandle, visible: bool) {
    let handle = app.clone();
    thread::spawn(move || {
        let handle_for_main = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            sync_overlays(&handle_for_main, visible);
            if visible {
                restore_interactive_focus(&handle_for_main);
            }
        });
    });
}

fn restore_interactive_focus(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.set_focus();
        return;
    }
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_focus();
    }
}

pub fn sync_overlays(app: &AppHandle, visible: bool) {
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
                let _ = win.set_ignore_cursor_events(true);
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
        let url = WebviewUrl::App("index.html".into());
        let result = WebviewWindowBuilder::new(app, &label, url)
            .title("Mouse Trail")
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .background_color(Color(0, 0, 0, 0))
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .visible(false)
            .focused(false)
            .position(logical_x, logical_y)
            .inner_size(layout.logical_size.width, layout.logical_size.height)
            .build();

        if let Ok(win) = result {
            let _ = win.set_ignore_cursor_events(true);
            let _ = win.show();
        }
    }

    if visible {
        if let Some(badge) = app.get_webview_window("badge") {
            let _ = badge.set_always_on_top(true);
        }
    }
}

pub fn monitor_bounds(app: &AppHandle, label: &str) -> Option<MouseTrailMonitorBounds> {
    let index = parse_overlay_index(label)?;
    let monitors = app.available_monitors().ok()?;
    let monitor = monitors.get(index)?;
    let work = monitor.work_area();
    Some(MouseTrailMonitorBounds {
        x: work.position.x,
        y: work.position.y,
        width: work.size.width,
        height: work.size.height,
        scale_factor: monitor.scale_factor(),
    })
}

fn ensure_cursor_loop(app: &AppHandle) {
    if CURSOR_LOOP_STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    let handle = app.clone();
    thread::spawn(move || cursor_loop(handle));
}

fn cursor_loop(app: AppHandle) {
    let mut last: Option<(i32, i32)> = None;
    loop {
        if TRAIL_ENABLED.load(Ordering::Relaxed) {
            if let Some((x, y)) = cursor_pos() {
                if last != Some((x, y)) {
                    last = Some((x, y));
                    let payload = MouseTrailCursor { x, y };
                    let _ = app.emit_filter("app://mouse-trail-cursor", payload, is_overlay_target);
                }
            }
            thread::sleep(Duration::from_millis(33));
            continue;
        }
        last = None;
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(windows)]
fn cursor_pos() -> Option<(i32, i32)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT { x: 0, y: 0 };
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
