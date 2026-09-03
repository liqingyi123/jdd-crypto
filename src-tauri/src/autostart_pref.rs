use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_store::StoreExt;

const SETTINGS_STORE: &str = "settings.json";
const PREF_KEY: &str = "autostartEnabled";

fn pref_key_present(app: &AppHandle) -> bool {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return false;
    };
    store.get(PREF_KEY).is_some()
}

pub fn load_pref(app: &AppHandle) -> bool {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return false;
    };
    store
        .get(PREF_KEY)
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn save_pref(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let store = app.store(SETTINGS_STORE).map_err(|e| e.to_string())?;
    store.set(PREF_KEY, serde_json::json!(enabled));
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// Persist intent and apply OS autostart registration.
pub fn set_pref(app: &AppHandle, enabled: bool) -> Result<bool, String> {
    save_pref(app, enabled)?;
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())?;
    } else {
        manager.disable().map_err(|e| e.to_string())?;
    }
    Ok(enabled)
}

/// After reinstall the OS Run/LaunchAgent entry is often cleared while
/// settings.json still has the user's intent — re-apply enable when needed.
/// If the store key is missing (upgrade from older builds), seed from OS once.
pub fn sync_from_store(app: &AppHandle) {
    let manager = app.autolaunch();
    let os_enabled = manager.is_enabled().unwrap_or(false);

    if !pref_key_present(app) {
        if os_enabled {
            let _ = save_pref(app, true);
        }
        return;
    }

    if !load_pref(app) {
        return;
    }
    if os_enabled {
        return;
    }
    let _ = manager.enable();
}
