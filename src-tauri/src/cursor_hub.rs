//! Unified global cursor sampling for mouse-trail + spotlight overlays.
//!
//! WebViews stay click-through (`ignore_cursor_events`), so they cannot use DOM
//! mousemove. Coordinates live in atomics; active-monitor overlays rAF-peek.
//! Shake detection runs at a low rate with no WebView traffic.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, EventTarget, Manager};

const INTERVAL_IDLE_MS: u64 = 200;
const INTERVAL_SHAKE_MS: u64 = 50;
const INTERVAL_SPOTLIGHT_PAINT_MS: u64 = 16;
const DEFAULT_PAINT_MS: u64 = 22;
const SHAKE_WINDOW: Duration = Duration::from_millis(800);
const SHAKE_MIN_DELTA: i32 = 80;
const SHAKE_REVERSALS_NEEDED: u8 = 4;

static STARTED: AtomicBool = AtomicBool::new(false);
static LOOP_THREAD: OnceLock<thread::Thread> = OnceLock::new();
static APP: OnceLock<AppHandle> = OnceLock::new();

static CURSOR_X: AtomicI32 = AtomicI32::new(0);
static CURSOR_Y: AtomicI32 = AtomicI32::new(0);
static CURSOR_SEQ: AtomicU64 = AtomicU64::new(0);
static CURSOR_MONITOR: AtomicI32 = AtomicI32::new(0);
static PAINT_INTERVAL_MS: AtomicU64 = AtomicU64::new(DEFAULT_PAINT_MS);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorHubSnapshot {
    pub x: i32,
    pub y: i32,
    pub seq: u64,
    pub monitor: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CursorMonitorPayload {
    index: i32,
}

fn is_overlay_target(target: &EventTarget) -> bool {
    match target {
        EventTarget::WebviewWindow { label }
        | EventTarget::Webview { label }
        | EventTarget::Window { label }
        | EventTarget::AnyLabel { label } => {
            label.starts_with("mouse-trail-") || label.starts_with("spotlight-")
        }
        _ => false,
    }
}

pub fn peek() -> CursorHubSnapshot {
    CursorHubSnapshot {
        x: CURSOR_X.load(Ordering::Relaxed),
        y: CURSOR_Y.load(Ordering::Relaxed),
        seq: CURSOR_SEQ.load(Ordering::Relaxed),
        monitor: CURSOR_MONITOR.load(Ordering::Relaxed),
    }
}

pub fn set_paint_interval_ms(ms: u64) {
    PAINT_INTERVAL_MS.store(ms.max(8), Ordering::Relaxed);
    wake();
}

pub fn ensure_started(app: &AppHandle) {
    let _ = APP.set(app.clone());
    if STARTED.swap(true, Ordering::Relaxed) {
        wake();
        return;
    }
    let handle = app.clone();
    thread::spawn(move || hub_loop(handle));
}

pub fn wake() {
    if let Some(handle) = LOOP_THREAD.get() {
        handle.unpark();
    }
}

fn hub_needed(app: &AppHandle) -> bool {
    let trail = crate::mouse_trail::is_visually_active();
    let state = app.state::<crate::state::AppState>();
    let spotlight_pref = state.spotlight_pref_enabled.load(Ordering::Relaxed);
    trail || spotlight_pref
}

fn paint_needed(_app: &AppHandle) -> bool {
    let trail = crate::mouse_trail::is_visually_active();
    let spotlight_active = crate::spotlight::is_active();
    trail || spotlight_active
}

fn shake_needed(app: &AppHandle) -> bool {
    let state = app.state::<crate::state::AppState>();
    state.spotlight_pref_enabled.load(Ordering::Relaxed)
        && state.spotlight_shake_enabled.load(Ordering::Relaxed)
        && !crate::spotlight::is_active()
        && !crate::screensaver::is_active()
}

fn resolve_monitor_index(app: &AppHandle, x: i32, y: i32) -> i32 {
    let monitors = app.available_monitors().unwrap_or_default();
    for (index, monitor) in monitors.iter().enumerate() {
        let pos = *monitor.position();
        let size = *monitor.size();
        let right = pos.x.saturating_add(size.width as i32);
        let bottom = pos.y.saturating_add(size.height as i32);
        if x >= pos.x && x < right && y >= pos.y && y < bottom {
            return index as i32;
        }
    }
    0
}

fn store_cursor(app: &AppHandle, x: i32, y: i32, last_monitor: &mut i32) {
    let prev_x = CURSOR_X.load(Ordering::Relaxed);
    let prev_y = CURSOR_Y.load(Ordering::Relaxed);
    if prev_x != x || prev_y != y {
        CURSOR_X.store(x, Ordering::Relaxed);
        CURSOR_Y.store(y, Ordering::Relaxed);
        CURSOR_SEQ.fetch_add(1, Ordering::Relaxed);
    }
    let monitor = resolve_monitor_index(app, x, y);
    if monitor != *last_monitor {
        *last_monitor = monitor;
        CURSOR_MONITOR.store(monitor, Ordering::Relaxed);
        let payload = CursorMonitorPayload { index: monitor };
        let _ = app.emit_filter("app://cursor-monitor", payload, is_overlay_target);
    } else if CURSOR_MONITOR.load(Ordering::Relaxed) != monitor {
        CURSOR_MONITOR.store(monitor, Ordering::Relaxed);
    }
}

fn reset_shake(
    last_pos: &mut Option<(i32, i32)>,
    axis: &mut i8,
    direction: &mut i8,
    reversals: &mut u8,
    window_start: &mut Option<Instant>,
    accum: &mut i32,
) {
    *last_pos = None;
    *axis = 0;
    *direction = 0;
    *reversals = 0;
    *window_start = None;
    *accum = 0;
}

fn process_shake(
    app: &AppHandle,
    x: i32,
    y: i32,
    last_pos: &mut Option<(i32, i32)>,
    axis: &mut i8,
    direction: &mut i8,
    reversals: &mut u8,
    window_start: &mut Option<Instant>,
    accum: &mut i32,
) -> bool {
    let mut triggered = false;
    if let Some((px, py)) = *last_pos {
        let dx = x - px;
        let dy = y - py;
        if dx.abs() >= 2 || dy.abs() >= 2 {
            let use_x = dx.abs() >= dy.abs();
            let next_axis: i8 = if use_x { 1 } else { 2 };
            let delta = if use_x { dx } else { dy };
            let next_dir: i8 = if delta > 0 { 1 } else { -1 };

            if *axis != 0 && next_axis != *axis {
                *axis = next_axis;
                *direction = next_dir;
                *accum = delta;
                if window_start.is_none() {
                    *window_start = Some(Instant::now());
                }
            } else {
                *axis = next_axis;
                *accum += delta;
                if *direction == 0 {
                    *direction = next_dir;
                    *window_start = Some(Instant::now());
                    *accum = delta;
                } else if next_dir != *direction {
                    if accum.abs() >= SHAKE_MIN_DELTA {
                        *reversals = reversals.saturating_add(1);
                        *direction = next_dir;
                        *accum = delta;
                        let expired = window_start
                            .map(|start| start.elapsed() > SHAKE_WINDOW)
                            .unwrap_or(true);
                        if expired {
                            *reversals = 1;
                            *window_start = Some(Instant::now());
                        } else if *reversals >= SHAKE_REVERSALS_NEEDED {
                            reset_shake(last_pos, axis, direction, reversals, window_start, accum);
                            let handle = app.clone();
                            let _ = handle.clone().run_on_main_thread(move || {
                                crate::spotlight::show(&handle);
                            });
                            triggered = true;
                        }
                    } else {
                        *direction = next_dir;
                        *accum = delta;
                    }
                }
            }
        }
    }
    *last_pos = Some((x, y));
    if let Some(start) = *window_start {
        if start.elapsed() > SHAKE_WINDOW {
            reset_shake(last_pos, axis, direction, reversals, window_start, accum);
            *last_pos = Some((x, y));
        }
    }
    triggered
}

fn hub_loop(app: AppHandle) {
    let _ = LOOP_THREAD.set(thread::current());
    let mut last_monitor = -1_i32;
    let mut last_pos: Option<(i32, i32)> = None;
    let mut axis: i8 = 0;
    let mut direction: i8 = 0;
    let mut reversals: u8 = 0;
    let mut window_start: Option<Instant> = None;
    let mut accum: i32 = 0;
    let mut prev_any_key = false;

    loop {
        if !hub_needed(&app) {
            reset_shake(
                &mut last_pos,
                &mut axis,
                &mut direction,
                &mut reversals,
                &mut window_start,
                &mut accum,
            );
            prev_any_key = false;
            last_monitor = -1;
            while !hub_needed(&app) {
                thread::park_timeout(Duration::from_millis(INTERVAL_IDLE_MS));
            }
            continue;
        }

        let painting = paint_needed(&app);
        let shaking = shake_needed(&app);

        if let Some((x, y)) = crate::windows::cursor_pos_public() {
            store_cursor(&app, x, y, &mut last_monitor);

            if shaking {
                if process_shake(
                    &app,
                    x,
                    y,
                    &mut last_pos,
                    &mut axis,
                    &mut direction,
                    &mut reversals,
                    &mut window_start,
                    &mut accum,
                ) {
                    thread::sleep(Duration::from_millis(400));
                    continue;
                }
            } else {
                reset_shake(
                    &mut last_pos,
                    &mut axis,
                    &mut direction,
                    &mut reversals,
                    &mut window_start,
                    &mut accum,
                );
            }
        }

        // Spotlight key-exit fallback while active (LL hook may miss).
        if crate::spotlight::is_active() {
            if crate::spotlight::key_exit_armed() {
                let down = crate::spotlight::any_physical_key_down_public();
                if down && !prev_any_key {
                    crate::spotlight::request_hide_from_hook_public();
                }
                prev_any_key = down;
            } else {
                prev_any_key = crate::spotlight::any_physical_key_down_public();
            }
        } else {
            prev_any_key = false;
        }

        let interval = if crate::spotlight::is_active() {
            INTERVAL_SPOTLIGHT_PAINT_MS
        } else if painting {
            PAINT_INTERVAL_MS.load(Ordering::Relaxed).max(1)
        } else if shaking {
            INTERVAL_SHAKE_MS
        } else {
            INTERVAL_IDLE_MS
        };

        thread::sleep(Duration::from_millis(interval));
    }
}
