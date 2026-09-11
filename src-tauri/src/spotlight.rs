//! Multi-monitor spotlight overlay: dim mask + cursor hole, shortcut / shake to open.

use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, EventTarget, LogicalSize, Manager, PhysicalPosition, WebviewUrl,
    WebviewWindowBuilder, window::Color,
};
use tauri_plugin_global_shortcut::Shortcut;
use tauri_plugin_store::StoreExt;

use crate::global_shortcuts;
use crate::state::{
    AppState, DEFAULT_SPOTLIGHT_SHORTCUT, DEFAULT_SPOTLIGHT_SIZE, MAX_SPOTLIGHT_SIZE,
    MIN_SPOTLIGHT_SIZE,
};

const MAX_OVERLAYS: usize = 8;
const SETTINGS_STORE: &str = "settings.json";
const SHORTCUT_KEY: &str = "spotlightShortcut";
const PREF_KEY: &str = "spotlightEnabled";
const SHAKE_KEY: &str = "spotlightShakeEnabled";
const SIZE_KEY: &str = "spotlightSize";
const CURSOR_INTERVAL: Duration = Duration::from_millis(16);
const KEY_ARM_DELAY: Duration = Duration::from_millis(350);
const SHAKE_WINDOW: Duration = Duration::from_millis(800);
const SHAKE_MIN_DELTA: i32 = 80;
const SHAKE_REVERSALS_NEEDED: u8 = 4;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static KEY_EXIT_ARMED: AtomicBool = AtomicBool::new(false);
static LOOP_STARTED: AtomicBool = AtomicBool::new(false);
static APP_FOR_SPOTLIGHT: OnceLock<AppHandle> = OnceLock::new();

#[cfg(windows)]
static KEYBOARD_HOOK: AtomicIsize = AtomicIsize::new(0);
#[cfg(windows)]
static HOOK_THREAD_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
#[cfg(any(windows, target_os = "macos"))]
static KEY_HOOK_RUNNING: AtomicBool = AtomicBool::new(false);
static KEYS_DOWN: OnceLock<Mutex<[bool; 256]>> = OnceLock::new();

fn keys_down_map() -> &'static Mutex<[bool; 256]> {
    KEYS_DOWN.get_or_init(|| Mutex::new([false; 256]))
}

fn clear_keys_down() {
    if let Ok(mut map) = keys_down_map().lock() {
        map.fill(false);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotlightCursor {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotlightMonitorBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotlightPrefPayload {
    pub size: u32,
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
    format!("spotlight-{index}")
}

fn parse_overlay_index(label: &str) -> Option<usize> {
    label
        .strip_prefix("spotlight-")?
        .parse::<usize>()
        .ok()
        .filter(|index| *index < MAX_OVERLAYS)
}

fn is_overlay_target(target: &EventTarget) -> bool {
    match target {
        EventTarget::WebviewWindow { label }
        | EventTarget::Webview { label }
        | EventTarget::Window { label }
        | EventTarget::AnyLabel { label } => label.starts_with("spotlight-"),
        _ => false,
    }
}

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

pub fn load_shortcut(app: &AppHandle) -> String {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return DEFAULT_SPOTLIGHT_SHORTCUT.to_string();
    };
    store
        .get(SHORTCUT_KEY)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|raw| crate::mouse_follow::validate_shortcut(&raw).ok())
        .unwrap_or_else(|| DEFAULT_SPOTLIGHT_SHORTCUT.to_string())
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

pub fn load_shake_pref(app: &AppHandle) -> bool {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return true;
    };
    store
        .get(SHAKE_KEY)
        .and_then(|value| value.as_bool())
        .unwrap_or(true)
}

pub fn save_shake_pref(app: &AppHandle, enabled: bool) {
    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(SHAKE_KEY, serde_json::json!(enabled));
        let _ = store.save();
    }
}

pub fn normalize_size(size: u32) -> u32 {
    size.clamp(MIN_SPOTLIGHT_SIZE, MAX_SPOTLIGHT_SIZE)
}

pub fn load_size(app: &AppHandle) -> u32 {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return DEFAULT_SPOTLIGHT_SIZE;
    };
    store
        .get(SIZE_KEY)
        .and_then(|value| value.as_u64())
        .map(|v| normalize_size(v as u32))
        .unwrap_or(DEFAULT_SPOTLIGHT_SIZE)
}

pub fn save_size(app: &AppHandle, size: u32) {
    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(SIZE_KEY, serde_json::json!(size));
        let _ = store.save();
    }
}

fn emit_pref(app: &AppHandle) {
    let size = app
        .state::<AppState>()
        .spotlight_size
        .load(Ordering::Relaxed);
    let payload = SpotlightPrefPayload { size };
    let _ = app.emit_filter("app://spotlight-pref", payload, is_overlay_target);
}

pub fn apply_size(app: &AppHandle, size: u32) -> u32 {
    let normalized = normalize_size(size);
    let state = app.state::<AppState>();
    state.spotlight_size.store(normalized, Ordering::Relaxed);
    save_size(app, normalized);
    emit_pref(app);
    normalized
}

pub fn apply_pref(app: &AppHandle, enabled: bool) {
    let state = app.state::<AppState>();
    state
        .spotlight_pref_enabled
        .store(enabled, Ordering::Relaxed);
    save_pref(app, enabled);
    if !enabled {
        hide(app);
    }
    ensure_loop(app);
    let _ = global_shortcuts::register_all(app);
}

pub fn apply_shake_pref(app: &AppHandle, enabled: bool) {
    let state = app.state::<AppState>();
    state
        .spotlight_shake_enabled
        .store(enabled, Ordering::Relaxed);
    save_shake_pref(app, enabled);
    ensure_loop(app);
}

pub fn handle_shortcut(app: &AppHandle, shortcut: &Shortcut) {
    let state = app.state::<AppState>();
    if !state.spotlight_pref_enabled.load(Ordering::Relaxed) {
        return;
    }
    if crate::screensaver::is_active() {
        return;
    }
    let expected = state
        .spotlight_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| DEFAULT_SPOTLIGHT_SHORTCUT.to_string());
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
    if is_active() {
        return;
    }
    if crate::screensaver::is_active() {
        return;
    }
    crate::mouse_trail::pause_for_screensaver(app);
    ACTIVE.store(true, Ordering::Relaxed);
    KEY_EXIT_ARMED.store(false, Ordering::Relaxed);
    let state = app.state::<AppState>();
    state.spotlight_active.store(true, Ordering::Relaxed);
    ensure_loop(app);
    sync_overlays(app, true);
    emit_pref(app);
    arm_key_exit_later(app);
    start_keyboard_hook();
}

pub fn hide(app: &AppHandle) {
    if !is_active() && !app.state::<AppState>().spotlight_active.load(Ordering::Relaxed) {
        KEY_EXIT_ARMED.store(false, Ordering::Relaxed);
        stop_keyboard_hook();
        return;
    }
    KEY_EXIT_ARMED.store(false, Ordering::Relaxed);
    stop_keyboard_hook();
    clear_keys_down();
    ACTIVE.store(false, Ordering::Relaxed);
    let state = app.state::<AppState>();
    state.spotlight_active.store(false, Ordering::Relaxed);
    sync_overlays(app, false);
    crate::mouse_trail::resume_after_screensaver(app);
}

pub fn on_display_changed(app: &AppHandle) {
    if !is_active() {
        return;
    }
    sync_overlays(app, true);
}

pub fn monitor_bounds(app: &AppHandle, label: &str) -> Option<SpotlightMonitorBounds> {
    let index = parse_overlay_index(label)?;
    let monitors = app.available_monitors().ok()?;
    let monitor = monitors.get(index)?;
    let size = *monitor.size();
    let pos = *monitor.position();
    Some(SpotlightMonitorBounds {
        x: pos.x,
        y: pos.y,
        width: size.width,
        height: size.height,
        scale_factor: monitor.scale_factor(),
    })
}

pub fn init_from_store(app: &AppHandle) {
    let _ = APP_FOR_SPOTLIGHT.set(app.clone());
    let state = app.state::<AppState>();
    let pref = load_pref(app);
    let shake = load_shake_pref(app);
    let shortcut = load_shortcut(app);
    let size = load_size(app);
    state
        .spotlight_pref_enabled
        .store(pref, Ordering::Relaxed);
    state
        .spotlight_shake_enabled
        .store(shake, Ordering::Relaxed);
    state.spotlight_size.store(size, Ordering::Relaxed);
    if let Ok(mut current) = state.spotlight_shortcut.lock() {
        *current = shortcut;
    }
    crate::mouse_trail::ensure_display_listener(app);
    ensure_loop(app);
}

fn ensure_loop(app: &AppHandle) {
    let _ = APP_FOR_SPOTLIGHT.set(app.clone());
    if LOOP_STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    let handle = app.clone();
    thread::spawn(move || cursor_and_shake_loop(handle));
}

fn arm_key_exit_later(app: &AppHandle) {
    let handle = app.clone();
    thread::spawn(move || {
        thread::sleep(KEY_ARM_DELAY);
        // Wait until activation chord is fully released to avoid auto-repeat exit.
        while is_active() && any_physical_key_down() {
            thread::sleep(Duration::from_millis(20));
        }
        clear_keys_down();
        if is_active() {
            KEY_EXIT_ARMED.store(true, Ordering::Relaxed);
        }
        let _ = handle;
    });
}

#[cfg(windows)]
fn any_physical_key_down() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

    // High bit set => key currently down. Scan common VKs + modifiers.
    const VKS: &[i32] = &[
        0x08, 0x09, 0x0D, 0x10, 0x11, 0x12, 0x1B, 0x20, // backspace tab enter shift ctrl alt esc space
        0x25, 0x26, 0x27, 0x28, // arrows
        0x2D, 0x2E, // insert delete
        0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, // L/R shift ctrl alt
    ];
    unsafe {
        for vk in VKS {
            if GetAsyncKeyState(*vk) as u16 & 0x8000 != 0 {
                return true;
            }
        }
        for vk in 0x30..=0x5A {
            // 0-9 A-Z
            if GetAsyncKeyState(vk) as u16 & 0x8000 != 0 {
                return true;
            }
        }
        for vk in 0x70..=0x7B {
            // F1-F12
            if GetAsyncKeyState(vk) as u16 & 0x8000 != 0 {
                return true;
            }
        }
    }
    false
}

#[cfg(not(windows))]
fn any_physical_key_down() -> bool {
    false
}

fn request_hide_from_hook() {
    if !KEY_EXIT_ARMED.swap(false, Ordering::Relaxed) {
        return;
    }
    if !is_active() {
        return;
    }
    let Some(app) = APP_FOR_SPOTLIGHT.get() else {
        KEY_EXIT_ARMED.store(true, Ordering::Relaxed);
        return;
    };
    let handle = app.clone();
    // Never block the LL-hook thread; hide may take the keys-down lock on main.
    thread::spawn(move || {
        let _ = handle.clone().run_on_main_thread(move || {
            hide(&handle);
        });
    });
}

fn sync_overlays(app: &AppHandle, visible: bool) {
    let monitors = app.available_monitors().unwrap_or_default();
    for index in 0..MAX_OVERLAYS {
        let label = overlay_label(index);
        if !visible || index >= monitors.len() {
            if let Some(win) = app.get_webview_window(&label) {
                let _ = win.close();
            }
            continue;
        }

        let layout = monitor_layout(&monitors[index]);

        if let Some(win) = app.get_webview_window(&label) {
            let _ = win.set_position(layout.physical_pos);
            let _ = win.set_size(layout.logical_size);
            let _ = win.set_ignore_cursor_events(true);
            let _ = win.set_always_on_top(true);
            let _ = win.show();
            continue;
        }

        let (logical_x, logical_y) = layout.logical_pos;
        let result = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
            .title("聚光灯")
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
            let _ = win.set_always_on_top(true);
            let _ = win.show();
        }
    }
}

fn cursor_and_shake_loop(app: AppHandle) {
    let mut last_emit: Option<(i32, i32)> = None;
    let mut last_pos: Option<(i32, i32)> = None;
    let mut axis: i8 = 0; // 1 = horizontal, 2 = vertical
    let mut direction: i8 = 0;
    let mut reversals: u8 = 0;
    let mut window_start: Option<Instant> = None;
    let mut accum: i32 = 0;
    let mut prev_any_key = false;

    loop {
        let state = app.state::<AppState>();
        let pref = state.spotlight_pref_enabled.load(Ordering::Relaxed);
        let shake = state.spotlight_shake_enabled.load(Ordering::Relaxed);
        let active = is_active();

        if !pref {
            last_emit = None;
            prev_any_key = false;
            reset_shake(
                &mut last_pos,
                &mut axis,
                &mut direction,
                &mut reversals,
                &mut window_start,
                &mut accum,
            );
            thread::sleep(Duration::from_millis(200));
            continue;
        }

        if active {
            reset_shake(
                &mut last_pos,
                &mut axis,
                &mut direction,
                &mut reversals,
                &mut window_start,
                &mut accum,
            );
            if let Some((x, y)) = crate::windows::cursor_pos_public() {
                if last_emit != Some((x, y)) {
                    last_emit = Some((x, y));
                    let payload = SpotlightCursor { x, y };
                    let _ = app.emit_filter("app://spotlight-cursor", payload, is_overlay_target);
                }
            }
            // Fallback when LL hook is dead: rising edge of any key while armed.
            if KEY_EXIT_ARMED.load(Ordering::Relaxed) {
                let down = any_physical_key_down();
                if down && !prev_any_key {
                    request_hide_from_hook();
                }
                prev_any_key = down;
            } else {
                prev_any_key = any_physical_key_down();
            }
            thread::sleep(CURSOR_INTERVAL);
            continue;
        }

        last_emit = None;
        prev_any_key = false;
        if !shake || crate::screensaver::is_active() {
            reset_shake(
                &mut last_pos,
                &mut axis,
                &mut direction,
                &mut reversals,
                &mut window_start,
                &mut accum,
            );
            thread::sleep(Duration::from_millis(100));
            continue;
        }

        if let Some((x, y)) = crate::windows::cursor_pos_public() {
            if let Some((px, py)) = last_pos {
                let dx = x - px;
                let dy = y - py;
                if dx.abs() >= 2 || dy.abs() >= 2 {
                    let use_x = dx.abs() >= dy.abs();
                    let next_axis: i8 = if use_x { 1 } else { 2 };
                    let delta = if use_x { dx } else { dy };
                    let next_dir: i8 = if delta > 0 { 1 } else { -1 };

                    if axis != 0 && next_axis != axis {
                        // Dominant axis changed — restart stroke accum on new axis.
                        axis = next_axis;
                        direction = next_dir;
                        accum = delta;
                        if window_start.is_none() {
                            window_start = Some(Instant::now());
                        }
                    } else {
                        axis = next_axis;
                        accum += delta;
                        if direction == 0 {
                            direction = next_dir;
                            window_start = Some(Instant::now());
                            accum = delta;
                        } else if next_dir != direction {
                            if accum.abs() >= SHAKE_MIN_DELTA {
                                reversals = reversals.saturating_add(1);
                                direction = next_dir;
                                accum = delta;
                                let expired = window_start
                                    .map(|start| start.elapsed() > SHAKE_WINDOW)
                                    .unwrap_or(true);
                                if expired {
                                    reversals = 1;
                                    window_start = Some(Instant::now());
                                } else if reversals >= SHAKE_REVERSALS_NEEDED {
                                    reset_shake(
                                        &mut last_pos,
                                        &mut axis,
                                        &mut direction,
                                        &mut reversals,
                                        &mut window_start,
                                        &mut accum,
                                    );
                                    let handle = app.clone();
                                    let _ = handle.clone().run_on_main_thread(move || {
                                        show(&handle);
                                    });
                                    thread::sleep(Duration::from_millis(400));
                                    continue;
                                }
                            } else {
                                direction = next_dir;
                                accum = delta;
                            }
                        }
                    }
                }
            }
            last_pos = Some((x, y));
            if let Some(start) = window_start {
                if start.elapsed() > SHAKE_WINDOW {
                    reset_shake(
                        &mut last_pos,
                        &mut axis,
                        &mut direction,
                        &mut reversals,
                        &mut window_start,
                        &mut accum,
                    );
                    last_pos = Some((x, y));
                }
            }
        }
        thread::sleep(Duration::from_millis(20));
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

#[cfg(windows)]
fn start_keyboard_hook() {
    if KEY_HOOK_RUNNING.swap(true, Ordering::Relaxed) {
        return;
    }
    clear_keys_down();
    thread::spawn(|| {
        use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
        use windows_sys::Win32::System::Threading::GetCurrentThreadId;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
            UnhookWindowsHookEx, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
            WM_SYSKEYDOWN, WM_SYSKEYUP,
        };

        const LLKHF_UP: u32 = 0x80;

        unsafe extern "system" fn hook_proc(
            code: i32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            if code >= 0 {
                let msg = wparam as u32;
                let info = &*(lparam as *const KBDLLHOOKSTRUCT);
                let vk = (info.vkCode & 0xFF) as usize;
                let is_up = msg == WM_KEYUP
                    || msg == WM_SYSKEYUP
                    || (info.flags & LLKHF_UP) != 0;
                if let Ok(mut map) = keys_down_map().lock() {
                    if is_up {
                        map[vk] = false;
                    } else if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                        let was_down = map[vk];
                        map[vk] = true;
                        // Ignore auto-repeat while the same key stays down.
                        if !was_down {
                            drop(map);
                            request_hide_from_hook();
                        }
                    }
                }
            }
            CallNextHookEx(
                KEYBOARD_HOOK.load(Ordering::Relaxed) as _,
                code,
                wparam,
                lparam,
            )
        }

        unsafe {
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), std::ptr::null_mut(), 0);
            if hook.is_null() {
                KEY_HOOK_RUNNING.store(false, Ordering::Relaxed);
                return;
            }
            KEYBOARD_HOOK.store(hook as isize, Ordering::Relaxed);
            HOOK_THREAD_ID.store(GetCurrentThreadId(), Ordering::Relaxed);
            let mut msg = std::mem::zeroed::<MSG>();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let current = KEYBOARD_HOOK.swap(0, Ordering::Relaxed);
            if current != 0 {
                let _ = UnhookWindowsHookEx(current as _);
            }
            HOOK_THREAD_ID.store(0, Ordering::Relaxed);
            KEY_HOOK_RUNNING.store(false, Ordering::Relaxed);
        }
    });
}

#[cfg(windows)]
fn stop_keyboard_hook() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        PostThreadMessageW, UnhookWindowsHookEx, WM_QUIT,
    };

    let hook = KEYBOARD_HOOK.swap(0, Ordering::Relaxed);
    if hook != 0 {
        unsafe {
            let _ = UnhookWindowsHookEx(hook as _);
        }
    }
    let tid = HOOK_THREAD_ID.swap(0, Ordering::Relaxed);
    if tid != 0 {
        unsafe {
            let _ = PostThreadMessageW(tid, WM_QUIT, 0, 0);
        }
    }
}

#[cfg(target_os = "macos")]
fn start_keyboard_hook() {
    if KEY_HOOK_RUNNING.swap(true, Ordering::Relaxed) {
        return;
    }
    thread::spawn(|| {
        use core_graphics::event_source::CGEventSourceStateID;

        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGEventSourceKeyState(state_id: CGEventSourceStateID, key: u16) -> bool;
        }

        let mut prev = [false; 128];
        while is_active() {
            if KEY_EXIT_ARMED.load(Ordering::Relaxed) {
                for key in 0u16..128 {
                    let down = unsafe {
                        CGEventSourceKeyState(CGEventSourceStateID::CombinedSessionState, key)
                    };
                    if down && !prev[key as usize] {
                        request_hide_from_hook();
                        break;
                    }
                    prev[key as usize] = down;
                }
            } else {
                prev = [false; 128];
            }
            thread::sleep(Duration::from_millis(16));
        }
        KEY_HOOK_RUNNING.store(false, Ordering::Relaxed);
    });
}

#[cfg(target_os = "macos")]
fn stop_keyboard_hook() {
    // Poller exits when ACTIVE is cleared.
}

#[cfg(not(any(windows, target_os = "macos")))]
fn start_keyboard_hook() {}

#[cfg(not(any(windows, target_os = "macos")))]
fn stop_keyboard_hook() {}
