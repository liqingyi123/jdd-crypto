use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_store::StoreExt;

use crate::state::{AppState, ClipboardCandidate};
use crate::windows::{self, CryptoHint};

const SETTINGS_STORE: &str = "settings.json";
const WATCH_KEY: &str = "clipboardWatchEnabled";

pub fn load_watch(app: &AppHandle) -> bool {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return true;
    };
    store
        .get(WATCH_KEY)
        .and_then(|value| value.as_bool())
        .unwrap_or(true)
}

pub fn save_watch(app: &AppHandle, enabled: bool) {
    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(WATCH_KEY, serde_json::json!(enabled));
        let _ = store.save();
    }
}

pub fn classify_clipboard(text: &str) -> &'static str {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "unknown";
    }

    let compact: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    let looks_base64 = compact.len() >= 16
        && compact.len() % 4 == 0
        && compact
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '='));

    if looks_base64 {
        "maybe_cipher"
    } else {
        "maybe_plain"
    }
}

/// Platform clipboard revision: increments on each Copy/Cut even when text is unchanged.
#[cfg(windows)]
pub fn clipboard_change_id() -> u32 {
    use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;

    // SAFETY: GetClipboardSequenceNumber is a parameterless system query.
    unsafe { GetClipboardSequenceNumber() }
}

#[cfg(target_os = "macos")]
pub fn clipboard_change_id() -> u32 {
    use std::ffi::c_void;
    use std::os::raw::c_char;

    type Id = *mut c_void;
    type MsgSendId = unsafe extern "C" fn(Id, Id) -> Id;
    type MsgSendNsint = unsafe extern "C" fn(Id, Id) -> isize;

    #[link(name = "objc")]
    extern "C" {
        fn sel_registerName(name: *const c_char) -> Id;
        fn objc_getClass(name: *const c_char) -> Id;
        fn objc_msgSend();
    }

    // SAFETY: NSPasteboard class/methods are stable AppKit APIs.
    unsafe {
        let class = objc_getClass(c"NSPasteboard".as_ptr());
        if class.is_null() {
            return 0;
        }
        let msg_id: MsgSendId = std::mem::transmute(objc_msgSend as *const ());
        let pasteboard = msg_id(class, sel_registerName(c"generalPasteboard".as_ptr()));
        if pasteboard.is_null() {
            return 0;
        }
        let msg_int: MsgSendNsint = std::mem::transmute(objc_msgSend as *const ());
        let count = msg_int(pasteboard, sel_registerName(c"changeCount".as_ptr()));
        count.max(0) as u32
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn clipboard_change_id() -> u32 {
    0
}

/// Mark the current clipboard revision as already handled (suppress watcher re-fire).
pub fn mark_clipboard_seen(state: &AppState) {
    let seq = clipboard_change_id();
    state.last_clipboard_seq.store(seq, Ordering::Relaxed);
}

/// Force next watcher poll to treat clipboard as new (e.g. after clear_clipboard_dedup).
pub fn clear_clipboard_seen(state: &AppState) {
    state.last_clipboard_seq.store(0, Ordering::Relaxed);
    if let Ok(mut last) = state.last_clipboard.lock() {
        last.clear();
    }
}

pub fn start_clipboard_watcher(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(900));

        let Some(state) = app.try_state::<AppState>() else {
            continue;
        };
        if !state.clipboard_watch_enabled.load(Ordering::Relaxed) || state.follow_blocks_resize() {
            continue;
        }

        let Ok(text) = app.clipboard().read_text() else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }

        #[cfg(any(windows, target_os = "macos"))]
        {
            let seq = clipboard_change_id();
            let last_seq = state.last_clipboard_seq.load(Ordering::Relaxed);
            if seq == last_seq {
                continue;
            }
            state.last_clipboard_seq.store(seq, Ordering::Relaxed);
            if let Ok(mut last) = state.last_clipboard.lock() {
                *last = text.clone();
            }
        }

        #[cfg(not(any(windows, target_os = "macos")))]
        {
            let mut last = match state.last_clipboard.lock() {
                Ok(guard) => guard,
                Err(_) => continue,
            };
            if *last == text {
                continue;
            }
            *last = text.clone();
        }

        let kind = classify_clipboard(&text);
        let payload = ClipboardCandidate {
            text: text.clone(),
            kind: kind.to_string(),
        };
        if let Ok(mut candidate) = state.last_candidate.lock() {
            *candidate = Some(payload.clone());
        }
        let _ = app.emit("clipboard://candidate", &payload);
        // Only try silent decrypt for base64-like cipher; plain copy must not touch the bubble.
        if kind == "maybe_cipher" {
            windows::schedule_show_crypto_bubble(
                &app,
                CryptoHint {
                    text,
                    mode: "silent_json".to_string(),
                },
            );
        }
    });
}
