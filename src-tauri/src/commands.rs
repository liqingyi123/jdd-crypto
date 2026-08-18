use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager, State, WebviewWindow};

use crate::state::AppState;
use crate::windows::{self, CryptoHint};
use crate::{plugin_host, tray};

#[derive(Deserialize)]
pub struct CryptoRequest {
    pub mode: String,
    pub algorithm: String,
    pub key_id: Option<String>,
    pub iv_id: Option<String>,
    pub plaintext: Option<String>,
    pub ciphertext: Option<String>,
}

#[derive(Serialize)]
pub struct CryptoResponse {
    pub ok: bool,
    pub result: String,
}

#[tauri::command]
pub fn navigate_main(
    app: AppHandle,
    route: String,
    mode: Option<String>,
    text: Option<String>,
) {
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
    state.badge_expanded.store(expanded, Ordering::Relaxed);
    windows::set_badge_size(&app, expanded);
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
    let expanded = state.badge_expanded.load(Ordering::Relaxed);
    windows::set_badge_size(&app, expanded);
}

#[tauri::command]
pub fn set_clipboard_watch(state: State<AppState>, enabled: bool) {
    state
        .clipboard_watch_enabled
        .store(enabled, Ordering::Relaxed);
}

#[tauri::command]
pub fn get_clipboard_watch(state: State<AppState>) -> bool {
    state.clipboard_watch_enabled.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn list_plugins(app: AppHandle) -> Vec<plugin_host::PluginManifest> {
    plugin_host::scan_plugins(&app)
}

#[tauri::command]
pub fn crypto_transform(request: CryptoRequest) -> Result<CryptoResponse, String> {
    let _ = (request.algorithm, request.key_id, request.iv_id);
    Err(format!(
        "crypto_transform is a stub (mode={}, plaintext={}, ciphertext={})",
        request.mode,
        request.plaintext.as_deref().unwrap_or(""),
        request.ciphertext.as_deref().unwrap_or("")
    ))
}
