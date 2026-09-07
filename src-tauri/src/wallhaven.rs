//! wallhaven.rs —— Wallhaven 公开 API 搜索（SFW，无需 API Key）。
//!
//! GET https://wallhaven.cc/api/v1/search?q=&page=&purity=100&categories=100&sorting=&atleast=
//! 返回缩略图列表供前端展示，选中后复用 download::import_from_url 走原图直链入库。

use crate::models::CmdResult;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const API_BASE: &str = "https://wallhaven.cc/api/v1/search";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhThumb {
    pub id: String,
    /// 原图直链（导入用）
    pub path: String,
    /// 缩略图（列表展示用）
    pub thumb: String,
    pub resolution: String,
    pub purity: String,
    pub colors: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WhResponse {
    data: Vec<WhData>,
}

#[derive(Debug, Deserialize)]
struct WhData {
    id: String,
    path: String,
    resolution: String,
    purity: String,
    colors: Vec<String>,
    thumbs: WhThumbs,
}

#[derive(Debug, Deserialize)]
struct WhThumbs {
    small: String,
}

/// 搜索 Wallhaven（SFW）。query 可为空（配合 toplist 浏览热门）。
pub async fn search(
    query: String,
    page: u32,
    sorting: String,
    atleast: String,
) -> CmdResult<Vec<WhThumb>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let empty_query = query.trim().is_empty();
    let sorting = if empty_query && sorting == "relevance" {
        // 空关键词 relevance 无意义，退回热门
        "toplist".to_string()
    } else {
        sorting
    };

    let mut params: Vec<(String, String)> = vec![
        ("q".to_string(), query.trim().to_string()),
        ("page".to_string(), page.max(1).to_string()),
        ("purity".to_string(), "100".to_string()),
        ("categories".to_string(), "100".to_string()),
        ("sorting".to_string(), sorting),
    ];
    if !atleast.trim().is_empty() {
        params.push(("atleast".to_string(), atleast.trim().to_string()));
    }

    let resp = client
        .get(API_BASE)
        .query(&params)
        .send()
        .await
        .map_err(|e| format!("请求 Wallhaven 失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Wallhaven 返回 HTTP {}", resp.status().as_u16()));
    }
    let payload: WhResponse = resp
        .json()
        .await
        .map_err(|e| format!("解析 Wallhaven 响应失败: {e}"))?;

    Ok(payload
        .data
        .into_iter()
        .map(|d| WhThumb {
            id: d.id,
            path: d.path,
            thumb: d.thumbs.small,
            resolution: d.resolution,
            purity: d.purity,
            colors: d.colors,
        })
        .collect())
}
