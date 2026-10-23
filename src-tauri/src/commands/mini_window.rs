use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

use crate::state::AppState;

#[tauri::command]
pub fn get_mini_window_prefs(app: AppHandle) -> crate::mini_window::MiniWindowPrefs {
    crate::mini_window::current_prefs(&app)
}

#[tauri::command]
pub fn mini_window_navigate(app: AppHandle, url: String) -> Result<(), String> {
    crate::mini_window::navigate(&app, url)
}

#[tauri::command]
pub fn mini_window_show_home(app: AppHandle) -> Result<(), String> {
    crate::mini_window::show_home(&app)
}

#[tauri::command]
pub fn mini_window_go_back(app: AppHandle) -> Result<(), String> {
    crate::mini_window::go_back(&app)
}

#[tauri::command]
pub fn mini_window_go_forward(app: AppHandle) -> Result<(), String> {
    crate::mini_window::go_forward(&app)
}

#[tauri::command]
pub fn mini_window_minimize(app: AppHandle) {
    crate::mini_window::minimize(&app);
}

#[tauri::command]
pub fn mini_window_close(app: AppHandle) {
    crate::mini_window::close(&app);
}

#[tauri::command]
pub fn set_mini_window_auto_hide(app: AppHandle, enabled: bool) {
    crate::mini_window::set_auto_hide(&app, enabled);
}

#[tauri::command]
pub fn mini_window_set_dragging(dragging: bool) {
    crate::mini_window::set_dragging(dragging);
}

#[tauri::command]
pub fn set_mini_window_always_on_top(app: AppHandle, enabled: bool) {
    crate::mini_window::set_always_on_top(&app, enabled);
}

#[tauri::command]
pub fn set_mini_window_content_opacity(app: AppHandle, opacity: f64) {
    crate::mini_window::set_content_opacity(&app, opacity);
}

#[tauri::command]
pub fn set_mini_window_background_opacity(app: AppHandle, opacity: f64) {
    crate::mini_window::set_background_opacity(&app, opacity);
}

#[tauri::command]
pub fn show_mini_opacity_bubble(app: AppHandle, x: i32, y: i32) {
    crate::mini_window::show_opacity_bubble(&app, x, y);
}

#[tauri::command]
pub fn hide_mini_opacity_bubble(app: AppHandle) {
    crate::mini_window::hide_opacity_bubble(&app);
}

#[tauri::command]
pub fn get_mini_window_pref(state: State<AppState>) -> bool {
    state.mini_window_pref_enabled.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn set_mini_window_pref(app: AppHandle, enabled: bool) {
    crate::mini_window::apply_pref(&app, enabled);
}

#[tauri::command]
pub fn get_mini_window_shortcut(state: State<AppState>) -> String {
    state
        .mini_window_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| crate::state::DEFAULT_MINI_WINDOW_SHORTCUT.to_string())
}

#[tauri::command]
pub fn set_mini_window_shortcut(
    app: AppHandle,
    state: State<AppState>,
    shortcut: String,
) -> Result<String, String> {
    let normalized = crate::mouse_follow::validate_shortcut(&shortcut)?;
    crate::global_shortcuts::ensure_no_conflict(
        &app,
        &normalized,
        crate::global_shortcuts::ShortcutOwner::MiniWindow,
    )?;
    if let Ok(mut current) = state.mini_window_shortcut.lock() {
        *current = normalized.clone();
    }
    crate::mini_window::save_shortcut(&app, &normalized);
    crate::global_shortcuts::register_all(&app)?;
    Ok(normalized)
}
