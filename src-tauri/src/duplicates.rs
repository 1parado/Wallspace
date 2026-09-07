//! duplicates.rs —— 重复图片检测。
//!
//! 两阶段：先按文件大小分桶（零 IO 成本），仅对大小相同的文件计算内容哈希
//! （流式 SipHash，与文件大小无关的碰撞概率即可满足查重场景），输出重复组。

use crate::library;
use crate::models::{CmdResult, WallpaperItem};
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::Read;
use tauri::AppHandle;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupGroup {
    /// 组内单张文件大小（字节）
    pub file_size: u64,
    /// 组内条目 id（全部互为重复）
    pub ids: Vec<String>,
}

/// 流式计算文件内容哈希；读失败返回 None（跳过该文件）。
fn hash_file(path: &str) -> Option<u64> {
    let f = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(f);
    let mut hasher = DefaultHasher::new();
    let mut buf = [0u8; 65536];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => buf[..n].hash(&mut hasher),
            Err(_) => return None,
        }
    }
    Some(hasher.finish())
}

/// 检测库内重复图片，按单张文件大小降序返回。
pub async fn find(app: AppHandle) -> CmdResult<Vec<DupGroup>> {
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<DupGroup>, String> {
        let items: Vec<WallpaperItem> = library::load(&app);

        // 1) 按文件大小分桶
        let mut by_size: HashMap<u64, Vec<&WallpaperItem>> = HashMap::new();
        for it in &items {
            by_size.entry(it.file_size).or_default().push(it);
        }

        // 2) 仅对大小相同的组计算内容哈希
        let mut out: Vec<DupGroup> = Vec::new();
        for (size, group) in by_size {
            if group.len() < 2 {
                continue;
            }
            let mut by_hash: HashMap<u64, Vec<String>> = HashMap::new();
            for it in group {
                if let Some(h) = hash_file(&it.file_path) {
                    by_hash.entry(h).or_default().push(it.id.clone());
                }
            }
            for (_, mut ids) in by_hash {
                if ids.len() > 1 {
                    ids.sort();
                    out.push(DupGroup { file_size: size, ids });
                }
            }
        }
        out.sort_by(|a, b| b.file_size.cmp(&a.file_size));
        Ok(out)
    })
    .await
    .map_err(|e| format!("查重任务失败: {e}"))?
}
