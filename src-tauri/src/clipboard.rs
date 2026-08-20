use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::state::{AppState, ClipboardCandidate};
use crate::windows;

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

pub fn start_clipboard_watcher(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(900));

        let Some(state) = app.try_state::<AppState>() else {
            continue;
        };
        if !state.clipboard_watch_enabled.load(Ordering::Relaxed)
            || state.follow_blocks_resize()
        {
            continue;
        }

        let Ok(text) = app.clipboard().read_text() else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }

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
            text,
            kind: kind.to_string(),
        };
        if let Ok(mut candidate) = state.last_candidate.lock() {
            *candidate = Some(payload.clone());
        }
        let _ = app.emit("clipboard://candidate", &payload);
        windows::schedule_show_clipboard_prompt(&app);
    });
}
