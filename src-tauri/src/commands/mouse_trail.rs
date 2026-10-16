use tauri::AppHandle;

#[tauri::command]
pub fn begin_shortcut_capture(app: AppHandle) -> Result<(), String> {
    crate::mouse_follow::unregister_all(&app)
}

#[tauri::command]
pub fn end_shortcut_capture(app: AppHandle) -> Result<(), String> {
    crate::mouse_follow::register_current(&app)
}

#[tauri::command]
pub fn get_mouse_trail_monitor_bounds(
    app: AppHandle,
    window_label: String,
) -> Result<crate::mouse_trail::MouseTrailMonitorBounds, String> {
    crate::mouse_trail::monitor_bounds(&app, &window_label)
        .ok_or_else(|| "无法读取显示器边界".to_string())
}

#[tauri::command]
pub fn get_mouse_trail_pref(app: AppHandle) -> crate::mouse_trail::MouseTrailPref {
    crate::mouse_trail::get_pref(app)
}

#[tauri::command]
pub fn set_mouse_trail_enabled(
    app: AppHandle,
    enabled: bool,
) -> Result<crate::mouse_trail::MouseTrailPref, String> {
    crate::mouse_trail::set_enabled_pref(app, enabled)
}

#[tauri::command]
pub fn set_mouse_trail_effect(
    app: AppHandle,
    effect: String,
) -> Result<crate::mouse_trail::MouseTrailPref, String> {
    crate::mouse_trail::set_effect_pref(app, effect)
}

#[tauri::command]
pub fn set_mouse_trail_performance_mode(
    app: AppHandle,
    mode: String,
) -> Result<crate::mouse_trail::MouseTrailPref, String> {
    crate::mouse_trail::set_performance_mode_pref(app, mode)
}

#[tauri::command]
pub fn set_mouse_trail_color(
    app: AppHandle,
    effect: String,
    color: String,
) -> Result<crate::mouse_trail::MouseTrailPref, String> {
    crate::mouse_trail::set_color_pref(app, effect, color)
}

#[tauri::command]
pub fn reset_mouse_trail_colors(
    app: AppHandle,
    effect: String,
) -> Result<crate::mouse_trail::MouseTrailPref, String> {
    crate::mouse_trail::reset_color_pref(app, effect)
}

#[tauri::command]
pub fn reset_mouse_trail_pref(
    app: AppHandle,
) -> Result<crate::mouse_trail::MouseTrailPref, String> {
    crate::mouse_trail::reset_pref(app)
}
