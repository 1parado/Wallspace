//! collections.rs —— 用户自定义集合（类似播放列表）。
//!
//! 整存整取：前端拼好完整集合数组后一次 save，与 settings 同款原子写。

use crate::models::{now_ms, CmdResult};
use crate::paths;
use serde::{Deserialize, Serialize};
use std::fs;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub item_ids: Vec<String>,
    pub created_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CollectionsFile {
    version: u32,
    collections: Vec<Collection>,
}

impl Default for CollectionsFile {
    fn default() -> Self {
        Self {
            version: 1,
            collections: Vec::new(),
        }
    }
}

fn collections_file(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(paths::data_root(app)?.join("collections.json"))
}

pub fn load(app: &AppHandle) -> Vec<Collection> {
    let Ok(path) = collections_file(app) else {
        return Vec::new();
    };
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<CollectionsFile>(&text)
            .map(|f| f.collections)
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn save(app: &AppHandle, collections: &[Collection]) -> CmdResult<()> {
    // 基本校验：名称非空、id 唯一
    let mut seen = std::collections::HashSet::new();
    for c in collections {
        if c.name.trim().is_empty() {
            return Err("集合名称不能为空".into());
        }
        if !seen.insert(c.id.clone()) {
            return Err(format!("集合 id 重复: {}", c.id));
        }
    }
    let path = collections_file(app)?;
    let tmp = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(&CollectionsFile {
        version: 1,
        collections: collections.to_vec(),
    })
    .map_err(|e| format!("序列化失败: {e}"))?;
    fs::write(&tmp, content).map_err(|e| format!("写入失败: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("保存失败: {e}"))?;
    Ok(())
}

pub fn new_collection(name: String) -> Collection {
    Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.trim().to_string(),
        item_ids: Vec::new(),
        created_at: now_ms(),
    }
}
