//! TeamAI tool preferences (repo URL + access token).

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

#[cfg(any(windows, target_os = "macos"))]
use std::process::{Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const SETTINGS_STORE: &str = "settings.json";
const PREF_KEY: &str = "teamaiPref";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub const DEFAULT_REPO_URL: &str = "http://git.jiangduoduo.com/qianduan/ai-standards.git";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamaiPref {
    #[serde(default = "default_repo_url")]
    pub repo_url: String,
    #[serde(default)]
    pub access_token: String,
}

fn default_repo_url() -> String {
    DEFAULT_REPO_URL.to_string()
}

impl Default for TeamaiPref {
    fn default() -> Self {
        Self {
            repo_url: DEFAULT_REPO_URL.to_string(),
            access_token: String::new(),
        }
    }
}

fn normalize(pref: &mut TeamaiPref) {
    pref.repo_url = pref.repo_url.trim().to_string();
    pref.access_token = pref.access_token.trim().to_string();
}

fn fill_token_from_env(pref: &mut TeamaiPref) {
    if !pref.access_token.is_empty() {
        return;
    }
    let Ok(raw) = std::env::var("GITLAB_TOKEN") else {
        return;
    };
    let trimmed = raw.trim();
    if !trimmed.is_empty() {
        pref.access_token = trimmed.to_string();
    }
}

/// Extract `scheme://host[:port]` from a GitLab repo URL (no path).
pub fn gitlab_base_url(repo_url: &str) -> Result<String, String> {
    let trimmed = repo_url.trim();
    if trimmed.is_empty() {
        return Err("仓库地址不能为空".to_string());
    }

    let (scheme, rest) = if let Some(rest) = trimmed.strip_prefix("https://") {
        ("https", rest)
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        ("http", rest)
    } else {
        return Err("仓库地址需以 http:// 或 https:// 开头".to_string());
    };

    let authority = rest.split(['/', '?', '#']).next().unwrap_or("").trim();
    if authority.is_empty() || authority.starts_with('@') {
        return Err("仓库地址无效".to_string());
    }

    // Reject empty host when only ":" or ":port" appears.
    let host_part = authority.split('@').next_back().unwrap_or(authority);
    if host_part.is_empty() || host_part == ":" || host_part.starts_with(':') {
        return Err("仓库地址无效".to_string());
    }

    Ok(format!("{scheme}://{authority}"))
}

fn sync_gitlab_env(token: &str, gitlab_url: &str) -> Result<(), String> {
    // Current process — so subsequent loads in this session see the values.
    std::env::set_var("GITLAB_TOKEN", token);
    std::env::set_var("GITLAB_URL", gitlab_url);

    #[cfg(windows)]
    {
        persist_user_env_windows("GITLAB_TOKEN", token)?;
        persist_user_env_windows("GITLAB_URL", gitlab_url)?;
    }

    #[cfg(target_os = "macos")]
    {
        persist_user_env_macos("GITLAB_TOKEN", token)?;
        persist_user_env_macos("GITLAB_URL", gitlab_url)?;
    }

    Ok(())
}

#[cfg(windows)]
fn persist_user_env_windows(name: &str, value: &str) -> Result<(), String> {
    // Escape for single-quoted PowerShell string.
    let name_esc = name.replace('\'', "''");
    let value_esc = value.replace('\'', "''");
    let script =
        format!("[Environment]::SetEnvironmentVariable('{name_esc}', '{value_esc}', 'User')");
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
    .stderr(Stdio::piped())
    .creation_flags(CREATE_NO_WINDOW);

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

#[cfg(target_os = "macos")]
fn persist_user_env_macos(name: &str, value: &str) -> Result<(), String> {
    let status = Command::new("launchctl")
        .args(["setenv", name, value])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("写入环境变量 {name} 失败: {e}"))?;
    if !status.success() {
        return Err(format!("写入环境变量 {name} 失败"));
    }
    Ok(())
}

pub fn load(app: &AppHandle) -> TeamaiPref {
    let mut pref = match app.store(SETTINGS_STORE) {
        Ok(store) => store
            .get(PREF_KEY)
            .and_then(|value| serde_json::from_value::<TeamaiPref>(value).ok())
            .unwrap_or_default(),
        Err(_) => TeamaiPref::default(),
    };
    normalize(&mut pref);
    fill_token_from_env(&mut pref);
    pref
}

pub fn save(app: &AppHandle, mut pref: TeamaiPref) -> Result<TeamaiPref, String> {
    normalize(&mut pref);
    let gitlab_url = gitlab_base_url(&pref.repo_url)?;

    let store = app.store(SETTINGS_STORE).map_err(|e| e.to_string())?;
    store.set(
        PREF_KEY,
        serde_json::to_value(&pref).map_err(|e| e.to_string())?,
    );
    store.save().map_err(|e| e.to_string())?;

    sync_gitlab_env(&pref.access_token, &gitlab_url)?;
    Ok(pref)
}

#[cfg(test)]
mod tests {
    use super::gitlab_base_url;

    #[test]
    fn extracts_http_host() {
        assert_eq!(
            gitlab_base_url("http://git.jiangduoduo.com/qianduan/ai-standards.git").unwrap(),
            "http://git.jiangduoduo.com"
        );
    }

    #[test]
    fn extracts_https_with_port() {
        assert_eq!(
            gitlab_base_url("https://gitlab.example.com:8443/group/repo.git").unwrap(),
            "https://gitlab.example.com:8443"
        );
    }

    #[test]
    fn rejects_non_http() {
        assert!(gitlab_base_url("git@host:group/repo.git").is_err());
    }

    #[test]
    fn rejects_empty() {
        assert!(gitlab_base_url("   ").is_err());
    }
}
