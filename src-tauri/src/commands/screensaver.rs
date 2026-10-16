use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

use crate::state::AppState;

#[tauri::command]
pub fn hide_screensaver(app: AppHandle) {
    crate::screensaver::hide(&app);
}

#[tauri::command]
pub fn toggle_screensaver(app: AppHandle) {
    crate::screensaver::toggle(&app);
}

#[tauri::command]
pub fn get_screensaver_pref(state: State<AppState>) -> bool {
    state.screensaver_pref_enabled.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn set_screensaver_pref(app: AppHandle, enabled: bool) {
    crate::screensaver::apply_pref(&app, enabled);
}

#[tauri::command]
pub fn get_screensaver_shortcut(state: State<AppState>) -> String {
    state
        .screensaver_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| crate::state::DEFAULT_SCREENSAVER_SHORTCUT.to_string())
}

#[tauri::command]
pub fn set_screensaver_shortcut(
    app: AppHandle,
    state: State<AppState>,
    shortcut: String,
) -> Result<String, String> {
    let normalized = crate::mouse_follow::validate_shortcut(&shortcut)?;
    crate::global_shortcuts::ensure_no_conflict(
        &app,
        &normalized,
        crate::global_shortcuts::ShortcutOwner::Screensaver,
    )?;
    if let Ok(mut current) = state.screensaver_shortcut.lock() {
        *current = normalized.clone();
    }
    crate::screensaver::save_shortcut(&app, &normalized);
    crate::global_shortcuts::register_all(&app)?;
    Ok(normalized)
}

#[tauri::command]
pub fn get_screensaver_effect_pref(app: AppHandle) -> crate::screensaver::ScreensaverPref {
    crate::screensaver::load_effect_pref(&app)
}

#[tauri::command]
pub fn set_screensaver_background(
    app: AppHandle,
    background: String,
) -> crate::screensaver::ScreensaverPref {
    crate::screensaver::set_background(&app, background)
}

#[tauri::command]
pub fn set_screensaver_clock(app: AppHandle, clock: String) -> crate::screensaver::ScreensaverPref {
    crate::screensaver::set_clock(&app, clock)
}
