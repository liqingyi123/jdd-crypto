use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const SETTINGS_STORE: &str = "settings.json";
const PREF_KEY: &str = "intranetServerBase";

/// Default intranet http-server root (trailing slash).
pub const DEFAULT_SERVER_BASE: &str = "http://172.20.2.169:7101/";

/// App resources under the server root (updates + hosts preset).
const APP_RESOURCE_PATH: &str = "appStore/Software/PC/developer/jdd-crypto";

/// Normalize user input to an http(s) base URL with a trailing slash.
pub fn normalize_base(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(DEFAULT_SERVER_BASE.to_string());
    }
    let without_trailing = trimmed.trim_end_matches('/');
    if !(without_trailing.starts_with("http://") || without_trailing.starts_with("https://")) {
        return Err("服务器地址需以 http:// 或 https:// 开头".to_string());
    }
    if without_trailing.len() <= "http://".len() {
        return Err("服务器地址无效".to_string());
    }
    Ok(format!("{without_trailing}/"))
}

pub fn load_base(app: &AppHandle) -> String {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return DEFAULT_SERVER_BASE.to_string();
    };
    let Some(value) = store.get(PREF_KEY) else {
        return DEFAULT_SERVER_BASE.to_string();
    };
    let Some(raw) = value.as_str() else {
        return DEFAULT_SERVER_BASE.to_string();
    };
    normalize_base(raw).unwrap_or_else(|_| DEFAULT_SERVER_BASE.to_string())
}

pub fn save_base(app: &AppHandle, raw: &str) -> Result<String, String> {
    let normalized = normalize_base(raw)?;
    let store = app.store(SETTINGS_STORE).map_err(|e| e.to_string())?;
    store.set(PREF_KEY, serde_json::json!(normalized));
    store.save().map_err(|e| e.to_string())?;
    Ok(normalized)
}

fn resource_base(app: &AppHandle) -> String {
    format!("{}{}", load_base(app), APP_RESOURCE_PATH)
}

/// Base URL for update changelog / installer packages (no trailing slash).
pub fn update_resource_base(app: &AppHandle) -> String {
    resource_base(app)
}

/// Full URL for the SwitchHosts preset JSON used by hosts pull.
pub fn hosts_preset_url(app: &AppHandle) -> String {
    format!("{}/swh_data.json", resource_base(app))
}

#[cfg(test)]
mod tests {
    use super::normalize_base;

    #[test]
    fn normalize_adds_trailing_slash() {
        assert_eq!(
            normalize_base("http://172.20.2.169:7101").unwrap(),
            "http://172.20.2.169:7101/"
        );
    }

    #[test]
    fn normalize_empty_uses_default() {
        assert_eq!(
            normalize_base("  ").unwrap(),
            super::DEFAULT_SERVER_BASE
        );
    }

    #[test]
    fn normalize_rejects_non_http() {
        assert!(normalize_base("ftp://example.com").is_err());
    }
}
