use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::StoreExt;

use crate::mouse_trail;

pub const KIND_MOUSE_TRAIL: &str = "mouse-trail";
pub const KIND_EDITOR_THEME: &str = "editor-theme";
pub const KIND_CRYPTO_PRESET: &str = "crypto-preset";

const SETTINGS_STORE: &str = "settings.json";
const PLUGIN_SLOTS_KEY: &str = "pluginSlots";
const BUILTIN_METEOR_ID: &str = "builtin-meteor";
const BUILTIN_METEOR_NAME: &str = "绚丽流星";
const DEFAULT_METEOR_COLOR: &str = "#F8EC85";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default = "default_entry")]
    pub entry: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub contributes: serde_json::Value,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSlotCurrent {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSlot {
    pub kind: String,
    pub enabled: bool,
    pub source: String,
    pub current: Option<PluginSlotCurrent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSlotsState {
    pub slots: Vec<PluginSlot>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseTrailEffectOptions {
    pub color: String,
}

fn default_entry() -> String {
    "index.js".into()
}

fn default_enabled() -> bool {
    true
}

fn plugin_search_dirs(app: &AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(resource) = app.path().resource_dir() {
        dirs.push(resource.join("plugins"));
    }
    if let Ok(data) = app.path().app_data_dir() {
        dirs.push(data.join("plugins"));
    }
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins"));
    dirs
}

fn imported_kind_dir(app: &AppHandle, kind: &str) -> Result<PathBuf, String> {
    let data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(data.join("plugins").join(kind))
}

fn read_manifest(dir: &Path) -> Option<PluginManifest> {
    let path = dir.join("plugin.json");
    let raw = fs::read_to_string(path).ok()?;
    let mut manifest: PluginManifest = serde_json::from_str(&raw).ok()?;
    if manifest.id.trim().is_empty() {
        return None;
    }
    manifest.dir = dir.to_string_lossy().into_owned();
    Some(manifest)
}

pub fn scan_plugins(app: &AppHandle) -> Vec<PluginManifest> {
    let mut found: Vec<PluginManifest> = Vec::new();

    for root in plugin_search_dirs(app) {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if let Some(manifest) = read_manifest(&path) {
                if found.iter().any(|item| item.id == manifest.id) {
                    continue;
                }
                found.push(manifest);
            }
        }
    }

    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

fn default_slots() -> PluginSlotsState {
    PluginSlotsState {
        slots: vec![
            PluginSlot {
                kind: KIND_MOUSE_TRAIL.into(),
                enabled: false,
                source: "preset".into(),
                current: Some(PluginSlotCurrent {
                    id: BUILTIN_METEOR_ID.into(),
                    name: BUILTIN_METEOR_NAME.into(),
                    file_name: None,
                }),
            },
            PluginSlot {
                kind: KIND_EDITOR_THEME.into(),
                enabled: false,
                source: "preset".into(),
                current: None,
            },
            PluginSlot {
                kind: KIND_CRYPTO_PRESET.into(),
                enabled: false,
                source: "preset".into(),
                current: None,
            },
        ],
    }
}

fn normalize_kind(kind: &str) -> Option<String> {
    match kind {
        KIND_MOUSE_TRAIL | KIND_EDITOR_THEME | KIND_CRYPTO_PRESET => Some(kind.to_string()),
        _ => None,
    }
}

fn merge_slots(raw: Option<PluginSlotsState>) -> PluginSlotsState {
    let defaults = default_slots();
    let Some(raw) = raw else {
        return defaults;
    };

    let mut merged = defaults.slots;
    for slot in &mut merged {
        if let Some(saved) = raw.slots.iter().find(|item| item.kind == slot.kind) {
            slot.enabled = saved.enabled;
            slot.source = saved.source.clone();
            slot.current = saved.current.clone();
        }
    }

    if let Some(slot) = merged.iter_mut().find(|item| item.kind == KIND_MOUSE_TRAIL) {
        if slot.current.is_none() {
            slot.current = Some(PluginSlotCurrent {
                id: BUILTIN_METEOR_ID.into(),
                name: BUILTIN_METEOR_NAME.into(),
                file_name: None,
            });
            slot.source = "preset".into();
        }
    }

    PluginSlotsState { slots: merged }
}

pub fn load_plugin_slots(app: &AppHandle) -> PluginSlotsState {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return default_slots();
    };
    let raw = store
        .get(PLUGIN_SLOTS_KEY)
        .and_then(|value| serde_json::from_value::<PluginSlotsState>(value).ok());
    merge_slots(raw)
}

fn save_plugin_slots(app: &AppHandle, state: &PluginSlotsState) -> Result<(), String> {
    let store = app.store(SETTINGS_STORE).map_err(|e| e.to_string())?;
    store.set(
        PLUGIN_SLOTS_KEY,
        serde_json::to_value(state).map_err(|e| e.to_string())?,
    );
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

fn emit_slots(app: &AppHandle, state: &PluginSlotsState) {
    let _ = app.emit("app://plugin-slots", state);
}

fn apply_mouse_trail_enabled(app: &AppHandle, enabled: bool) {
    mouse_trail::set_enabled(app, enabled);
}

fn reset_slot_defaults(slot: &mut PluginSlot) {
    slot.enabled = false;
    slot.source = "preset".into();
    match slot.kind.as_str() {
        KIND_MOUSE_TRAIL => {
            slot.current = Some(PluginSlotCurrent {
                id: BUILTIN_METEOR_ID.into(),
                name: BUILTIN_METEOR_NAME.into(),
                file_name: None,
            });
        }
        _ => slot.current = None,
    }
}

fn clear_import_dir(dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
        } else {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn validate_import_manifest(raw: &str, kind: &str) -> Result<(String, String), String> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|_| "插件文件必须是合法的 JSON".to_string())?;
    let manifest_type = value
        .get("type")
        .and_then(|item| item.as_str())
        .ok_or_else(|| "插件文件缺少 type 字段".to_string())?;
    if manifest_type != kind {
        return Err(format!("插件类型不匹配，期望 {kind}"));
    }
    let id = value
        .get("id")
        .and_then(|item| item.as_str())
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| "插件文件缺少 id 字段".to_string())?;
    let name = value
        .get("name")
        .and_then(|item| item.as_str())
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| "插件文件缺少 name 字段".to_string())?;
    Ok((id.to_string(), name.to_string()))
}

fn read_imported_manifest(app: &AppHandle, kind: &str) -> Option<serde_json::Value> {
    let dir = imported_kind_dir(app, kind).ok()?;
    let raw = fs::read_to_string(dir.join("plugin.json")).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn get_plugin_slots(app: AppHandle) -> PluginSlotsState {
    load_plugin_slots(&app)
}

pub fn set_plugin_slot_enabled(app: AppHandle, kind: String, enabled: bool) -> Result<PluginSlotsState, String> {
    normalize_kind(&kind).ok_or_else(|| format!("未知插件类型: {kind}"))?;
    let mut state = load_plugin_slots(&app);
    let slot = state
        .slots
        .iter_mut()
        .find(|item| item.kind == kind)
        .ok_or_else(|| format!("未找到插件槽位: {kind}"))?;
    slot.enabled = enabled;
    save_plugin_slots(&app, &state)?;
    if kind == KIND_MOUSE_TRAIL {
        apply_mouse_trail_enabled(&app, enabled);
    }
    emit_slots(&app, &state);
    Ok(state)
}

pub fn import_plugin(
    app: AppHandle,
    kind: String,
    file_name: String,
    bytes: Vec<u8>,
) -> Result<PluginSlotsState, String> {
    normalize_kind(&kind).ok_or_else(|| format!("未知插件类型: {kind}"))?;
    if bytes.len() > 1024 * 1024 {
        return Err("插件文件不能超过 1MB".to_string());
    }
    let raw = String::from_utf8(bytes).map_err(|_| "插件文件必须是 UTF-8 编码".to_string())?;
    let (id, name) = validate_import_manifest(&raw, &kind)?;

    let dir = imported_kind_dir(&app, &kind)?;
    clear_import_dir(&dir)?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::write(dir.join("plugin.json"), raw.as_bytes()).map_err(|e| e.to_string())?;

    let mut state = load_plugin_slots(&app);
    let slot = state
        .slots
        .iter_mut()
        .find(|item| item.kind == kind)
        .ok_or_else(|| format!("未找到插件槽位: {kind}"))?;
    slot.source = "imported".into();
    slot.current = Some(PluginSlotCurrent {
        id,
        name,
        file_name: Some(file_name),
    });
    save_plugin_slots(&app, &state)?;
    emit_slots(&app, &state);
    Ok(state)
}

pub fn reset_plugin_slot(app: AppHandle, kind: String) -> Result<PluginSlotsState, String> {
    normalize_kind(&kind).ok_or_else(|| format!("未知插件类型: {kind}"))?;
    if let Ok(dir) = imported_kind_dir(&app, &kind) {
        clear_import_dir(&dir)?;
    }

    let mut state = load_plugin_slots(&app);
    let slot = state
        .slots
        .iter_mut()
        .find(|item| item.kind == kind)
        .ok_or_else(|| format!("未找到插件槽位: {kind}"))?;
    reset_slot_defaults(slot);
    save_plugin_slots(&app, &state)?;
    if kind == KIND_MOUSE_TRAIL {
        apply_mouse_trail_enabled(&app, false);
    }
    emit_slots(&app, &state);
    Ok(state)
}

pub fn get_mouse_trail_effect_options(app: AppHandle) -> MouseTrailEffectOptions {
    if let Some(value) = read_imported_manifest(&app, KIND_MOUSE_TRAIL) {
        if let Some(color) = value
            .pointer("/contributes/options/color")
            .and_then(|item| item.as_str())
            .filter(|item| !item.is_empty())
        {
            return MouseTrailEffectOptions {
                color: color.to_string(),
            };
        }
    }
    MouseTrailEffectOptions {
        color: DEFAULT_METEOR_COLOR.to_string(),
    }
}

pub fn init_plugin_slots(app: &AppHandle) {
    let state = load_plugin_slots(app);
    let enabled = state
        .slots
        .iter()
        .find(|item| item.kind == KIND_MOUSE_TRAIL)
        .map(|slot| slot.enabled)
        .unwrap_or(false);
    apply_mouse_trail_enabled(app, enabled);
}
