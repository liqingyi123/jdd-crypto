//! Local hosts schemes: persist, apply to system hosts, import SwitchHosts backup JSON.
//!
//! SwitchHosts backup shape locked to `swh_data.json` (PotDb export):
//! ```json
//! {
//!   "version": [4, 1, 1, 6077],
//!   "data": {
//!     "list": { "tree": [ { "id", "title", "on?", "type?", "children?" } ] },
//!     "collection": { "hosts": { "data": [ { "id", "content" } ] } }
//!   }
//! }
//! ```
//! - Tree: metadata; missing `type` => local
//! - Content: join by `id` from `collection.hosts.data` (skip id `"0"` system)
//! - Import `local` / `remote` as content snapshots; flatten `folder`/`group` children

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const STORE_NAME: &str = "hosts-manager.json";
const SCHEMES_KEY: &str = "schemes";

const MARKER_BEGIN: &str = "# >>> jdd-crypto-hosts-begin";
const MARKER_END: &str = "# <<< jdd-crypto-hosts-end";

static ID_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostsScheme {
    pub id: String,
    pub title: String,
    pub content: String,
    pub enabled: bool,
    pub source: String,
    /// "keep" | "exclusive" — missing in old store => keep
    #[serde(default = "default_nature")]
    pub nature: String,
    /// Readonly schemes cannot edit hosts content.
    #[serde(default)]
    pub readonly: bool,
    /// "local" | "remote". Import reads SwitchHosts tree `type`; IPC uses schemeType.
    #[serde(
        default = "default_scheme_type",
        alias = "type",
        alias = "scheme_type"
    )]
    pub scheme_type: String,
    /// Remote URL. Import reads SwitchHosts `url`.
    #[serde(default)]
    pub url: String,
    /// Seconds; 0 = never. Import reads SwitchHosts `refresh_interval`.
    #[serde(default, alias = "refresh_interval")]
    pub refresh_interval: u64,
    /// Import reads SwitchHosts `last_refresh`.
    #[serde(default, alias = "last_refresh")]
    pub last_refresh: String,
    /// Import reads SwitchHosts `last_refresh_ms`.
    #[serde(default, alias = "last_refresh_ms")]
    pub last_refresh_ms: u64,
}

fn default_nature() -> String {
    "keep".to_string()
}

fn default_scheme_type() -> String {
    TYPE_LOCAL.to_string()
}

pub const NATURE_KEEP: &str = "keep";
pub const NATURE_EXCLUSIVE: &str = "exclusive";
pub const TYPE_LOCAL: &str = "local";
pub const TYPE_REMOTE: &str = "remote";

const ALLOWED_REFRESH_INTERVALS: &[u64] = &[
    0, 300, 600, 1800, 3600, 7200, 14400, 21600, 43200, 86400,
];

fn normalize_nature(raw: &str) -> Result<String, String> {
    match raw.trim() {
        NATURE_KEEP => Ok(NATURE_KEEP.to_string()),
        NATURE_EXCLUSIVE => Ok(NATURE_EXCLUSIVE.to_string()),
        _ => Err("性质无效，仅支持 keep（保留）或 exclusive（单开）".to_string()),
    }
}

fn is_exclusive(scheme: &HostsScheme) -> bool {
    scheme.nature == NATURE_EXCLUSIVE
}

fn is_remote(scheme: &HostsScheme) -> bool {
    scheme.scheme_type == TYPE_REMOTE || !scheme.url.trim().is_empty()
}

/// Fill remote metadata from url / legacy fields after load.
fn normalize_scheme(scheme: &mut HostsScheme) {
    if !scheme.url.trim().is_empty() {
        scheme.scheme_type = TYPE_REMOTE.to_string();
        scheme.readonly = true;
    } else if scheme.scheme_type != TYPE_REMOTE {
        scheme.scheme_type = TYPE_LOCAL.to_string();
    }
}

fn normalize_scheme_type(raw: &str) -> Result<String, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | TYPE_LOCAL => Ok(TYPE_LOCAL.to_string()),
        TYPE_REMOTE => Ok(TYPE_REMOTE.to_string()),
        _ => Err("类型无效，仅支持 local 或 remote".to_string()),
    }
}

fn normalize_refresh_interval(raw: u64) -> Result<u64, String> {
    if ALLOWED_REFRESH_INTERVALS.contains(&raw) {
        Ok(raw)
    } else {
        Err("刷新间隔无效".to_string())
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// UTC wall time `YYYY-MM-DD HH:mm:ss` (SwitchHosts-style display string).
fn format_last_refresh(ms: u64) -> String {
    let total_secs = (ms / 1000) as i64;
    let days = total_secs.div_euclid(86_400);
    let tod = total_secs.rem_euclid(86_400) as u32;
    let hour = tod / 3600;
    let min = (tod % 3600) / 60;
    let sec = tod % 60;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02} {hour:02}:{min:02}:{sec:02}")
}

/// Howard Hinnant civil_from_days (UTC).
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

fn fetch_remote_content(url: &str) -> Result<String, String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("远程 URL 不能为空".to_string());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let response = client
        .get(url)
        .send()
        .map_err(|e| format!("请求远程 hosts 失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("远程 hosts 返回 HTTP {}", response.status()));
    }
    let text = response
        .text()
        .map_err(|e| format!("读取远程 hosts 失败: {e}"))?;
    Ok(text)
}

fn mark_refreshed(scheme: &mut HostsScheme, content: String) {
    let ms = now_ms();
    scheme.content = content;
    scheme.last_refresh_ms = ms;
    scheme.last_refresh = format_last_refresh(ms);
}

/// Refresh one remote scheme by id; re-applies system hosts when enabled.
pub fn refresh_scheme(app: &AppHandle, id: &str) -> Result<Vec<HostsScheme>, String> {
    let mut schemes = load_schemes(app);
    let Some(item) = schemes.iter_mut().find(|s| s.id == id) else {
        return Err("方案不存在".to_string());
    };
    if !is_remote(item) {
        return Err("仅远程方案支持刷新".to_string());
    }
    let url = item.url.clone();
    if url.trim().is_empty() {
        return Err("远程 URL 不能为空".to_string());
    }
    let content = fetch_remote_content(&url)?;
    mark_refreshed(item, content);
    let should_apply = item.enabled;
    save_schemes(app, &schemes)?;
    if should_apply {
        apply_enabled(app)?;
    }
    Ok(schemes)
}

fn refresh_due_schemes(app: &AppHandle) {
    let schemes = load_schemes(app);
    let now = now_ms();
    let due_ids: Vec<String> = schemes
        .iter()
        .filter(|s| {
            is_remote(s)
                && !s.url.trim().is_empty()
                && s.refresh_interval > 0
                && now.saturating_sub(s.last_refresh_ms) >= s.refresh_interval.saturating_mul(1000)
        })
        .map(|s| s.id.clone())
        .collect();
    for id in due_ids {
        if let Err(err) = refresh_scheme(app, &id) {
            eprintln!("[hosts] auto refresh failed for {id}: {err}");
        }
    }
}

/// Background poller for remote hosts refresh intervals.
pub fn start_refresh_loop(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(30));
        refresh_due_schemes(&app);
    });
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported: u32,
    pub skipped: u32,
    pub schemes: Vec<HostsScheme>,
}

fn new_id() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = ID_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("h-{ms:x}-{seq:x}")
}

fn load_schemes(app: &AppHandle) -> Vec<HostsScheme> {
    let Ok(store) = app.store(STORE_NAME) else {
        return Vec::new();
    };
    let Some(value) = store.get(SCHEMES_KEY) else {
        return Vec::new();
    };
    let mut schemes: Vec<HostsScheme> = serde_json::from_value(value).unwrap_or_default();
    for scheme in &mut schemes {
        normalize_scheme(scheme);
    }
    schemes
}

fn save_schemes(app: &AppHandle, schemes: &[HostsScheme]) -> Result<(), String> {
    let store = app.store(STORE_NAME).map_err(|e| e.to_string())?;
    let value = serde_json::to_value(schemes).map_err(|e| e.to_string())?;
    store.set(SCHEMES_KEY, value);
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_schemes(app: &AppHandle) -> Vec<HostsScheme> {
    load_schemes(app)
}

pub fn upsert_scheme(
    app: &AppHandle,
    id: Option<String>,
    title: String,
    content: String,
    enabled: Option<bool>,
    nature: Option<String>,
    scheme_type: Option<String>,
    url: Option<String>,
    refresh_interval: Option<u64>,
) -> Result<Vec<HostsScheme>, String> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err("方案标题不能为空".to_string());
    }

    let mut schemes = load_schemes(app);
    let target_id;
    let should_apply;
    let need_first_refresh;

    if let Some(id) = id.filter(|s| !s.is_empty()) {
        let Some(item) = schemes.iter_mut().find(|s| s.id == id) else {
            return Err("方案不存在".to_string());
        };
        let prev_url = item.url.clone();
        let prev_last = item.last_refresh_ms;

        if let Some(raw_type) = scheme_type {
            let next_type = normalize_scheme_type(&raw_type)?;
            if next_type == TYPE_LOCAL {
                // Explicit local: clear remote metadata.
                item.scheme_type = TYPE_LOCAL.to_string();
                item.url.clear();
                item.refresh_interval = 0;
                item.last_refresh.clear();
                item.last_refresh_ms = 0;
                item.readonly = false;
            } else {
                item.scheme_type = TYPE_REMOTE.to_string();
                item.readonly = true;
            }
        }
        if let Some(url) = url {
            item.url = url.trim().to_string();
        }
        if let Some(refresh_interval) = refresh_interval {
            item.refresh_interval = normalize_refresh_interval(refresh_interval)?;
        }

        // URL present => always treat as remote (prevents accidental wipe via omitted type).
        if !item.url.trim().is_empty() {
            item.scheme_type = TYPE_REMOTE.to_string();
            item.readonly = true;
        }

        if item.scheme_type == TYPE_REMOTE && item.url.trim().is_empty() {
            return Err("远程方案必须填写 URL".to_string());
        }

        item.title = title;
        if item.scheme_type != TYPE_REMOTE {
            item.content = content;
        }
        if let Some(enabled) = enabled {
            item.enabled = enabled;
        }
        if let Some(nature) = nature {
            item.nature = normalize_nature(&nature)?;
        }

        need_first_refresh = item.scheme_type == TYPE_REMOTE
            && !item.url.trim().is_empty()
            && (prev_url != item.url || prev_last == 0);
        should_apply = item.enabled;
        target_id = item.id.clone();
    } else {
        let enabled = enabled.unwrap_or(false);
        let nature = match nature {
            Some(n) => normalize_nature(&n)?,
            None => NATURE_EXCLUSIVE.to_string(),
        };
        let scheme_type = match scheme_type {
            Some(t) => normalize_scheme_type(&t)?,
            None => TYPE_LOCAL.to_string(),
        };
        let url = url.unwrap_or_default().trim().to_string();
        let refresh_interval = match refresh_interval {
            Some(v) => normalize_refresh_interval(v)?,
            None => 0,
        };
        if scheme_type == TYPE_REMOTE && url.is_empty() {
            return Err("远程方案必须填写 URL".to_string());
        }
        let remote = scheme_type == TYPE_REMOTE;
        let new_id = new_id();
        schemes.push(HostsScheme {
            id: new_id.clone(),
            title,
            content: if remote { String::new() } else { content },
            enabled,
            source: "local".to_string(),
            nature,
            readonly: remote,
            scheme_type,
            url,
            refresh_interval: if remote { refresh_interval } else { 0 },
            last_refresh: String::new(),
            last_refresh_ms: 0,
        });
        need_first_refresh = remote;
        should_apply = enabled;
        target_id = new_id;
    }

    if should_apply {
        apply_exclusive_mutex(&mut schemes, &target_id);
    }

    save_schemes(app, &schemes)?;

    if need_first_refresh {
        // First save / URL change: always fetch once even when refresh_interval is 0.
        refresh_scheme(app, &target_id)?;
        return Ok(load_schemes(app));
    }

    if should_apply {
        apply_enabled(app)?;
    }
    Ok(schemes)
}

fn apply_exclusive_mutex(schemes: &mut [HostsScheme], enabled_id: &str) {
    let Some(target) = schemes.iter().find(|s| s.id == enabled_id) else {
        return;
    };
    if !target.enabled || !is_exclusive(target) {
        return;
    }
    for s in schemes.iter_mut() {
        if s.id != enabled_id && is_exclusive(s) {
            s.enabled = false;
        }
    }
}

pub fn delete_scheme(app: &AppHandle, id: String) -> Result<Vec<HostsScheme>, String> {
    let mut schemes = load_schemes(app);
    let was_enabled = schemes
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.enabled)
        .unwrap_or(false);
    let before = schemes.len();
    schemes.retain(|s| s.id != id);
    if schemes.len() == before {
        return Err("方案不存在".to_string());
    }
    save_schemes(app, &schemes)?;
    if was_enabled {
        apply_enabled(app)?;
    }
    Ok(schemes)
}

pub fn rename_scheme(app: &AppHandle, id: String, title: String) -> Result<Vec<HostsScheme>, String> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err("方案标题不能为空".to_string());
    }
    let mut schemes = load_schemes(app);
    let Some(item) = schemes.iter_mut().find(|s| s.id == id) else {
        return Err("方案不存在".to_string());
    };
    item.title = title;
    save_schemes(app, &schemes)?;
    Ok(schemes)
}

pub fn set_nature(app: &AppHandle, id: String, nature: String) -> Result<Vec<HostsScheme>, String> {
    let nature = normalize_nature(&nature)?;
    let mut schemes = load_schemes(app);
    let Some(item) = schemes.iter_mut().find(|s| s.id == id) else {
        return Err("方案不存在".to_string());
    };
    item.nature = nature;
    let enabled = item.enabled;
    let exclusive = is_exclusive(item);
    if enabled && exclusive {
        apply_exclusive_mutex(&mut schemes, &id);
    }
    save_schemes(app, &schemes)?;
    if enabled {
        apply_enabled(app)?;
    }
    Ok(schemes)
}

pub fn set_enabled(app: &AppHandle, id: String, enabled: bool) -> Result<Vec<HostsScheme>, String> {
    let mut schemes = load_schemes(app);
    let Some(item) = schemes.iter_mut().find(|s| s.id == id) else {
        return Err("方案不存在".to_string());
    };
    item.enabled = enabled;
    if enabled {
        apply_exclusive_mutex(&mut schemes, &id);
    }
    save_schemes(app, &schemes)?;
    apply_enabled(app)?;
    Ok(schemes)
}

pub fn apply_enabled(app: &AppHandle) -> Result<(), String> {
    let schemes = load_schemes(app);
    let mut parts: Vec<String> = Vec::new();
    for scheme in schemes.iter().filter(|s| s.enabled) {
        let body = scheme.content.trim();
        if body.is_empty() {
            continue;
        }
        parts.push(format!("# [{}]\n{}", scheme.title, body));
    }
    let managed = parts.join("\n\n");
    write_system_hosts(&managed)
}

/// Whether the current process can write the system hosts file without elevation.
pub fn has_write_access() -> bool {
    let path = system_hosts_path();
    fs::OpenOptions::new().write(true).open(&path).is_ok()
}

/// Re-apply current managed hosts (may trigger UAC / elevation).
pub fn request_permission(app: &AppHandle) -> Result<(), String> {
    apply_enabled(app)
}

/// Read the current system hosts file (view-only for UI).
pub fn read_system_hosts() -> Result<String, String> {
    let path = system_hosts_path();
    fs::read_to_string(&path).map_err(|e| format!("读取系统 hosts 失败: {e}"))
}

/// Open the system hosts file with the OS default application.
pub fn open_system_hosts(app: &AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = system_hosts_path();
    if !path.exists() {
        return Err(format!("系统 hosts 不存在: {}", path.display()));
    }
    app.opener()
        .open_path(path.to_string_lossy().as_ref(), None::<&str>)
        .map_err(|e| format!("打开系统 hosts 失败: {e}"))
}

fn system_hosts_path() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts")
    } else {
        PathBuf::from("/etc/hosts")
    }
}

fn strip_managed_block(raw: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed == MARKER_BEGIN {
            skipping = true;
            continue;
        }
        if trimmed == MARKER_END {
            skipping = false;
            continue;
        }
        if skipping {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn compose_hosts_file(existing: &str, managed: &str) -> String {
    let mut base = strip_managed_block(existing);
    while base.ends_with("\n\n\n") {
        base.pop();
    }
    if !base.is_empty() && !base.ends_with('\n') {
        base.push('\n');
    }
    let managed = managed.trim();
    if managed.is_empty() {
        return base;
    }
    if !base.is_empty() && !base.ends_with("\n\n") {
        if base.ends_with('\n') {
            base.push('\n');
        } else {
            base.push_str("\n\n");
        }
    }
    base.push_str(MARKER_BEGIN);
    base.push('\n');
    base.push_str(managed);
    if !managed.ends_with('\n') {
        base.push('\n');
    }
    base.push_str(MARKER_END);
    base.push('\n');
    base
}

fn write_system_hosts(managed: &str) -> Result<(), String> {
    let path = system_hosts_path();
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let next = compose_hosts_file(&existing, managed);

    match fs::write(&path, next.as_bytes()) {
        Ok(()) => {
            flush_dns();
            Ok(())
        }
        Err(err) => {
            let kind = err.kind();
            if kind == std::io::ErrorKind::PermissionDenied
                || kind == std::io::ErrorKind::Other
                || cfg!(windows)
            {
                write_hosts_elevated(&path, &next)?;
                flush_dns();
                Ok(())
            } else {
                Err(format!("写入系统 hosts 失败: {err}"))
            }
        }
    }
}

fn write_hosts_elevated(dest: &Path, content: &str) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!("jdd-crypto-hosts-{}.tmp", new_id()));
    fs::write(&tmp, content.as_bytes()).map_err(|e| format!("写入临时 hosts 失败: {e}"))?;

    let result = if cfg!(windows) {
        elevate_copy_windows(&tmp, dest)
    } else if cfg!(target_os = "macos") {
        elevate_copy_macos(&tmp, dest)
    } else {
        elevate_copy_linux(&tmp, dest)
    };

    let _ = fs::remove_file(&tmp);
    result
}

#[cfg(windows)]
fn elevate_copy_windows(src: &Path, dest: &Path) -> Result<(), String> {
    let ps1 = std::env::temp_dir().join(format!("jdd-crypto-hosts-copy-{}.ps1", new_id()));
    let src_s = src.to_string_lossy().replace('\'', "''");
    let dest_s = dest.to_string_lossy().replace('\'', "''");
    let script = format!(
        "Copy-Item -LiteralPath '{src_s}' -Destination '{dest_s}' -Force\r\n"
    );
    fs::write(&ps1, script.as_bytes()).map_err(|e| format!("写入提权脚本失败: {e}"))?;

    let ps1_arg = ps1.to_string_lossy().replace('\'', "''");
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &format!(
                "Start-Process -FilePath powershell -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','{ps1_arg}'"
            ),
        ])
        .status()
        .map_err(|e| {
            let _ = fs::remove_file(&ps1);
            format!("提权写入失败（无法启动 PowerShell）: {e}")
        })?;
    let _ = fs::remove_file(&ps1);

    if !status.success() {
        return Err(
            "提权写入系统 hosts 失败或被取消，请以管理员身份运行应用后重试".to_string(),
        );
    }

    let expect_marker = fs::read_to_string(src)
        .map(|s| s.contains(MARKER_BEGIN))
        .unwrap_or(false);
    let written = fs::read_to_string(dest).unwrap_or_default();
    if expect_marker && !written.contains(MARKER_BEGIN) {
        return Err(
            "提权写入未生效（可能取消了 UAC），请以管理员身份运行应用后重试".to_string(),
        );
    }
    Ok(())
}

#[cfg(not(windows))]
fn elevate_copy_windows(_src: &Path, _dest: &Path) -> Result<(), String> {
    Err("非 Windows 平台".to_string())
}

#[cfg(target_os = "macos")]
fn elevate_copy_macos(src: &Path, dest: &Path) -> Result<(), String> {
    let src_s = src.to_string_lossy().replace('"', "\\\"");
    let dest_s = dest.to_string_lossy().replace('"', "\\\"");
    let script = format!(
        r#"do shell script "cp \"{src_s}\" \"{dest_s}\"" with administrator privileges"#
    );
    let status = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .status()
        .map_err(|e| format!("提权写入失败（无法启动 osascript）: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("提权写入系统 hosts 失败或被取消".to_string())
    }
}

#[cfg(not(target_os = "macos"))]
fn elevate_copy_macos(_src: &Path, _dest: &Path) -> Result<(), String> {
    Err("非 macOS 平台".to_string())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn elevate_copy_linux(src: &Path, dest: &Path) -> Result<(), String> {
    let attempts = [
        ("pkexec", vec!["cp", src.to_str().unwrap_or(""), dest.to_str().unwrap_or("")]),
        (
            "sudo",
            vec!["cp", src.to_str().unwrap_or(""), dest.to_str().unwrap_or("")],
        ),
    ];
    for (bin, args) in attempts {
        if let Ok(status) = Command::new(bin).args(&args).status() {
            if status.success() {
                return Ok(());
            }
        }
    }
    Err("写入系统 hosts 需要管理员权限，请使用 pkexec/sudo 或提升权限后重试".to_string())
}

#[cfg(not(all(unix, not(target_os = "macos"))))]
fn elevate_copy_linux(_src: &Path, _dest: &Path) -> Result<(), String> {
    Err("非 Linux 平台".to_string())
}

fn flush_dns() {
    if cfg!(windows) {
        let _ = Command::new("ipconfig").arg("/flushdns").status();
    } else if cfg!(target_os = "macos") {
        let _ = Command::new("dscacheutil").args(["-flushcache"]).status();
        let _ = Command::new("killall").args(["-HUP", "mDNSResponder"]).status();
    }
    // Linux: no portable flush; skip.
}

/// Import SwitchHosts backup JSON locked to `swh_data.json` PotDb shape:
/// `{ version, data: { list: { tree: [...] }, collection: { hosts: { data: [{id,content}] } } } }`
/// Tree nodes carry metadata (`title`/`type`/`on`/`id`/`children`); hosts body is joined by `id`.
pub fn import_switchhosts(app: &AppHandle, raw: String) -> Result<ImportResult, String> {
    let value: Value = serde_json::from_str(&raw).map_err(|e| format!("JSON 解析失败: {e}"))?;
    let data = value
        .get("data")
        .ok_or_else(|| "备份格式不正确：缺少 data 字段".to_string())?;
    if !data.is_object() {
        return Err(
            "备份格式不正确：data 应为对象（SwitchHosts PotDb：list.tree + collection.hosts）"
                .to_string(),
        );
    }

    let tree = data
        .pointer("/list/tree")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "备份格式不正确：缺少 data.list.tree 数组".to_string())?;

    let content_by_id = build_hosts_content_map(data);
    let mut imported = 0u32;
    let mut skipped = 0u32;
    let mut schemes = load_schemes(app);

    for node in tree {
        collect_import_nodes(node, &content_by_id, &mut schemes, &mut imported, &mut skipped);
    }

    // Imports are exclusive: keep at most one exclusive scheme enabled.
    let mut exclusive_on = false;
    for s in schemes.iter_mut() {
        if !is_exclusive(s) || !s.enabled {
            continue;
        }
        if exclusive_on {
            s.enabled = false;
        } else {
            exclusive_on = true;
        }
    }

    save_schemes(app, &schemes)?;
    if schemes.iter().any(|s| s.enabled) {
        apply_enabled(app)?;
    }
    Ok(ImportResult {
        imported,
        skipped,
        schemes,
    })
}

/// Export schemes as SwitchHosts PotDb backup JSON (same shape as the sample fixture).
pub fn export_switchhosts(app: &AppHandle) -> Result<String, String> {
    let schemes = load_schemes(app);
    build_switchhosts_export(&schemes, &current_system_hosts_base())
}

/// Disable all schemes and strip the managed block from the system hosts file.
pub fn reset_system_hosts(app: &AppHandle) -> Result<Vec<HostsScheme>, String> {
    let mut schemes = load_schemes(app);
    for scheme in &mut schemes {
        scheme.enabled = false;
    }
    save_schemes(app, &schemes)?;
    write_system_hosts("")?;
    Ok(schemes)
}

fn current_system_hosts_base() -> String {
    let path = system_hosts_path();
    let raw = fs::read_to_string(&path).unwrap_or_default();
    strip_managed_block(&raw)
}

fn build_switchhosts_export(schemes: &[HostsScheme], system_content: &str) -> Result<String, String> {
    let mut tree = Vec::with_capacity(schemes.len());
    let mut hosts_data = Vec::with_capacity(schemes.len() + 1);

    hosts_data.push(serde_json::json!({
        "id": "0",
        "content": system_content,
        "_id": "1"
    }));

    let mut next_id = 2u64;
    for scheme in schemes {
        let mut node = serde_json::Map::new();
        if is_remote(scheme) {
            node.insert("type".into(), Value::String(TYPE_REMOTE.to_string()));
            node.insert("title".into(), Value::String(scheme.title.clone()));
            node.insert("url".into(), Value::String(scheme.url.clone()));
            if scheme.refresh_interval > 0 {
                node.insert(
                    "refresh_interval".into(),
                    Value::Number(scheme.refresh_interval.into()),
                );
            }
            node.insert("id".into(), Value::String(scheme.id.clone()));
            if !scheme.last_refresh.is_empty() {
                node.insert(
                    "last_refresh".into(),
                    Value::String(scheme.last_refresh.clone()),
                );
            }
            if scheme.last_refresh_ms > 0 {
                node.insert(
                    "last_refresh_ms".into(),
                    Value::Number(scheme.last_refresh_ms.into()),
                );
            }
            node.insert("on".into(), Value::Bool(scheme.enabled));
        } else {
            node.insert("title".into(), Value::String(scheme.title.clone()));
            node.insert("id".into(), Value::String(scheme.id.clone()));
            node.insert("on".into(), Value::Bool(scheme.enabled));
        }
        tree.push(Value::Object(node));

        hosts_data.push(serde_json::json!({
            "id": scheme.id,
            "content": scheme.content,
            "_id": next_id.to_string()
        }));
        next_id += 1;
    }

    let hosts_index = hosts_data.len() as u64;
    let export = serde_json::json!({
        "data": {
            "dict": {},
            "list": { "tree": tree },
            "set": {},
            "collection": {
                "history": {
                    "meta": { "index": 0 },
                    "data": [],
                    "index_keys": []
                },
                "hosts": {
                    "meta": { "index": hosts_index },
                    "data": hosts_data,
                    "index_keys": []
                }
            }
        },
        "version": [4, 1, 1, 6077]
    });

    serde_json::to_string_pretty(&export).map_err(|e| format!("导出序列化失败: {e}"))
}

fn build_hosts_content_map(data: &Value) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let Some(rows) = data
        .pointer("/collection/hosts/data")
        .and_then(|v| v.as_array())
    else {
        return map;
    };
    for row in rows {
        let id = row
            .get("id")
            .map(|v| match v {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                _ => String::new(),
            })
            .unwrap_or_default();
        if id.is_empty() || id == "0" {
            continue;
        }
        let Some(content) = row.get("content").and_then(|v| v.as_str()) else {
            continue;
        };
        map.insert(id, content.to_string());
    }
    map
}

fn collect_import_nodes(
    node: &Value,
    content_by_id: &std::collections::HashMap<String, String>,
    schemes: &mut Vec<HostsScheme>,
    imported: &mut u32,
    skipped: &mut u32,
) {
    let typ = node
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("local")
        .to_ascii_lowercase();

    match typ.as_str() {
        "system" => {
            *skipped += 1;
        }
        "folder" | "group" => {
            if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
                for child in children {
                    collect_import_nodes(child, content_by_id, schemes, imported, skipped);
                }
            } else {
                *skipped += 1;
            }
        }
        "local" | "remote" => {
            let id = node
                .get("id")
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    _ => String::new(),
                })
                .unwrap_or_default();
            if id == "0" {
                *skipped += 1;
                return;
            }

            let content = content_by_id
                .get(&id)
                .map(|s| s.as_str())
                .or_else(|| node.get("content").and_then(|v| v.as_str()))
                .unwrap_or("")
                .trim()
                .to_string();
            let enabled = node.get("on").and_then(|v| v.as_bool()).unwrap_or(false);
            let remote = typ == "remote";
            let url = node
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            // Allow remote with URL even when snapshot content is empty (will refresh later).
            if content.is_empty() && !(remote && !url.is_empty()) {
                *skipped += 1;
                return;
            }
            let refresh_interval = node
                .get("refresh_interval")
                .and_then(|v| v.as_u64())
                .filter(|v| ALLOWED_REFRESH_INTERVALS.contains(v))
                .unwrap_or(0);
            let last_refresh = node
                .get("last_refresh")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let last_refresh_ms = node
                .get("last_refresh_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            let title = node
                .get("title")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("未命名方案")
                .to_string();
            // Use SwitchHosts tree id so re-import updates the same scheme (join key with collection).
            let scheme_id = if id.is_empty() { new_id() } else { id };
            let incoming = HostsScheme {
                id: scheme_id,
                title,
                content,
                enabled,
                source: "imported".to_string(),
                nature: NATURE_EXCLUSIVE.to_string(),
                readonly: remote,
                scheme_type: if remote {
                    TYPE_REMOTE.to_string()
                } else {
                    TYPE_LOCAL.to_string()
                },
                url: if remote { url } else { String::new() },
                refresh_interval: if remote { refresh_interval } else { 0 },
                last_refresh: if remote { last_refresh } else { String::new() },
                last_refresh_ms: if remote { last_refresh_ms } else { 0 },
            };
            upsert_imported_scheme(schemes, incoming);
            *imported += 1;
        }
        _ => {
            *skipped += 1;
        }
    }
}

/// Merge imported scheme by SwitchHosts id; fall back to same title among prior imports.
fn upsert_imported_scheme(schemes: &mut Vec<HostsScheme>, incoming: HostsScheme) {
    if let Some(existing) = schemes.iter_mut().find(|s| s.id == incoming.id) {
        let kept_nature = existing.nature.clone();
        *existing = incoming;
        existing.nature = kept_nature;
        existing.source = "imported".to_string();
        return;
    }
    // Legacy rows used random `h-...` ids — update by title so re-import repairs missing url.
    if let Some(idx) = schemes.iter().position(|s| {
        s.source == "imported" && s.title == incoming.title
    }) {
        let kept_nature = schemes[idx].nature.clone();
        let mut next = incoming;
        next.nature = kept_nature;
        schemes[idx] = next;
        return;
    }
    schemes.push(incoming);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_inserts_and_replaces_marker_block() {
        let existing = "127.0.0.1 localhost\n";
        let once = compose_hosts_file(existing, "1.1.1.1 a.test");
        assert!(once.contains(MARKER_BEGIN));
        assert!(once.contains("1.1.1.1 a.test"));
        assert!(once.contains("127.0.0.1 localhost"));

        let twice = compose_hosts_file(&once, "2.2.2.2 b.test");
        assert_eq!(twice.matches(MARKER_BEGIN).count(), 1);
        assert!(twice.contains("2.2.2.2 b.test"));
        assert!(!twice.contains("1.1.1.1 a.test"));
    }

    #[test]
    fn compose_clears_block_when_empty() {
        let with_block = format!(
            "127.0.0.1 localhost\n\n{MARKER_BEGIN}\nold\n{MARKER_END}\n"
        );
        let cleared = compose_hosts_file(&with_block, "");
        assert!(!cleared.contains(MARKER_BEGIN));
        assert!(cleared.contains("127.0.0.1 localhost"));
    }

    #[test]
    fn export_matches_switchhosts_shape_and_roundtrips() {
        let schemes = vec![
            HostsScheme {
                id: "19d546e9-6488-48fb-b471-47403388d9e7".into(),
                title: "本地".into(),
                content: "1.1.1.1 local.test".into(),
                enabled: true,
                source: "imported".into(),
                nature: NATURE_KEEP.into(),
                readonly: false,
                scheme_type: TYPE_LOCAL.into(),
                url: String::new(),
                refresh_interval: 0,
                last_refresh: String::new(),
                last_refresh_ms: 0,
            },
            HostsScheme {
                id: "2e24af79-dabe-4981-aec0-09ae83a7ad4a".into(),
                title: "测试-流量池".into(),
                content: "2.2.2.2 remote.test".into(),
                enabled: false,
                source: "imported".into(),
                nature: NATURE_EXCLUSIVE.into(),
                readonly: true,
                scheme_type: TYPE_REMOTE.into(),
                url: "http://example.com/hosts".into(),
                refresh_interval: 300,
                last_refresh: "2026-09-03 14:39:00".into(),
                last_refresh_ms: 1788417540808,
            },
        ];
        let raw = build_switchhosts_export(&schemes, "127.0.0.1 localhost\n").unwrap();
        let value: Value = serde_json::from_str(&raw).unwrap();
        assert!(value.pointer("/data/list/tree").and_then(|v| v.as_array()).is_some());
        assert!(value
            .pointer("/data/collection/hosts/data")
            .and_then(|v| v.as_array())
            .is_some());
        assert_eq!(
            value.pointer("/version").and_then(|v| v.as_array()).map(|a| a.len()),
            Some(4)
        );
        let tree = value.pointer("/data/list/tree").unwrap().as_array().unwrap();
        assert!(tree[0].get("type").is_none());
        assert_eq!(tree[1].get("type").and_then(|v| v.as_str()), Some("remote"));
        assert_eq!(
            tree[1].get("refresh_interval").and_then(|v| v.as_u64()),
            Some(300)
        );

        let data = value.get("data").unwrap();
        let content_by_id = build_hosts_content_map(data);
        let mut imported_schemes = Vec::new();
        let mut imported = 0u32;
        let mut skipped = 0u32;
        for node in tree {
            collect_import_nodes(
                node,
                &content_by_id,
                &mut imported_schemes,
                &mut imported,
                &mut skipped,
            );
        }
        assert_eq!(imported_schemes.len(), 2);
        assert_eq!(imported_schemes[0].id, schemes[0].id);
        assert_eq!(imported_schemes[1].scheme_type, TYPE_REMOTE);
        assert_eq!(imported_schemes[1].url, "http://example.com/hosts");
        assert_eq!(imported_schemes[1].refresh_interval, 300);
    }

    #[test]
    fn import_parses_swh_potdb_shape() {
        let raw = include_str!("../../docs/fixtures/switchhosts-backup.sample.json");
        let value: Value = serde_json::from_str(raw).unwrap();
        let data = value.get("data").unwrap();
        let tree = data.pointer("/list/tree").unwrap().as_array().unwrap();
        let content_by_id = build_hosts_content_map(data);
        let mut schemes = Vec::new();
        let mut imported = 0u32;
        let mut skipped = 0u32;
        for node in tree {
            collect_import_nodes(node, &content_by_id, &mut schemes, &mut imported, &mut skipped);
        }
        assert!(imported >= 2);
        assert_eq!(schemes[0].title, "本地");
        assert_eq!(schemes[0].id, "19d546e9-6488-48fb-b471-47403388d9e7");
        assert!(schemes[0].enabled);
        assert_eq!(schemes[0].scheme_type, TYPE_LOCAL);
        assert_eq!(schemes[1].title, "基础");
        assert_eq!(schemes[1].id, "a73cf8fd-fed2-413a-83b8-8b7b86a9f02f");
        assert_eq!(schemes[1].scheme_type, TYPE_REMOTE);
        assert!(!schemes[1].url.is_empty());
        assert_eq!(schemes[1].refresh_interval, 0); // missing in fixture => never
        assert!(!schemes[0].readonly);
        assert!(schemes[1].readonly);
        let with_interval = schemes
            .iter()
            .find(|s| s.title.contains("流量池"))
            .expect("remote with refresh_interval");
        assert_eq!(with_interval.id, "2e24af79-dabe-4981-aec0-09ae83a7ad4a");
        assert_eq!(with_interval.refresh_interval, 300);
        assert!(!with_interval.url.is_empty());
        assert!(with_interval.last_refresh_ms > 0);

        // IPC/store shape uses camelCase schemeType while SwitchHosts import still reads `type`.
        let json = serde_json::to_value(&schemes[1]).unwrap();
        assert_eq!(json.get("schemeType").and_then(|v| v.as_str()), Some("remote"));
        assert!(json.get("url").and_then(|v| v.as_str()).unwrap_or("").len() > 0);

        // Re-import merges by id — count must not grow.
        let before = schemes.len();
        let mut imported2 = 0u32;
        let mut skipped2 = 0u32;
        for node in tree {
            collect_import_nodes(node, &content_by_id, &mut schemes, &mut imported2, &mut skipped2);
        }
        assert_eq!(schemes.len(), before);
        assert_eq!(imported2, imported);

        // Legacy h- id + same title should be repaired (url filled, id replaced).
        let mut legacy = vec![HostsScheme {
            id: "h-legacy-1".into(),
            title: "基础".into(),
            content: "old".into(),
            enabled: false,
            source: "imported".into(),
            nature: NATURE_KEEP.into(),
            readonly: false,
            scheme_type: TYPE_LOCAL.into(),
            url: String::new(),
            refresh_interval: 0,
            last_refresh: String::new(),
            last_refresh_ms: 0,
        }];
        let mut imp = 0u32;
        let mut skip = 0u32;
        // Only import the "基础" remote node
        collect_import_nodes(&tree[1], &content_by_id, &mut legacy, &mut imp, &mut skip);
        assert_eq!(legacy.len(), 1);
        assert_eq!(legacy[0].id, "a73cf8fd-fed2-413a-83b8-8b7b86a9f02f");
        assert_eq!(legacy[0].scheme_type, TYPE_REMOTE);
        assert!(!legacy[0].url.is_empty());
        assert_eq!(legacy[0].nature, NATURE_KEEP); // preserved
    }
}
