mod app_update;
mod clipboard;
mod commands;
mod mouse_follow;
mod mouse_trail;
mod plugin_host;
mod state;
mod tray;
mod windows;

use state::AppState;
use std::sync::atomic::Ordering;

use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    // Must be first: second launches exit before other plugins / windows init.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            windows::show_main(app, None);
        }));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed
                        && app
                            .state::<AppState>()
                            .mouse_follow_pref_enabled
                            .load(Ordering::Relaxed)
                    {
                        mouse_follow::toggle(app);
                    }
                })
                .build(),
        )
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::navigate_main,
            commands::popup_app_menu,
            commands::set_badge_prompt_mode,
            commands::hide_clipboard_prompt,
            commands::get_clipboard_candidate,
            commands::get_badge_size,
            commands::set_badge_size_pref,
            commands::get_theme_pref,
            commands::set_theme_pref,
            commands::set_clipboard_watch,
            commands::get_clipboard_watch,
            commands::list_plugins,
            commands::crypto_transform,
            commands::get_mouse_follow_shortcut,
            commands::set_mouse_follow_shortcut,
            commands::get_mouse_follow_pref,
            commands::set_mouse_follow_pref,
            commands::begin_shortcut_capture,
            commands::end_shortcut_capture,
            commands::get_plugin_slots,
            commands::set_plugin_slot_enabled,
            commands::import_plugin,
            commands::reset_plugin_slot,
            commands::get_mouse_trail_monitor_bounds,
            commands::get_mouse_trail_pref,
            commands::set_mouse_trail_enabled,
            commands::set_mouse_trail_effect,
            commands::set_mouse_trail_color,
            commands::reset_mouse_trail_colors,
            commands::reset_mouse_trail_pref,
            commands::check_app_update,
            commands::download_app_update,
            commands::install_app_update,
            commands::take_pending_app_update,
        ])
        .setup(|app| {
            let badge_size = windows::load_badge_size(app.handle());
            app.state::<AppState>()
                .badge_size
                .store(badge_size, Ordering::Relaxed);
            windows::set_badge_size(app.handle(), false);
            windows::position_badge_on_startup(app.handle());
            windows::watch_badge_position(app.handle());
            if let Some(main) = app.get_webview_window("main") {
                windows::bind_close_to_hide(&main);
            }
            tray::setup_tray(app)?;
            let clipboard_watch = clipboard::load_watch(app.handle());
            app.state::<AppState>()
                .clipboard_watch_enabled
                .store(clipboard_watch, Ordering::Relaxed);
            clipboard::start_clipboard_watcher(app.handle().clone());
            let shortcut = mouse_follow::load_shortcut(app.handle());
            if let Ok(mut current) = app.state::<AppState>().mouse_follow_shortcut.lock() {
                *current = shortcut.clone();
            }
            let pref = mouse_follow::load_pref(app.handle());
            app.state::<AppState>()
                .mouse_follow_pref_enabled
                .store(pref, Ordering::Relaxed);
            if pref {
                let _ = mouse_follow::register_shortcut(app.handle(), &shortcut);
            }
            mouse_follow::start_follow_loop(app.handle().clone());
            mouse_trail::init_from_store(app.handle());
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                if let Ok(result) = app_update::check_update(&handle, false) {
                    if result.available {
                        if let Ok(mut pending) = handle.state::<AppState>().pending_update.lock() {
                            *pending = Some(result.clone());
                        }
                        let _ = handle.emit("app://update-available", &result);
                        windows::show_feature(&handle, "about");
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
