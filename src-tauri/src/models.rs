use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperItem {
    pub id: String,
    pub title: String,
    pub source: String, // "ai" | "url" | "local"
    pub file_path: String,
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
    pub category: String,
    pub tags: Vec<String>,
    pub favorite: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_url: Option<String>,
    pub created_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub id: String, // IDesktopWallpaper 设备路径
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub api_base_url: String,
    pub api_key: String,
    pub api_model: String,
    pub fill_mode: String,    // "fill" (裁剪) | "fit" (留边)
    pub default_size: String, // 生图默认尺寸
    #[serde(default = "default_locale")]
    pub locale: String, // "zh-CN" | "en"
    #[serde(default = "default_theme")]
    pub theme: String, // "system" | "dark" | "light"
    #[serde(default)]
    pub sidebar_hidden: bool,
}

fn default_locale() -> String {
    "zh-CN".into()
}

fn default_theme() -> String {
    "system".into()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_base_url: "https://api.openai.com/v1".into(),
            api_key: String::new(),
            api_model: "gpt-image-1".into(),
            fill_mode: "fill".into(),
            default_size: "1536x1024".into(),
            locale: default_locale(),
            theme: default_theme(),
            sidebar_hidden: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub imported: Vec<WallpaperItem>,
    pub failed: Vec<FailedImport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedImport {
    pub path: String,
    pub reason: String,
}

pub type CmdResult<T> = Result<T, String>;

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
