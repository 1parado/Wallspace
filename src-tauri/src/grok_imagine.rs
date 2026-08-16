//! grok_imagine.rs —— 基于 grok.com 网页端 /imagine WebSocket 的生图引擎。
//!
//! 与 grok_switch（Go）的 ImagineEngine 保持一致的实现：
//! - 账号池来自 ~/.grok_switch/registrar/cookies/*.json（仅保留 grok.com/x.ai 域的
//!   sso / sso-rw / cf_clearance / __cf_bm / __cuid / xai_anon_id）
//! - 每次生图前 GET https://grok.com/imagine 刷新 Cloudflare/SSO Cookie
//! - 经 HTTP 代理（IMAGINE_PROXY > HTTPS_PROXY > HTTP_PROXY > 127.0.0.1:7897）
//!   建立 wss://grok.com/ws/imagine/listen，先发 update_session，350ms 后发 input_text
//! - 账号轮询；usage_pool_exhausted / usage_limit_reached / concurrency_limit 标记耗尽

use crate::library::{self, ExtraMeta};
use crate::models::{now_ms, CmdResult, WallpaperItem};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{client_async_tls_with_config, Connector, MaybeTlsStream, WebSocketStream};

const WS_URL: &str = "wss://grok.com/ws/imagine/listen";
const REFRESH_URL: &str = "https://grok.com/imagine";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

/// 保留的 Cookie 字段（与 grok_switch 一致）。
const KEEP_COOKIES: [&str; 6] = [
    "sso",
    "sso-rw",
    "cf_clearance",
    "__cf_bm",
    "__cuid",
    "xai_anon_id",
];

/// 额度类错误码：标记账号耗尽。
const EXHAUST_CODES: [&str; 3] = [
    "usage_pool_exhausted",
    "usage_limit_reached",
    "concurrency_limit",
];

#[derive(Debug, Deserialize)]
struct CookieFileEntry {
    name: String,
    value: String,
    domain: String,
}

#[derive(Debug, Deserialize)]
struct CookieFile {
    cookies: Vec<CookieFileEntry>,
}

struct GrokAccount {
    id: String,
    cookies: HashMap<String, String>,
    exhausted: bool,
}

impl GrokAccount {
    fn from_file(path: &PathBuf, id: String) -> Option<Self> {
        let raw = std::fs::read_to_string(path).ok()?;
        let cf: CookieFile = serde_json::from_str(&raw).ok()?;
        let mut cookies = HashMap::new();
        for c in cf.cookies {
            if !KEEP_COOKIES.contains(&c.name.as_str()) {
                continue;
            }
            let dom = c.domain.to_lowercase();
            if dom.contains("grok.com") || dom.contains("x.ai") {
                cookies.insert(c.name, c.value);
            }
        }
        if cookies.is_empty() {
            return None;
        }
        Some(Self {
            id,
            cookies,
            exhausted: false,
        })
    }

    fn cookie_header(&self) -> String {
        self.cookies
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

struct GrokEngine {
    accounts: Vec<GrokAccount>,
    index: usize,
    proxy_url: String,
}

impl GrokEngine {
    fn load(app: &AppHandle) -> Self {
        let proxy_url = resolve_imagine_proxy();
        let home = app.path().home_dir().unwrap_or_else(|_| PathBuf::from("."));
        let cookie_dir = home.join(".grok_switch").join("registrar").join("cookies");
        let mut accounts = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&cookie_dir) {
            for ent in entries.flatten() {
                let name = ent.file_name().to_string_lossy().into_owned();
                if ent.path().is_dir() || !name.ends_with(".json") {
                    continue;
                }
                let id = name.trim_end_matches(".json").to_string();
                if let Some(acc) = GrokAccount::from_file(&ent.path(), id) {
                    accounts.push(acc);
                }
            }
        }
        Self {
            accounts,
            index: 0,
            proxy_url,
        }
    }

    fn status(&self) -> (usize, usize) {
        let total = self.accounts.len();
        let available = self.accounts.iter().filter(|a| !a.exhausted).count();
        (total, available)
    }

    /// 轮询取一个未耗尽账号的下标。
    fn next(&mut self) -> Option<usize> {
        if self.accounts.is_empty() {
            return None;
        }
        let start = self.index;
        loop {
            let i = self.index;
            self.index = (self.index + 1) % self.accounts.len();
            if !self.accounts[i].exhausted {
                return Some(i);
            }
            if self.index == start {
                break;
            }
        }
        None
    }
}

/// 代理优先级：IMAGINE_PROXY > HTTPS_PROXY > HTTP_PROXY > 默认 Clash 127.0.0.1:7897。
fn resolve_imagine_proxy() -> String {
    for key in ["IMAGINE_PROXY", "HTTPS_PROXY", "HTTP_PROXY"] {
        if let Ok(v) = std::env::var(key) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return v;
            }
        }
    }
    "http://127.0.0.1:7897/".to_string()
}

static ENGINE: OnceLock<Mutex<GrokEngine>> = OnceLock::new();

fn engine(app: &AppHandle) -> &Mutex<GrokEngine> {
    ENGINE.get_or_init(|| Mutex::new(GrokEngine::load(app)))
}

fn rustls_config() -> Arc<rustls::ClientConfig> {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let mut cfg = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    cfg.alpn_protocols = vec![b"http/1.1".to_vec()];
    Arc::new(cfg)
}

fn parse_proxy_host_port(proxy: &str) -> Option<String> {
    let rest = proxy
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("socks5://");
    let hostport = rest.split('/').next()?.trim();
    if hostport.is_empty() {
        return None;
    }
    if hostport.contains(':') {
        Some(hostport.to_string())
    } else {
        Some(format!("{hostport}:80"))
    }
}

/// 通过 HTTP 代理 CONNECT 隧道连到 grok.com:443，再完成 wss 握手。
async fn dial_ws(
    proxy_url: &str,
    cookie_header: &str,
    request_id: &str,
) -> CmdResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    let hostport = parse_proxy_host_port(proxy_url)
        .ok_or_else(|| format!("生图代理地址无效: {proxy_url}"))?;
    let mut tcp = TcpStream::connect(hostport.as_str())
        .await
        .map_err(|e| format!("连接代理失败（{hostport}）: {e}"))?;

    tcp.write_all(b"CONNECT grok.com:443 HTTP/1.1\r\nHost: grok.com:443\r\n\r\n")
        .await
        .map_err(|e| format!("代理请求失败: {e}"))?;
    let mut acc: Vec<u8> = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        let n = tcp
            .read(&mut buf)
            .await
            .map_err(|e| format!("读取代理响应失败: {e}"))?;
        if n == 0 {
            return Err("代理连接意外中断".into());
        }
        acc.extend_from_slice(&buf[..n]);
        if acc.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if acc.len() > 8192 {
            return Err("代理响应过大".into());
        }
    }
    let head = String::from_utf8_lossy(&acc);
    let status_line = head.lines().next().unwrap_or_default().to_string();
    if !status_line.contains(" 200 ") {
        return Err(format!("代理 CONNECT 失败: {status_line}"));
    }

    let mut request = WS_URL
        .into_client_request()
        .map_err(|e| format!("构造 WS 请求失败: {e}"))?;
    {
        use tokio_tungstenite::tungstenite::http::HeaderValue;
        let h = request.headers_mut();
        h.insert("Cookie", HeaderValue::from_str(cookie_header).unwrap());
        h.insert("Origin", HeaderValue::from_static("https://grok.com"));
        h.insert("User-Agent", HeaderValue::from_static(USER_AGENT));
        h.insert("x-xai-request-id", HeaderValue::from_str(request_id).unwrap());
        h.insert("Accept-Language", HeaderValue::from_static("en-US,en;q=0.9"));
    }
    let config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
        max_message_size: Some(16 << 20),
        max_frame_size: Some(16 << 20),
        ..Default::default()
    };
    let (ws, _resp) = client_async_tls_with_config(
        request,
        tcp,
        Some(config),
        Some(Connector::Rustls(rustls_config())),
    )
    .await
    .map_err(|e| format!("WebSocket 握手失败: {e}"))?;
    Ok(ws)
}

/// 请求属性（与 grok.com 网页端一致）。
fn imagine_props(model: &str, aspect_ratio: &str, is_initial: bool) -> Value {
    let quality = model.contains("quality");
    json!({
        "is_initial": is_initial,
        "image_model_name": model,
        "enable_side_by_side": false,
        "enable_pro": quality,
        "resolution_name": "1k",
        "aspect_ratio": aspect_ratio,
        "num_generations": 1,
    })
}

fn envelope(content: Value) -> Value {
    json!({
        "type": "conversation.item.create",
        "timestamp": now_ms(),
        "item": { "type": "message", "content": [content] },
    })
}

#[derive(Clone)]
struct InnerResult {
    ok: bool,
    blobs: Vec<String>,
    err_code: String,
    err_msg: String,
}

impl Default for InnerResult {
    fn default() -> Self {
        Self {
            ok: false,
            blobs: Vec::new(),
            err_code: String::new(),
            err_msg: String::new(),
        }
    }
}

/// 单账号一次生图会话（与 Go wsGenerate 相同的时序与结算逻辑）。
async fn ws_generate(
    proxy_url: &str,
    cookie_header: &str,
    prompt: &str,
    model: &str,
    aspect_ratio: &str,
) -> InnerResult {
    let request_id = uuid::Uuid::new_v4().simple().to_string();
    let ws = match tokio::time::timeout(
        Duration::from_secs(60),
        dial_ws(proxy_url, cookie_header, &request_id),
    )
    .await
    {
        Ok(Ok(ws)) => ws,
        Ok(Err(e)) => {
            return InnerResult {
                err_code: "ws_dial".into(),
                err_msg: e,
                ..Default::default()
            };
        }
        Err(_) => {
            return InnerResult {
                err_code: "ws_dial".into(),
                err_msg: "连接超时".into(),
                ..Default::default()
            };
        }
    };
    let (mut write, mut read) = ws.split();

    // 1) 连接后立即发送 update_session
    let open = envelope(json!({
        "type": "update_session",
        "properties": imagine_props(model, aspect_ratio, false),
    }));
    if write.send(Message::Text(open.to_string())).await.is_err() {
        return InnerResult {
            err_code: "ws_write".into(),
            err_msg: "发送会话请求失败".into(),
            ..Default::default()
        };
    }
    // 2) 350ms 后发送 input_text
    tokio::time::sleep(Duration::from_millis(350)).await;
    let input = envelope(json!({
        "type": "input_text",
        "requestId": request_id,
        "text": prompt,
        "properties": imagine_props(model, aspect_ratio, true),
    }));
    if write.send(Message::Text(input.to_string())).await.is_err() {
        return InnerResult {
            err_code: "ws_write".into(),
            err_msg: "发送提示词失败".into(),
            ..Default::default()
        };
    }

    // 3) 读取结果流：image 消息收 blob，json 消息看状态/错误
    let mut result = InnerResult::default();
    let mut completed = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    // 连接关闭/出错时的结算：completed 且有 blob 则成功，否则按已有错误或 closed 上报
    let settle_on_close = |result: &InnerResult, completed: bool, why: &str| -> InnerResult {
        if completed && !result.blobs.is_empty() {
            InnerResult {
                ok: true,
                ..result.clone()
            }
        } else if !result.err_code.is_empty() {
            result.clone()
        } else {
            InnerResult {
                ok: false,
                blobs: result.blobs.clone(),
                err_code: if completed { "no_image".into() } else { "closed".into() },
                err_msg: why.to_string(),
            }
        }
    };
    loop {
        let msg = match tokio::time::timeout_at(deadline, read.next()).await {
            Ok(Some(Ok(m))) => m,
            Ok(Some(Err(_))) | Ok(None) => {
                return settle_on_close(&result, completed, "连接中断");
            }
            Err(_) => {
                return InnerResult {
                    err_code: "timeout".into(),
                    err_msg: "生图超时（90s）".into(),
                    ..Default::default()
                };
            }
        };
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => {
                return settle_on_close(&result, completed, "服务端关闭连接");
            }
            _ => continue,
        };
        let Ok(v) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        match v["type"].as_str().unwrap_or_default() {
            "session" => {}
            "error" => {
                result.err_code = v["err_code"].as_str().unwrap_or("").to_string();
                result.err_msg = v["err_msg"].as_str().unwrap_or("").to_string();
                if !result.err_code.is_empty() {
                    break;
                }
            }
            "image" => {
                if let Some(b) = v["blob"].as_str() {
                    if !b.is_empty() {
                        result.blobs.push(b.to_string());
                    }
                }
            }
            "json" => {
                if let Some(e) = v["err_code"].as_str() {
                    if !e.is_empty() {
                        result.err_code = e.to_string();
                        result.err_msg = v["err_msg"]
                            .as_str()
                            .or_else(|| v["err_message"].as_str())
                            .unwrap_or("")
                            .to_string();
                        break;
                    }
                }
                if v["current_status"].as_str() == Some("completed") {
                    completed = true;
                    if !result.blobs.is_empty() {
                        result.ok = true;
                        break;
                    }
                    // blob 尚未到达：继续读，直到连接关闭再结算
                }
            }
            _ => {}
        }
    }
    result
}

/// base64 blob 解码：去 data: 前缀、兼容 padding 缺失。
fn decode_blob(s: &str) -> Option<Vec<u8>> {
    let mut t = s.trim();
    if t.starts_with("data:") {
        if let Some(i) = t.find(',') {
            t = &t[i + 1..];
        }
    }
    let std = base64::engine::general_purpose::STANDARD;
    let nopad = base64::engine::general_purpose::STANDARD_NO_PAD;
    std.decode(t)
        .ok()
        .or_else(|| nopad.decode(t.trim_end_matches('=')).ok())
}

/// GET https://grok.com/imagine 刷新 Cloudflare/SSO Cookie（失败忽略，同 Go）。
async fn refresh_cookies(proxy_url: &str, account: &mut GrokAccount) {
    let Ok(proxy) = reqwest::Proxy::all(proxy_url) else {
        return;
    };
    let Ok(client) = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(15))
        .build()
    else {
        return;
    };
    let Ok(resp) = client
        .get(REFRESH_URL)
        .header("Cookie", account.cookie_header())
        .header("Origin", "https://grok.com")
        .header("User-Agent", USER_AGENT)
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
    else {
        return;
    };
    for sc in resp.headers().get_all(reqwest::header::SET_COOKIE) {
        let Ok(line) = sc.to_str() else { continue };
        let Some((name, rest)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if !matches!(name, "cf_clearance" | "__cf_bm" | "sso" | "sso-rw") {
            continue;
        }
        let value = rest.split(';').next().unwrap_or("").trim();
        if !value.is_empty() {
            account.cookies.insert(name.to_string(), value.to_string());
        }
    }
}

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

/// 对外入口：账号轮询生成一张图并入库。
pub async fn generate(
    app: &AppHandle,
    prompt: String,
    model: String,
    aspect_ratio: String,
    category: String,
) -> CmdResult<WallpaperItem> {
    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        return Err("提示词不能为空".into());
    }
    let mut eng = engine(app).lock().await;
    if eng.accounts.is_empty() {
        return Err(
            "未找到任何 Grok 账号（~/.grok_switch/registrar/cookies 为空或无效）".into(),
        );
    }
    let attempts = eng.accounts.len();
    let mut last_err = String::new();
    for _ in 0..attempts {
        let Some(idx) = eng.next() else {
            return Err("所有 Grok 账号均已耗尽额度".into());
        };
        let proxy_url = eng.proxy_url.clone();
        let cookie_header = {
            refresh_cookies(&proxy_url, &mut eng.accounts[idx]).await;
            eng.accounts[idx].cookie_header()
        };
        let inner = ws_generate(&proxy_url, &cookie_header, &prompt, &model, &aspect_ratio).await;
        if inner.ok && !inner.blobs.is_empty() {
            // 保留体积最大的一张（过滤缩略图）
            let mut best: Option<Vec<u8>> = None;
            for b in &inner.blobs {
                if let Some(buf) = decode_blob(b) {
                    if best.as_ref().is_none_or(|cur| buf.len() > cur.len()) {
                        best = Some(buf);
                    }
                }
            }
            match best {
                Some(bytes) => {
                    return library::add_image_bytes(
                        app,
                        bytes,
                        "ai",
                        short_title(&prompt),
                        &category,
                        ExtraMeta {
                            prompt: Some(prompt),
                            model: Some(model),
                            origin_url: None,
                        },
                    );
                }
                None => {
                    last_err = "未收到图片数据".into();
                    continue;
                }
            }
        }
        // 失败：额度类错误标记耗尽，轮询下一个账号
        let acc_id = eng.accounts[idx].id.clone();
        last_err = if inner.err_msg.is_empty() {
            format!("[{}] {}", acc_id, inner.err_code)
        } else {
            format!("[{}] {}: {}", acc_id, inner.err_code, inner.err_msg)
        };
        if EXHAUST_CODES.contains(&inner.err_code.as_str()) {
            eng.accounts[idx].exhausted = true;
        }
    }
    Err(format!("Grok 生图失败：{last_err}"))
}

/// 账号池状态：(总数, 可用)。
pub async fn status(app: &AppHandle) -> (usize, usize) {
    engine(app).lock().await.status()
}
