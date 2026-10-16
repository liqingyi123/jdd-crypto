use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

use crate::state::AppState;

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
    crate::global_shortcuts::ensure_no_conflict(
        &app,
        &normalized,
        crate::global_shortcuts::ShortcutOwner::Compare,
    )?;
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
