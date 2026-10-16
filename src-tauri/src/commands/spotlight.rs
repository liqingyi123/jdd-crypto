use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

use crate::state::AppState;

#[tauri::command]
pub fn hide_spotlight(app: AppHandle) {
    crate::spotlight::hide(&app);
}

#[tauri::command]
pub fn get_spotlight_pref(state: State<AppState>) -> bool {
    state.spotlight_pref_enabled.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn set_spotlight_pref(app: AppHandle, enabled: bool) {
    crate::spotlight::apply_pref(&app, enabled);
}

#[tauri::command]
pub fn get_spotlight_shake_pref(state: State<AppState>) -> bool {
    state.spotlight_shake_enabled.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn set_spotlight_shake_pref(app: AppHandle, enabled: bool) {
    crate::spotlight::apply_shake_pref(&app, enabled);
}

#[tauri::command]
pub fn get_spotlight_shortcut(state: State<AppState>) -> String {
    state
        .spotlight_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| crate::state::DEFAULT_SPOTLIGHT_SHORTCUT.to_string())
}

#[tauri::command]
pub fn set_spotlight_shortcut(
    app: AppHandle,
    state: State<AppState>,
    shortcut: String,
) -> Result<String, String> {
    let normalized = crate::mouse_follow::validate_shortcut(&shortcut)?;
    crate::global_shortcuts::ensure_no_conflict(
        &app,
        &normalized,
        crate::global_shortcuts::ShortcutOwner::Spotlight,
    )?;
    if let Ok(mut current) = state.spotlight_shortcut.lock() {
        *current = normalized.clone();
    }
    crate::spotlight::save_shortcut(&app, &normalized);
    crate::global_shortcuts::register_all(&app)?;
    Ok(normalized)
}

#[tauri::command]
pub fn get_spotlight_size(state: State<AppState>) -> u32 {
    state.spotlight_size.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn peek_cursor_hub() -> crate::cursor_hub::CursorHubSnapshot {
    crate::cursor_hub::peek()
}

#[tauri::command]
pub fn get_spotlight_cursor() -> Option<crate::spotlight::SpotlightCursor> {
    let snap = crate::cursor_hub::peek();
    Some(crate::spotlight::SpotlightCursor {
        x: snap.x,
        y: snap.y,
    })
}

#[tauri::command]
pub fn set_spotlight_size(app: AppHandle, size: u32) -> u32 {
    crate::spotlight::apply_size(&app, size)
}

#[tauri::command]
pub fn get_spotlight_monitor_bounds(
    app: AppHandle,
    window_label: String,
) -> Option<crate::spotlight::SpotlightMonitorBounds> {
    crate::spotlight::monitor_bounds(&app, &window_label)
}
