use crate::auto_classify;
use crate::library::{self, ExtraMeta};
use crate::models::{CmdResult, WallpaperItem};
use crate::settings::normalize_base;
use crate::settings;
use base64::Engine;
use serde_json::{json, Value};
use std::time::Duration;
use tauri::AppHandle;

fn short_title(prompt: &str) -> String {
    let t = prompt.trim();
    let cut: String = t.chars().take(32).collect();
    if t.chars().count() > 32 {
        format!("{cut}…")
    } else if cut.is_empty() {
        "Untitled Scene".into()
    } else {
        cut
    }
}

pub async fn generate(
    app: AppHandle,
    prompt: String,
    size: String,
    category: Option<String>,
    tags: Vec<String>,
) -> CmdResult<WallpaperItem> {
    let cfg = settings::load(&app);
    if cfg.api_key.trim().is_empty() {
        return Err("尚未配置 API Key，请先在设置中填写".into());
    }
    let base = normalize_base(&cfg.api_base_url);
    let url = format!("{base}/images/generations");
    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        return Err("提示词不能为空".into());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let body = json!({
        "model": cfg.api_model,
        "prompt": prompt,
        "n": 1,
        "size": size,
        "response_format": "b64_json",
    });

    let mut resp = client
        .post(&url)
        .bearer_auth(&cfg.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;

    // 某些严格实现（如 OpenAI gpt-image-1）不接受 response_format，去掉后重试一次
    if resp.status().as_u16() == 400 {
        let text = resp.text().await.unwrap_or_default();
        if text.contains("response_format") {
            let retry_body = json!({
                "model": cfg.api_model,
                "prompt": prompt,
                "n": 1,
                "size": size,
            });
            resp = client
                .post(&url)
                .bearer_auth(&cfg.api_key)
                .json(&retry_body)
                .send()
                .await
                .map_err(|e| format!("请求失败: {e}"))?;
        } else {
            return Err(format!("生图服务返回错误: {text}"));
        }
    }

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        let brief = text.chars().take(400).collect::<String>();
        return Err(format!("生图服务返回 {status}: {brief}"));
    }

    let payload: Value = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {e}"))?;

    let first = payload
        .pointer("/data/0")
        .ok_or("响应中没有图片数据")?;

    let bytes: Vec<u8> = if let Some(b64) = first.get("b64_json").and_then(|v| v.as_str()) {
        base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| format!("解码图片失败: {e}"))?
    } else if let Some(u) = first.get("url").and_then(|v| v.as_str()) {
        client
            .get(u)
            .timeout(Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| format!("下载生成结果失败: {e}"))?
            .bytes()
            .await
            .map_err(|e| format!("下载生成结果失败: {e}"))?
            .to_vec()
    } else {
        return Err("响应中既无 b64_json 也无 url".into());
    };

    let title = short_title(&prompt);
    // 智能打标：配置了轻量文本模型时，让 LLM 给出分类（可含子分类）与标签；
    // 失败静默回退到关键词规则结果
    let mut merged_tags = tags;
    let mut final_category = category;
    if !cfg.classify_model.trim().is_empty() {
        if let Ok(llm) =
            auto_classify::classify(&cfg.api_base_url, &cfg.api_key, &cfg.classify_model, &prompt)
                .await
        {
            if final_category.is_none() && llm.category.is_some() {
                final_category = llm.category;
            }
            for t in llm.tags {
                if !merged_tags.contains(&t) {
                    merged_tags.push(t);
                }
            }
            merged_tags.truncate(8);
        }
    }
    library::add_image_bytes(
        &app,
        bytes,
        "ai",
        title,
        final_category,
        ExtraMeta {
            prompt: Some(prompt),
            model: Some(cfg.api_model.clone()),
            origin_url: None,
            tags: merged_tags,
        },
    )
}
