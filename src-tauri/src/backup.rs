//! backup.rs —— 库备份与迁移。
//!
//! 备份 = zip 包：`manifest.json`（条目元数据 + 集合）+ `images/<uuid>.<ext>`。
//! 不包含 API 密钥等敏感设置；恢复时按 id 合并（已存在则跳过），文件重名自动改名。

use crate::collections::{self, Collection};
use crate::library;
use crate::models::{now_ms, CmdResult, WallpaperItem};
use crate::paths;
use crate::store;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use uuid::Uuid;
use zip::write::SimpleFileOptions;

const MANIFEST: &str = "manifest.json";
const FORMAT: u32 = 1;

#[derive(Serialize, Deserialize)]
struct BackupItem {
    #[serde(flatten)]
    item: WallpaperItem,
    /// 备份包内的相对路径（images/<uuid>.<ext>）
    file: String,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    app: String,
    format: u32,
    exported_at: u64,
    items: Vec<BackupItem>,
    collections: Vec<Collection>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    pub imported: u32,
    pub skipped: u32,
    pub collections_added: u32,
}

fn basename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "image".into())
}

/// 导出库到 zip 备份；返回导出的条目数。
pub fn export(app: &AppHandle, save_path: &Path) -> CmdResult<u32> {
    let items = library::load(app);
    let cols = collections::load(app);

    let file = fs::File::create(save_path).map_err(|e| format!("创建备份文件失败: {e}"))?;
    let mut zw = zip::ZipWriter::new(file);
    let opts = SimpleFileOptions::default();

    let backup_items: Vec<BackupItem> = items
        .iter()
        .map(|it| BackupItem {
            item: it.clone(),
            file: format!("images/{}", basename(&it.file_path)),
        })
        .collect();

    let manifest = Manifest {
        app: "wallspace".into(),
        format: FORMAT,
        exported_at: now_ms(),
        items: backup_items,
        collections: cols,
    };

    zw.start_file(MANIFEST, opts)
        .map_err(|e| format!("写入备份失败: {e}"))?;
    let json = serde_json::to_vec_pretty(&manifest).map_err(|e| format!("序列化失败: {e}"))?;
    zw.write_all(&json).map_err(|e| format!("写入备份失败: {e}"))?;

    for it in &items {
        let bytes = fs::read(&it.file_path)
            .map_err(|e| format!("读取 {} 失败: {e}", it.file_path))?;
        zw.start_file(format!("images/{}", basename(&it.file_path)), opts)
            .map_err(|e| format!("写入备份失败: {e}"))?;
        zw.write_all(&bytes).map_err(|e| format!("写入备份失败: {e}"))?;
    }

    zw.finish().map_err(|e| format!("完成备份失败: {e}"))?;
    Ok(items.len() as u32)
}

/// 从 zip 备份恢复：按 id 合并（已存在跳过），文件重名自动改名，集合同名同 id 跳过。
pub fn import(app: &AppHandle, zip_path: &Path) -> CmdResult<ImportOutcome> {
    let file = fs::File::open(zip_path).map_err(|e| format!("打开备份失败: {e}"))?;
    let mut ar =
        zip::ZipArchive::new(file).map_err(|e| format!("读取备份失败: {e}"))?;

    let mut mf = String::new();
    ar.by_name(MANIFEST)
        .map_err(|_| "备份缺少 manifest.json，文件可能损坏")?
        .read_to_string(&mut mf)
        .map_err(|e| format!("读取清单失败: {e}"))?;
    let manifest: Manifest =
        serde_json::from_str(&mf).map_err(|e| format!("清单解析失败: {e}"))?;
    if manifest.format > FORMAT {
        return Err("备份格式过新，请先升级 Wallspace".into());
    }

    let existing: HashSet<String> = library::load(app).into_iter().map(|i| i.id).collect();
    let root: PathBuf = paths::data_root(app)?;
    let wp = root.join("wallpapers");
    let mut id_map: HashMap<String, String> = HashMap::new();
    let mut new_items: Vec<WallpaperItem> = Vec::new();
    let mut imported = 0u32;
    let mut skipped = 0u32;

    for bi in &manifest.items {
        if existing.contains(&bi.item.id) {
            id_map.insert(bi.item.id.clone(), bi.item.id.clone());
            skipped += 1;
            continue;
        }
        let new_id = bi.item.id.clone();
        id_map.insert(bi.item.id.clone(), new_id.clone());

        let src_name = bi.file.trim_start_matches("images/").to_string();
        let ext = Path::new(&src_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg")
            .to_string();
        let mut file_name = format!("{new_id}.{ext}");
        while wp.join(&file_name).exists() {
            file_name = format!("{}.{}", Uuid::new_v4(), ext);
        }

        let mut data = Vec::new();
        ar.by_name(&bi.file)
            .map_err(|e| format!("读取 {} 失败: {e}", bi.file))?
            .read_to_end(&mut data)
            .map_err(|e| format!("读取 {} 失败: {e}", bi.file))?;
        fs::write(wp.join(&file_name), &data).map_err(|e| format!("写入文件失败: {e}"))?;

        let mut item = bi.item.clone();
        item.file_path = wp.join(&file_name).to_string_lossy().into_owned();
        new_items.push(item);
        imported += 1;
    }

    store::with(app, |c| {
        for it in &new_items {
            store::insert_item(c, it)?;
        }
        Ok(())
    })?;

    // 恢复集合：id 已存在则跳过；itemIds 按 id 映射换算并过滤掉库中不存在的
    let mut cols = collections::load(app);
    let present: HashSet<String> = {
        let mut s = HashSet::new();
        for it in library::load(app) {
            s.insert(it.id);
        }
        s
    };
    let mut added = 0u32;
    for col in &manifest.collections {
        if cols.iter().any(|c| c.id == col.id) {
            continue;
        }
        cols.push(Collection {
            id: col.id.clone(),
            name: col.name.clone(),
            created_at: col.created_at,
            item_ids: col
                .item_ids
                .iter()
                .filter_map(|old| id_map.get(old).cloned())
                .filter(|nid| present.contains(nid))
                .collect(),
        });
        added += 1;
    }
    if added > 0 {
        collections::save(app, &cols)?;
    }

    Ok(ImportOutcome {
        imported,
        skipped,
        collections_added: added,
    })
}
