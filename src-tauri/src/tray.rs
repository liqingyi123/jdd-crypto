use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

use crate::windows;

pub fn build_app_menu<R: Runtime, M: Manager<R>>(app: &M) -> tauri::Result<Menu<R>> {
    let show_main = MenuItem::with_id(app, "show_main", "打开主界面", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "功能设置", true, None::<&str>)?;
    let feedback = MenuItem::with_id(app, "feedback", "意见反馈", true, None::<&str>)?;
    let plugins = MenuItem::with_id(app, "plugins", "插件管理", true, None::<&str>)?;
    let about = MenuItem::with_id(app, "about", "关于", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;

    Menu::with_items(
        app,
        &[
            &show_main,
            &settings,
            &feedback,
            &plugins,
            &about,
            &sep,
            &quit,
        ],
    )
}

pub fn handle_menu_id(app: &AppHandle, id: &str) {
    match id {
        "show_main" => windows::show_main(app, None),
        "settings" => windows::show_feature(app, "settings"),
        "feedback" => windows::show_feature(app, "feedback"),
        "plugins" => windows::show_feature(app, "plugins"),
        "about" => windows::show_feature(app, "about"),
        "quit" => app.exit(0),
        _ => {}
    }
}

pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let menu = build_app_menu(app)?;
    let mut builder = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("多多解密\n点击打开加解密主界面\n右击打开菜单")
        .on_menu_event(|app, event| {
            handle_menu_id(app, event.id.as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                windows::show_main(tray.app_handle(), None);
            }
        });

    let icon = match app.default_window_icon().cloned() {
        Some(icon) => icon,
        None => Image::from_bytes(include_bytes!("../icons/icon.png"))?,
    };
    builder = builder.icon(icon);

    builder.build(app)?;
    Ok(())
}
