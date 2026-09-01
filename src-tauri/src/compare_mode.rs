use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri::{PhysicalPosition, window::Color};
use tauri_plugin_global_shortcut::Shortcut;
use tauri_plugin_store::StoreExt;

use crate::global_shortcuts;
use crate::state::AppState;
use crate::windows;

const SETTINGS_STORE: &str = "settings.json";
const PREF_KEY: &str = "compareModeEnabled";
const SHORTCUT_KEY: &str = "compareModeShortcut";
pub const COMPARE_TIP_LABEL: &str = "compare-tip";
pub const COMPARE_BUBBLE_LABEL: &str = "compare-bubble";
const COMPARE_TIP_WIDTH: f64 = 280.0;
const COMPARE_TIP_HEIGHT: f64 = 44.0;
const COMPARE_BUBBLE_WIDTH: f64 = 1000.0;
const COMPARE_BUBBLE_HEIGHT: f64 = 600.0;
const TIP_CURSOR_OFFSET: i32 = 18;

pub const TIP_WAIT_FIRST: &str = "等待文本选中...";
pub const TIP_WAIT_SECOND: &str = "正在等待后续对比文本";
pub const TIP_DECRYPT_FAIL: &str = "解密失败请重新选择文本";

#[derive(Clone, Serialize)]
pub struct CompareBubblePayload {
    pub left: String,
    pub right: String,
}

#[derive(Clone)]
enum ComparePhase {
    Off,
    WaitingFirst,
    WaitingSecond { first: String },
    ShowingResult,
}

static PHASE: Mutex<ComparePhase> = Mutex::new(ComparePhase::Off);
static PENDING_BUBBLE: Mutex<Option<CompareBubblePayload>> = Mutex::new(None);

pub fn load_shortcut(app: &AppHandle) -> String {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return crate::state::DEFAULT_COMPARE_MODE_SHORTCUT.to_string();
    };
    store
        .get(SHORTCUT_KEY)
        .and_then(|value| value.as_str().map(str::to_string))
        .and_then(|raw| crate::mouse_follow::validate_shortcut(&raw).ok())
        .unwrap_or_else(|| crate::state::DEFAULT_COMPARE_MODE_SHORTCUT.to_string())
}

pub fn save_shortcut(app: &AppHandle, shortcut: &str) {
    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(SHORTCUT_KEY, serde_json::json!(shortcut));
        let _ = store.save();
    }
}

pub fn handle_shortcut(app: &AppHandle, shortcut: &Shortcut) {
    let state = app.state::<AppState>();
    if !state.compare_pref_enabled.load(Ordering::Relaxed) {
        return;
    }
    let expected = state
        .compare_mode_shortcut
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| crate::state::DEFAULT_COMPARE_MODE_SHORTCUT.to_string());
    let Ok(expected_shortcut) = Shortcut::from_str(&expected) else {
        return;
    };
    if shortcut.id() != expected_shortcut.id() {
        return;
    }
    toggle(app);
}

pub fn load_pref(app: &AppHandle) -> bool {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return true;
    };
    store
        .get(PREF_KEY)
        .and_then(|value| value.as_bool())
        .unwrap_or(true)
}

pub fn save_pref(app: &AppHandle, enabled: bool) {
    if let Ok(store) = app.store(SETTINGS_STORE) {
        store.set(PREF_KEY, serde_json::json!(enabled));
        let _ = store.save();
    }
}

pub fn apply_pref(app: &AppHandle, enabled: bool) {
    let state = app.state::<AppState>();
    state
        .compare_pref_enabled
        .store(enabled, Ordering::Relaxed);
    save_pref(app, enabled);
    if !enabled && state.compare_active.load(Ordering::Relaxed) {
        stop(app);
    }
    let _ = global_shortcuts::register_all(app);
}

pub fn is_active(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .compare_active
        .load(Ordering::Relaxed)
}

pub fn toggle(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.compare_pref_enabled.load(Ordering::Relaxed) {
        return;
    }
    if state.compare_active.load(Ordering::Relaxed) {
        stop(app);
    } else {
        start(app);
    }
}

fn start(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.compare_active.store(true, Ordering::Relaxed);
    if let Ok(mut phase) = PHASE.lock() {
        *phase = ComparePhase::WaitingFirst;
    }
    windows::hide_clipboard_prompt(app);
    windows::hide_crypto_bubble(app);
    ensure_tip_window(app);
    show_tip(app, TIP_WAIT_FIRST);
    let _ = app.emit("app://compare-mode", true);
}

pub fn stop(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.compare_active.store(false, Ordering::Relaxed);
    if let Ok(mut phase) = PHASE.lock() {
        *phase = ComparePhase::Off;
    }
    if let Ok(mut pending) = PENDING_BUBBLE.lock() {
        *pending = None;
    }
    hide_tip(app);
    hide_compare_bubble(app);
    let _ = app.emit("app://compare-mode", false);
}

fn phase_shows_tip() -> bool {
    matches!(
        PHASE.lock().ok().as_deref(),
        Some(ComparePhase::WaitingFirst | ComparePhase::WaitingSecond { .. })
    )
}

pub fn handle_captured_cipher(app: &AppHandle, text: String) {
    if text.trim().is_empty() || !is_active(app) || !phase_shows_tip() {
        return;
    }
    if let Ok(mut last) = app.state::<AppState>().last_clipboard.lock() {
        *last = text.clone();
    }
    // Tip window decrypts; keep phase as-is until report_plain / fail UI.
    let _ = app.emit("app://compare-selection", &text);
    if let Some(win) = app.get_webview_window(COMPARE_TIP_LABEL) {
        let _ = win.emit("app://compare-selection", &text);
    }
}

pub fn report_plain(app: &AppHandle, plain: String) {
    if !is_active(app) {
        return;
    }
    let plain = plain.trim().to_string();
    if plain.is_empty() {
        return;
    }

    let next = {
        let Ok(mut phase) = PHASE.lock() else {
            return;
        };
        match phase.clone() {
            ComparePhase::WaitingFirst => {
                *phase = ComparePhase::WaitingSecond {
                    first: plain.clone(),
                };
                Some(("second", None))
            }
            ComparePhase::WaitingSecond { first } => {
                *phase = ComparePhase::ShowingResult;
                Some(("bubble", Some((first, plain))))
            }
            ComparePhase::Off | ComparePhase::ShowingResult => None,
        }
    };

    match next {
        Some(("second", _)) => {
            show_tip(app, TIP_WAIT_SECOND);
        }
        Some(("bubble", Some((left, right)))) => {
            hide_tip(app);
            show_compare_bubble(
                app,
                CompareBubblePayload { left, right },
            );
        }
        _ => {}
    }
}

pub fn report_fail(app: &AppHandle) {
    if !is_active(app) {
        return;
    }
    show_tip(app, TIP_DECRYPT_FAIL);
}

pub fn update_tip_position(app: &AppHandle) {
    if !is_active(app) || !phase_shows_tip() {
        return;
    }
    let Some(win) = app.get_webview_window(COMPARE_TIP_LABEL) else {
        return;
    };
    // Keep tip hidden while result bubble is showing.
    if !win.is_visible().unwrap_or(false) {
        return;
    }
    let (cx, cy) = windows::cursor_pos_public().unwrap_or((0, 0));
    let (x, y) = windows::clamp_popup_origin_public(
        app,
        cx,
        cy,
        COMPARE_TIP_WIDTH,
        COMPARE_TIP_HEIGHT,
        TIP_CURSOR_OFFSET,
    );
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

fn ensure_tip_window(app: &AppHandle) {
    if app.get_webview_window(COMPARE_TIP_LABEL).is_some() {
        return;
    }
    let _ = WebviewWindowBuilder::new(app, COMPARE_TIP_LABEL, WebviewUrl::App("index.html".into()))
        .title("对比模式提示")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .background_color(Color(0, 0, 0, 0))
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .focused(false)
        .visible(false)
        .inner_size(COMPARE_TIP_WIDTH, COMPARE_TIP_HEIGHT)
        .build();
}

fn show_tip(app: &AppHandle, message: &str) {
    ensure_tip_window(app);
    update_tip_position(app);
    if let Some(win) = app.get_webview_window(COMPARE_TIP_LABEL) {
        let _ = win.set_size(tauri::LogicalSize::new(
            COMPARE_TIP_WIDTH,
            COMPARE_TIP_HEIGHT,
        ));
        let _ = win.set_always_on_top(true);
        let _ = win.set_ignore_cursor_events(true);
        let _ = win.show();
        let _ = win.emit("app://compare-tip", message);
        let _ = app.emit("app://compare-tip", message);
    }
}

fn hide_tip(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(COMPARE_TIP_LABEL) {
        let _ = win.hide();
    }
}

pub fn show_compare_bubble(app: &AppHandle, payload: CompareBubblePayload) {
    // 结果弹窗出现时取消鼠标跟随提示语
    hide_tip(app);

    if let Ok(mut pending) = PENDING_BUBBLE.lock() {
        *pending = Some(payload.clone());
    }

    let (x, y) = center_origin(app, COMPARE_BUBBLE_WIDTH, COMPARE_BUBBLE_HEIGHT);

    if let Some(win) = app.get_webview_window(COMPARE_BUBBLE_LABEL) {
        let _ = win.set_size(tauri::LogicalSize::new(
            COMPARE_BUBBLE_WIDTH,
            COMPARE_BUBBLE_HEIGHT,
        ));
        let _ = win.set_position(PhysicalPosition::new(x, y));
        let _ = win.set_always_on_top(true);
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.emit("app://compare-bubble", &payload);
        let _ = app.emit("app://compare-bubble", &payload);
        return;
    }

    let result = WebviewWindowBuilder::new(
        app,
        COMPARE_BUBBLE_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("文本对比")
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .background_color(Color(0, 0, 0, 0))
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .focused(true)
    .visible(true)
    .inner_size(COMPARE_BUBBLE_WIDTH, COMPARE_BUBBLE_HEIGHT)
    .build();

    if let Ok(win) = result {
        windows::bind_close_to_hide(&win);
        let _ = win.set_position(PhysicalPosition::new(x, y));
        let _ = win.set_always_on_top(true);
        let _ = win.emit("app://compare-bubble", &payload);
        let handle = app.clone();
        let delayed = payload.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(200));
            if let Some(win) = handle.get_webview_window(COMPARE_BUBBLE_LABEL) {
                let _ = win.emit("app://compare-bubble", &delayed);
            }
            let _ = handle.emit("app://compare-bubble", &delayed);
        });
    }
}

pub fn get_pending_compare_bubble() -> Option<CompareBubblePayload> {
    PENDING_BUBBLE
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
}

pub fn hide_compare_bubble(app: &AppHandle) {
    if let Ok(mut pending) = PENDING_BUBBLE.lock() {
        *pending = None;
    }
    if let Some(win) = app.get_webview_window(COMPARE_BUBBLE_LABEL) {
        let _ = win.hide();
    }
}

fn center_origin(app: &AppHandle, logical_width: f64, logical_height: f64) -> (i32, i32) {
    let monitors = app.available_monitors().unwrap_or_default();
    let (cursor_x, cursor_y) = windows::cursor_pos_public().unwrap_or((0, 0));
    let area = monitors
        .iter()
        .find(|m| {
            let work = m.work_area();
            let px = work.position.x;
            let py = work.position.y;
            let pw = work.size.width as i32;
            let ph = work.size.height as i32;
            cursor_x >= px && cursor_x < px + pw && cursor_y >= py && cursor_y < py + ph
        })
        .or_else(|| monitors.first());

    let Some(monitor) = area else {
        return (100, 100);
    };
    let work = monitor.work_area();
    let scale = monitor.scale_factor();
    let width_px = (logical_width * scale).round() as i32;
    let height_px = (logical_height * scale).round() as i32;
    let x = work.position.x + (work.size.width as i32 - width_px).max(0) / 2;
    let y = work.position.y + (work.size.height as i32 - height_px).max(0) / 2;
    (x, y)
}
