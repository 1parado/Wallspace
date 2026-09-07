//! tray.rs —— 系统托盘：当前壁纸 / 打开主界面 / 立即轮换下一张 / 暂停轮换 / 退出。
//!
//! 菜单文案按启动时的界面语言选择（settings.locale）。
//! 暂停项文案由 `sync_pause_label` 维护；当前壁纸标题由 `sync_current_title` 维护
//! （托盘菜单、自动轮换、手动应用三条路径共用）。

use crate::autoswitch;
use crate::library;
use crate::settings;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

/// 托盘菜单项 + 双语文案，挂在 AppHandle 状态上供外部同步
pub struct TrayState {
    pause: Mutex<Option<MenuItem<Wry>>>,
    title: Mutex<Option<MenuItem<Wry>>>,
    label_pause: String,
    label_resume: String,
    title_prefix: String,
}

/// 同步「暂停/恢复轮换」菜单文案（托盘与快捷键触发后都要调用）
pub fn sync_pause_label(app: &AppHandle, paused: bool) {
    let Some(state) = app.try_state::<TrayState>() else {
        return;
    };
    let Ok(guard) = state.pause.lock() else {
        return;
    };
    if let Some(item) = guard.as_ref() {
        let text = if paused { &state.label_resume } else { &state.label_pause };
        let _ = item.set_text(text);
    }
}

/// 同步托盘上的当前壁纸标题
pub fn sync_current_title(app: &AppHandle, title: &str) {
    let Some(state) = app.try_state::<TrayState>() else {
        return;
    };
    let Ok(guard) = state.title.lock() else {
        return;
    };
    if let Some(item) = guard.as_ref() {
        let mut short = title.trim().to_string();
        if short.chars().count() > 40 {
            short = short.chars().take(39).collect::<String>() + "…";
        }
        let _ = item.set_text(&format!("{}{}", state.title_prefix, short));
    }
}

pub fn init(app: &tauri::App) -> tauri::Result<()> {
    let zh = settings::load(app.handle()).locale != "en";
    let (label_show, label_next, label_quit, tip, title_prefix) = if zh {
        (
            "打开 Wallspace",
            "下一张壁纸",
            "退出 Wallspace",
            "Wallspace",
            "当前壁纸：",
        )
    } else {
        (
            "Open Wallspace",
            "Next wallpaper",
            "Quit Wallspace",
            "Wallspace",
            "Current: ",
        )
    };
    let (label_pause, label_resume) = if zh {
        ("暂停轮换", "恢复轮换")
    } else {
        ("Pause rotation", "Resume rotation")
    };

    // 顶部禁用项：显示当前壁纸标题
    let title_item = MenuItemBuilder::with_id("current-title", "—").enabled(false).build(app)?;
    let show = MenuItem::with_id(app, "show", label_show, true, None::<&str>)?;
    let next = MenuItem::with_id(app, "next", label_next, true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", label_pause, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", label_quit, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&title_item, &show, &next, &pause, &quit])?;

    app.manage(TrayState {
        pause: Mutex::new(Some(pause.clone())),
        title: Mutex::new(Some(title_item.clone())),
        label_pause: label_pause.into(),
        label_resume: label_resume.into(),
        title_prefix: title_prefix.into(),
    });

    // 启动时先回填上次应用的壁纸标题
    if let Some(last) = library::load(app.handle())
        .into_iter()
        .filter(|i| i.applied_at.is_some())
        .max_by_key(|i| i.applied_at.unwrap_or(0))
    {
        sync_current_title(app.handle(), &last.title);
    }

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
                sync_pause_label(app, paused);
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
