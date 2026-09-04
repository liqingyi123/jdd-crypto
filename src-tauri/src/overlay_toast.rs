//! Shared screen-center overlay toast window.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, window::Color,
};

use crate::windows;

pub const OVERLAY_TOAST_LABEL: &str = "overlay-toast";
const OVERLAY_TOAST_MIN_WIDTH: f64 = 320.0;
const OVERLAY_TOAST_MAX_WIDTH: f64 = 1100.0;
const OVERLAY_TOAST_HEIGHT: f64 = 56.0;
const TOAST_VISIBLE_MS: u64 = 1800;
const TOAST_VISIBLE_FIRST_MS: u64 = 2800;

static TOAST_SEQ: AtomicU64 = AtomicU64::new(0);
static PENDING_TOAST: Mutex<Option<String>> = Mutex::new(None);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayToastPayload {
    pub message: String,
}

fn set_pending(message: &str) {
    if let Ok(mut pending) = PENDING_TOAST.lock() {
        *pending = Some(message.to_string());
    }
}

pub fn take_pending() -> Option<String> {
    PENDING_TOAST.lock().ok().and_then(|mut g| g.take())
}

fn toast_window_width(message: &str) -> f64 {
    // ~15px per CJK/latin glyph + horizontal padding; keep single-line room.
    let estimated = message.chars().count() as f64 * 15.0 + 64.0;
    estimated.clamp(OVERLAY_TOAST_MIN_WIDTH, OVERLAY_TOAST_MAX_WIDTH)
}

fn toast_center_origin(app: &AppHandle, logical_width: f64) -> (i32, i32) {
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
        return windows::clamp_popup_origin_public(
            app,
            cursor_x,
            cursor_y,
            logical_width,
            OVERLAY_TOAST_HEIGHT,
            0,
        );
    };
    let work = monitor.work_area();
    let scale = monitor.scale_factor();
    let toast_w = (logical_width * scale).round() as i32;
    let toast_h = (OVERLAY_TOAST_HEIGHT * scale).round() as i32;
    let max_w = work.size.width as i32;
    let toast_w = toast_w.min(max_w.saturating_sub(24));
    let x = work.position.x + (max_w - toast_w) / 2;
    let y = work.position.y + (work.size.height as i32 - toast_h) / 2;
    (x, y)
}

fn emit_toast(app: &AppHandle, message: &str) {
    let payload = OverlayToastPayload {
        message: message.to_string(),
    };
    if let Some(win) = app.get_webview_window(OVERLAY_TOAST_LABEL) {
        let _ = win.emit("app://overlay-toast", &payload);
    }
    let _ = app.emit("app://overlay-toast", &payload);
}

/// Non-blocking from invoke handlers.
///
/// Never call `run_on_main_thread` directly inside a command: the command already
/// runs on the main thread and that wait deadlocks. Escape via a worker thread first.
pub fn schedule_show(app: &AppHandle, message: String) {
    let message = message.trim().to_string();
    if message.is_empty() {
        return;
    }
    set_pending(&message);
    let handle = app.clone();
    thread::spawn(move || {
        let app = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            show_on_main(&app, message);
        });
    });
}

fn show_on_main(app: &AppHandle, message: String) {
    let width = toast_window_width(&message);
    let target = toast_center_origin(app, width);
    let seq = TOAST_SEQ.fetch_add(1, Ordering::Relaxed) + 1;

    if let Some(win) = app.get_webview_window(OVERLAY_TOAST_LABEL) {
        let _ = win.set_size(tauri::LogicalSize::new(width, OVERLAY_TOAST_HEIGHT));
        let _ = win.set_position(PhysicalPosition::new(target.0, target.1));
        let _ = win.set_always_on_top(true);
        let _ = win.set_ignore_cursor_events(true);
        let _ = win.show();
        emit_toast(app, &message);
        let handle = app.clone();
        let msg = message.clone();
        thread::spawn(move || {
            for wait_ms in [60_u64, 180, 360] {
                thread::sleep(Duration::from_millis(wait_ms));
                emit_toast(&handle, &msg);
            }
        });
        schedule_hide(app, seq, TOAST_VISIBLE_MS);
        return;
    }

    let result = WebviewWindowBuilder::new(
        app,
        OVERLAY_TOAST_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("提示")
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
    .inner_size(width, OVERLAY_TOAST_HEIGHT)
    .build();

    match result {
        Ok(win) => {
            let _ = win.set_position(PhysicalPosition::new(target.0, target.1));
            let _ = win.set_always_on_top(true);
            let _ = win.set_ignore_cursor_events(true);
            windows::bind_close_to_hide(&win);
            let handle = app.clone();
            let msg = message.clone();
            thread::spawn(move || {
                for wait_ms in [80_u64, 200, 450, 900, 1400] {
                    thread::sleep(Duration::from_millis(wait_ms));
                    emit_toast(&handle, &msg);
                }
            });
            schedule_hide(app, seq, TOAST_VISIBLE_FIRST_MS);
        }
        Err(_) => {
            let _ = take_pending();
        }
    }
}

fn schedule_hide(app: &AppHandle, seq: u64, wait_ms: u64) {
    let handle = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(wait_ms));
        if TOAST_SEQ.load(Ordering::Relaxed) != seq {
            return;
        }
        let app = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            if let Some(win) = app.get_webview_window(OVERLAY_TOAST_LABEL) {
                let _ = win.hide();
            }
        });
    });
}
