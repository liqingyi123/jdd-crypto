use tauri::{AppHandle, State};

use crate::state::AppState;

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
