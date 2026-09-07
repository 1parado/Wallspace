//! autoswitch.rs —— 定时轮换壁纸（后台线程）。
//!
//! 每 30 秒检查一次设置：
//! - 启用条件：auto_switch_collection_id 非空且对应集合存在有效条目
//! - 距上次切换超过 auto_switch_interval_min 分钟 → 取集合内下一张（循环）
//! - 应用范围：主屏（display=None）或全部显示器逐屏适配
//!
//! 状态持久化在 rotation.json（index + last_switch），重启后按上次位置继续轮换。

use crate::collections;
use crate::library;
use crate::models::{now_ms, CmdResult, Settings, WallpaperItem};
use crate::paths;
use crate::settings;
use crate::wallpaper;
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Duration;
use tauri::AppHandle;

#[derive(Debug, Serialize, Deserialize)]
struct SwitchState {
    index: usize,
    last_switch: u64,
}

impl Default for SwitchState {
    fn default() -> Self {
        Self {
            index: 0,
            last_switch: 0,
        }
    }
}

/// 启动后台轮换线程（应用 setup 时调用一次）。
pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(30));
        // 轮换失败静默忽略（如显示器暂时不可用），下一轮重试
        let _ = tick(&app);
    });
}

fn tick(app: &AppHandle) -> CmdResult<()> {
    let cfg = settings::load(app);
    let Some(cid) = cfg
        .auto_switch_collection_id
        .clone()
        .filter(|s| !s.trim().is_empty())
    else {
        return Ok(());
    };
    let interval_ms = (cfg.auto_switch_interval_min.max(1) as u64) * 60_000;

    let state = load_state(app);
    let now = now_ms();
    if state.last_switch != 0 && now.saturating_sub(state.last_switch) < interval_ms {
        return Ok(());
    }

    let mut items = items_of_collection(app, &cid);
    // 单张集合轮换无意义，但保留（相当于定时重设同一张）
    if items.is_empty() {
        return Ok(());
    }
    // index 越界（集合缩小过）时取模回绕
    let idx = state.index % items.len();
    let item = items.swap_remove(idx);

    apply(app, &cfg, &item)?;
    library::touch_applied(app, &item.id)?;

    save_state(
        app,
        &SwitchState {
            index: (idx + 1) % items.len(),
            last_switch: now,
        },
    )
}

fn items_of_collection(app: &AppHandle, cid: &str) -> Vec<WallpaperItem> {
    let Some(col) = collections::load(app).into_iter().find(|c| c.id == cid) else {
        return Vec::new();
    };
    let lib = library::load(app);
    col.item_ids
        .iter()
        .filter_map(|id| lib.iter().find(|i| &i.id == id).cloned())
        .collect()
}

fn apply(app: &AppHandle, cfg: &Settings, item: &WallpaperItem) -> CmdResult<()> {
    if cfg.auto_switch_scope == "all" {
        for m in wallpaper::list_monitors_impl()? {
            wallpaper::adapt_and_apply(app, item, Some(&m.id))?;
        }
        Ok(())
    } else {
        wallpaper::adapt_and_apply(app, item, None)
    }
}

fn load_state(app: &AppHandle) -> SwitchState {
    let Ok(root) = paths::data_root(app) else {
        return SwitchState::default();
    };
    match fs::read_to_string(root.join("rotation.json")) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => SwitchState::default(),
    }
}

fn save_state(app: &AppHandle, s: &SwitchState) -> CmdResult<()> {
    let root = paths::data_root(app)?;
    let path = root.join("rotation.json");
    let tmp = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(s).map_err(|e| format!("序列化失败: {e}"))?;
    fs::write(&tmp, content).map_err(|e| format!("写入失败: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("保存失败: {e}"))?;
    Ok(())
}
