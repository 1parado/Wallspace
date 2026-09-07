//! watch.rs —— 监视文件夹自动导入。
//!
//! 设置了 watch_folder 时，跟随轮换后台线程的 30 秒节拍扫描该文件夹，
//! 把新出现的图片自动导入媒体库（复用 add_image_bytes 管线：分类留空、主色提取）。
//! 已处理的文件名记录在 kv，避免重复导入；导入后向前端发 library-changed 事件。

use crate::library::{self, ExtraMeta};
use crate::models::CmdResult;
use crate::settings;
use crate::store;
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Emitter};

const PROCESSED_KEY: &str = "watch_processed";
const EXTENSIONS: [&str; 7] = ["jpg", "jpeg", "png", "webp", "gif", "bmp", "avif"];
const MAX_PER_TICK: usize = 20;

fn processed_names(app: &AppHandle) -> Vec<String> {
    store::kv_get(app, PROCESSED_KEY)
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .unwrap_or_default()
}

fn save_processed(app: &AppHandle, names: &[String]) -> CmdResult<()> {
    let text = serde_json::to_string(names).unwrap_or_else(|_| "[]".into());
    store::kv_set(app, PROCESSED_KEY, &text)
}

pub fn tick(app: &AppHandle) -> CmdResult<()> {
    let cfg = settings::load(app);
    let Some(folder) = cfg
        .watch_folder
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return Ok(());
    };
    let dir = Path::new(folder);
    if !dir.is_dir() {
        return Ok(());
    }

    let processed = processed_names(app);
    let mut new_files: Vec<(String, std::path::PathBuf)> = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let ext_ok = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
            .unwrap_or(false);
        if !ext_ok {
            continue;
        }
        if !processed.iter().any(|n| n == name) {
            new_files.push((name.to_string(), p));
        }
    }
    if new_files.is_empty() {
        return Ok(());
    }
    new_files.sort();
    new_files.truncate(MAX_PER_TICK);

    let mut processed_now = processed;
    let mut imported = 0u32;
    for (name, path) in new_files {
        // 无论成功与否都标记为已处理，避免坏文件每 30 秒重试
        if let Ok(bytes) = fs::read(&path) {
            let title = Path::new(&name)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| name.clone());
            if library::add_image_bytes(
                app,
                bytes,
                "local",
                title,
                None,
                ExtraMeta {
                    origin_url: Some(path.to_string_lossy().into_owned()),
                    ..Default::default()
                },
            )
            .is_ok()
            {
                imported += 1;
            }
        }
        processed_now.push(name);
    }
    save_processed(app, &processed_now)?;

    if imported > 0 {
        let _ = app.emit("library-changed", imported);
    }
    Ok(())
}
