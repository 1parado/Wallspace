//! tray.rs —— 系统托盘：打开主界面 / 立即轮换下一张 / 退出。
//!
//! 菜单文案按启动时的界面语言选择（settings.locale）。

use crate::autoswitch;
use crate::settings;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;

pub fn init(app: &tauri::App) -> tauri::Result<()> {
    let zh = settings::load(app.handle()).locale != "en";
    let (label_show, label_next, label_quit, tip) = if zh {
        (
            "打开 Wallspace",
            "下一张壁纸",
            "退出 Wallspace",
            "Wallspace",
        )
    } else {
        ("Open Wallspace", "Next wallpaper", "Quit Wallspace", "Wallspace")
    };
    let (label_pause, label_resume) = if zh {
        ("暂停轮换", "恢复轮换")
    } else {
        ("Pause rotation", "Resume rotation")
    };

    let show = MenuItem::with_id(app, "show", label_show, true, None::<&str>)?;
    let next = MenuItem::with_id(app, "next", label_next, true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", label_pause, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", label_quit, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &next, &pause, &quit])?;

    let pause_for_handler = pause.clone();
    let mut builder = TrayIconBuilder::with_id("wallspace-tray")
        .menu(&menu)
        .tooltip(tip)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.unminimize();
                    let _ = win.set_focus();
                }
            }
            "next" => {
                // 裁剪 + COM 调用是阻塞操作，放到独立线程避免卡 UI
                let handle = app.clone();
                std::thread::spawn(move || autoswitch::force_next(&handle));
            }
            "pause" => {
                let paused = autoswitch::pause_toggle();
                let _ = pause_for_handler.set_text(if paused {
                    label_resume
                } else {
                    label_pause
                });
            }
            "quit" => app.exit(0),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
