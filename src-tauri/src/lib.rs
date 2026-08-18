mod clipboard;
mod commands;
mod plugin_host;
mod state;
mod tray;
mod windows;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::navigate_main,
            commands::popup_app_menu,
            commands::set_badge_prompt_mode,
            commands::get_badge_size,
            commands::set_badge_size_pref,
            commands::set_clipboard_watch,
            commands::get_clipboard_watch,
            commands::list_plugins,
            commands::crypto_transform,
        ])
        .setup(|app| {
            use std::sync::atomic::Ordering;

            let badge_size = windows::load_badge_size(app.handle());
            app.state::<AppState>()
                .badge_size
                .store(badge_size, Ordering::Relaxed);
            windows::set_badge_size(app.handle(), false);
            if let Some(main) = app.get_webview_window("main") {
                windows::bind_close_to_hide(&main);
            }
            tray::setup_tray(app)?;
            clipboard::start_clipboard_watcher(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
