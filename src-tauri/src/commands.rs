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
    windows::schedule_hide_clipboard_prompt(&app);
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

#[tauri::command]
pub fn get_theme_pref(app: AppHandle) -> String {
    windows::load_theme_pref(&app)
}

#[tauri::command]
pub fn set_theme_pref(app: AppHandle, preference: String) -> String {
    windows::save_theme_pref(&app, &preference)
}

#[tauri::command]
pub fn get_mouse_follow_shortcut(state: State<AppState>) -> String {
    state
        .mouse_follow_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| crate::state::DEFAULT_MOUSE_FOLLOW_SHORTCUT.to_string())
}

#[tauri::command]
pub fn set_mouse_follow_shortcut(
    app: AppHandle,
    state: State<AppState>,
    shortcut: String,
) -> Result<String, String> {
    let normalized = crate::mouse_follow::validate_shortcut(&shortcut)?;
    if let Ok(mut current) = state.mouse_follow_shortcut.lock() {
        *current = normalized.clone();
    }
    crate::mouse_follow::save_shortcut(&app, &normalized);
    crate::mouse_follow::register_current(&app)?;
    Ok(normalized)
}

#[tauri::command]
pub fn get_mouse_follow_pref(state: State<AppState>) -> bool {
    state.mouse_follow_pref_enabled.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn set_mouse_follow_pref(app: AppHandle, enabled: bool) {
    crate::mouse_follow::apply_pref(&app, enabled);
}

#[tauri::command]
pub fn begin_shortcut_capture(app: AppHandle) -> Result<(), String> {
    crate::mouse_follow::unregister_all(&app)
}

#[tauri::command]
pub fn end_shortcut_capture(app: AppHandle) -> Result<(), String> {
    crate::mouse_follow::register_current(&app)
}

#[tauri::command]
pub fn get_plugin_slots(app: AppHandle) -> plugin_host::PluginSlotsState {
    plugin_host::get_plugin_slots(app)
}

#[tauri::command]
pub fn set_plugin_slot_enabled(
    app: AppHandle,
    kind: String,
    enabled: bool,
) -> Result<plugin_host::PluginSlotsState, String> {
    plugin_host::set_plugin_slot_enabled(app, kind, enabled)
}

#[tauri::command]
pub fn import_plugin(
    app: AppHandle,
    kind: String,
    file_name: String,
    bytes: Vec<u8>,
) -> Result<plugin_host::PluginSlotsState, String> {
    plugin_host::import_plugin(app, kind, file_name, bytes)
}

#[tauri::command]
pub fn reset_plugin_slot(
    app: AppHandle,
    kind: String,
) -> Result<plugin_host::PluginSlotsState, String> {
    plugin_host::reset_plugin_slot(app, kind)
}

#[tauri::command]
pub fn get_mouse_trail_monitor_bounds(
    app: AppHandle,
    window_label: String,
) -> Result<crate::mouse_trail::MouseTrailMonitorBounds, String> {
    crate::mouse_trail::monitor_bounds(&app, &window_label)
        .ok_or_else(|| "无法读取显示器边界".to_string())
}

#[tauri::command]
pub fn get_mouse_trail_pref(app: AppHandle) -> crate::mouse_trail::MouseTrailPref {
    crate::mouse_trail::get_pref(app)
}

#[tauri::command]
pub fn set_mouse_trail_enabled(
    app: AppHandle,
    enabled: bool,
) -> Result<crate::mouse_trail::MouseTrailPref, String> {
    crate::mouse_trail::set_enabled_pref(app, enabled)
}

#[tauri::command]
pub fn set_mouse_trail_effect(
    app: AppHandle,
    effect: String,
) -> Result<crate::mouse_trail::MouseTrailPref, String> {
    crate::mouse_trail::set_effect_pref(app, effect)
}

#[tauri::command]
pub fn reset_mouse_trail_pref(app: AppHandle) -> Result<crate::mouse_trail::MouseTrailPref, String> {
    crate::mouse_trail::reset_pref(app)
}
