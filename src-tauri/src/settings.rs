use crate::models::{CmdResult, Settings};
use crate::store;
use serde_json::Value;
use std::time::Duration;
use tauri::AppHandle;

const SETTINGS_KEY: &str = "settings";

pub fn load(app: &AppHandle) -> Settings {
    store::kv_get(app, SETTINGS_KEY)
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, s: &Settings) -> CmdResult<()> {
    let text = serde_json::to_string_pretty(s).map_err(|e| format!("序列化失败: {e}"))?;
    store::kv_set(app, SETTINGS_KEY, &text)
}

/// 规整 Base URL：去尾部斜杠；若未以 /v1 结尾则自动补上（兼容用户填写裸域名）。
pub fn normalize_base(base: &str) -> String {
    let b = base.trim().trim_end_matches('/');
    if b.ends_with("/v1") {
        b.to_string()
    } else {
        format!("{b}/v1")
    }
}

/// 用 GET /models 验证连通性，返回前若干模型 id 供参考。
pub async fn test_connection(base_url: String, api_key: String) -> CmdResult<Vec<String>> {
    let url = format!("{}/models", normalize_base(&base_url));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let resp = client
        .get(&url)
        .bearer_auth(api_key.trim())
        .send()
        .await
        .map_err(|e| format!("连接失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("连接失败: HTTP {}", resp.status().as_u16()));
    }
    let payload: Value = resp.json().await.map_err(|e| format!("解析响应失败: {e}"))?;
    let ids: Vec<String> = payload["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m["id"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    if ids.is_empty() {
        Err("连接成功，但未返回模型列表".into())
    } else {
        Ok(ids)
    }
}
