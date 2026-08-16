mod download;
mod generate;
mod grok_imagine;
mod library;
mod models;
mod paths;
mod settings;
mod wallpaper;

use models::{CmdResult, ImportReport, MonitorInfo, Settings, WallpaperItem};
use serde::Serialize;
use tauri::AppHandle;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GrokImagineStatus {
    total: usize,
    available: usize,
}

#[tauri::command]
fn get_library(app: AppHandle) -> CmdResult<Vec<WallpaperItem>> {
    Ok(library::load(&app))
}

#[tauri::command]
fn import_local_files(
    app: AppHandle,
    paths: Vec<String>,
    category: String,
) -> CmdResult<ImportReport> {
    library::import_local_files(&app, paths, &category)
}

#[tauri::command]
async fn import_from_url(
    app: AppHandle,
    url: String,
    category: String,
) -> CmdResult<WallpaperItem> {
    download::import_from_url(app, url, category).await
}

#[tauri::command]
async fn generate_wallpaper(
    app: AppHandle,
    prompt: String,
    size: String,
    category: String,
) -> CmdResult<WallpaperItem> {
    generate::generate(app, prompt, size, category).await
}

#[tauri::command]
fn update_item(app: AppHandle, item: WallpaperItem) -> CmdResult<WallpaperItem> {
    library::update_item(&app, item)
}

#[tauri::command]
fn delete_item(app: AppHandle, id: String) -> CmdResult<()> {
    library::delete_item(&app, &id)
}

#[tauri::command]
async fn apply_wallpaper(app: AppHandle, id: String, display: Option<String>) -> CmdResult<()> {
    let item = library::load(&app)
        .into_iter()
        .find(|i| i.id == id)
        .ok_or_else(|| "条目不存在".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        wallpaper::adapt_and_apply(&app, &item, display.as_deref())
    })
    .await
    .map_err(|e| format!("任务执行失败: {e}"))?
}

#[tauri::command]
fn list_monitors() -> CmdResult<Vec<MonitorInfo>> {
    wallpaper::list_monitors_impl()
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    settings::load(&app)
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings) -> CmdResult<()> {
    settings::save(&app, &settings)
}

#[tauri::command]
async fn test_connection(base_url: String, api_key: String) -> CmdResult<Vec<String>> {
    settings::test_connection(base_url, api_key).await
}

/// Grok（grok.com 账号池）直连生图，实现与 grok_switch ImagineEngine 一致。
#[tauri::command]
async fn grok_imagine(
    app: AppHandle,
    prompt: String,
    model: String,
    aspect_ratio: String,
    category: String,
) -> CmdResult<WallpaperItem> {
    grok_imagine::generate(&app, prompt, model, aspect_ratio, category).await
}

#[tauri::command]
async fn grok_imagine_status(app: AppHandle) -> CmdResult<GrokImagineStatus> {
    let (total, available) = grok_imagine::status(&app).await;
    Ok(GrokImagineStatus { total, available })
}

#[tauri::command]
fn reveal_item(path: String) -> CmdResult<()> {
    tauri_plugin_opener::reveal_item_in_dir(std::path::Path::new(&path))
        .map_err(|e| format!("打开资源管理器失败: {e}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    wallpaper::set_dpi_awareness();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_library,
            import_local_files,
            import_from_url,
            generate_wallpaper,
            update_item,
            delete_item,
            apply_wallpaper,
            list_monitors,
            get_settings,
            save_settings,
            test_connection,
            grok_imagine,
            grok_imagine_status,
            reveal_item
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
