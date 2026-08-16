use crate::models::{now_ms, CmdResult, FailedImport, ImportReport, WallpaperItem};
use crate::paths;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::Path;
use tauri::AppHandle;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct LibraryFile {
    version: u32,
    items: Vec<WallpaperItem>,
}

impl Default for LibraryFile {
    fn default() -> Self {
        Self {
            version: 1,
            items: Vec::new(),
        }
    }
}

pub fn load(app: &AppHandle) -> Vec<WallpaperItem> {
    let path = match paths::library_file(app) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<LibraryFile>(&text)
            .map(|f| f.items)
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn save(app: &AppHandle, items: &[WallpaperItem]) -> CmdResult<()> {
    let path = paths::library_file(app)?;
    let tmp = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(&LibraryFile {
        version: 1,
        items: items.to_vec(),
    })
    .map_err(|e| format!("序列化失败: {e}"))?;
    fs::write(&tmp, content).map_err(|e| format!("写入失败: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("保存失败: {e}"))?;
    Ok(())
}

fn sniff_image_ext(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.len() > 2 && bytes[0] == 0xFF && bytes[1] == 0xD8 {
        Some("jpg")
    } else if bytes.starts_with(b"GIF8") {
        Some("gif")
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else if bytes.starts_with(b"BM") {
        Some("bmp")
    } else {
        None
    }
}

fn dimensions_of(bytes: &[u8]) -> CmdResult<(u32, u32)> {
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| format!("无法识别图片格式: {e}"))?;
    reader
        .into_dimensions()
        .map_err(|e| format!("读取图片尺寸失败: {e}"))
}

/// 将图片字节落盘并写入元数据，返回新条目。
pub fn add_image_bytes(
    app: &AppHandle,
    bytes: Vec<u8>,
    source: &str,
    title: String,
    category: &str,
    extra: ExtraMeta,
) -> CmdResult<WallpaperItem> {
    let ext = sniff_image_ext(&bytes).ok_or_else(|| "不支持的图片格式".to_string())?;
    let (w, h) = dimensions_of(&bytes)?;
    let id = Uuid::new_v4().to_string();
    let file_name = format!("{id}.{ext}");

    let root = paths::data_root(app)?;
    let dest = root.join("wallpapers").join(&file_name);
    fs::write(&dest, &bytes).map_err(|e| format!("写入文件失败: {e}"))?;

    let item = WallpaperItem {
        id,
        title,
        source: source.into(),
        file_path: dest.to_string_lossy().into_owned(),
        width: w,
        height: h,
        file_size: bytes.len() as u64,
        category: if category.is_empty() { "Minimal".into() } else { category.into() },
        tags: Vec::new(),
        favorite: false,
        prompt: extra.prompt,
        model: extra.model,
        origin_url: extra.origin_url,
        created_at: now_ms(),
        applied_at: None,
    };

    let mut items = load(app);
    items.insert(0, item.clone());
    save(app, &items)?;
    Ok(item)
}

#[derive(Default)]
pub struct ExtraMeta {
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub origin_url: Option<String>,
}

/// 导入本地图片文件（复制进 wallpapers/ 目录）。
pub fn import_local_files(
    app: &AppHandle,
    paths_in: Vec<String>,
    category: &str,
) -> CmdResult<ImportReport> {
    let mut imported = Vec::new();
    let mut failed = Vec::new();
    for p in paths_in {
        let path = Path::new(&p);
        match fs::read(path) {
            Ok(bytes) => {
                let title = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Imported".into());
                match add_image_bytes(app, bytes, "local", title, category, ExtraMeta::default()) {
                    Ok(item) => imported.push(item),
                    Err(e) => failed.push(FailedImport { path: p, reason: e }),
                }
            }
            Err(e) => failed.push(FailedImport {
                path: p,
                reason: format!("读取失败: {e}"),
            }),
        }
    }
    Ok(ImportReport { imported, failed })
}

pub fn update_item(app: &AppHandle, item: WallpaperItem) -> CmdResult<WallpaperItem> {
    let mut items = load(app);
    let idx = items
        .iter()
        .position(|i| i.id == item.id)
        .ok_or_else(|| "条目不存在".to_string())?;
    items[idx] = item.clone();
    save(app, &items)?;
    Ok(item)
}

pub fn delete_item(app: &AppHandle, id: &str) -> CmdResult<()> {
    let mut items = load(app);
    let idx = items
        .iter()
        .position(|i| i.id == id)
        .ok_or_else(|| "条目不存在".to_string())?;
    let item = items.remove(idx);

    // 删除原图与适配缓存
    let _ = fs::remove_file(Path::new(&item.file_path));
    if let Ok(root) = paths::data_root(app) {
        if let Ok(entries) = fs::read_dir(root.join("adapted")) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with(id) {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }
    save(app, &items)
}

pub fn touch_applied(app: &AppHandle, id: &str) -> CmdResult<()> {
    let mut items = load(app);
    if let Some(item) = items.iter_mut().find(|i| i.id == id) {
        item.applied_at = Some(now_ms());
        save(app, &items)?;
    }
    Ok(())
}
