use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

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
