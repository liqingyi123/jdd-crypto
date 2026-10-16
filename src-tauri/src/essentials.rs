//! 装机必备：内网工具下载 / 打开，以及 Volta 安装与依赖管理。

use serde::Serialize;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::intranet_server;

const PROGRESS_EVENT: &str = "app://essentials-download-progress";
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);
const VPN_DOC_URL: &str = "https://docx.jddops.com/#/doc/Kd1s3qBu";

const OS_TOOLS_DIR: &str =
    "appStore/Software/PC/Windows效率提升、系统重装、系统激活";
const VOLTA_DIR: &str = "appStore/Software/PC/developer/volta";

const BASE_PACKAGES: &[&str] = &[
    "node@24.20.0",
    "pnpm@8.15.9",
    "http-server",
    "npkill",
];

static DOWNLOADING: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolId {
    Activator,
    DriverMgr,
    AuthHelper,
}

impl ToolId {
    pub fn from_str(id: &str) -> Result<Self, String> {
        match id {
            "activator" => Ok(Self::Activator),
            "driverMgr" | "driver_mgr" => Ok(Self::DriverMgr),
            "authHelper" | "auth_helper" => Ok(Self::AuthHelper),
            _ => Err(format!("未知工具: {id}")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Activator => "activator",
            Self::DriverMgr => "driverMgr",
            Self::AuthHelper => "authHelper",
        }
    }

    fn relative_path(self) -> String {
        match self {
            Self::Activator => format!("{OS_TOOLS_DIR}/HEU_KMS_Activator_30.3.0.exe"),
            Self::DriverMgr => {
                format!("{OS_TOOLS_DIR}/驱动管理破解版/驱动精灵SVIP v2023去广告不限速版.zip")
            }
            Self::AuthHelper => format!("{OS_TOOLS_DIR}/zr.exe"),
        }
    }

    fn file_name(self) -> &'static str {
        match self {
            Self::Activator => "HEU_KMS_Activator_30.3.0.exe",
            Self::DriverMgr => "驱动精灵SVIP v2023去广告不限速版.zip",
            Self::AuthHelper => "zr.exe",
        }
    }
}

/// Editor / browser assets: download then always open containing folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetId {
    EditorConfig,
    Eslint,
    Gitlens,
    Prettier,
    Volar,
    VueDevtools,
    JddCryptoExt,
    Tampermonkey,
    BaiduPanHelper,
}

enum AssetUrl {
    Absolute(&'static str),
    Intranet(&'static str),
}

impl AssetId {
    pub fn from_str(id: &str) -> Result<Self, String> {
        match id {
            "editorconfig" => Ok(Self::EditorConfig),
            "eslint" => Ok(Self::Eslint),
            "gitlens" => Ok(Self::Gitlens),
            "prettier" => Ok(Self::Prettier),
            "volar" => Ok(Self::Volar),
            "vueDevtools" | "vue_devtools" => Ok(Self::VueDevtools),
            "jddCryptoExt" | "jdd_crypto_ext" => Ok(Self::JddCryptoExt),
            "tampermonkey" => Ok(Self::Tampermonkey),
            "baiduPanHelper" | "baidu_pan_helper" => Ok(Self::BaiduPanHelper),
            _ => Err(format!("未知资源: {id}")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::EditorConfig => "editorconfig",
            Self::Eslint => "eslint",
            Self::Gitlens => "gitlens",
            Self::Prettier => "prettier",
            Self::Volar => "volar",
            Self::VueDevtools => "vueDevtools",
            Self::JddCryptoExt => "jddCryptoExt",
            Self::Tampermonkey => "tampermonkey",
            Self::BaiduPanHelper => "baiduPanHelper",
        }
    }

    fn file_name(self) -> &'static str {
        match self {
            Self::EditorConfig => "EditorConfig-0.18.2.vsix",
            Self::Eslint => "vscode-eslint-3.0.34.vsix",
            Self::Gitlens => "gitlens-2026.10.151351.vsix",
            Self::Prettier => "prettier-vscode-12.4.0.vsix",
            Self::Volar => "volar-3.3.13.vsix",
            Self::VueDevtools => "vue devtools 6.5.0.zip",
            Self::JddCryptoExt => "jdd-crypto 0.3.1.zip",
            Self::Tampermonkey => "Tampermonkey 4.18.1.zip",
            Self::BaiduPanHelper => "百度网盘文件转存助手0.0.2.zip",
        }
    }

    fn url(self) -> AssetUrl {
        match self {
            Self::EditorConfig => AssetUrl::Absolute(
                "https://marketplace.visualstudio.com/_apis/public/gallery/publishers/EditorConfig/vsextensions/EditorConfig/0.18.2/vspackage",
            ),
            Self::Eslint => AssetUrl::Absolute(
                "https://marketplace.visualstudio.com/_apis/public/gallery/publishers/dbaeumer/vsextensions/vscode-eslint/3.0.34/vspackage",
            ),
            Self::Gitlens => AssetUrl::Absolute(
                "https://marketplace.visualstudio.com/_apis/public/gallery/publishers/eamodio/vsextensions/gitlens/2026.10.151351/vspackage",
            ),
            Self::Prettier => AssetUrl::Absolute(
                "https://marketplace.visualstudio.com/_apis/public/gallery/publishers/esbenp/vsextensions/prettier-vscode/12.4.0/vspackage",
            ),
            Self::Volar => AssetUrl::Absolute(
                "https://marketplace.visualstudio.com/_apis/public/gallery/publishers/Vue/vsextensions/volar/3.3.13/vspackage",
            ),
            Self::VueDevtools => {
                AssetUrl::Intranet("appStore/BrowserPlugIns/vue devtools 6.5.0.zip")
            }
            Self::JddCryptoExt => {
                AssetUrl::Intranet("appStore/BrowserPlugIns/jdd-crypto 0.3.1.zip")
            }
            Self::Tampermonkey => {
                AssetUrl::Intranet("appStore/BrowserPlugIns/Tampermonkey 4.18.1.zip")
            }
            Self::BaiduPanHelper => {
                AssetUrl::Intranet("appStore/BrowserPlugIns/百度网盘文件转存助手0.0.2.zip")
            }
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub id: String,
    pub downloaded: u64,
    pub total: Option<u64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenOrDownloadResult {
    pub path: String,
    pub downloaded: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltaStatus {
    pub installed: bool,
    pub bin: Option<String>,
    pub version: Option<String>,
    pub log: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltaRunResult {
    pub ok: bool,
    pub log: String,
    pub version: Option<String>,
}

fn essentials_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    let dir = root.join("essentials");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Absolute path of the essentials download/cache directory.
pub fn cache_dir(app: &AppHandle) -> Result<String, String> {
    let dir = essentials_dir(app)?;
    Ok(dir.to_string_lossy().into_owned())
}

/// Open the essentials cache directory in the system file manager.
pub fn open_cache_dir(app: &AppHandle) -> Result<(), String> {
    let dir = essentials_dir(app)?;
    open_path(app, &dir)
}

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(DOWNLOAD_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())
}

fn emit_progress(app: &AppHandle, id: &str, downloaded: u64, total: Option<u64>) {
    let _ = app.emit(
        PROGRESS_EVENT,
        DownloadProgress {
            id: id.to_string(),
            downloaded,
            total,
        },
    );
}

fn try_begin_download(id: &str) -> Result<(), String> {
    let mut guard = DOWNLOADING
        .lock()
        .map_err(|_| "下载状态锁异常".to_string())?;
    if !guard.insert(id.to_string()) {
        return Err("该资源正在下载中，请稍候".to_string());
    }
    Ok(())
}

fn end_download(id: &str) {
    if let Ok(mut guard) = DOWNLOADING.lock() {
        guard.remove(id);
    }
}

fn download_file(app: &AppHandle, id: &str, url: &str, dest: &Path) -> Result<(), String> {
    try_begin_download(id)?;
    let result = (|| {
        let client = http_client()?;
        let response = client
            .get(url)
            .send()
            .map_err(|_| "下载失败，无法连接服务器".to_string())?;
        if !response.status().is_success() {
            return Err("下载失败，文件不存在或服务器错误".to_string());
        }
        let total = response.content_length();
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = dest.with_extension(format!(
            "{}.part",
            dest.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("download")
        ));
        let mut file = File::create(&tmp).map_err(|e| e.to_string())?;
        let mut source = response;
        let mut buffer = [0_u8; 8192];
        let mut downloaded = 0_u64;
        emit_progress(app, id, downloaded, total);
        loop {
            let read = source.read(&mut buffer).map_err(|e| e.to_string())?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read])
                .map_err(|e| e.to_string())?;
            downloaded += read as u64;
            emit_progress(app, id, downloaded, total);
        }
        drop(file);
        if dest.exists() {
            let _ = fs::remove_file(dest);
        }
        fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
        Ok(())
    })();
    end_download(id);
    result
}

fn is_archive(path: &Path) -> bool {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("zip" | "7z" | "rar") => true,
        Some("gz") => path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_ascii_lowercase().ends_with(".tar.gz"))
            .unwrap_or(false),
        _ => false,
    }
}

fn open_path(app: &AppHandle, path: &Path) -> Result<(), String> {
    app.opener()
        .open_path(path.to_string_lossy().as_ref(), None::<&str>)
        .map_err(|e| format!("打开失败: {e}"))
}

fn open_downloaded(app: &AppHandle, path: &Path) -> Result<(), String> {
    if is_archive(path) {
        let parent = path.parent().unwrap_or(path);
        open_path(app, parent)
    } else {
        open_path(app, path)
    }
}

/// Open local cached tool, or download then open. Archives open the parent folder.
pub fn open_or_download(app: &AppHandle, tool_id: &str) -> Result<OpenOrDownloadResult, String> {
    let tool = ToolId::from_str(tool_id)?;
    let dir = essentials_dir(app)?;
    let dest = dir.join(tool.file_name());
    let id = tool.as_str();

    let downloaded = if dest.is_file()
        && fs::metadata(&dest)
            .map(|m| m.len() > 0)
            .unwrap_or(false)
    {
        false
    } else {
        let url = intranet_server::join_url(app, &tool.relative_path());
        download_file(app, id, &url, &dest)?;
        true
    };

    open_downloaded(app, &dest)?;
    Ok(OpenOrDownloadResult {
        path: dest.to_string_lossy().into_owned(),
        downloaded,
    })
}

/// Download editor/browser asset into essentials cache, then open the parent directory.
pub fn download_and_open_dir(app: &AppHandle, asset_id: &str) -> Result<OpenOrDownloadResult, String> {
    let asset = AssetId::from_str(asset_id)?;
    let dir = essentials_dir(app)?;
    let dest = dir.join(asset.file_name());
    let id = asset.as_str();

    let downloaded = if dest.is_file()
        && fs::metadata(&dest)
            .map(|m| m.len() > 0)
            .unwrap_or(false)
    {
        false
    } else {
        let url = match asset.url() {
            AssetUrl::Absolute(u) => u.to_string(),
            AssetUrl::Intranet(rel) => intranet_server::join_url(app, rel),
        };
        download_file(app, id, &url, &dest)?;
        true
    };

    let parent = dest.parent().unwrap_or(&dir);
    open_path(app, parent)?;
    Ok(OpenOrDownloadResult {
        path: dest.to_string_lossy().into_owned(),
        downloaded,
    })
}

pub fn vpn_doc_url() -> &'static str {
    VPN_DOC_URL
}

fn apply_no_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = cmd;
}

fn volta_path_candidates() -> Vec<PathBuf> {
    let mut list = Vec::new();
    #[cfg(windows)]
    {
        list.push(PathBuf::from(r"C:\Program Files\Volta\volta.exe"));
        list.push(PathBuf::from(r"C:\Program Files (x86)\Volta\volta.exe"));
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            list.push(PathBuf::from(local).join(r"Volta\bin\volta.exe"));
        }
        if let Ok(user) = std::env::var("USERPROFILE") {
            list.push(PathBuf::from(user).join(r".volta\bin\volta.exe"));
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            list.push(PathBuf::from(&home).join(".volta/bin/volta"));
        }
        list.push(PathBuf::from("/usr/local/bin/volta"));
    }
    list
}

fn find_volta_bin() -> Option<PathBuf> {
    for c in volta_path_candidates() {
        if c.is_file() {
            return Some(c);
        }
    }
    #[cfg(windows)]
    {
        let mut cmd = Command::new("where.exe");
        cmd.arg("volta.exe")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        apply_no_window(&mut cmd);
        if let Ok(out) = cmd.output() {
            if out.status.success() {
                for line in String::from_utf8_lossy(&out.stdout).lines() {
                    let p = PathBuf::from(line.trim());
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
    }
    #[cfg(not(windows))]
    {
        let mut cmd = Command::new("which");
        cmd.arg("volta")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if let Ok(out) = cmd.output() {
            if out.status.success() {
                let line = String::from_utf8_lossy(&out.stdout);
                let p = PathBuf::from(line.trim());
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    None
}

fn run_volta(args: &[&str]) -> Result<(bool, String), String> {
    let bin = find_volta_bin().ok_or_else(|| "未找到 volta，请先完成安装".to_string())?;
    let mut cmd = Command::new(&bin);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Ensure ~/.volta/bin is visible for shims after install.
    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let volta_bin = format!("{home}/.volta/bin");
            let path = std::env::var("PATH").unwrap_or_default();
            if !path.split(':').any(|p| p == volta_bin) {
                cmd.env("PATH", format!("{volta_bin}:{path}"));
            }
        }
    }
    #[cfg(windows)]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let volta_bin = PathBuf::from(&local).join(r"Volta\bin");
            if volta_bin.is_dir() {
                let path = std::env::var("PATH").unwrap_or_default();
                let extra = volta_bin.to_string_lossy();
                if !path
                    .split(';')
                    .any(|p| p.eq_ignore_ascii_case(extra.as_ref()))
                {
                    cmd.env("PATH", format!("{extra};{path}"));
                }
            }
        }
    }
    apply_no_window(&mut cmd);
    let out = cmd
        .output()
        .map_err(|e| format!("执行 volta 失败: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let mut combined = String::new();
    if !stdout.trim().is_empty() {
        combined.push_str(&stdout);
    }
    if !stderr.trim().is_empty() {
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&stderr);
    }
    Ok((out.status.success(), combined))
}

fn read_volta_version() -> Option<String> {
    let (ok, out) = run_volta(&["--version"]).ok()?;
    if !ok {
        return None;
    }
    let ver = out
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(|s| s.trim_start_matches('v').to_string())?;
    Some(ver)
}

pub fn volta_status() -> VoltaStatus {
    let mut log = String::from("# Volta 安装检测\n");
    match find_volta_bin() {
        Some(bin) => {
            log.push_str(&format!("找到: {}\n", bin.display()));
            let version = read_volta_version();
            if let Some(ref v) = version {
                log.push_str(&format!("版本: {v}\n"));
            } else {
                log.push_str("已找到二进制，但读取版本失败\n");
            }
            VoltaStatus {
                installed: true,
                bin: Some(bin.to_string_lossy().into_owned()),
                version,
                log,
            }
        }
        None => {
            log.push_str("未找到 volta（PATH / 默认安装目录）\n");
            VoltaStatus {
                installed: false,
                bin: None,
                version: None,
                log,
            }
        }
    }
}

/// Hardcoded installer relative path under intranet volta directory.
fn volta_installer_relative() -> Result<&'static str, String> {
    #[cfg(all(windows, target_arch = "x86_64"))]
    {
        return Ok("volta-2.0.2-windows-x86_64.msi");
    }
    #[cfg(all(windows, target_arch = "aarch64"))]
    {
        return Ok("volta-2.0.2-windows-arm64.msi");
    }
    #[cfg(target_os = "macos")]
    {
        return Ok("volta-2.0.2-macos.tar.gz");
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return Ok("volta-2.0.2-linux.tar.gz");
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        return Ok("volta-2.0.2-linux-arm.tar.gz");
    }
    #[allow(unreachable_code)]
    Err("当前平台暂不支持一键安装 Volta".to_string())
}

fn volta_installer_url(app: &AppHandle) -> Result<(String, String), String> {
    let file = volta_installer_relative()?;
    let relative = format!("{VOLTA_DIR}/{file}");
    Ok((file.to_string(), intranet_server::join_url(app, &relative)))
}

#[cfg(unix)]
fn clear_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|e| format!("清理目录失败: {e}"))?;
    }
    fs::create_dir_all(path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(unix)]
fn find_install_sh(root: &Path) -> Option<PathBuf> {
    if root.join("install.sh").is_file() {
        return Some(root.join("install.sh"));
    }
    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let candidate = path.join("install.sh");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(unix)]
fn run_unix_install_sh(archive: &Path, extract_dir: &Path, log: &mut String) -> Result<(), String> {
    clear_dir(extract_dir)?;
    log.push_str(&format!(
        "> tar -xzf {} -C {}\n",
        archive.display(),
        extract_dir.display()
    ));
    let status = Command::new("tar")
        .args([
            "-xzf",
            archive.to_string_lossy().as_ref(),
            "-C",
            extract_dir.to_string_lossy().as_ref(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("解压失败: {e}"))?;
    if !status.status.success() {
        let err = String::from_utf8_lossy(&status.stderr);
        return Err(format!("解压失败: {err}"));
    }
    let script = find_install_sh(extract_dir)
        .ok_or_else(|| "解压后未找到 install.sh".to_string())?;
    log.push_str(&format!("> bash {}\n", script.display()));
    let mut cmd = Command::new("bash");
    cmd.arg(&script)
        .current_dir(script.parent().unwrap_or(extract_dir))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = cmd
        .output()
        .map_err(|e| format!("执行 install.sh 失败: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !stdout.trim().is_empty() {
        log.push_str(&stdout);
        if !stdout.ends_with('\n') {
            log.push('\n');
        }
    }
    if !stderr.trim().is_empty() {
        log.push_str(&stderr);
        if !stderr.ends_with('\n') {
            log.push('\n');
        }
    }
    if !out.status.success() {
        return Err("install.sh 执行失败".to_string());
    }
    Ok(())
}

fn install_base_deps(log: &mut String) -> bool {
    let mut all_ok = true;
    for pkg in BASE_PACKAGES {
        log.push_str(&format!("> volta install {pkg}\n"));
        match run_volta(&["install", pkg]) {
            Ok((ok, out)) => {
                if !out.trim().is_empty() {
                    log.push_str(&out);
                    if !out.ends_with('\n') {
                        log.push('\n');
                    }
                }
                if !ok {
                    all_ok = false;
                    log.push_str(&format!("失败: {pkg}\n"));
                }
            }
            Err(e) => {
                all_ok = false;
                log.push_str(&format!("{e}\n"));
            }
        }
    }
    all_ok
}

/// One-click Volta deploy: download platform package, then MSI open or tar.gz+install.sh.
/// When `with_base_deps` is true and Volta is already detectable, also install base packages.
pub fn volta_install(app: &AppHandle) -> VoltaRunResult {
    volta_install_inner(app, true)
}

fn volta_install_inner(app: &AppHandle, with_base_deps: bool) -> VoltaRunResult {
    let mut log = String::from(if with_base_deps {
        "# Volta 一键部署\n"
    } else {
        "# Volta 升级版本（安装包）\n"
    });
    let (file_name, url) = match volta_installer_url(app) {
        Ok(v) => v,
        Err(e) => {
            log.push_str(&format!("{e}\n"));
            return VoltaRunResult {
                ok: false,
                log,
                version: None,
            };
        }
    };
    log.push_str(&format!("安装包: {file_name}\nURL: {url}\n"));

    let dir = match essentials_dir(app) {
        Ok(d) => d,
        Err(e) => {
            log.push_str(&format!("{e}\n"));
            return VoltaRunResult {
                ok: false,
                log,
                version: None,
            };
        }
    };
    let dest = dir.join(&file_name);
    let need_download = !(dest.is_file()
        && fs::metadata(&dest)
            .map(|m| m.len() > 0)
            .unwrap_or(false));
    if need_download {
        log.push_str("开始下载…\n");
        if let Err(e) = download_file(app, "volta", &url, &dest) {
            log.push_str(&format!("{e}\n"));
            return VoltaRunResult {
                ok: false,
                log,
                version: None,
            };
        }
        log.push_str("下载完成\n");
    } else {
        log.push_str("使用本地已缓存安装包\n");
    }

    #[cfg(windows)]
    {
        log.push_str("打开 MSI 安装程序…\n");
        if let Err(e) = open_path(app, &dest) {
            log.push_str(&format!("{e}\n"));
            return VoltaRunResult {
                ok: false,
                log,
                version: None,
            };
        }
        log.push_str("已打开安装程序。请完成安装后如需依赖请点击「基础依赖安装」。\n");
        if with_base_deps && find_volta_bin().is_some() {
            log.push_str("检测到已有 Volta，继续安装基础依赖…\n");
            let ok = install_base_deps(&mut log);
            let version = read_volta_version();
            return VoltaRunResult {
                ok,
                log,
                version,
            };
        }
        if !with_base_deps {
            log.push_str("升级未自动安装基础依赖；如需请点击「基础依赖安装」。\n");
        }
        return VoltaRunResult {
            ok: true,
            log,
            version: read_volta_version(),
        };
    }

    #[cfg(unix)]
    {
        let extract_dir = dir.join("volta-extract");
        if let Err(e) = run_unix_install_sh(&dest, &extract_dir, &mut log) {
            log.push_str(&format!("{e}\n"));
            return VoltaRunResult {
                ok: false,
                log,
                version: None,
            };
        }
        log.push_str("install.sh 执行完成。若终端仍找不到 volta，请新开终端后再试。\n");
        if with_base_deps && find_volta_bin().is_some() {
            log.push_str("检测到 Volta，继续安装基础依赖…\n");
            let ok = install_base_deps(&mut log);
            let version = read_volta_version();
            return VoltaRunResult {
                ok,
                log,
                version,
            };
        }
        if !with_base_deps {
            log.push_str("升级未自动安装基础依赖；如需请点击「基础依赖安装」。\n");
        } else if find_volta_bin().is_none() {
            log.push_str("暂未检测到 volta，请新开终端后点击「基础依赖安装」。\n");
        }
        return VoltaRunResult {
            ok: true,
            log,
            version: read_volta_version(),
        };
    }

    #[cfg(not(any(windows, unix)))]
    {
        let _ = with_base_deps;
        log.push_str("当前平台暂不支持一键部署 Volta\n");
        VoltaRunResult {
            ok: false,
            log,
            version: None,
        }
    }
}

/// Parse `2.0.2` from filenames like `volta-2.0.2-windows-x86_64.msi`.
fn parse_version_from_installer_name(file_name: &str) -> Option<String> {
    let rest = file_name.strip_prefix("volta-")?;
    let first = rest.split('-').next()?;
    let ok = first.split('.').count() == 3
        && first
            .split('.')
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    if ok {
        Some(first.to_string())
    } else {
        None
    }
}

fn normalize_volta_version(raw: &str) -> String {
    raw.trim().trim_start_matches('v').trim().to_string()
}

fn intranet_target_version() -> Result<String, String> {
    let file = volta_installer_relative()?;
    parse_version_from_installer_name(file)
        .ok_or_else(|| format!("无法从安装包名解析版本: {file}"))
}

fn already_on_target_version(log: &mut String, target: &str) -> bool {
    match read_volta_version() {
        Some(current) => {
            let current = normalize_volta_version(&current);
            log.push_str(&format!("当前版本: {current}\n目标版本: {target}\n"));
            if current == target {
                log.push_str("已是最新版本，无需下载安装\n");
                true
            } else {
                false
            }
        }
        None => {
            log.push_str(&format!("未检测到已安装 Volta，目标版本: {target}\n"));
            false
        }
    }
}

#[cfg(windows)]
fn try_winget_upgrade_volta(log: &mut String) -> bool {
    log.push_str("> winget upgrade --id Volta.Volta …\n");
    let mut upgrade = Command::new("winget");
    upgrade
        .args([
            "upgrade",
            "--id",
            "Volta.Volta",
            "--accept-package-agreements",
            "--accept-source-agreements",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    apply_no_window(&mut upgrade);
    match upgrade.output() {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if !stdout.trim().is_empty() {
                log.push_str(&stdout);
                if !stdout.ends_with('\n') {
                    log.push('\n');
                }
            }
            if !stderr.trim().is_empty() {
                log.push_str(&stderr);
                if !stderr.ends_with('\n') {
                    log.push('\n');
                }
            }
            log.push_str("winget upgrade 成功\n");
            true
        }
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let combined = format!("{stdout}{stderr}");
            if !combined.trim().is_empty() {
                log.push_str(&combined);
                if !combined.ends_with('\n') {
                    log.push('\n');
                }
            }
            // No upgrade available / already latest via winget
            let lower = combined.to_ascii_lowercase();
            if lower.contains("no applicable update")
                || lower.contains("no available upgrade")
                || lower.contains("already installed")
                || combined.contains("没有可用的升级")
                || combined.contains("未找到")
            {
                log.push_str("winget 未完成升级，尝试 winget install…\n");
            } else {
                log.push_str("winget upgrade 失败，尝试 winget install…\n");
            }
            let mut install = Command::new("winget");
            install
                .args([
                    "install",
                    "--id",
                    "Volta.Volta",
                    "-e",
                    "--accept-package-agreements",
                    "--accept-source-agreements",
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            apply_no_window(&mut install);
            match install.output() {
                Ok(out2) if out2.status.success() => {
                    let s = String::from_utf8_lossy(&out2.stdout);
                    let e = String::from_utf8_lossy(&out2.stderr);
                    if !s.trim().is_empty() {
                        log.push_str(&s);
                        if !s.ends_with('\n') {
                            log.push('\n');
                        }
                    }
                    if !e.trim().is_empty() {
                        log.push_str(&e);
                        if !e.ends_with('\n') {
                            log.push('\n');
                        }
                    }
                    log.push_str("winget install 成功\n");
                    true
                }
                Ok(out2) => {
                    let s = String::from_utf8_lossy(&out2.stdout);
                    let e = String::from_utf8_lossy(&out2.stderr);
                    if !s.trim().is_empty() {
                        log.push_str(&s);
                        if !s.ends_with('\n') {
                            log.push('\n');
                        }
                    }
                    if !e.trim().is_empty() {
                        log.push_str(&e);
                        if !e.ends_with('\n') {
                            log.push('\n');
                        }
                    }
                    log.push_str("winget install 失败\n");
                    false
                }
                Err(err) => {
                    log.push_str(&format!("winget install 启动失败: {err}\n"));
                    false
                }
            }
        }
        Err(err) => {
            log.push_str(&format!("未找到 winget 或启动失败: {err}\n"));
            false
        }
    }
}

fn upgrade_via_intranet(app: &AppHandle, log: &mut String, target: &str) -> VoltaRunResult {
    if already_on_target_version(log, target) {
        return VoltaRunResult {
            ok: true,
            log: log.clone(),
            version: Some(target.to_string()),
        };
    }
    log.push_str("回退内网安装包升级…\n");
    // Force re-download when version differs.
    if let (Ok(dir), Ok(file)) = (essentials_dir(app), volta_installer_relative()) {
        let dest = dir.join(file);
        if dest.is_file() {
            let _ = fs::remove_file(&dest);
            log.push_str("已清除本地缓存安装包\n");
        }
    }
    let mut result = volta_install_inner(app, false);
    result.log = format!("{}{}", log, result.log);
    result
}

/// Upgrade Volta: prefer winget on Windows; else intranet package after version check.
pub fn volta_upgrade(app: &AppHandle) -> VoltaRunResult {
    let mut log = String::from("# Volta 升级版本\n");
    let target = match intranet_target_version() {
        Ok(v) => v,
        Err(e) => {
            log.push_str(&format!("{e}\n"));
            return VoltaRunResult {
                ok: false,
                log,
                version: None,
            };
        }
    };
    log.push_str(&format!("内网目标版本: {target}\n"));

    #[cfg(windows)]
    {
        if try_winget_upgrade_volta(&mut log) {
            let version = read_volta_version().or_else(|| Some(target.clone()));
            log.push_str("已通过 winget 完成升级（未自动安装基础依赖）\n");
            return VoltaRunResult {
                ok: true,
                log,
                version,
            };
        }
        log.push_str("winget 不可用或失败，检查内网版本…\n");
        return upgrade_via_intranet(app, &mut log, &target);
    }

    #[cfg(unix)]
    {
        log.push_str("使用内网安装包升级（install.sh）…\n");
        return upgrade_via_intranet(app, &mut log, &target);
    }

    #[cfg(not(any(windows, unix)))]
    {
        let _ = app;
        log.push_str("当前平台暂不支持升级 Volta\n");
        VoltaRunResult {
            ok: false,
            log,
            version: None,
        }
    }
}

pub fn volta_install_base() -> VoltaRunResult {
    let mut log = String::from("# 基础依赖安装\n");
    log.push_str(&format!(
        "将安装: {}\n",
        BASE_PACKAGES.join(", ")
    ));
    if find_volta_bin().is_none() {
        log.push_str("未找到 volta，请先一键部署\n");
        return VoltaRunResult {
            ok: false,
            log,
            version: None,
        };
    }
    let ok = install_base_deps(&mut log);
    VoltaRunResult {
        ok,
        log,
        version: read_volta_version(),
    }
}

const VOLTA_FEATURE_PNPM: &str = "VOLTA_FEATURE_PNPM";
const VOLTA_FEATURE_PNPM_VALUE: &str = "1";

#[cfg(windows)]
fn persist_user_env_windows(name: &str, value: &str) -> Result<(), String> {
    let name_esc = name.replace('\'', "''");
    let value_esc = value.replace('\'', "''");
    let script = format!(
        "[Environment]::SetEnvironmentVariable('{name_esc}', '{value_esc}', 'User')"
    );
    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-WindowStyle",
        "Hidden",
        "-Command",
        &script,
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::piped());
    apply_no_window(&mut cmd);
    let output = cmd
        .output()
        .map_err(|e| format!("写入环境变量 {name} 失败: {e}"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let brief: String = err.trim().chars().take(160).collect();
        return Err(if brief.is_empty() {
            format!("写入环境变量 {name} 失败")
        } else {
            format!("写入环境变量 {name} 失败: {brief}")
        });
    }
    Ok(())
}

#[cfg(unix)]
fn append_export_to_profile(path: &Path, name: &str, value: &str, log: &mut String) {
    let export_line = format!("export {name}={value}");
    let existing = fs::read_to_string(path).unwrap_or_default();
    if existing.lines().any(|l| {
        let t = l.trim();
        t == export_line
            || t.starts_with(&format!("export {name}="))
            || t.starts_with(&format!("{name}="))
    }) {
        log.push_str(&format!("已存在于 {}: {name}\n", path.display()));
        return;
    }
    let mut content = existing;
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&export_line);
    content.push('\n');
    match fs::write(path, content) {
        Ok(()) => log.push_str(&format!("已写入 {}: {export_line}\n", path.display())),
        Err(e) => log.push_str(&format!("写入 {} 失败: {e}\n", path.display())),
    }
}

#[cfg(unix)]
fn persist_user_env_unix(name: &str, value: &str, log: &mut String) -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|_| "无法读取 HOME".to_string())?;
    let home = PathBuf::from(home);
    for rel in [".zshrc", ".bash_profile", ".bashrc"] {
        let path = home.join(rel);
        if path.exists() || rel == ".zshrc" || rel == ".bash_profile" {
            // Always ensure zshrc / bash_profile exist for common shells; only
            // touch bashrc if it already exists.
            if rel == ".bashrc" && !path.exists() {
                continue;
            }
            if (rel == ".zshrc" || rel == ".bash_profile") && !path.exists() {
                if let Err(e) = fs::File::create(&path) {
                    log.push_str(&format!("创建 {} 失败: {e}\n", path.display()));
                    continue;
                }
            }
            append_export_to_profile(&path, name, value, log);
        }
    }
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("launchctl")
            .args(["setenv", name, value])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        match status {
            Ok(s) if s.success() => log.push_str("已执行 launchctl setenv\n"),
            Ok(_) => log.push_str("launchctl setenv 未成功（可忽略，已写 profile）\n"),
            Err(e) => log.push_str(&format!("launchctl setenv 失败: {e}\n")),
        }
    }
    Ok(())
}

/// Enable Volta native pnpm support by setting `VOLTA_FEATURE_PNPM=1`.
pub fn volta_enable_pnpm() -> VoltaRunResult {
    let mut log = String::from("# 开启 pnpm 特性\n");
    log.push_str(&format!(
        "设置环境变量 {VOLTA_FEATURE_PNPM}={VOLTA_FEATURE_PNPM_VALUE}\n"
    ));

    std::env::set_var(VOLTA_FEATURE_PNPM, VOLTA_FEATURE_PNPM_VALUE);
    log.push_str("已写入当前进程环境\n");

    #[cfg(windows)]
    {
        match persist_user_env_windows(VOLTA_FEATURE_PNPM, VOLTA_FEATURE_PNPM_VALUE) {
            Ok(()) => {
                log.push_str("已写入 Windows 用户环境变量\n");
                log.push_str("请新开终端后生效；之后可用「基础依赖安装」安装 pnpm。\n");
                return VoltaRunResult {
                    ok: true,
                    log,
                    version: read_volta_version(),
                };
            }
            Err(e) => {
                log.push_str(&format!("{e}\n"));
                return VoltaRunResult {
                    ok: false,
                    log,
                    version: read_volta_version(),
                };
            }
        }
    }

    #[cfg(unix)]
    {
        if let Err(e) =
            persist_user_env_unix(VOLTA_FEATURE_PNPM, VOLTA_FEATURE_PNPM_VALUE, &mut log)
        {
            log.push_str(&format!("{e}\n"));
            return VoltaRunResult {
                ok: false,
                log,
                version: read_volta_version(),
            };
        }
        log.push_str("请新开终端后生效；之后可用「基础依赖安装」安装 pnpm。\n");
        return VoltaRunResult {
            ok: true,
            log,
            version: read_volta_version(),
        };
    }

    #[cfg(not(any(windows, unix)))]
    {
        log.push_str("当前平台暂不支持持久写入该环境变量\n");
        VoltaRunResult {
            ok: false,
            log,
            version: None,
        }
    }
}

/// List installed Volta toolchain (`volta list all`).
pub fn volta_list_installed() -> VoltaRunResult {
    let mut log = String::from("# 查看已装依赖\n");
    if find_volta_bin().is_none() {
        log.push_str("未找到 volta，请先一键部署\n");
        return VoltaRunResult {
            ok: false,
            log,
            version: None,
        };
    }
    log.push_str("> volta list all\n");
    match run_volta(&["list", "all"]) {
        Ok((ok, out)) => {
            if !out.trim().is_empty() {
                log.push_str(&out);
                if !out.ends_with('\n') {
                    log.push('\n');
                }
            } else {
                log.push_str("(无输出)\n");
            }
            VoltaRunResult {
                ok,
                log,
                version: read_volta_version(),
            }
        }
        Err(e) => {
            log.push_str(&format!("{e}\n"));
            VoltaRunResult {
                ok: false,
                log,
                version: None,
            }
        }
    }
}

pub fn volta_install_pkg(name: String, version: Option<String>) -> VoltaRunResult {
    let mut log = String::from("# 依赖安装\n");
    let name = name.trim().to_string();
    if name.is_empty() {
        log.push_str("请填写包名\n");
        return VoltaRunResult {
            ok: false,
            log,
            version: None,
        };
    }
    if find_volta_bin().is_none() {
        log.push_str("未找到 volta，请先一键部署\n");
        return VoltaRunResult {
            ok: false,
            log,
            version: None,
        };
    }
    let spec = match version.as_ref().map(|v| v.trim()).filter(|v| !v.is_empty()) {
        Some(v) => format!("{name}@{v}"),
        None => name,
    };
    log.push_str(&format!("> volta install {spec}\n"));
    match run_volta(&["install", &spec]) {
        Ok((ok, out)) => {
            if !out.trim().is_empty() {
                log.push_str(&out);
                if !out.ends_with('\n') {
                    log.push('\n');
                }
            }
            VoltaRunResult {
                ok,
                log,
                version: read_volta_version(),
            }
        }
        Err(e) => {
            log.push_str(&format!("{e}\n"));
            VoltaRunResult {
                ok: false,
                log,
                version: None,
            }
        }
    }
}
