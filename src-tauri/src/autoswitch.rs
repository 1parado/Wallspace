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
use crate::settings;
use crate::store;
use crate::wallpaper;
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::AppHandle;

const ROTATION_KEY: &str = "rotation";

/// 轮换暂停标志（托盘切换；仅暂停定时轮换，不影响手动/托盘「下一张」）
static PAUSED: AtomicBool = AtomicBool::new(false);

/// 托盘切换暂停状态，返回切换后是否处于暂停
pub fn pause_toggle() -> bool {
    let paused = !PAUSED.load(Ordering::SeqCst);
    PAUSED.store(paused, Ordering::SeqCst);
    paused
}

#[derive(Debug, Serialize, Deserialize)]
struct SwitchState {
    index: usize,
    last_switch: u64,
    /// 日/夜模式的当前时间窗（"day#2026-09-07"），跨窗才切换
    #[serde(default)]
    window_key: Option<String>,
}

impl Default for SwitchState {
    fn default() -> Self {
        Self {
            index: 0,
            last_switch: 0,
            window_key: None,
        }
    }
}

/// 启动后台轮换线程（应用 setup 时调用一次）。
pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(30));
        // 轮换失败静默忽略（如显示器暂时不可用），下一轮重试
        let _ = rotate(&app, false);
    });
}

/// 托盘「下一张」：无视间隔立即轮换。
pub fn force_next(app: &AppHandle) {
    let _ = rotate(app, true);
}

fn rotate(app: &AppHandle, force: bool) -> CmdResult<()> {
    if !force && PAUSED.load(Ordering::SeqCst) {
        return Ok(());
    }
    let cfg = settings::load(app);
    let state = load_state(app);
    let now = now_ms();

    // 确定轮换目标集合与当前时间窗
    let daynight = cfg.auto_switch_mode == "daynight";
    let (cid, window_key) = if daynight {
        let Some(day_cid) = cfg
            .day_collection_id
            .clone()
            .filter(|s| !s.trim().is_empty())
        else {
            return Ok(());
        };
        let Some(night_cid) = cfg
            .night_collection_id
            .clone()
            .filter(|s| !s.trim().is_empty())
        else {
            return Ok(());
        };
        let now_local = chrono::Local::now();
        let now_min = now_local.hour() as u32 * 60 + now_local.minute() as u32;
        let day_s = hm_to_min(&cfg.day_start, 7 * 60);
        let night_s = hm_to_min(&cfg.night_start, 19 * 60);
        let is_day = if day_s <= night_s {
            now_min >= day_s && now_min < night_s
        } else {
            // 跨零点的窗口划分
            now_min >= day_s || now_min < night_s
        };
        let key = format!(
            "{}#{}",
            if is_day { "day" } else { "night" },
            now_local.format("%Y-%m-%d")
        );
        (if is_day { day_cid } else { night_cid }, Some(key))
    } else {
        let Some(cid) = cfg
            .auto_switch_collection_id
            .clone()
            .filter(|s| !s.trim().is_empty())
        else {
            return Ok(());
        };
        let interval_ms = (cfg.auto_switch_interval_min.max(1) as u64) * 60_000;
        if !force
            && state.last_switch != 0
            && now.saturating_sub(state.last_switch) < interval_ms
        {
            return Ok(());
        }
        (cid, None)
    };

    // 日/夜模式：同一时间窗内不重复切换（force 除外）
    if !force && daynight_gate(&state.window_key, &window_key) {
        return Ok(());
    }

    let mut items = items_of_collection(app, &cid);
    if items.is_empty() {
        return Ok(());
    }
    let idx = state.index % items.len();
    let item = items.swap_remove(idx);

    apply(app, &cfg, &item)?;
    library::touch_applied(app, &item.id)?;

    save_state(
        app,
        &SwitchState {
            index: (idx + 1) % items.len(),
            last_switch: now,
            window_key,
        },
    )
}

/// 日/夜模式闸门：窗口未变化则不切换
fn daynight_gate(state_key: &Option<String>, current: &Option<String>) -> bool {
    match (state_key, current) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

fn hm_to_min(s: &str, fallback: u32) -> u32 {
    let mut it = s.split(':');
    let h = it
        .next()
        .and_then(|v| v.parse::<u32>().ok())
        .map(|h| h.min(23))
        .unwrap_or(fallback / 60);
    let m = it
        .next()
        .and_then(|v| v.parse::<u32>().ok())
        .map(|m| m.min(59))
        .unwrap_or(fallback % 60);
    h * 60 + m
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
    store::kv_get(app, ROTATION_KEY)
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_state(app: &AppHandle, s: &SwitchState) -> CmdResult<()> {
    let text = serde_json::to_string_pretty(s).map_err(|e| format!("序列化失败: {e}"))?;
    store::kv_set(app, ROTATION_KEY, &text)
}
