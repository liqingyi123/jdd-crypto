use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::state::AppState;
use crate::tray;
use crate::windows::{self, CryptoHint};

#[tauri::command]
pub fn navigate_main(app: AppHandle, route: String, mode: Option<String>, text: Option<String>) {
    let hint = match (mode, text) {
        (Some(mode), Some(text)) => Some(CryptoHint { text, mode }),
        _ => None,
    };
    let _ = route;
    windows::show_main(&app, hint);
}

#[tauri::command]
pub fn popup_app_menu(window: WebviewWindow) -> Result<(), String> {
    let menu = tray::build_app_menu(window.app_handle()).map_err(|e| e.to_string())?;
    window.popup_menu(&menu).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_badge_prompt_mode(app: AppHandle, state: State<AppState>, expanded: bool) {
    // Badge no longer expands for clipboard prompts; keep collapsed size.
    let _ = expanded;
    state.badge_expanded.store(false, Ordering::Relaxed);
    if state.follow_blocks_resize() {
        return;
    }
    windows::set_badge_size(&app, false);
}

#[tauri::command]
pub fn hide_clipboard_prompt(app: AppHandle, state: State<AppState>) {
    if let Ok(mut candidate) = state.last_candidate.lock() {
        *candidate = None;
    }
    windows::hide_clipboard_prompt(&app);
}

/// Clear clipboard dedup so the same text can trigger the ask prompt again.
#[tauri::command]
pub fn clear_clipboard_dedup(state: State<AppState>) {
    crate::clipboard::clear_clipboard_seen(&state);
}

/// Write clipboard without triggering the ask prompt (marks text as already seen).
#[tauri::command]
pub fn copy_text_silent(
    app: AppHandle,
    state: State<AppState>,
    text: String,
) -> Result<(), String> {
    if let Ok(mut last) = state.last_clipboard.lock() {
        *last = text.clone();
    }
    app.clipboard()
        .write_text(text)
        .map_err(|e| e.to_string())?;
    crate::clipboard::mark_clipboard_seen(&state);
    Ok(())
}

/// Handle 加密/解密 from the clipboard ask prompt on the Rust side.
/// `text` is a frontend fallback when blur races cleared `last_candidate`.
/// Returns false when there is no text to act on.
#[tauri::command]
pub fn accept_clipboard_action(
    app: AppHandle,
    state: State<AppState>,
    mode: String,
    text: Option<String>,
) -> bool {
    let mode = if mode == "encrypt" {
        "encrypt"
    } else {
        "decrypt"
    };
    let from_state = state
        .last_candidate
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|c| c.text.clone()));
    let text = text
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .or(from_state);
    let Some(text) = text else {
        return false;
    };
    if let Ok(mut candidate) = state.last_candidate.lock() {
        *candidate = None;
    }
    windows::hide_clipboard_prompt(&app);

    let hint = CryptoHint {
        text: text.clone(),
        mode: mode.to_string(),
    };
    if windows::is_short_bubble_text(&text) {
        windows::show_crypto_bubble(&app, hint);
    } else {
        windows::show_main(&app, Some(hint));
    }
    true
}

#[tauri::command]
pub fn show_crypto_bubble(app: AppHandle, mode: String, text: String) {
    // Direct call: commands already run where window APIs are usable.
    windows::show_crypto_bubble(&app, CryptoHint { text, mode });
}

#[tauri::command]
pub fn hide_crypto_bubble(app: AppHandle) {
    windows::hide_crypto_bubble(&app);
}

#[tauri::command]
pub fn reveal_crypto_bubble(app: AppHandle) {
    windows::reveal_crypto_bubble(&app);
}

#[tauri::command]
pub fn get_crypto_bubble_payload() -> Option<CryptoHint> {
    windows::get_pending_crypto_bubble()
}

#[tauri::command]
pub fn get_clipboard_candidate(state: State<AppState>) -> Option<crate::state::ClipboardCandidate> {
    state
        .last_candidate
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
}

#[tauri::command]
pub fn get_badge_size(state: State<AppState>) -> u32 {
    state.badge_size.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn set_badge_size_pref(app: AppHandle, state: State<AppState>, size: u32) {
    let size = crate::state::normalize_badge_size(size);
    state.badge_size.store(size, Ordering::Relaxed);
    windows::save_badge_size(&app, size);
    if state.follow_blocks_resize() {
        return;
    }
    let expanded = state.badge_expanded.load(Ordering::Relaxed);
    windows::set_badge_size(&app, expanded);
}

#[tauri::command]
pub fn set_clipboard_watch(app: AppHandle, state: State<AppState>, enabled: bool) {
    state
        .clipboard_watch_enabled
        .store(enabled, Ordering::Relaxed);
    crate::clipboard::save_watch(&app, enabled);
}

#[tauri::command]
pub fn get_clipboard_watch(state: State<AppState>) -> bool {
    state.clipboard_watch_enabled.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn get_theme_pref(app: AppHandle) -> String {
    windows::load_theme_pref(&app)
}

#[tauri::command]
pub fn set_theme_pref(app: AppHandle, preference: String) -> String {
    windows::save_theme_pref(&app, &preference)
}
