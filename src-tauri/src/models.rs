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
    /// 可空分类；「自动」模式下由 tags + palette 承担归类
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 主色调色板（hex，最多 5 色，按占比排序），用于按颜色过滤
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette: Option<Vec<String>>,
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
    /// 智能打标用的轻量文本模型（chat/completions）；空 = 仅关键词规则
    #[serde(default)]
    pub classify_model: String,
    /// 定时轮换壁纸：来源集合 id；空 = 关闭
    #[serde(default)]
    pub auto_switch_collection_id: Option<String>,
    /// 轮换间隔（分钟，最小 1）
    #[serde(default = "default_switch_interval")]
    pub auto_switch_interval_min: u32,
    /// 轮换应用范围："primary" 主屏 | "all" 全部显示器
    #[serde(default = "default_switch_scope")]
    pub auto_switch_scope: String,
    /// 轮换模式："interval" 按间隔轮换 | "daynight" 日/夜分时
    #[serde(default = "default_switch_mode")]
    pub auto_switch_mode: String,
    /// 日/夜模式：白天来源集合
    #[serde(default)]
    pub day_collection_id: Option<String>,
    /// 日/夜模式：夜间来源集合
    #[serde(default)]
    pub night_collection_id: Option<String>,
    /// 白天开始时间 "HH:MM"
    #[serde(default = "default_day_start")]
    pub day_start: String,
    /// 夜晚开始时间 "HH:MM"
    #[serde(default = "default_night_start")]
    pub night_start: String,
    /// 轮换时随机挑选（否则按集合顺序取下一张）
    #[serde(default)]
    pub auto_switch_random: bool,
    /// 全局快捷键：Ctrl+Alt+N 下一张 / Ctrl+Alt+P 暂停/恢复轮换
    #[serde(default)]
    pub global_shortcuts: bool,
    /// 自动备份间隔（天）：0 = 关闭
    #[serde(default)]
    pub auto_backup_days: u32,
}

fn default_locale() -> String {
    "zh-CN".into()
}

fn default_theme() -> String {
    "system".into()
}

fn default_switch_interval() -> u32 {
    30
}

fn default_switch_scope() -> String {
    "primary".into()
}

fn default_switch_mode() -> String {
    "interval".into()
}

fn default_day_start() -> String {
    "07:00".into()
}

fn default_night_start() -> String {
    "19:00".into()
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
            classify_model: String::new(),
            auto_switch_collection_id: None,
            auto_switch_interval_min: default_switch_interval(),
            auto_switch_scope: default_switch_scope(),
            auto_switch_mode: default_switch_mode(),
            day_collection_id: None,
            night_collection_id: None,
            day_start: default_day_start(),
            night_start: default_night_start(),
            auto_switch_random: false,
            global_shortcuts: false,
            auto_backup_days: 0,
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
