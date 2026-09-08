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

#[derive(Clone, Copy)]
pub enum ShortcutOwner {
    MouseFollow,
    Compare,
    HostsQuick,
    BrightnessQuick,
}

fn shortcut_id(raw: &str) -> Result<u32, String> {
    Ok(Shortcut::from_str(raw).map_err(|err| err.to_string())?.id())
}

/// Ensure `candidate` does not collide with fixed trail keys or other custom slots.
pub fn ensure_no_conflict(
    app: &AppHandle,
    candidate: &str,
    owner: ShortcutOwner,
) -> Result<(), String> {
    let candidate_id = shortcut_id(candidate)?;

    if shortcut_id(TRAIL_ARM_SHORTCUT)? == candidate_id {
        return Err(format!("与固定快捷键 {TRAIL_ARM_SHORTCUT} 冲突"));
    }
    for fixed in TRAIL_EFFECT_SHORTCUTS {
        if shortcut_id(fixed)? == candidate_id {
            return Err(format!("与固定快捷键 {fixed} 冲突"));
        }
    }

    let state = app.state::<AppState>();
    let follow = state
        .mouse_follow_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| crate::state::DEFAULT_MOUSE_FOLLOW_SHORTCUT.to_string());
    let compare = state
        .compare_mode_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| crate::state::DEFAULT_COMPARE_MODE_SHORTCUT.to_string());
    let hosts_quick = state
        .hosts_quick_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| crate::state::DEFAULT_HOSTS_QUICK_SHORTCUT.to_string());
    let brightness_quick = state
        .brightness_quick_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| crate::state::DEFAULT_BRIGHTNESS_QUICK_SHORTCUT.to_string());

    if !matches!(owner, ShortcutOwner::MouseFollow)
        && shortcut_id(&follow)? == candidate_id
    {
        return Err(format!("与鼠标跟随快捷键 {follow} 冲突"));
    }
    if !matches!(owner, ShortcutOwner::Compare) && shortcut_id(&compare)? == candidate_id {
        return Err(format!("与对照模式快捷键 {compare} 冲突"));
    }
    if !matches!(owner, ShortcutOwner::HostsQuick)
        && shortcut_id(&hosts_quick)? == candidate_id
    {
        return Err(format!("与 Hosts 快捷切换快捷键 {hosts_quick} 冲突"));
    }
    if !matches!(owner, ShortcutOwner::BrightnessQuick)
        && shortcut_id(&brightness_quick)? == candidate_id
    {
        return Err(format!("与屏幕亮度快捷键 {brightness_quick} 冲突"));
    }

    Ok(())
}

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

    if state.hosts_quick_pref_enabled.load(Ordering::Relaxed) {
        let hosts_quick = state
            .hosts_quick_shortcut
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| crate::state::DEFAULT_HOSTS_QUICK_SHORTCUT.to_string());
        let hosts_quick = Shortcut::from_str(&hosts_quick).map_err(|err| err.to_string())?;
        global
            .register(hosts_quick)
            .map_err(|err| err.to_string())?;
    }

    if state.brightness_quick_pref_enabled.load(Ordering::Relaxed) {
        let brightness_quick = state
            .brightness_quick_shortcut
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| crate::state::DEFAULT_BRIGHTNESS_QUICK_SHORTCUT.to_string());
        let brightness_quick =
            Shortcut::from_str(&brightness_quick).map_err(|err| err.to_string())?;
        global
            .register(brightness_quick)
            .map_err(|err| err.to_string())?;
    }

    Ok(())
}

pub fn unregister_all(app: &AppHandle) -> Result<(), String> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|err| err.to_string())
}
