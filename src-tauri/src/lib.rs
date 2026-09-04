mod app_update;
mod autostart_pref;
mod clipboard;
mod commands;
mod compare_mode;
mod global_shortcuts;
mod hosts_manager;
mod hosts_quick;
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
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    let state = event.state();
                    if mouse_trail::handle_trail_shortcut(app, shortcut, state) {
                        return;
                    }
                    if state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        mouse_follow::handle_follow_shortcut(app, shortcut);
                        compare_mode::handle_shortcut(app, shortcut);
                        hosts_quick::handle_shortcut(app, shortcut);
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
            commands::clear_clipboard_dedup,
            commands::copy_text_silent,
            commands::accept_clipboard_action,
            commands::show_crypto_bubble,
            commands::hide_crypto_bubble,
            commands::get_crypto_bubble_payload,
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
            commands::get_compare_mode_pref,
            commands::set_compare_mode_pref,
            commands::get_compare_mode_shortcut,
            commands::set_compare_mode_shortcut,
            commands::toggle_compare_mode,
            commands::compare_report_plain,
            commands::compare_report_fail,
            commands::hide_compare_bubble,
            commands::get_compare_bubble_payload,
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
            commands::get_autostart_pref,
            commands::set_autostart_pref,
            commands::hosts_list,
            commands::hosts_upsert,
            commands::hosts_refresh,
            commands::hosts_delete,
            commands::hosts_rename,
            commands::hosts_set_nature,
            commands::hosts_set_enabled,
            commands::hosts_apply,
            commands::hosts_read_system,
            commands::hosts_has_write_access,
            commands::hosts_request_permission,
            commands::hosts_open_system,
            commands::hosts_import_switchhosts,
            commands::hosts_export_switchhosts,
            commands::hosts_reset_system,
            commands::open_hosts_window,
            commands::hide_hosts_quick,
            commands::get_hosts_quick_shortcut,
            commands::set_hosts_quick_shortcut,
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
            if let Some(bubble) = app.get_webview_window("crypto-bubble") {
                windows::bind_close_to_hide(&bubble);
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
            let compare_pref = compare_mode::load_pref(app.handle());
            app.state::<AppState>()
                .compare_pref_enabled
                .store(compare_pref, Ordering::Relaxed);
            let compare_shortcut = compare_mode::load_shortcut(app.handle());
            if let Ok(mut current) = app.state::<AppState>().compare_mode_shortcut.lock() {
                *current = compare_shortcut;
            }
            let hosts_quick_shortcut = hosts_quick::load_shortcut(app.handle());
            if let Ok(mut current) = app.state::<AppState>().hosts_quick_shortcut.lock() {
                *current = hosts_quick_shortcut;
            }
            mouse_follow::start_follow_loop(app.handle().clone());
            let _ = global_shortcuts::register_all(app.handle());
            if let Some(tip) = app.get_webview_window("compare-tip") {
                let _ = tip.set_ignore_cursor_events(true);
                windows::bind_close_to_hide(&tip);
            }
            if let Some(compare) = app.get_webview_window("compare-bubble") {
                windows::bind_close_to_hide(&compare);
            }
            mouse_trail::init_from_store(app.handle());
            autostart_pref::sync_from_store(app.handle());
            crate::hosts_manager::start_refresh_loop(app.handle().clone());
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
