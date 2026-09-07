//! collections.rs —— 用户自定义集合（类似播放列表）。
//!
//! SQLite 持久化：collections + collection_items（position 维护用户排序）。
//! 整存整取：前端拼好完整集合数组后一次 save，事务内全量重写。

use crate::models::{now_ms, CmdResult};
use crate::store;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub item_ids: Vec<String>,
    pub created_at: u64,
    /// 自定义封面条目 id；空则前端回退到集合首图
    #[serde(default)]
    pub cover_item_id: Option<String>,
}

pub fn load(app: &AppHandle) -> Vec<Collection> {
    store::with(app, |c| {
        let mut stmt = c.prepare("SELECT id, name, created_at, cover_item_id FROM collections")?;
        let mut cols: Vec<Collection> = stmt
            .query_map([], |r| {
                Ok(Collection {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    item_ids: Vec::new(),
                    created_at: r.get(2)?,
                    cover_item_id: r.get(3)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        for col in &mut cols {
            let mut stmt = c.prepare(
                "SELECT item_id FROM collection_items WHERE collection_id = ?1 ORDER BY position",
            )?;
            let ids = stmt
                .query_map([&col.id], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            col.item_ids = ids;
        }
        Ok(cols)
    })
    .unwrap_or_default()
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
    store::with(app, |c| {
        let tx = c.unchecked_transaction()?;
        tx.execute("DELETE FROM collection_items", [])?;
        tx.execute("DELETE FROM collections", [])?;
        for col in collections {
            tx.execute(
                "INSERT INTO collections (id, name, created_at, cover_item_id) VALUES (?1,?2,?3,?4)",
                rusqlite::params![col.id, col.name, col.created_at, col.cover_item_id],
            )?;
            for (i, iid) in col.item_ids.iter().enumerate() {
                tx.execute(
                    "INSERT OR IGNORE INTO collection_items (collection_id, item_id, position) VALUES (?1,?2,?3)",
                    rusqlite::params![col.id, iid, i as i64],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    })
}

pub fn new_collection(name: String) -> Collection {
    Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.trim().to_string(),
        item_ids: Vec::new(),
        created_at: now_ms(),
        cover_item_id: None,
    }
}
