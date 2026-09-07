use crate::library::{self, ExtraMeta};
use crate::models::{CmdResult, WallpaperItem};
use std::time::Duration;
use tauri::AppHandle;

const MAX_BYTES: usize = 60 * 1024 * 1024;

fn title_from_url(url: &str) -> String {
    let name = url
        .rsplit(['/', '?', '#'])
        .find(|s| !s.is_empty())
        .unwrap_or("Downloaded");
    let stem = name.split('.').next().unwrap_or(name);
    let stem = stem.replace(['-', '_'], " ").trim().to_string();
    if stem.is_empty() {
        "Downloaded".into()
    } else {
        stem
    }
}

pub async fn import_from_url(
    app: AppHandle,
    url: String,
    category: Option<String>,
    tags: Vec<String>,
) -> CmdResult<WallpaperItem> {
    let url = url.trim().to_string();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("仅支持 http/https 链接".into());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("下载失败: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("下载失败: HTTP {}", resp.status().as_u16()));
    }

    if let Some(len) = resp.content_length() {
        if len as usize > MAX_BYTES {
            return Err("文件过大（上限 60MB）".into());
        }
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    if !content_type.is_empty() && !content_type.starts_with("image/") {
        return Err(format!("链接不是图片（Content-Type: {content_type}）"));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("下载失败: {e}"))?;
    if bytes.len() > MAX_BYTES {
        return Err("文件过大（上限 60MB）".into());
    }

    let title = title_from_url(&url);
    library::add_image_bytes(
        &app,
        bytes.to_vec(),
        "url",
        title,
        category,
        ExtraMeta {
            prompt: None,
            model: None,
            origin_url: Some(url),
            tags,
        },
    )
}
