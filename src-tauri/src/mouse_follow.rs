use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::Shortcut;
use tauri_plugin_store::StoreExt;

use crate::clipboard::classify_clipboard;
use crate::state::{AppState, DEFAULT_MOUSE_FOLLOW_SHORTCUT, MIN_BADGE_SIZE};
use crate::windows::{self, CryptoHint};

const SETTINGS_STORE: &str = "settings.json";
const SHORTCUT_KEY: &str = "mouseFollowShortcut";
const PREF_KEY: &str = "mouseFollowEnabled";
const MAX_KEYS: usize = 4;
#[cfg(any(windows, target_os = "macos"))]
const CURSOR_OFFSET: i32 = 16;
#[cfg(any(windows, target_os = "macos"))]
const DRAG_THRESHOLD: i32 = 12;
#[cfg(any(windows, target_os = "macos"))]
const LERP: f64 = 0.35;
#[cfg(any(windows, target_os = "macos"))]
const SETTLE: f64 = 1.0;

pub fn load_shortcut(app: &AppHandle) -> String {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return DEFAULT_MOUSE_FOLLOW_SHORTCUT.to_string();
    };
    store
        .get(SHORTCUT_KEY)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|raw| validate_shortcut(&raw).ok())
        .unwrap_or_else(|| DEFAULT_MOUSE_FOLLOW_SHORTCUT.to_string())
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
        .mouse_follow_pref_enabled
        .store(enabled, Ordering::Relaxed);
    save_pref(app, enabled);
    if enabled {
        let _ = register_current(app);
        return;
    }
    let _ = unregister_all(app);
    if state.mouse_follow_enabled.load(Ordering::Relaxed) {
        stop(app);
    }
}

pub fn validate_shortcut(raw: &str) -> Result<String, String> {
    let parts: Vec<&str> = raw
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 2 || parts.len() > MAX_KEYS {
        return Err("快捷键需为 2–4 个键，且包含修饰键和主键".into());
    }

    let mut has_modifier = false;
    let mut has_key = false;
    for part in &parts {
        if is_modifier(part) {
            has_modifier = true;
        } else {
            has_key = true;
        }
    }
    if !has_modifier || !has_key {
        return Err("至少需要一个修饰键和一个主键".into());
    }

    let normalized = parts.join("+");
    Shortcut::from_str(&normalized).map_err(|err| err.to_string())?;
    Ok(normalized)
}

fn is_modifier(part: &str) -> bool {
    matches!(
        part.to_ascii_lowercase().as_str(),
        "ctrl" | "control" | "shift" | "alt" | "option" | "meta" | "super" | "cmd" | "command"
    )
}

pub fn register_current(app: &AppHandle) -> Result<(), String> {
    crate::global_shortcuts::register_all(app)
}

pub fn unregister_all(app: &AppHandle) -> Result<(), String> {
    crate::global_shortcuts::unregister_all(app)
}

pub fn handle_follow_shortcut(app: &AppHandle, shortcut: &Shortcut) {
    let state = app.state::<AppState>();
    if !state.mouse_follow_pref_enabled.load(Ordering::Relaxed) {
        return;
    }
    let expected = state
        .mouse_follow_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| DEFAULT_MOUSE_FOLLOW_SHORTCUT.to_string());
    let Ok(expected_shortcut) = Shortcut::from_str(&expected) else {
        return;
    };
    if shortcut.id() == expected_shortcut.id() {
        toggle(app);
    }
}

pub fn toggle(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.mouse_follow_pref_enabled.load(Ordering::Relaxed) {
        return;
    }
    if state.mouse_follow_enabled.load(Ordering::Relaxed) {
        stop(app);
    } else {
        start(app);
    }
}

fn start(app: &AppHandle) {
    let state = app.state::<AppState>();
    // Enable first so the follow loop never sees both flags false (which would reset anim).
    state.mouse_follow_enabled.store(true, Ordering::Relaxed);
    let was_returning = state.mouse_follow_returning.swap(false, Ordering::Relaxed);
    if let Some(win) = app.get_webview_window("badge") {
        if !was_returning {
            if let Ok(pos) = win.outer_position() {
                if let Ok(mut saved) = state.saved_badge_pos.lock() {
                    *saved = Some((pos.x, pos.y));
                }
            }
        }
        state.badge_expanded.store(false, Ordering::Relaxed);
        windows::apply_badge_window_size(app, MIN_BADGE_SIZE, false);
        let _ = win.set_ignore_cursor_events(true);
    }
    if let Ok(mut candidate) = state.last_candidate.lock() {
        *candidate = None;
    }
    windows::hide_clipboard_prompt(app);
    let _ = app.emit("app://mouse-follow", true);
}

fn stop(app: &AppHandle) {
    let state = app.state::<AppState>();
    // Returning first so the follow loop keeps `anim` across the handoff.
    state.mouse_follow_returning.store(true, Ordering::Relaxed);
    state.mouse_follow_enabled.store(false, Ordering::Relaxed);
    let _ = app.emit("app://mouse-follow", false);

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        snap_home_and_restore(app);
    }
}

fn restore_badge_after_follow(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Some(win) = app.get_webview_window("badge") {
        let _ = win.set_ignore_cursor_events(false);
    }
    windows::set_badge_size(app, false);
    state.mouse_follow_returning.store(false, Ordering::Relaxed);
}

fn snap_home_and_restore(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Some(win) = app.get_webview_window("badge") {
        if let Ok(saved) = state.saved_badge_pos.lock() {
            if let Some((x, y)) = *saved {
                let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
            }
        }
    }
    restore_badge_after_follow(app);
}

pub fn start_follow_loop(app: AppHandle) {
    thread::spawn(move || {
        let mut anim: Option<(f64, f64)> = None;
        let mut was_down = false;
        let mut press = (0, 0);
        #[cfg(any(windows, target_os = "macos"))]
        let mut last_short_up: Option<(Instant, i32, i32)> = None;

        loop {
            let Some(state) = app.try_state::<AppState>() else {
                thread::sleep(Duration::from_millis(50));
                continue;
            };
            let enabled = state.mouse_follow_enabled.load(Ordering::Relaxed);
            let returning = state.mouse_follow_returning.load(Ordering::Relaxed);
            let compare_active = state.compare_active.load(Ordering::Relaxed);
            if !enabled && !returning && !compare_active {
                anim = None;
                was_down = false;
                #[cfg(any(windows, target_os = "macos"))]
                {
                    last_short_up = None;
                }
                thread::sleep(Duration::from_millis(50));
                continue;
            }

            #[cfg(any(windows, target_os = "macos"))]
            {
                if compare_active {
                    crate::compare_mode::update_tip_position(&app);
                }

                if returning && !enabled {
                    let target = state
                        .saved_badge_pos
                        .lock()
                        .ok()
                        .and_then(|guard| *guard);
                    match target {
                        Some((x, y)) => {
                            let dist = lerp_badge(&app, &mut anim, (f64::from(x), f64::from(y)));
                            if dist <= SETTLE {
                                if let Some(win) = app.get_webview_window("badge") {
                                    let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
                                }
                                restore_badge_after_follow(&app);
                            }
                        }
                        None => snap_home_and_restore(&app),
                    }
                    // Still allow compare selection while badge is returning.
                    if !compare_active {
                        thread::sleep(Duration::from_millis(16));
                        continue;
                    }
                }

                if enabled {
                    let Some(cursor) = cursor_physical_pos(&app) else {
                        thread::sleep(Duration::from_millis(16));
                        continue;
                    };
                    let _ = lerp_badge(
                        &app,
                        &mut anim,
                        (
                            f64::from(cursor.0 + CURSOR_OFFSET),
                            f64::from(cursor.1 + CURSOR_OFFSET),
                        ),
                    );
                }

                if enabled || compare_active {
                    let Some(cursor) = cursor_physical_pos(&app) else {
                        thread::sleep(Duration::from_millis(16));
                        continue;
                    };
                    let down = left_button_down();
                    if down && !was_down {
                        press = cursor;
                    }
                    if !down && was_down {
                        let dx = cursor.0 - press.0;
                        let dy = cursor.1 - press.1;
                        let dragged = dx * dx + dy * dy >= DRAG_THRESHOLD * DRAG_THRESHOLD;
                        if dragged {
                            last_short_up = None;
                            capture_selection(&app);
                        } else if is_double_click(&last_short_up, press) {
                            last_short_up = None;
                            capture_selection(&app);
                        } else {
                            last_short_up = Some((Instant::now(), press.0, press.1));
                        }
                    }
                    was_down = down;
                }
            }

            thread::sleep(Duration::from_millis(16));
        }
    });
}

#[cfg(any(windows, target_os = "macos"))]
fn lerp_badge(app: &AppHandle, anim: &mut Option<(f64, f64)>, target: (f64, f64)) -> f64 {
    let Some(win) = app.get_webview_window("badge") else {
        return 0.0;
    };
    let (target_x, target_y) = target;
    let (current_x, current_y) = match *anim {
        Some(pos) => pos,
        None => win
            .outer_position()
            .map(|pos| (f64::from(pos.x), f64::from(pos.y)))
            .unwrap_or((target_x, target_y)),
    };
    let next_x = current_x + (target_x - current_x) * LERP;
    let next_y = current_y + (target_y - current_y) * LERP;
    *anim = Some((next_x, next_y));

    let x = next_x.round() as i32;
    let y = next_y.round() as i32;
    let unchanged = win
        .outer_position()
        .map(|pos| pos.x == x && pos.y == y)
        .unwrap_or(false);
    if !unchanged {
        let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
    }

    let dx = target_x - next_x;
    let dy = target_y - next_y;
    (dx * dx + dy * dy).sqrt()
}

#[cfg(any(windows, target_os = "macos"))]
fn open_main_with_capture(app: &AppHandle, text: String) {
    if let Ok(mut last) = app.state::<AppState>().last_clipboard.lock() {
        *last = text.clone();
    }

    let kind = classify_clipboard(&text);
    let mode = if kind == "maybe_cipher" {
        "decrypt"
    } else {
        "encrypt"
    };
    let use_bubble = windows::is_short_bubble_text(&text);
    let hint = CryptoHint {
        text,
        mode: mode.to_string(),
    };
    if use_bubble {
        windows::schedule_show_crypto_bubble(app, hint);
    } else {
        windows::show_main(app, Some(hint));
    }
    stop(app);
}

#[cfg(windows)]
fn clipboard_sequence() -> u32 {
    use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;

    // SAFETY: GetClipboardSequenceNumber is a parameterless system query.
    unsafe { GetClipboardSequenceNumber() }
}

#[cfg(windows)]
fn capture_selection_text(app: &AppHandle) -> Option<String> {
    // Slightly longer wait so double-click word selection can settle.
    thread::sleep(Duration::from_millis(70));
    let seq_before = clipboard_sequence();
    send_copy_shortcut();

    let deadline = Instant::now() + Duration::from_millis(450);
    while Instant::now() < deadline {
        thread::sleep(Duration::from_millis(30));
        if clipboard_sequence() == seq_before {
            continue;
        }
        let Ok(current) = app.clipboard().read_text() else {
            continue;
        };
        if current.trim().is_empty() {
            continue;
        }
        return Some(current);
    }
    None
}

#[cfg(target_os = "macos")]
fn capture_selection_text(app: &AppHandle) -> Option<String> {
    thread::sleep(Duration::from_millis(70));
    let before = app.clipboard().read_text().unwrap_or_default();
    send_copy_shortcut();

    let deadline = Instant::now() + Duration::from_millis(450);
    while Instant::now() < deadline {
        thread::sleep(Duration::from_millis(30));
        let Ok(current) = app.clipboard().read_text() else {
            continue;
        };
        if current.trim().is_empty() || current == before {
            continue;
        }
        return Some(current);
    }
    None
}

#[cfg(any(windows, target_os = "macos"))]
fn capture_selection(app: &AppHandle) {
    let Some(text) = capture_selection_text(app) else {
        return;
    };
    if crate::compare_mode::is_active(app) {
        crate::compare_mode::handle_captured_cipher(app, text);
        return;
    }
    open_main_with_capture(app, text);
}

#[cfg(any(windows, target_os = "macos"))]
fn is_double_click(last_short_up: &Option<(Instant, i32, i32)>, press: (i32, i32)) -> bool {
    let Some((prev_at, prev_x, prev_y)) = *last_short_up else {
        return false;
    };
    if prev_at.elapsed() > double_click_interval() {
        return false;
    }
    let (tol_x, tol_y) = double_click_tolerance();
    let dx = (press.0 - prev_x).abs();
    let dy = (press.1 - prev_y).abs();
    dx <= tol_x && dy <= tol_y
}

#[cfg(windows)]
fn double_click_interval() -> Duration {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime;

    // SAFETY: GetDoubleClickTime is a parameterless system query.
    let ms = unsafe { GetDoubleClickTime() };
    Duration::from_millis(u64::from(ms.max(1)))
}

#[cfg(target_os = "macos")]
fn double_click_interval() -> Duration {
    Duration::from_millis(500)
}

#[cfg(windows)]
fn double_click_tolerance() -> (i32, i32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXDOUBLECLK, SM_CYDOUBLECLK,
    };

    // SAFETY: GetSystemMetrics with documented SM_* constants is read-only.
    let cx = unsafe { GetSystemMetrics(SM_CXDOUBLECLK) };
    let cy = unsafe { GetSystemMetrics(SM_CYDOUBLECLK) };
    (cx.max(4), cy.max(4))
}

#[cfg(target_os = "macos")]
fn double_click_tolerance() -> (i32, i32) {
    (5, 5)
}

#[cfg(any(windows, target_os = "macos"))]
fn cursor_physical_pos(app: &AppHandle) -> Option<(i32, i32)> {
    let (x, y) = cursor_pos()?;
    #[cfg(windows)]
    {
        let _ = app;
        return Some((x, y));
    }
    #[cfg(target_os = "macos")]
    {
        let scale = app
            .get_webview_window("badge")
            .and_then(|win| win.scale_factor().ok())
            .or_else(|| {
                app.primary_monitor()
                    .ok()
                    .flatten()
                    .map(|monitor| monitor.scale_factor())
            })
            .unwrap_or(1.0);
        Some((
            (f64::from(x) * scale).round() as i32,
            (f64::from(y) * scale).round() as i32,
        ))
    }
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

#[cfg(target_os = "macos")]
fn cursor_pos() -> Option<(i32, i32)> {
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok()?;
    let event = CGEvent::new(source).ok()?;
    let loc = event.location();
    Some((loc.x.round() as i32, loc.y.round() as i32))
}

#[cfg(windows)]
fn left_button_down() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};

    // SAFETY: GetAsyncKeyState is a read-only input query with a well-known VK code.
    unsafe { GetAsyncKeyState(i32::from(VK_LBUTTON)) as u16 & 0x8000 != 0 }
}

#[cfg(target_os = "macos")]
fn left_button_down() -> bool {
    use core_graphics::event::CGMouseButton;
    use core_graphics::event_source::CGEventSourceStateID;

    extern "C" {
        fn CGEventSourceButtonState(
            state_id: CGEventSourceStateID,
            button: CGMouseButton,
        ) -> bool;
    }

    // SAFETY: 只读查询当前鼠标按键状态，无内存副作用。
    unsafe {
        CGEventSourceButtonState(
            CGEventSourceStateID::CombinedSessionState,
            CGMouseButton::Left,
        )
    }
}

#[cfg(windows)]
fn send_copy_shortcut() {
    use std::mem::{size_of, zeroed};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_C, VK_CONTROL,
    };

    fn key_event(vk: u16, flags: u32) -> INPUT {
        // SAFETY: INPUT is a C union; zeroed then filled for INPUT_KEYBOARD is the Win32 pattern.
        let mut input: INPUT = unsafe { zeroed() };
        input.r#type = INPUT_KEYBOARD;
        input.Anonymous.ki = KEYBDINPUT {
            wVk: vk,
            wScan: 0,
            dwFlags: flags,
            time: 0,
            dwExtraInfo: 0,
        };
        input
    }

    let inputs = [
        key_event(VK_CONTROL as u16, 0),
        key_event(VK_C as u16, 0),
        key_event(VK_C as u16, KEYEVENTF_KEYUP),
        key_event(VK_CONTROL as u16, KEYEVENTF_KEYUP),
    ];
    unsafe {
        // SAFETY: `inputs` is a contiguous array of fully initialized INPUT_KEYBOARD structs.
        SendInput(inputs.len() as u32, inputs.as_ptr(), size_of::<INPUT>() as i32);
    }
}

#[cfg(target_os = "macos")]
fn send_copy_shortcut() {
    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    const KEY_C: CGKeyCode = 8;
    let Ok(source) = CGEventSource::new(CGEventSourceStateID::CombinedSessionState) else {
        return;
    };
    let flags = CGEventFlags::CGEventFlagCommand;
    if let Ok(down) = CGEvent::new_keyboard_event(source.clone(), KEY_C, true) {
        down.set_flags(flags);
        down.post(CGEventTapLocation::HID);
    }
    if let Ok(up) = CGEvent::new_keyboard_event(source, KEY_C, false) {
        up.set_flags(flags);
        up.post(CGEventTapLocation::HID);
    }
}
