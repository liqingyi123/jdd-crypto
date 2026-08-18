use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

#[derive(Clone, Serialize)]
pub struct CryptoHint {
    pub text: String,
    pub mode: String,
}

struct FeatureSpec {
    label: &'static str,
    title: &'static str,
    width: f64,
    height: f64,
    min_width: f64,
    min_height: f64,
}

fn feature_spec(label: &str) -> Option<FeatureSpec> {
    match label {
        "settings" => Some(FeatureSpec {
            label: "settings",
            title: "功能设置",
            width: 720.0,
            height: 560.0,
            min_width: 560.0,
            min_height: 420.0,
        }),
        "feedback" => Some(FeatureSpec {
            label: "feedback",
            title: "意见反馈",
            width: 720.0,
            height: 560.0,
            min_width: 560.0,
            min_height: 420.0,
        }),
        "plugins" => Some(FeatureSpec {
            label: "plugins",
            title: "插件管理",
            width: 720.0,
            height: 560.0,
            min_width: 560.0,
            min_height: 420.0,
        }),
        "about" => Some(FeatureSpec {
            label: "about",
            title: "关于",
            width: 480.0,
            height: 420.0,
            min_width: 400.0,
            min_height: 360.0,
        }),
        _ => None,
    }
}

fn focus_window(win: &WebviewWindow) {
    let _ = win.unminimize();
    let _ = win.show();
    let _ = win.set_focus();
}

pub fn show_main(app: &AppHandle, hint: Option<CryptoHint>) {
    if let Some(win) = app.get_webview_window("main") {
        focus_window(&win);
        if let Some(hint) = hint {
            let _ = win.emit("app://crypto-payload", hint);
        }
    }
}

pub fn show_feature(app: &AppHandle, label: &str) {
    if let Some(win) = app.get_webview_window(label) {
        focus_window(&win);
        return;
    }

    let Some(spec) = feature_spec(label) else {
        return;
    };

    let result = WebviewWindowBuilder::new(app, spec.label, WebviewUrl::App("index.html".into()))
        .title(spec.title)
        .inner_size(spec.width, spec.height)
        .min_inner_size(spec.min_width, spec.min_height)
        .visible(true)
        .build();

    if let Ok(win) = result {
        bind_close_to_hide(&win);
    }
}

pub fn bind_close_to_hide(win: &WebviewWindow) {
    let hidden = win.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = hidden.hide();
        }
    });
}

const SETTINGS_STORE: &str = "settings.json";
const BADGE_SIZE_KEY: &str = "badgeSize";
const EXPANDED_EXTRA_WIDTH: f64 = 188.0;
const EXPANDED_EXTRA_HEIGHT: f64 = 116.0;

pub fn load_badge_size(app: &AppHandle) -> u32 {
    use crate::state::normalize_badge_size;
    use crate::state::DEFAULT_BADGE_SIZE;
    use tauri_plugin_store::StoreExt;

    let Ok(store) = app.store(SETTINGS_STORE) else {
        return DEFAULT_BADGE_SIZE;
    };
    store
        .get(BADGE_SIZE_KEY)
        .and_then(|value| value.as_u64())
        .map(|value| normalize_badge_size(value as u32))
        .unwrap_or(DEFAULT_BADGE_SIZE)
}

pub fn save_badge_size(app: &AppHandle, size: u32) {
    use tauri_plugin_store::StoreExt;

    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(BADGE_SIZE_KEY, serde_json::json!(size));
        let _ = store.save();
    }
}

pub fn set_badge_size(app: &AppHandle, expanded: bool) {
    use std::sync::atomic::Ordering;

    use crate::state::AppState;

    let size = app.state::<AppState>().badge_size.load(Ordering::Relaxed);
    if let Some(win) = app.get_webview_window("badge") {
        let (width, height) = if expanded {
            (
                f64::from(size) + EXPANDED_EXTRA_WIDTH,
                f64::from(size) + EXPANDED_EXTRA_HEIGHT,
            )
        } else {
            (f64::from(size), f64::from(size))
        };
        let _ = win.set_size(tauri::LogicalSize::new(width, height));
        let _ = win.emit("app://badge-size", size);
    }
}
