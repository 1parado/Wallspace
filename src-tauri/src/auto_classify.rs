//! auto_classify.rs —— 智能自动分类（轻量模型）。
//!
//! 生图成功后调用用户配置的 OpenAI 兼容 chat/completions 接口，
//! 让轻量文本模型根据提示词输出分类（可含 '父/子' 层级）+ 标签 JSON。
//! 任何失败都静默回退：调用方保留关键词规则结果即可。

use serde_json::{json, Value};
use std::time::Duration;

/// LLM 打标结果
pub struct LlmClassify {
    pub category: Option<String>,
    pub tags: Vec<String>,
}

const SYSTEM_PROMPT: &str = r#"You label wallpaper-generation prompts for a wallpaper library.
Reply with ONLY a JSON object, no markdown, no extra text:
{"category": string or null, "tags": string[]}
- "category": the best-fit category path. Prefer one of:
  "Nature", "Space", "Abstract", "Cinematic", "Minimal".
  You may append ONE subcategory level after a slash, e.g. "Nature/Mountains",
  "Cinematic/Cyberpunk", "Minimal/LineArt". Use null if unsure.
- "tags": 2-6 lowercase English theme/style tags (e.g. dark, anime, vibrant,
  minimal, cyberpunk, retro, photography, nature, space, icon).
Reply with the JSON object alone."#;

/// 调 chat/completions 做打标；base 形如 https://host/v1。
pub async fn classify(
    base: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
) -> Result<LlmClassify, String> {
    if model.trim().is_empty() || api_key.trim().is_empty() {
        return Err("classify model/key 未配置".into());
    }
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let body = json!({
        "model": model.trim(),
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": format!("Prompt: {prompt}") }
        ],
        "temperature": 0,
        "max_tokens": 120,
    });

    let resp = client
        .post(&url)
        .bearer_auth(api_key.trim())
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status().as_u16()));
    }

    let payload: Value = resp.json().await.map_err(|e| format!("解析响应失败: {e}"))?;
    let content = payload["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim();
    if content.is_empty() {
        return Err("模型返回为空".into());
    }

    let v = extract_json(content).ok_or("未找到 JSON")?;
    let category = v["category"]
        .as_str()
        .map(|s| s.trim().trim_matches('/').to_string())
        .filter(|s| !s.is_empty());
    let mut tags: Vec<String> = Vec::new();
    if let Some(arr) = v["tags"].as_array() {
        for t in arr {
            if let Some(s) = t.as_str() {
                let s = s.trim().to_lowercase();
                if !s.is_empty() && s.len() <= 32 && !tags.contains(&s) {
                    tags.push(s);
                }
                if tags.len() >= 6 {
                    break;
                }
            }
        }
    }
    Ok(LlmClassify { category, tags })
}

/// 从模型输出中提取第一个平衡的 {...} JSON 对象（容错 markdown 代码围栏等）。
fn extract_json(text: &str) -> Option<Value> {
    let start = text.find('{')?;
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut in_str = false;
    let mut esc = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if esc {
            esc = false;
            continue;
        }
        match b {
            b'\\' if in_str => esc = true,
            b'"' => in_str = !in_str,
            b'{' if !in_str => depth += 1,
            b'}' if !in_str => {
                depth -= 1;
                if depth == 0 {
                    return serde_json::from_str(&text[start..=i]).ok();
                }
            }
            _ => {}
        }
    }
    None
}
