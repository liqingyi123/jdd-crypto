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
}

fn default_nature() -> String {
    "keep".to_string()
}

pub const NATURE_KEEP: &str = "keep";
pub const NATURE_EXCLUSIVE: &str = "exclusive";

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
    serde_json::from_value(value).unwrap_or_default()
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
) -> Result<Vec<HostsScheme>, String> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err("方案标题不能为空".to_string());
    }

    let mut schemes = load_schemes(app);
    let target_id;
    let should_apply;

    if let Some(id) = id.filter(|s| !s.is_empty()) {
        let Some(item) = schemes.iter_mut().find(|s| s.id == id) else {
            return Err("方案不存在".to_string());
        };
        if item.readonly && item.content != content {
            return Err("只读方案不可编辑 Host 内容".to_string());
        }
        item.title = title;
        if !item.readonly {
            item.content = content;
        }
        if let Some(enabled) = enabled {
            item.enabled = enabled;
        }
        if let Some(nature) = nature {
            item.nature = normalize_nature(&nature)?;
        }
        should_apply = item.enabled;
        target_id = item.id.clone();
    } else {
        let enabled = enabled.unwrap_or(false);
        let nature = match nature {
            Some(n) => normalize_nature(&n)?,
            None => NATURE_EXCLUSIVE.to_string(),
        };
        let new_id = new_id();
        schemes.push(HostsScheme {
            id: new_id.clone(),
            title,
            content,
            enabled,
            source: "local".to_string(),
            nature,
            readonly: false,
        });
        should_apply = enabled;
        target_id = new_id;
    }

    if should_apply {
        apply_exclusive_mutex(&mut schemes, &target_id);
    }

    save_schemes(app, &schemes)?;
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
            if content.is_empty() {
                *skipped += 1;
                return;
            }

            let title = node
                .get("title")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("未命名方案")
                .to_string();
            let enabled = node.get("on").and_then(|v| v.as_bool()).unwrap_or(false);
            // Remote snapshots stay readonly; local imports remain editable.
            let readonly = typ == "remote";
            schemes.push(HostsScheme {
                id: new_id(),
                title,
                content,
                enabled,
                source: "imported".to_string(),
                nature: NATURE_EXCLUSIVE.to_string(),
                readonly,
            });
            *imported += 1;
        }
        _ => {
            *skipped += 1;
        }
    }
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
        // local-1, remote-1, remote-2, folder->local-2  => 4 imported; folder itself not counted
        assert_eq!(imported, 4);
        assert_eq!(skipped, 0);
        assert_eq!(schemes[0].title, "本地");
        assert!(schemes[0].enabled);
        assert!(schemes[0].content.contains("local.dev.example"));
        assert_eq!(schemes[1].title, "基础");
        assert!(schemes[1].enabled);
        assert_eq!(schemes[2].title, "测试环境");
        assert!(!schemes[2].enabled);
        assert_eq!(schemes[3].title, "子方案");
        assert!(schemes.iter().all(|s| s.nature == NATURE_EXCLUSIVE));
        assert!(!schemes[0].readonly); // local
        assert!(schemes[1].readonly); // remote
        assert!(schemes[2].readonly); // remote
        assert!(!schemes[3].readonly); // local under folder
    }
}
