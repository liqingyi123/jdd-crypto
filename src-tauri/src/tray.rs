use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

use crate::brightness_quick;
use crate::screensaver;
use crate::windows;

pub fn build_app_menu<R: Runtime, M: Manager<R>>(app: &M) -> tauri::Result<Menu<R>> {
    let settings = MenuItem::with_id(app, "settings", "功能设置", true, None::<&str>)?;
    let show_main = MenuItem::with_id(app, "show_main", "加解密", true, None::<&str>)?;
    let hosts = MenuItem::with_id(app, "hosts", "Host管理", true, None::<&str>)?;
    let brightness = MenuItem::with_id(app, "brightness", "亮度调节", true, None::<&str>)?;
    let screensaver_item = MenuItem::with_id(app, "screensaver", "屏幕保护", true, None::<&str>)?;
    let about = MenuItem::with_id(app, "about", "关于", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;

    Menu::with_items(
        app,
        &[
            &settings,
            &show_main,
            &hosts,
            &brightness,
            &screensaver_item,
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
        "hosts" => windows::show_feature(app, "hosts"),
        "brightness" => brightness_quick::show(app),
        "screensaver" => screensaver::show(app),
        "about" => windows::show_feature(app, "about"),
        "quit" => app.exit(0),
        _ => {}
    }
}

pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let menu = build_app_menu(app)?;
    let version = app.package_info().version.to_string();
    let tooltip = format!(
        "多多工具箱 v{version}\n左键打开加解密\n中键打开 Host 管理\n右击打开菜单"
    );
    let mut builder = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip(tooltip)
        .on_menu_event(|app, event| {
            handle_menu_id(app, event.id.as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    windows::show_main(tray.app_handle(), None);
                }
                TrayIconEvent::Click {
                    button: MouseButton::Middle,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    windows::show_feature(tray.app_handle(), "hosts");
                }
                _ => {}
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
