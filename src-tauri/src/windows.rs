use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

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
    // Feature windows are created lazily (not declared in tauri.conf.json) so
    // cold start only pays for badge + main. Re-open reuses the existing webview.
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
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .visible(true)
        .build();

    if let Ok(win) = result {
        bind_close_to_hide(&win);
    }
}

/// Map the window close button to hide instead of destroy.
/// Keeps tray/badge workflows snappy and avoids recreating webviews on every open.
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
const BADGE_POS_X_KEY: &str = "badgePosX";
const BADGE_POS_Y_KEY: &str = "badgePosY";
const THEME_KEY: &str = "themePreference";
const DEFAULT_THEME_PREF: &str = "system";
// Keep in sync with `src/constants/badge.ts` (`EXPANDED_EXTRA_*`).
const EXPANDED_EXTRA_WIDTH: f64 = 188.0;
const EXPANDED_EXTRA_HEIGHT: f64 = 116.0;
// Keep in sync with `src/constants/badge.ts` (`BADGE_EDGE_MARGIN`).
const BADGE_EDGE_MARGIN: f64 = 50.0;

static SUPPRESS_BADGE_POS_SAVE: AtomicBool = AtomicBool::new(false);
static BADGE_MOVE_SEQ: AtomicU64 = AtomicU64::new(0);

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

pub fn load_badge_position(app: &AppHandle) -> Option<(i32, i32)> {
    use tauri_plugin_store::StoreExt;

    let store = app.store(SETTINGS_STORE).ok()?;
    let x = store.get(BADGE_POS_X_KEY)?.as_i64()? as i32;
    let y = store.get(BADGE_POS_Y_KEY)?.as_i64()? as i32;
    Some((x, y))
}

pub fn save_badge_position(app: &AppHandle, x: i32, y: i32) {
    use tauri_plugin_store::StoreExt;

    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(BADGE_POS_X_KEY, serde_json::json!(x));
        store.set(BADGE_POS_Y_KEY, serde_json::json!(y));
        let _ = store.save();
    }
}

fn set_badge_position(win: &WebviewWindow, x: i32, y: i32) {
    SUPPRESS_BADGE_POS_SAVE.store(true, Ordering::Relaxed);
    let _ = win.set_position(PhysicalPosition::new(x, y));
    thread::spawn(|| {
        thread::sleep(Duration::from_millis(100));
        SUPPRESS_BADGE_POS_SAVE.store(false, Ordering::Relaxed);
    });
}

pub fn position_badge_on_startup(app: &AppHandle) {
    let Some(win) = app.get_webview_window("badge") else {
        return;
    };

    if let Some((x, y)) = load_badge_position(app) {
        set_badge_position(&win, x, y);
        return;
    }

    let monitor = win
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        return;
    };

    let work = monitor.work_area();
    let scale = monitor.scale_factor();
    let Ok(outer_size) = win.outer_size() else {
        return;
    };

    let margin = (BADGE_EDGE_MARGIN * scale).round() as i32;
    let x = work.position.x + work.size.width as i32 - outer_size.width as i32 - margin;
    let y = work.position.y + work.size.height as i32 - outer_size.height as i32 - margin;
    set_badge_position(&win, x, y);
}

pub fn watch_badge_position(app: &AppHandle) {
    let Some(win) = app.get_webview_window("badge") else {
        return;
    };

    let app_handle = app.clone();
    win.on_window_event(move |event| {
        if !matches!(event, tauri::WindowEvent::Moved(_)) {
            return;
        }
        if SUPPRESS_BADGE_POS_SAVE.swap(false, Ordering::Relaxed) {
            return;
        }

        let seq = BADGE_MOVE_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
        let app = app_handle.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            if BADGE_MOVE_SEQ.load(Ordering::Relaxed) != seq {
                return;
            }
            let Some(win) = app.get_webview_window("badge") else {
                return;
            };
            if let Ok(pos) = win.outer_position() {
                save_badge_position(&app, pos.x, pos.y);
            }
        });
    });
}

pub fn normalize_theme_pref(raw: &str) -> String {
    match raw {
        "system" | "light" | "dark" => raw.to_string(),
        _ => DEFAULT_THEME_PREF.to_string(),
    }
}

pub fn load_theme_pref(app: &AppHandle) -> String {
    use tauri_plugin_store::StoreExt;

    let Ok(store) = app.store(SETTINGS_STORE) else {
        return DEFAULT_THEME_PREF.to_string();
    };
    store
        .get(THEME_KEY)
        .and_then(|value| value.as_str().map(str::to_string))
        .map(|raw| normalize_theme_pref(&raw))
        .unwrap_or_else(|| DEFAULT_THEME_PREF.to_string())
}

pub fn save_theme_pref(app: &AppHandle, preference: &str) -> String {
    use tauri_plugin_store::StoreExt;

    let normalized = normalize_theme_pref(preference);
    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(THEME_KEY, serde_json::json!(normalized));
        let _ = store.save();
    }
    let _ = app.emit("app://theme-preference", &normalized);
    normalized
}

pub fn set_badge_size(app: &AppHandle, expanded: bool) {
    use std::sync::atomic::Ordering;

    use crate::state::AppState;

    let size = app.state::<AppState>().badge_size.load(Ordering::Relaxed);
    apply_badge_window_size(app, size, expanded);
}

pub fn apply_badge_window_size(app: &AppHandle, size: u32, expanded: bool) {
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
