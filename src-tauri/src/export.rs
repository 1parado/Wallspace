//! export.rs —— 按预设尺寸导出/裁剪（阶段 1：本地裁剪导出）。
//!
//! 支持两种去向：
//! - add_to_library = true：裁剪结果编码为 JPEG 后复用 add_image_bytes 入库（source = "export"）
//! - save_path 指定：直接写入用户通过系统「另存为」选择的目标路径
//! - 两者皆无：写入 exports/ 目录并仅返回路径

use crate::library::{self, ExtraMeta};
use crate::models::{CmdResult, WallpaperItem};
use crate::wallpaper;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    /// 实际写入的文件路径（入库模式下为新条目的文件路径）
    pub path: String,
    /// 加入媒体库时生成的新条目；另存为时为 null
    pub item: Option<WallpaperItem>,
}

/// 按目标尺寸裁剪导出。
/// mode: "cover"（可带 offset_x/offset_y 取景偏移）| "fit"（黑边完整显示）。
#[allow(clippy::too_many_arguments)]
pub fn export(
    app: &AppHandle,
    item: &WallpaperItem,
    width: u32,
    height: u32,
    mode: &str,
    offset_x: f32,
    offset_y: f32,
    add_to_library: bool,
    save_path: Option<String>,
    title: Option<String>,
) -> CmdResult<ExportResult> {
    let img = image::open(PathBuf::from(&item.file_path))
        .map_err(|e| format!("读取图片失败: {e}"))?;
    let tw = width.max(1);
    let th = height.max(1);

    let out = if mode == "fit" {
        wallpaper::fit_canvas(img, tw, th)
    } else {
        wallpaper::cover_crop_at(img, tw, th, offset_x, offset_y)
    };

    // 统一编码为 JPEG（quality 95）
    let mut jpeg_bytes: Vec<u8> = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, 95);
    out.write_with_encoder(encoder)
        .map_err(|e| format!("编码图片失败: {e}"))?;

    // 1) 加入媒体库
    if add_to_library {
        let new_item = library::add_image_bytes(
            app,
            jpeg_bytes,
            "export",
            title.unwrap_or_else(|| format!("{} (导出)", item.title)),
            item.category.clone(),
            ExtraMeta {
                prompt: item.prompt.clone(),
                ..Default::default()
            },
        )?;
        return Ok(ExportResult {
            path: new_item.file_path.clone(),
            item: Some(new_item),
        });
    }

    // 2) 另存为（前端已通过系统对话框取得目标路径）
    if let Some(dest) = save_path {
        fs::write(&dest, &jpeg_bytes).map_err(|e| format!("写入文件失败: {e}"))?;
        return Ok(ExportResult {
            path: dest,
            item: None,
        });
    }

    // 3) 兜底：写入 exports/ 目录
    let dir = crate::paths::data_root(app)?.join("exports");
    fs::create_dir_all(&dir).map_err(|e| format!("创建导出目录失败: {e}"))?;
    let stamp = chrono_ms();
    let file_name = format!("{}_{}x{}_{}.jpg", sanitize(&item.title), tw, th, stamp);
    let dest = dir.join(file_name);
    fs::write(&dest, &jpeg_bytes).map_err(|e| format!("写入文件失败: {e}"))?;
    Ok(ExportResult {
        path: dest.to_string_lossy().into_owned(),
        item: None,
    })
}

fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "export".into()
    } else {
        trimmed.chars().take(40).collect()
    }
}

fn chrono_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
