use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

use crate::state::AppState;
use crate::windows;

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
pub fn hosts_pull_preset(app: AppHandle) -> Result<crate::hosts_manager::ImportResult, String> {
    crate::hosts_manager::pull_preset_config(&app)
}

#[tauri::command]
pub fn hosts_export_switchhosts(app: AppHandle) -> Result<String, String> {
    crate::hosts_manager::export_switchhosts(&app)
}

#[tauri::command]
pub fn hosts_reset_system(
    app: AppHandle,
) -> Result<Vec<crate::hosts_manager::HostsScheme>, String> {
    crate::hosts_manager::reset_system_hosts(&app)
}

#[tauri::command]
pub fn open_hosts_window(app: AppHandle) {
    windows::show_feature(&app, "hosts");
}

#[tauri::command]
pub fn open_settings_window(app: AppHandle) {
    windows::show_feature(&app, "settings");
}

#[tauri::command]
pub fn hide_hosts_quick(app: AppHandle) {
    crate::hosts_quick::hide(&app);
}

#[tauri::command]
pub fn show_overlay_toast(app: AppHandle, message: String) {
    crate::overlay_toast::schedule_show(&app, message);
}

#[tauri::command]
pub fn take_pending_overlay_toast() -> Option<String> {
    crate::overlay_toast::take_pending()
}

#[tauri::command]
pub fn get_hosts_quick_pref(state: State<AppState>) -> bool {
    state.hosts_quick_pref_enabled.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn set_hosts_quick_pref(app: AppHandle, enabled: bool) {
    crate::hosts_quick::apply_pref(&app, enabled);
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
    crate::global_shortcuts::ensure_no_conflict(
        &app,
        &normalized,
        crate::global_shortcuts::ShortcutOwner::HostsQuick,
    )?;
    if let Ok(mut current) = state.hosts_quick_shortcut.lock() {
        *current = normalized.clone();
    }
    crate::hosts_quick::save_shortcut(&app, &normalized);
    crate::global_shortcuts::register_all(&app)?;
    Ok(normalized)
}

#[tauri::command]
pub fn hide_hosts_search(app: AppHandle) {
    crate::hosts_search::hide(&app);
}

#[tauri::command]
pub fn hosts_search_recenter(app: AppHandle) {
    crate::hosts_search::recenter(&app);
}

#[tauri::command]
pub fn hosts_search_begin_drag() {
    crate::hosts_search::begin_drag();
}

#[tauri::command]
pub fn open_hosts_scheme(app: AppHandle, scheme_id: String, line: Option<String>) {
    crate::hosts_search::open_hosts_scheme(&app, &scheme_id, line.as_deref());
}

#[tauri::command]
pub fn take_pending_hosts_focus() -> Option<crate::hosts_search::FocusSchemePayload> {
    crate::hosts_search::take_pending_focus()
}

#[tauri::command]
pub fn hosts_search(app: AppHandle, query: String) -> Vec<crate::hosts_manager::HostsSearchHit> {
    crate::hosts_manager::search_schemes(&app, &query)
}

#[tauri::command]
pub fn get_hosts_search_pref(state: State<AppState>) -> bool {
    state.hosts_search_pref_enabled.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn set_hosts_search_pref(app: AppHandle, enabled: bool) {
    crate::hosts_search::apply_pref(&app, enabled);
}

#[tauri::command]
pub fn get_hosts_search_shortcut(state: State<AppState>) -> String {
    state
        .hosts_search_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| crate::state::DEFAULT_HOSTS_SEARCH_SHORTCUT.to_string())
}

#[tauri::command]
pub fn set_hosts_search_shortcut(
    app: AppHandle,
    state: State<AppState>,
    shortcut: String,
) -> Result<String, String> {
    let normalized = crate::mouse_follow::validate_shortcut(&shortcut)?;
    crate::global_shortcuts::ensure_no_conflict(
        &app,
        &normalized,
        crate::global_shortcuts::ShortcutOwner::HostsSearch,
    )?;
    if let Ok(mut current) = state.hosts_search_shortcut.lock() {
        *current = normalized.clone();
    }
    crate::hosts_search::save_shortcut(&app, &normalized);
    crate::global_shortcuts::register_all(&app)?;
    Ok(normalized)
}
