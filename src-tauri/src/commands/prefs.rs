use tauri::AppHandle;

#[tauri::command]
pub fn get_autostart_pref(app: AppHandle) -> bool {
    crate::autostart_pref::load_pref(&app)
}

#[tauri::command]
pub fn set_autostart_pref(app: AppHandle, enabled: bool) -> Result<bool, String> {
    crate::autostart_pref::set_pref(&app, enabled)
}

#[tauri::command]
pub fn get_intranet_server_base(app: AppHandle) -> String {
    crate::intranet_server::load_base(&app)
}

#[tauri::command]
pub fn set_intranet_server_base(app: AppHandle, base: String) -> Result<String, String> {
    crate::intranet_server::save_base(&app, &base)
}

#[tauri::command]
pub fn get_teamai_pref(app: AppHandle) -> crate::teamai_pref::TeamaiPref {
    crate::teamai_pref::load(&app)
}

#[tauri::command]
pub fn set_teamai_pref(
    app: AppHandle,
    pref: crate::teamai_pref::TeamaiPref,
) -> Result<crate::teamai_pref::TeamaiPref, String> {
    crate::teamai_pref::save(&app, pref)
}

#[tauri::command]
pub fn is_teamai_cli_installed() -> crate::teamai_cli::TeamaiCliStatus {
    crate::teamai_cli::detect()
}

#[tauri::command]
pub async fn teamai_cli_version() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(crate::teamai_cli::cli_version)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn teamai_cli_upgrade() -> crate::teamai_cli::TeamaiUpgradeResult {
    match tauri::async_runtime::spawn_blocking(crate::teamai_cli::cli_upgrade).await {
        Ok(result) => result,
        Err(e) => crate::teamai_cli::TeamaiUpgradeResult {
            ok: false,
            up_to_date: false,
            version: None,
            log: format!("升级任务失败: {e}\n"),
        },
    }
}

#[tauri::command]
pub async fn teamai_cli_pull() -> crate::teamai_cli::TeamaiRunResult {
    match tauri::async_runtime::spawn_blocking(crate::teamai_cli::cli_pull).await {
        Ok(result) => result,
        Err(e) => crate::teamai_cli::TeamaiRunResult {
            ok: false,
            log: format!("拉取任务失败: {e}\n"),
        },
    }
}

#[tauri::command]
pub async fn teamai_cli_has_push_changes() -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(crate::teamai_cli::has_push_changes)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn teamai_cli_push(
    branch: String,
    message: String,
) -> crate::teamai_cli::TeamaiRunResult {
    match tauri::async_runtime::spawn_blocking(move || crate::teamai_cli::cli_push(branch, message))
        .await
    {
        Ok(result) => result,
        Err(e) => crate::teamai_cli::TeamaiRunResult {
            ok: false,
            log: format!("上传任务失败: {e}\n"),
        },
    }
}

#[tauri::command]
pub async fn teamai_cli_deploy(
    app: AppHandle,
    repo_url: String,
    access_token: String,
) -> crate::teamai_cli::TeamaiDeployResult {
    match tauri::async_runtime::spawn_blocking(move || {
        crate::teamai_cli::cli_deploy(&app, repo_url, access_token)
    })
    .await
    {
        Ok(result) => result,
        Err(e) => crate::teamai_cli::TeamaiDeployResult {
            ok: false,
            version: None,
            log: format!("部署任务失败: {e}\n"),
        },
    }
}

#[tauri::command]
pub async fn open_teamai_local_dir(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || crate::teamai_cli::open_local_dir(&app))
        .await
        .map_err(|e| e.to_string())?
}
