use tauri::AppHandle;

#[tauri::command]
pub async fn open_or_download_essential(
    app: AppHandle,
    tool_id: String,
) -> Result<crate::essentials::OpenOrDownloadResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::essentials::open_or_download(&app, &tool_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn download_essential_asset(
    app: AppHandle,
    asset_id: String,
) -> Result<crate::essentials::OpenOrDownloadResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::essentials::download_and_open_dir(&app, &asset_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn essentials_vpn_doc_url() -> String {
    crate::essentials::vpn_doc_url().to_string()
}

#[tauri::command]
pub fn essentials_cache_dir(app: AppHandle) -> Result<String, String> {
    crate::essentials::cache_dir(&app)
}

#[tauri::command]
pub fn open_essentials_cache_dir(app: AppHandle) -> Result<(), String> {
    crate::essentials::open_cache_dir(&app)
}

#[tauri::command]
pub async fn essentials_volta_status() -> crate::essentials::VoltaStatus {
    match tauri::async_runtime::spawn_blocking(crate::essentials::volta_status).await {
        Ok(result) => result,
        Err(e) => crate::essentials::VoltaStatus {
            installed: false,
            bin: None,
            version: None,
            log: format!("检测任务失败: {e}\n"),
        },
    }
}

#[tauri::command]
pub async fn essentials_volta_install(app: AppHandle) -> crate::essentials::VoltaRunResult {
    match tauri::async_runtime::spawn_blocking(move || crate::essentials::volta_install(&app)).await
    {
        Ok(result) => result,
        Err(e) => crate::essentials::VoltaRunResult {
            ok: false,
            log: format!("安装任务失败: {e}\n"),
            version: None,
        },
    }
}

#[tauri::command]
pub async fn essentials_volta_upgrade(app: AppHandle) -> crate::essentials::VoltaRunResult {
    match tauri::async_runtime::spawn_blocking(move || crate::essentials::volta_upgrade(&app)).await
    {
        Ok(result) => result,
        Err(e) => crate::essentials::VoltaRunResult {
            ok: false,
            log: format!("升级任务失败: {e}\n"),
            version: None,
        },
    }
}

#[tauri::command]
pub async fn essentials_volta_install_base() -> crate::essentials::VoltaRunResult {
    match tauri::async_runtime::spawn_blocking(crate::essentials::volta_install_base).await {
        Ok(result) => result,
        Err(e) => crate::essentials::VoltaRunResult {
            ok: false,
            log: format!("基础依赖安装任务失败: {e}\n"),
            version: None,
        },
    }
}

#[tauri::command]
pub async fn essentials_volta_enable_pnpm() -> crate::essentials::VoltaRunResult {
    match tauri::async_runtime::spawn_blocking(crate::essentials::volta_enable_pnpm).await {
        Ok(result) => result,
        Err(e) => crate::essentials::VoltaRunResult {
            ok: false,
            log: format!("开启 pnpm 任务失败: {e}\n"),
            version: None,
        },
    }
}

#[tauri::command]
pub async fn essentials_volta_list() -> crate::essentials::VoltaRunResult {
    match tauri::async_runtime::spawn_blocking(crate::essentials::volta_list_installed).await {
        Ok(result) => result,
        Err(e) => crate::essentials::VoltaRunResult {
            ok: false,
            log: format!("查看已装依赖任务失败: {e}\n"),
            version: None,
        },
    }
}

#[tauri::command]
pub async fn essentials_volta_install_pkg(
    name: String,
    version: Option<String>,
) -> crate::essentials::VoltaRunResult {
    match tauri::async_runtime::spawn_blocking(move || {
        crate::essentials::volta_install_pkg(name, version)
    })
    .await
    {
        Ok(result) => result,
        Err(e) => crate::essentials::VoltaRunResult {
            ok: false,
            log: format!("依赖安装任务失败: {e}\n"),
            version: None,
        },
    }
}
