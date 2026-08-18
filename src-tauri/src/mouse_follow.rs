use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use tauri_plugin_store::StoreExt;

use crate::clipboard::classify_clipboard;
use crate::state::{AppState, DEFAULT_MOUSE_FOLLOW_SHORTCUT, MIN_BADGE_SIZE};
use crate::windows::{self, CryptoHint};

const SETTINGS_STORE: &str = "settings.json";
const SHORTCUT_KEY: &str = "mouseFollowShortcut";
const PREF_KEY: &str = "mouseFollowEnabled";
const MAX_KEYS: usize = 4;
#[cfg(windows)]
const CURSOR_OFFSET: i32 = 16;
#[cfg(windows)]
const DRAG_THRESHOLD: i32 = 12;
#[cfg(windows)]
const LERP: f64 = 0.35;
#[cfg(windows)]
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
    let state = app.state::<AppState>();
    if !state.mouse_follow_pref_enabled.load(Ordering::Relaxed) {
        return unregister_all(app);
    }
    let shortcut = state
        .mouse_follow_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| DEFAULT_MOUSE_FOLLOW_SHORTCUT.to_string());
    register_shortcut(app, &shortcut)
}

pub fn unregister_all(app: &AppHandle) -> Result<(), String> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|err| err.to_string())
}

pub fn register_shortcut(app: &AppHandle, shortcut: &str) -> Result<(), String> {
    let parsed = Shortcut::from_str(shortcut).map_err(|err| err.to_string())?;
    let _ = app.global_shortcut().unregister_all();
    app.global_shortcut()
        .register(parsed)
        .map_err(|err| err.to_string())
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
    let _ = app.emit("app://mouse-follow", true);
}

fn stop(app: &AppHandle) {
    let state = app.state::<AppState>();
    // Returning first so the follow loop keeps `anim` across the handoff.
    state.mouse_follow_returning.store(true, Ordering::Relaxed);
    state.mouse_follow_enabled.store(false, Ordering::Relaxed);
    let _ = app.emit("app://mouse-follow", false);

    #[cfg(not(windows))]
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

        loop {
            let Some(state) = app.try_state::<AppState>() else {
                thread::sleep(Duration::from_millis(50));
                continue;
            };
            let enabled = state.mouse_follow_enabled.load(Ordering::Relaxed);
            let returning = state.mouse_follow_returning.load(Ordering::Relaxed);
            if !enabled && !returning {
                anim = None;
                was_down = false;
                thread::sleep(Duration::from_millis(50));
                continue;
            }

            #[cfg(windows)]
            {
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
                    thread::sleep(Duration::from_millis(16));
                    continue;
                }

                let Some(cursor) = cursor_pos() else {
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

                let down = left_button_down();
                if down && !was_down {
                    press = cursor;
                }
                if !down && was_down {
                    let dx = cursor.0 - press.0;
                    let dy = cursor.1 - press.1;
                    if dx * dx + dy * dy >= DRAG_THRESHOLD * DRAG_THRESHOLD {
                        capture_selection(&app);
                    }
                }
                was_down = down;
            }

            #[cfg(not(windows))]
            {
                let _ = (anim, was_down, press);
            }

            thread::sleep(Duration::from_millis(16));
        }
    });
}

#[cfg(windows)]
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

#[cfg(windows)]
fn capture_selection(app: &AppHandle) {
    thread::sleep(Duration::from_millis(40));
    send_ctrl_c();
    thread::sleep(Duration::from_millis(80));

    let Ok(text) = app.clipboard().read_text() else {
        return;
    };
    if text.trim().is_empty() {
        return;
    }

    if let Ok(mut last) = app.state::<AppState>().last_clipboard.lock() {
        *last = text.clone();
    }

    let kind = classify_clipboard(&text);
    let mode = if kind == "maybe_cipher" {
        "decrypt"
    } else {
        "encrypt"
    };
    windows::show_main(
        app,
        Some(CryptoHint {
            text,
            mode: mode.to_string(),
        }),
    );
    stop(app);
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

#[cfg(windows)]
fn left_button_down() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};

    unsafe { GetAsyncKeyState(i32::from(VK_LBUTTON)) as u16 & 0x8000 != 0 }
}

#[cfg(windows)]
fn send_ctrl_c() {
    use std::mem::{size_of, zeroed};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, KEYBDINPUT, KEYEVENTF_KEYUP, VK_C, VK_CONTROL, INPUT_KEYBOARD,
    };

    fn key_event(vk: u16, flags: u32) -> INPUT {
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
        SendInput(inputs.len() as u32, inputs.as_ptr(), size_of::<INPUT>() as i32);
    }
}
