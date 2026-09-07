//! store.rs —— SQLite 持久化层（library / collections / settings / 轮换状态）。
//!
//! 设计：
//! - 单连接 + Mutex 串行化，杜绝 JSON 时代的并发写坏档
//! - WAL + synchronous=NORMAL，读写性能与崩溃安全兼顾
//! - 首次启动自动迁移旧 JSON（library.json / collections.json / settings.json / rotation.json），
//!   成功后改名 *.bak 保留备份，不删原文件
//! - items/tags/palette 内的复杂结构以 JSON 文本列存储，保持与前端结构零转换成本

use crate::models::{CmdResult, WallpaperItem};
use crate::paths;
use rusqlite::{params, Connection};
use std::fs;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct Db(pub Mutex<Connection>);

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS items (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  source TEXT NOT NULL,
  file_path TEXT NOT NULL,
  width INTEGER NOT NULL,
  height INTEGER NOT NULL,
  file_size INTEGER NOT NULL,
  category TEXT,
  tags TEXT NOT NULL DEFAULT '[]',
  palette TEXT,
  favorite INTEGER NOT NULL DEFAULT 0,
  prompt TEXT,
  model TEXT,
  origin_url TEXT,
  created_at INTEGER NOT NULL,
  applied_at INTEGER
);
CREATE INDEX IF NOT EXISTS idx_items_created ON items(created_at);
CREATE INDEX IF NOT EXISTS idx_items_category ON items(category);
CREATE INDEX IF NOT EXISTS idx_items_source ON items(source);
CREATE INDEX IF NOT EXISTS idx_items_favorite ON items(favorite);

CREATE TABLE IF NOT EXISTS collections (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS collection_items (
  collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
  item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  position INTEGER NOT NULL,
  PRIMARY KEY (collection_id, item_id)
);
CREATE INDEX IF NOT EXISTS idx_citems_coll ON collection_items(collection_id, position);

CREATE TABLE IF NOT EXISTS kv (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
"#;

/// 打开/初始化数据库并注册到 Tauri 状态；setup 时调用一次。
pub fn init(app: &AppHandle) -> CmdResult<()> {
    let root = paths::data_root(app)?;
    let conn = Connection::open(root.join("wallspace.db"))
        .map_err(|e| format!("打开数据库失败: {e}"))?;
    conn.pragma_update(None, "journal_mode", "WAL").ok();
    conn.pragma_update(None, "synchronous", "NORMAL").ok();
    conn.pragma_update(None, "foreign_keys", "ON").ok();
    conn.execute_batch(SCHEMA)
        .map_err(|e| format!("初始化数据库失败: {e}"))?;
    migrate_legacy(app, &conn)?;
    app.manage(Db(Mutex::new(conn)));
    Ok(())
}

/// 在托管连接上执行闭包（串行化访问）。
pub fn with<T>(
    app: &AppHandle,
    f: impl FnOnce(&Connection) -> Result<T, rusqlite::Error>,
) -> CmdResult<T> {
    let db = app.state::<Db>();
    let conn = db.0.lock().map_err(|_| "数据库被占用".to_string())?;
    f(&conn).map_err(|e| format!("数据库错误: {e}"))
}

// —— 行映射（library 复用） ——

pub fn row_to_item(row: &rusqlite::Row) -> rusqlite::Result<WallpaperItem> {
    let tags: String = row.get("tags")?;
    let palette: Option<String> = row.get("palette")?;
    Ok(WallpaperItem {
        id: row.get("id")?,
        title: row.get("title")?,
        source: row.get("source")?,
        file_path: row.get("file_path")?,
        width: row.get("width")?,
        height: row.get("height")?,
        file_size: row.get("file_size")?,
        category: row.get("category")?,
        tags: serde_json::from_str(&tags).unwrap_or_default(),
        palette: palette.and_then(|p| serde_json::from_str(&p).ok()),
        favorite: row.get::<_, i64>("favorite")? != 0,
        prompt: row.get("prompt")?,
        model: row.get("model")?,
        origin_url: row.get("origin_url")?,
        created_at: row.get("created_at")?,
        applied_at: row.get("applied_at")?,
    })
}

pub fn insert_item(conn: &Connection, it: &WallpaperItem) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO items
         (id, title, source, file_path, width, height, file_size, category, tags, palette,
          favorite, prompt, model, origin_url, created_at, applied_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
        params![
            it.id,
            it.title,
            it.source,
            it.file_path,
            it.width,
            it.height,
            it.file_size,
            it.category,
            serde_json::to_string(&it.tags).unwrap_or_else(|_| "[]".into()),
            it.palette
                .as_ref()
                .map(|p| serde_json::to_string(p).unwrap_or_default()),
            it.favorite as i64,
            it.prompt,
            it.model,
            it.origin_url,
            it.created_at,
            it.applied_at,
        ],
    )?;
    Ok(())
}

// —— kv 通用读写（settings / 轮换状态） ——

pub fn kv_get(app: &AppHandle, key: &str) -> Option<String> {
    with(app, |c| {
        c.query_row(
            "SELECT value FROM kv WHERE key = ?1",
            params![key],
            |r| r.get::<_, String>(0),
        )
    })
    .ok()
}

pub fn kv_set(app: &AppHandle, key: &str, value: &str) -> CmdResult<()> {
    with(app, |c| {
        c.execute(
            "INSERT OR REPLACE INTO kv (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    })
}

// —— 旧 JSON 一次性迁移 ——

fn migrate_legacy(app: &AppHandle, conn: &Connection) -> CmdResult<()> {
    let root = paths::data_root(app)?;

    // 1) library.json → items（仅在库为空时导入）
    let lib_path = root.join("library.json");
    if lib_path.exists() {
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
            .unwrap_or(0);
        if count == 0 {
            if let Ok(text) = fs::read_to_string(&lib_path) {
                if let Ok(file) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(arr) = file["items"].as_array() {
                        for v in arr {
                            if let Ok(item) = serde_json::from_value::<WallpaperItem>(v.clone()) {
                                insert_item(conn, &item).map_err(|e| format!("迁移条目失败: {e}"))?;
                            }
                        }
                    }
                }
            }
        }
        let _ = fs::rename(&lib_path, root.join("library.json.bak"));
    }

    // 2) collections.json → collections + collection_items
    let coll_path = root.join("collections.json");
    if coll_path.exists() {
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM collections", [], |r| r.get(0))
            .unwrap_or(0);
        if count == 0 {
            if let Ok(text) = fs::read_to_string(&coll_path) {
                if let Ok(file) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(arr) = file["collections"].as_array() {
                        for v in arr {
                            let id = v["id"].as_str().unwrap_or_default().to_string();
                            let name = v["name"].as_str().unwrap_or_default().to_string();
                            let created = v["createdAt"].as_u64().unwrap_or(0);
                            if id.is_empty() {
                                continue;
                            }
                            conn.execute(
                                "INSERT OR REPLACE INTO collections (id, name, created_at) VALUES (?1,?2,?3)",
                                params![id, name, created],
                            )
                            .map_err(|e| format!("迁移集合失败: {e}"))?;
                            if let Some(ids) = v["itemIds"].as_array() {
                                for (i, iid) in ids.iter().enumerate() {
                                    if let Some(iid) = iid.as_str() {
                                        conn.execute(
                                            "INSERT OR IGNORE INTO collection_items (collection_id, item_id, position) VALUES (?1,?2,?3)",
                                            params![id, iid, i as i64],
                                        )
                                        .map_err(|e| format!("迁移集合条目失败: {e}"))?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        let _ = fs::rename(&coll_path, root.join("collections.json.bak"));
    }

    // 3) settings.json → kv.settings
    let settings_path = root.join("settings.json");
    if settings_path.exists() {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM kv WHERE key = 'settings'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if exists == 0 {
            if let Ok(text) = fs::read_to_string(&settings_path) {
                if serde_json::from_str::<serde_json::Value>(&text).is_ok() {
                    conn.execute(
                        "INSERT OR REPLACE INTO kv (key, value) VALUES ('settings', ?1)",
                        params![text],
                    )
                    .map_err(|e| format!("迁移设置失败: {e}"))?;
                }
            }
        }
        let _ = fs::rename(&settings_path, root.join("settings.json.bak"));
    }

    // 4) rotation.json → kv.rotation
    let rot_path = root.join("rotation.json");
    if rot_path.exists() {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM kv WHERE key = 'rotation'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if exists == 0 {
            if let Ok(text) = fs::read_to_string(&rot_path) {
                if serde_json::from_str::<serde_json::Value>(&text).is_ok() {
                    conn.execute(
                        "INSERT OR REPLACE INTO kv (key, value) VALUES ('rotation', ?1)",
                        params![text],
                    )
                    .map_err(|e| format!("迁移轮换状态失败: {e}"))?;
                }
            }
        }
        let _ = fs::rename(&rot_path, root.join("rotation.json.bak"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str) -> WallpaperItem {
        serde_json::from_value(serde_json::json!({
            "id": id, "title": format!("t-{id}"), "source": "ai",
            "filePath": "C:/x.png", "width": 100, "height": 50, "fileSize": 9,
            "category": "Nature/Mountains", "tags": ["dark", "anime"],
            "palette": ["#112233"], "favorite": true, "createdAt": 123
        }))
        .unwrap()
    }

    #[test]
    fn schema_insert_load_roundtrip() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        insert_item(&conn, &sample("a")).unwrap();
        insert_item(&conn, &sample("b")).unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id,title,source,file_path,width,height,file_size,category,tags,palette,
                        favorite,prompt,model,origin_url,created_at,applied_at
                 FROM items ORDER BY created_at",
            )
            .unwrap();
        let items: Vec<WallpaperItem> = stmt
            .query_map([], row_to_item)
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "a");
        assert_eq!(items[0].tags, vec!["dark", "anime"]);
        assert_eq!(items[0].category.as_deref(), Some("Nature/Mountains"));
        assert!(items[0].favorite);
        assert_eq!(items[0].palette.as_deref(), Some(&["#112233".to_string()][..]));
        // 删除联动：集合引用清理
        conn.execute(
            "INSERT INTO collections (id,name,created_at) VALUES ('c1','n',1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO collection_items (collection_id,item_id,position) VALUES ('c1','a',0)",
            [],
        )
        .unwrap();
        conn.execute("DELETE FROM items WHERE id='a'", []).unwrap();
        let left: i64 = conn
            .query_row("SELECT COUNT(*) FROM collection_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(left, 0);
    }

    #[test]
    fn settings_kv_roundtrip() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO kv (key,value) VALUES ('settings','{\"locale\":\"en\"}')",
            [],
        )
        .unwrap();
        let v: String = conn
            .query_row("SELECT value FROM kv WHERE key='settings'", [], |r| r.get(0))
            .unwrap();
        assert!(v.contains("\"en\""));
    }
}
