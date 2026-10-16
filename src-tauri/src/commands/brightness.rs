use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

use crate::state::AppState;

#[tauri::command]
pub fn list_display_brightness() -> Result<Vec<crate::brightness_control::DisplayBrightness>, String>
{
    crate::brightness_control::list_displays()
}

#[tauri::command]
pub fn set_display_brightness(id: String, value: u32) -> Result<(), String> {
    crate::brightness_control::set_display_brightness(&id, value)
}

#[tauri::command]
pub fn hide_brightness_bubble(app: AppHandle) {
    crate::brightness_quick::hide(&app);
}

#[tauri::command]
pub fn get_brightness_quick_pref(state: State<AppState>) -> bool {
    state.brightness_quick_pref_enabled.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn set_brightness_quick_pref(app: AppHandle, enabled: bool) {
    crate::brightness_quick::apply_pref(&app, enabled);
}

#[tauri::command]
pub fn get_brightness_quick_shortcut(state: State<AppState>) -> String {
    state
        .brightness_quick_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| crate::state::DEFAULT_BRIGHTNESS_QUICK_SHORTCUT.to_string())
}

#[tauri::command]
pub fn set_brightness_quick_shortcut(
    app: AppHandle,
    state: State<AppState>,
    shortcut: String,
) -> Result<String, String> {
    let normalized = crate::mouse_follow::validate_shortcut(&shortcut)?;
    crate::global_shortcuts::ensure_no_conflict(
        &app,
        &normalized,
        crate::global_shortcuts::ShortcutOwner::BrightnessQuick,
    )?;
    if let Ok(mut current) = state.brightness_quick_shortcut.lock() {
        *current = normalized.clone();
    }
    crate::brightness_quick::save_shortcut(&app, &normalized);
    crate::global_shortcuts::register_all(&app)?;
    Ok(normalized)
}
