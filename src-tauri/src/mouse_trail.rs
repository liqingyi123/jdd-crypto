use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, EventTarget, LogicalSize, Manager, PhysicalPosition, WebviewUrl,
    WebviewWindowBuilder, window::Color,
};
use tauri_plugin_store::StoreExt;

const MAX_OVERLAYS: usize = 8;
const SETTINGS_STORE: &str = "settings.json";
const PREF_KEY: &str = "mouseTrail";
const DEFAULT_EFFECT: &str = "ribbon";
const DEFAULT_METEOR_COLOR: &str = "#F8EC85";
const DEFAULT_DOTS_COLOR: &str = "#00D1CE";
const DEFAULT_HEART_COLOR: &str = "#FF2EC8";
const DISPLAY_CHANGE_DEBOUNCE: Duration = Duration::from_millis(300);

static CURSOR_LOOP_STARTED: AtomicBool = AtomicBool::new(false);
static TRAIL_ENABLED: AtomicBool = AtomicBool::new(false);
static DISPLAY_LISTENER_STARTED: AtomicBool = AtomicBool::new(false);
static DISPLAY_CHANGE_SEQ: AtomicU64 = AtomicU64::new(0);
static APP_FOR_DISPLAY: OnceLock<AppHandle> = OnceLock::new();

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseTrailCursor {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseTrailMonitorBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseTrailColors {
    #[serde(default = "default_meteor_color")]
    pub meteor: String,
    #[serde(default = "default_dots_color")]
    pub dots: String,
    #[serde(default = "default_heart_color")]
    pub heart: String,
}

impl Default for MouseTrailColors {
    fn default() -> Self {
        Self {
            meteor: default_meteor_color(),
            dots: default_dots_color(),
            heart: default_heart_color(),
        }
    }
}

fn default_meteor_color() -> String {
    DEFAULT_METEOR_COLOR.to_string()
}

fn default_dots_color() -> String {
    DEFAULT_DOTS_COLOR.to_string()
}

fn default_heart_color() -> String {
    DEFAULT_HEART_COLOR.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseTrailPref {
    pub enabled: bool,
    pub effect: String,
    #[serde(default)]
    pub colors: MouseTrailColors,
}

impl Default for MouseTrailPref {
    fn default() -> Self {
        Self {
            enabled: false,
            effect: DEFAULT_EFFECT.to_string(),
            colors: MouseTrailColors::default(),
        }
    }
}

struct MonitorLayout {
    physical_pos: PhysicalPosition<i32>,
    logical_pos: (f64, f64),
    logical_size: LogicalSize<f64>,
}

fn monitor_layout(monitor: &tauri::Monitor) -> MonitorLayout {
    let work = monitor.work_area();
    let scale = monitor.scale_factor();
    MonitorLayout {
        physical_pos: PhysicalPosition::new(work.position.x, work.position.y),
        logical_pos: (
            work.position.x as f64 / scale,
            work.position.y as f64 / scale,
        ),
        logical_size: LogicalSize::new(
            work.size.width as f64 / scale,
            work.size.height as f64 / scale,
        ),
    }
}

pub fn overlay_label(index: usize) -> String {
    format!("mouse-trail-{index}")
}

pub fn parse_overlay_index(label: &str) -> Option<usize> {
    label
        .strip_prefix("mouse-trail-")
        .and_then(|rest| rest.parse().ok())
}

fn is_overlay_target(target: &EventTarget) -> bool {
    match target {
        EventTarget::WebviewWindow { label }
        | EventTarget::Webview { label }
        | EventTarget::Window { label }
        | EventTarget::AnyLabel { label } => label.starts_with("mouse-trail-"),
        _ => false,
    }
}

fn normalize_effect(raw: &str) -> String {
    match raw {
        "meteor" => "meteor".to_string(),
        "graffiti" => "graffiti".to_string(),
        "dots" => "dots".to_string(),
        "heart" => "heart".to_string(),
        _ => DEFAULT_EFFECT.to_string(),
    }
}

fn normalize_color(raw: &str, fallback: &str) -> String {
    let cleaned = raw.trim().trim_start_matches('#').to_ascii_lowercase();
    if cleaned.len() == 6 && cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
        return format!("#{cleaned}");
    }
    let fb = fallback.trim().trim_start_matches('#').to_ascii_lowercase();
    if fb.len() == 6 && fb.chars().all(|c| c.is_ascii_hexdigit()) {
        return format!("#{fb}");
    }
    "#ffffff".to_string()
}

fn normalize_colors(colors: &mut MouseTrailColors) {
    colors.meteor = normalize_color(&colors.meteor, DEFAULT_METEOR_COLOR);
    colors.dots = normalize_color(&colors.dots, DEFAULT_DOTS_COLOR);
    colors.heart = normalize_color(&colors.heart, DEFAULT_HEART_COLOR);
}

fn normalize_pref(pref: &mut MouseTrailPref) {
    pref.effect = normalize_effect(&pref.effect);
    normalize_colors(&mut pref.colors);
}

pub fn load_pref(app: &AppHandle) -> MouseTrailPref {
    let Ok(store) = app.store(SETTINGS_STORE) else {
        return MouseTrailPref::default();
    };
    let Some(value) = store.get(PREF_KEY) else {
        return MouseTrailPref::default();
    };
    let Ok(mut pref) = serde_json::from_value::<MouseTrailPref>(value) else {
        return MouseTrailPref::default();
    };
    normalize_pref(&mut pref);
    pref
}

fn save_pref(app: &AppHandle, pref: &MouseTrailPref) -> Result<(), String> {
    let store = app.store(SETTINGS_STORE).map_err(|e| e.to_string())?;
    store.set(
        PREF_KEY,
        serde_json::to_value(pref).map_err(|e| e.to_string())?,
    );
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

fn emit_pref(app: &AppHandle, pref: &MouseTrailPref) {
    let _ = app.emit("app://mouse-trail-pref", pref);
}

pub fn get_pref(app: AppHandle) -> MouseTrailPref {
    load_pref(&app)
}

pub fn set_enabled_pref(app: AppHandle, enabled: bool) -> Result<MouseTrailPref, String> {
    let mut pref = load_pref(&app);
    pref.enabled = enabled;
    save_pref(&app, &pref)?;
    set_enabled(&app, enabled);
    emit_pref(&app, &pref);
    Ok(pref)
}

pub fn set_effect_pref(app: AppHandle, effect: String) -> Result<MouseTrailPref, String> {
    let mut pref = load_pref(&app);
    pref.effect = normalize_effect(&effect);
    save_pref(&app, &pref)?;
    emit_pref(&app, &pref);
    Ok(pref)
}

pub fn set_color_pref(app: AppHandle, effect: String, color: String) -> Result<MouseTrailPref, String> {
    let normalized_effect = normalize_effect(&effect);
    if normalized_effect != "meteor" && normalized_effect != "dots" && normalized_effect != "heart" {
        return Err("该特效不支持自定义颜色".into());
    }
    let mut pref = load_pref(&app);
    let normalized_color = match normalized_effect.as_str() {
        "meteor" => normalize_color(&color, DEFAULT_METEOR_COLOR),
        "dots" => normalize_color(&color, DEFAULT_DOTS_COLOR),
        _ => normalize_color(&color, DEFAULT_HEART_COLOR),
    };
    match normalized_effect.as_str() {
        "meteor" => pref.colors.meteor = normalized_color,
        "dots" => pref.colors.dots = normalized_color,
        _ => pref.colors.heart = normalized_color,
    }
    save_pref(&app, &pref)?;
    emit_pref(&app, &pref);
    Ok(pref)
}

pub fn reset_color_pref(app: AppHandle, effect: String) -> Result<MouseTrailPref, String> {
    let normalized_effect = normalize_effect(&effect);
    if normalized_effect != "meteor" && normalized_effect != "dots" && normalized_effect != "heart" {
        return Err("该特效不支持自定义颜色".into());
    }
    let mut pref = load_pref(&app);
    match normalized_effect.as_str() {
        "meteor" => pref.colors.meteor = default_meteor_color(),
        "dots" => pref.colors.dots = default_dots_color(),
        _ => pref.colors.heart = default_heart_color(),
    }
    save_pref(&app, &pref)?;
    emit_pref(&app, &pref);
    Ok(pref)
}

pub fn reset_pref(app: AppHandle) -> Result<MouseTrailPref, String> {
    let pref = MouseTrailPref::default();
    save_pref(&app, &pref)?;
    set_enabled(&app, false);
    emit_pref(&app, &pref);
    Ok(pref)
}

pub fn init_from_store(app: &AppHandle) {
    ensure_display_listener(app);
    let pref = load_pref(app);
    set_enabled(app, pref.enabled);
}

/// Flip the enabled flag and schedule overlay sync off the invoke path
/// so settings UI does not wait on WebView creation.
pub fn set_enabled(app: &AppHandle, enabled: bool) {
    ensure_display_listener(app);
    TRAIL_ENABLED.store(enabled, Ordering::Relaxed);
    ensure_cursor_loop(app);
    schedule_sync_overlays(app, enabled);
}

fn ensure_display_listener(app: &AppHandle) {
    let _ = APP_FOR_DISPLAY.set(app.clone());
    if DISPLAY_LISTENER_STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    #[cfg(windows)]
    start_windows_display_listener();
    #[cfg(target_os = "macos")]
    start_macos_display_listener();
}

fn notify_display_changed() {
    let seq = DISPLAY_CHANGE_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    thread::spawn(move || {
        thread::sleep(DISPLAY_CHANGE_DEBOUNCE);
        if DISPLAY_CHANGE_SEQ.load(Ordering::Relaxed) != seq {
            return;
        }
        let Some(app) = APP_FOR_DISPLAY.get() else {
            return;
        };
        on_display_changed(app);
    });
}

fn on_display_changed(app: &AppHandle) {
    let handle = app.clone();
    let trail_on = TRAIL_ENABLED.load(Ordering::Relaxed);
    let _ = app.run_on_main_thread(move || {
        crate::windows::relocate_windows_to_visible_monitors(&handle);
        if !trail_on {
            return;
        }
        sync_overlays(&handle, true);
        let _ = handle.emit_filter(
            "app://mouse-trail-monitors-changed",
            (),
            is_overlay_target,
        );
    });
}

fn schedule_sync_overlays(app: &AppHandle, visible: bool) {
    let handle = app.clone();
    thread::spawn(move || {
        let handle_for_main = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            sync_overlays(&handle_for_main, visible);
            if visible {
                restore_interactive_focus(&handle_for_main);
            }
        });
    });
}

fn restore_interactive_focus(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.set_focus();
        return;
    }
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_focus();
    }
}

pub fn sync_overlays(app: &AppHandle, visible: bool) {
    let monitors = app.available_monitors().unwrap_or_default();
    for index in 0..MAX_OVERLAYS {
        let label = overlay_label(index);
        if index >= monitors.len() {
            if let Some(win) = app.get_webview_window(&label) {
                let _ = win.close();
            }
            continue;
        }

        let layout = monitor_layout(&monitors[index]);

        if let Some(win) = app.get_webview_window(&label) {
            let _ = win.set_position(layout.physical_pos);
            let _ = win.set_size(layout.logical_size);
            if visible {
                let _ = win.set_ignore_cursor_events(true);
                let _ = win.show();
            } else {
                let _ = win.hide();
            }
            continue;
        }

        if !visible {
            continue;
        }

        let (logical_x, logical_y) = layout.logical_pos;
        let url = WebviewUrl::App("index.html".into());
        let result = WebviewWindowBuilder::new(app, &label, url)
            .title("Mouse Trail")
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .background_color(Color(0, 0, 0, 0))
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .visible(false)
            .focused(false)
            .position(logical_x, logical_y)
            .inner_size(layout.logical_size.width, layout.logical_size.height)
            .build();

        if let Ok(win) = result {
            let _ = win.set_ignore_cursor_events(true);
            let _ = win.show();
        }
    }

    if visible {
        if let Some(badge) = app.get_webview_window("badge") {
            let _ = badge.set_always_on_top(true);
        }
    }
}

pub fn monitor_bounds(app: &AppHandle, label: &str) -> Option<MouseTrailMonitorBounds> {
    let index = parse_overlay_index(label)?;
    let monitors = app.available_monitors().ok()?;
    let monitor = monitors.get(index)?;
    let work = monitor.work_area();
    Some(MouseTrailMonitorBounds {
        x: work.position.x,
        y: work.position.y,
        width: work.size.width,
        height: work.size.height,
        scale_factor: monitor.scale_factor(),
    })
}

fn ensure_cursor_loop(app: &AppHandle) {
    if CURSOR_LOOP_STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    let handle = app.clone();
    thread::spawn(move || cursor_loop(handle));
}

fn cursor_loop(app: AppHandle) {
    let mut last: Option<(i32, i32)> = None;
    loop {
        if TRAIL_ENABLED.load(Ordering::Relaxed) {
            if let Some((x, y)) = cursor_pos() {
                if last != Some((x, y)) {
                    last = Some((x, y));
                    let payload = MouseTrailCursor { x, y };
                    let _ = app.emit_filter("app://mouse-trail-cursor", payload, is_overlay_target);
                }
            }
            thread::sleep(Duration::from_millis(33));
            continue;
        }
        last = None;
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(windows)]
fn cursor_pos() -> Option<(i32, i32)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut point) };
    if ok == 0 {
        return None;
    }
    Some((point.x, point.y))
}

#[cfg(not(windows))]
fn cursor_pos() -> Option<(i32, i32)> {
    None
}

#[cfg(windows)]
fn start_windows_display_listener() {
    thread::spawn(|| {
        use std::ptr;
        use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
        use windows_sys::Win32::Graphics::Gdi::HBRUSH;
        use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
            TranslateMessage, CS_HREDRAW, CS_VREDRAW, MSG, WM_DISPLAYCHANGE, WNDCLASSW,
            WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
        };

        unsafe extern "system" fn wnd_proc(
            hwnd: HWND,
            msg: u32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            if msg == WM_DISPLAYCHANGE {
                notify_display_changed();
                return 0;
            }
            // SAFETY: forward unhandled messages to the default window procedure.
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }

        // SAFETY: Win32 window class registration and message loop for display broadcasts.
        unsafe {
            let class_name: Vec<u16> = "jdd_crypto_display_listener\0"
                .encode_utf16()
                .collect();
            let hinstance = GetModuleHandleW(ptr::null());
            let class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: ptr::null_mut(),
                hCursor: ptr::null_mut(),
                hbrBackground: 0 as HBRUSH,
                lpszMenuName: ptr::null(),
                lpszClassName: class_name.as_ptr(),
            };
            if RegisterClassW(&class) == 0 {
                return;
            }

            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class_name.as_ptr(),
                ptr::null(),
                WS_POPUP,
                0,
                0,
                0,
                0,
                ptr::null_mut(),
                ptr::null_mut(),
                hinstance,
                ptr::null(),
            );
            if hwnd.is_null() {
                return;
            }

            let mut msg = MSG {
                hwnd: ptr::null_mut(),
                message: 0,
                wParam: 0,
                lParam: 0,
                time: 0,
                pt: windows_sys::Win32::Foundation::POINT { x: 0, y: 0 },
            };
            while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });
}

#[cfg(target_os = "macos")]
fn start_macos_display_listener() {
    use std::ffi::c_void;

    type CgDirectDisplayId = u32;
    type CgDisplayChangeSummaryFlags = u32;
    type CgError = i32;

    type CgDisplayReconfigurationCallback = Option<
        unsafe extern "C" fn(
            display: CgDirectDisplayId,
            flags: CgDisplayChangeSummaryFlags,
            user_info: *mut c_void,
        ),
    >;

    extern "C" {
        fn CGDisplayRegisterReconfigurationCallback(
            callback: CgDisplayReconfigurationCallback,
            user_info: *mut c_void,
        ) -> CgError;
    }

    unsafe extern "C" fn on_reconfig(
        _display: CgDirectDisplayId,
        _flags: CgDisplayChangeSummaryFlags,
        _user_info: *mut c_void,
    ) {
        notify_display_changed();
    }

    // SAFETY: registers a process-lifetime display reconfiguration callback.
    let status = unsafe {
        CGDisplayRegisterReconfigurationCallback(Some(on_reconfig), std::ptr::null_mut())
    };
    let _ = status;
}
