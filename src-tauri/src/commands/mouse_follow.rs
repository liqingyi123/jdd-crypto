use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

use crate::state::AppState;

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
    crate::global_shortcuts::ensure_no_conflict(
        &app,
        &normalized,
        crate::global_shortcuts::ShortcutOwner::MouseFollow,
    )?;
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
