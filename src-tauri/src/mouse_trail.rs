use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, EventTarget, LogicalSize, Manager, PhysicalPosition, WebviewUrl,
    WebviewWindowBuilder, window::Color,
};

const MAX_OVERLAYS: usize = 8;

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

pub fn set_enabled(app: &AppHandle, enabled: bool) {
    TRAIL_ENABLED.store(enabled, Ordering::Relaxed);
    sync_overlays(app, enabled);
    ensure_cursor_loop(app);
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
            .visible(true)
            .focused(false)
            .position(logical_x, logical_y)
            .inner_size(layout.logical_size.width, layout.logical_size.height)
            .build();

        if let Ok(win) = result {
            let _ = win.set_ignore_cursor_events(true);
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
