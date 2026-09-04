use std::str::FromStr;
use std::sync::atomic::Ordering;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::state::AppState;

const TRAIL_ARM_SHORTCUT: &str = "Ctrl+T";
const TRAIL_EFFECT_SHORTCUTS: [&str; 6] = [
    "Ctrl+1",
    "Ctrl+2",
    "Ctrl+3",
    "Ctrl+4",
    "Ctrl+5",
    "Ctrl+6",
];

pub fn register_all(app: &AppHandle) -> Result<(), String> {
    let global = app.global_shortcut();
    global.unregister_all().map_err(|err| err.to_string())?;

    let state = app.state::<AppState>();
    if state.mouse_follow_pref_enabled.load(Ordering::Relaxed) {
        let shortcut = state
            .mouse_follow_shortcut
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| crate::state::DEFAULT_MOUSE_FOLLOW_SHORTCUT.to_string());
        let parsed = Shortcut::from_str(&shortcut).map_err(|err| err.to_string())?;
        global.register(parsed).map_err(|err| err.to_string())?;
    }

    if state.compare_pref_enabled.load(Ordering::Relaxed) {
        let shortcut = state
            .compare_mode_shortcut
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| crate::state::DEFAULT_COMPARE_MODE_SHORTCUT.to_string());
        let compare = Shortcut::from_str(&shortcut).map_err(|err| err.to_string())?;
        global.register(compare).map_err(|err| err.to_string())?;
    }

    let arm = Shortcut::from_str(TRAIL_ARM_SHORTCUT).map_err(|err| err.to_string())?;
    global.register(arm).map_err(|err| err.to_string())?;

    for shortcut in TRAIL_EFFECT_SHORTCUTS {
        let parsed = Shortcut::from_str(shortcut).map_err(|err| err.to_string())?;
        global.register(parsed).map_err(|err| err.to_string())?;
    }

    let hosts_quick = state
        .hosts_quick_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| crate::state::DEFAULT_HOSTS_QUICK_SHORTCUT.to_string());
    let hosts_quick = Shortcut::from_str(&hosts_quick).map_err(|err| err.to_string())?;
    global
        .register(hosts_quick)
        .map_err(|err| err.to_string())?;

    Ok(())
}

pub fn unregister_all(app: &AppHandle) -> Result<(), String> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|err| err.to_string())
}
