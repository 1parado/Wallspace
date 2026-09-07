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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimGroup {
    /// 组内条目 id（感知上互为相似）
    pub ids: Vec<String>,
}

/// aHash 感知哈希：8×8 灰度缩略图，像素亮度与均值比较生成 64 位指纹。
fn ahash64(path: &str) -> Option<u64> {
    let img = image::open(path).ok()?;
    let small = img
        .resize_exact(8, 8, image::imageops::FilterType::Triangle)
        .to_luma8();
    let vals: Vec<u8> = small.pixels().map(|p| p.0[0]).collect();
    let mean = vals.iter().map(|&v| v as u32).sum::<u32>() / (vals.len() as u32).max(1);
    let mut h = 0u64;
    for (i, &v) in vals.iter().enumerate() {
        if u32::from(v) > mean {
            h |= 1 << i;
        }
    }
    Some(h)
}

/// 检测相似图片：aHash + 汉明距离 ≤ threshold，并查集归组（多线程解码）。
pub async fn find_similar(app: AppHandle, threshold: u32) -> CmdResult<Vec<SimGroup>> {
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<SimGroup>, String> {
        let items = library::load(&app);
        let list: Vec<(String, String)> = items
            .iter()
            .map(|it| (it.id.clone(), it.file_path.clone()))
            .collect();

        // 多线程并行解码计算哈希（CPU 密集，按线程数分块）
        let mut hashes: Vec<(String, u64)> = Vec::new();
        let chunk = (list.len() / 8 + 1).max(1);
        std::thread::scope(|s| {
            let handles: Vec<_> = list
                .chunks(chunk)
                .map(|c| {
                    let c = c.to_vec();
                    s.spawn(move || {
                        c.into_iter()
                            .filter_map(|(id, p)| ahash64(&p).map(|h| (id, h)))
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            for h in handles {
                if let Ok(part) = h.join() {
                    hashes.extend(part);
                }
            }
        });

        // 并查集归组：两两汉明距离 ≤ threshold 即同类
        let n = hashes.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn root(p: &mut Vec<usize>, mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        for i in 0..n {
            for j in i + 1..n {
                let d = (hashes[i].1 ^ hashes[j].1).count_ones();
                if d <= threshold {
                    let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
                    if ri != rj {
                        parent[ri] = rj;
                    }
                }
            }
        }

        // 收集组（保持库内原始顺序），只留 >1 的组
        let mut by_root: HashMap<usize, Vec<String>> = HashMap::new();
        for (idx, (id, _)) in hashes.iter().enumerate() {
            by_root.entry(root(&mut parent, idx)).or_default().push(id.clone());
        }
        let mut out: Vec<SimGroup> = by_root
            .into_values()
            .filter(|ids| ids.len() > 1)
            .map(|ids| SimGroup { ids })
            .collect();
        out.sort_by(|a, b| b.ids.len().cmp(&a.ids.len()));
        Ok(out)
    })
    .await
    .map_err(|e| format!("相似检测任务失败: {e}"))?
}
