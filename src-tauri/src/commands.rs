use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_clipboard_manager::ClipboardExt;

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
    windows::hide_clipboard_prompt(&app);
}

/// Clear clipboard dedup so the same text can trigger the ask prompt again.
#[tauri::command]
pub fn clear_clipboard_dedup(state: State<AppState>) {
    if let Ok(mut last) = state.last_clipboard.lock() {
        last.clear();
    }
}

/// Write clipboard without triggering the ask prompt (marks text as already seen).
#[tauri::command]
pub fn copy_text_silent(app: AppHandle, state: State<AppState>, text: String) -> Result<(), String> {
    if let Ok(mut last) = state.last_clipboard.lock() {
        *last = text.clone();
    }
    app.clipboard()
        .write_text(text)
        .map_err(|e| e.to_string())
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
    windows::show_crypto_bubble(
        &app,
        CryptoHint {
            text,
            mode,
        },
    );
}

#[tauri::command]
pub fn hide_crypto_bubble(app: AppHandle) {
    windows::hide_crypto_bubble(&app);
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
pub fn get_compare_mode_pref(state: State<AppState>) -> bool {
    state.compare_pref_enabled.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn set_compare_mode_pref(app: AppHandle, enabled: bool) {
    crate::compare_mode::apply_pref(&app, enabled);
}

#[tauri::command]
pub fn get_compare_mode_shortcut(state: State<AppState>) -> String {
    state
        .compare_mode_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| crate::state::DEFAULT_COMPARE_MODE_SHORTCUT.to_string())
}

#[tauri::command]
pub fn set_compare_mode_shortcut(
    app: AppHandle,
    state: State<AppState>,
    shortcut: String,
) -> Result<String, String> {
    let normalized = crate::mouse_follow::validate_shortcut(&shortcut)?;
    if let Ok(mut current) = state.compare_mode_shortcut.lock() {
        *current = normalized.clone();
    }
    crate::compare_mode::save_shortcut(&app, &normalized);
    crate::global_shortcuts::register_all(&app)?;
    Ok(normalized)
}

#[tauri::command]
pub fn toggle_compare_mode(app: AppHandle) {
    crate::compare_mode::toggle(&app);
}

#[tauri::command]
pub fn compare_report_plain(app: AppHandle, text: String) {
    crate::compare_mode::report_plain(&app, text);
}

#[tauri::command]
pub fn compare_report_fail(app: AppHandle) {
    crate::compare_mode::report_fail(&app);
}

#[tauri::command]
pub fn hide_compare_bubble(app: AppHandle) {
    // 关闭对比结果弹窗即退出本次对比模式
    crate::compare_mode::stop(&app);
}

#[tauri::command]
pub fn get_compare_bubble_payload() -> Option<crate::compare_mode::CompareBubblePayload> {
    crate::compare_mode::get_pending_compare_bubble()
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
pub fn set_mouse_trail_color(
    app: AppHandle,
    effect: String,
    color: String,
) -> Result<crate::mouse_trail::MouseTrailPref, String> {
    crate::mouse_trail::set_color_pref(app, effect, color)
}

#[tauri::command]
pub fn reset_mouse_trail_colors(
    app: AppHandle,
    effect: String,
) -> Result<crate::mouse_trail::MouseTrailPref, String> {
    crate::mouse_trail::reset_color_pref(app, effect)
}

#[tauri::command]
pub fn reset_mouse_trail_pref(app: AppHandle) -> Result<crate::mouse_trail::MouseTrailPref, String> {
    crate::mouse_trail::reset_pref(app)
}

#[tauri::command]
pub fn get_autostart_pref(app: AppHandle) -> bool {
    crate::autostart_pref::load_pref(&app)
}

#[tauri::command]
pub fn set_autostart_pref(app: AppHandle, enabled: bool) -> Result<bool, String> {
    crate::autostart_pref::set_pref(&app, enabled)
}

#[tauri::command]
pub fn hosts_list(app: AppHandle) -> Vec<crate::hosts_manager::HostsScheme> {
    crate::hosts_manager::list_schemes(&app)
}

#[tauri::command]
pub fn hosts_upsert(
    app: AppHandle,
    id: Option<String>,
    title: String,
    content: String,
    enabled: Option<bool>,
    nature: Option<String>,
    scheme_type: Option<String>,
    url: Option<String>,
    refresh_interval: Option<u64>,
) -> Result<Vec<crate::hosts_manager::HostsScheme>, String> {
    crate::hosts_manager::upsert_scheme(
        &app,
        id,
        title,
        content,
        enabled,
        nature,
        scheme_type,
        url,
        refresh_interval,
    )
}

#[tauri::command]
pub fn hosts_refresh(
    app: AppHandle,
    id: String,
) -> Result<Vec<crate::hosts_manager::HostsScheme>, String> {
    crate::hosts_manager::refresh_scheme(&app, &id)
}

#[tauri::command]
pub fn hosts_delete(
    app: AppHandle,
    id: String,
) -> Result<Vec<crate::hosts_manager::HostsScheme>, String> {
    crate::hosts_manager::delete_scheme(&app, id)
}

#[tauri::command]
pub fn hosts_rename(
    app: AppHandle,
    id: String,
    title: String,
) -> Result<Vec<crate::hosts_manager::HostsScheme>, String> {
    crate::hosts_manager::rename_scheme(&app, id, title)
}

#[tauri::command]
pub fn hosts_set_nature(
    app: AppHandle,
    id: String,
    nature: String,
) -> Result<Vec<crate::hosts_manager::HostsScheme>, String> {
    crate::hosts_manager::set_nature(&app, id, nature)
}

#[tauri::command]
pub fn hosts_set_enabled(
    app: AppHandle,
    id: String,
    enabled: bool,
) -> Result<Vec<crate::hosts_manager::HostsScheme>, String> {
    crate::hosts_manager::set_enabled(&app, id, enabled)
}

#[tauri::command]
pub fn hosts_apply(app: AppHandle) -> Result<(), String> {
    crate::hosts_manager::apply_enabled(&app)
}

#[tauri::command]
pub fn hosts_read_system() -> Result<String, String> {
    crate::hosts_manager::read_system_hosts()
}

#[tauri::command]
pub fn hosts_has_write_access() -> bool {
    crate::hosts_manager::has_write_access()
}

#[tauri::command]
pub fn hosts_request_permission(app: AppHandle) -> Result<(), String> {
    crate::hosts_manager::request_permission(&app)
}

#[tauri::command]
pub fn hosts_open_system(app: AppHandle) -> Result<(), String> {
    crate::hosts_manager::open_system_hosts(&app)
}

#[tauri::command]
pub fn hosts_import_switchhosts(
    app: AppHandle,
    raw: String,
) -> Result<crate::hosts_manager::ImportResult, String> {
    crate::hosts_manager::import_switchhosts(&app, raw)
}

#[tauri::command]
pub fn hosts_export_switchhosts(app: AppHandle) -> Result<String, String> {
    crate::hosts_manager::export_switchhosts(&app)
}

#[tauri::command]
pub fn hosts_reset_system(app: AppHandle) -> Result<Vec<crate::hosts_manager::HostsScheme>, String> {
    crate::hosts_manager::reset_system_hosts(&app)
}

#[tauri::command]
pub fn open_hosts_window(app: AppHandle) {
    windows::show_feature(&app, "hosts");
}

#[tauri::command]
pub fn hide_hosts_quick(app: AppHandle) {
    crate::hosts_quick::hide(&app);
}

#[tauri::command]
pub fn get_hosts_quick_shortcut(state: State<AppState>) -> String {
    state
        .hosts_quick_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| crate::state::DEFAULT_HOSTS_QUICK_SHORTCUT.to_string())
}

#[tauri::command]
pub fn set_hosts_quick_shortcut(
    app: AppHandle,
    state: State<AppState>,
    shortcut: String,
) -> Result<String, String> {
    let normalized = crate::mouse_follow::validate_shortcut(&shortcut)?;
    if let Ok(mut current) = state.hosts_quick_shortcut.lock() {
        *current = normalized.clone();
    }
    crate::hosts_quick::save_shortcut(&app, &normalized);
    crate::global_shortcuts::register_all(&app)?;
    Ok(normalized)
}

#[tauri::command]
pub fn check_app_update(
    app: AppHandle,
    manual: bool,
) -> Result<crate::app_update::UpdateCheckResult, String> {
    crate::app_update::check_update(&app, manual)
}

#[tauri::command]
pub fn download_app_update(app: AppHandle, version: String) -> Result<String, String> {
    crate::app_update::download_installer(&app, &version)
}

#[tauri::command]
pub fn install_app_update(app: AppHandle, path: String) -> Result<(), String> {
    crate::app_update::install_update(&app, &path)
}

#[tauri::command]
pub fn take_pending_app_update(
    state: State<AppState>,
) -> Option<crate::app_update::UpdateCheckResult> {
    state
        .pending_update
        .lock()
        .ok()
        .and_then(|mut pending| pending.take())
}
