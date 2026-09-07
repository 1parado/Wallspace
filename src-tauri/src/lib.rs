mod auto_classify;
mod autoswitch;
mod collections;
mod download;
mod export;
mod generate;
mod grok_imagine;
mod library;
mod models;
mod paths;
mod settings;
mod store;
mod tray;
mod wallpaper;
mod wallhaven;

use collections::Collection;
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
    category: Option<String>,
    tags: Option<Vec<String>>,
) -> CmdResult<ImportReport> {
    library::import_local_files(&app, paths, category, tags.unwrap_or_default())
}

#[tauri::command]
async fn import_from_url(
    app: AppHandle,
    url: String,
    category: Option<String>,
    tags: Option<Vec<String>>,
) -> CmdResult<WallpaperItem> {
    download::import_from_url(app, url, category, tags.unwrap_or_default()).await
}

#[tauri::command]
async fn generate_wallpaper(
    app: AppHandle,
    prompt: String,
    size: String,
    category: Option<String>,
    tags: Option<Vec<String>>,
) -> CmdResult<WallpaperItem> {
    generate::generate(app, prompt, size, category, tags.unwrap_or_default()).await
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
    category: Option<String>,
    tags: Option<Vec<String>>,
) -> CmdResult<WallpaperItem> {
    grok_imagine::generate(&app, prompt, model, aspect_ratio, category, tags.unwrap_or_default()).await
}

#[tauri::command]
async fn grok_imagine_status(app: AppHandle) -> CmdResult<GrokImagineStatus> {
    let (total, available) = grok_imagine::status(&app).await;
    Ok(GrokImagineStatus { total, available })
}

#[tauri::command]
fn list_collections(app: AppHandle) -> CmdResult<Vec<Collection>> {
    Ok(collections::load(&app))
}

#[tauri::command]
fn save_collections(app: AppHandle, collections: Vec<Collection>) -> CmdResult<()> {
    collections::save(&app, &collections)
}

#[tauri::command]
fn create_collection(name: String) -> CmdResult<Collection> {
    Ok(collections::new_collection(name))
}

/// Wallhaven 搜索（SFW 公开接口）。
#[tauri::command]
async fn wallhaven_search(
    query: String,
    page: Option<u32>,
    sorting: Option<String>,
    atleast: Option<String>,
) -> CmdResult<Vec<wallhaven::WhThumb>> {
    wallhaven::search(
        query,
        page.unwrap_or(1),
        sorting.unwrap_or_else(|| "relevance".into()),
        atleast.unwrap_or_default(),
    )
    .await
}

#[tauri::command]
fn reveal_item(path: String) -> CmdResult<()> {
    tauri_plugin_opener::reveal_item_in_dir(std::path::Path::new(&path))
        .map_err(|e| format!("打开资源管理器失败: {e}"))
}

/// 按预设尺寸裁剪导出：加入媒体库或另存为指定路径。
#[tauri::command]
async fn export_wallpaper(
    app: AppHandle,
    id: String,
    width: u32,
    height: u32,
    mode: Option<String>,
    offset_x: Option<f32>,
    offset_y: Option<f32>,
    add_to_library: Option<bool>,
    save_path: Option<String>,
    title: Option<String>,
    format: Option<String>,
    brightness: Option<i32>,
    contrast: Option<f32>,
    saturation: Option<f32>,
    blur: Option<f32>,
) -> CmdResult<export::ExportResult> {
    let item = library::load(&app)
        .into_iter()
        .find(|i| i.id == id)
        .ok_or_else(|| "条目不存在".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let adjust = if brightness.unwrap_or(0) != 0
            || (contrast.unwrap_or(1.0) - 1.0).abs() > f32::EPSILON
            || (saturation.unwrap_or(1.0) - 1.0).abs() > f32::EPSILON
            || blur.unwrap_or(0.0) > f32::EPSILON
        {
            Some(export::Adjust {
                brightness: brightness.unwrap_or(0).clamp(-128, 128),
                contrast: contrast.unwrap_or(1.0).clamp(0.1, 3.0),
                saturation: saturation.unwrap_or(1.0).clamp(0.0, 3.0),
                blur: blur.unwrap_or(0.0).clamp(0.0, 8.0),
            })
        } else {
            None
        };
        export::export(
            &app,
            &item,
            width,
            height,
            mode.as_deref().unwrap_or("cover"),
            offset_x.unwrap_or(0.5),
            offset_y.unwrap_or(0.5),
            add_to_library.unwrap_or(false),
            save_path,
            title,
            format.as_deref().unwrap_or("jpg"),
            adjust,
        )
    })
    .await
    .map_err(|e| format!("任务执行失败: {e}"))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    wallpaper::set_dpi_awareness();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            store::init(app.handle())?;
            autoswitch::spawn(app.handle().clone());
            tray::init(app)?;
            Ok(())
        })
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
            list_collections,
            save_collections,
            create_collection,
            reveal_item,
            export_wallpaper,
            wallhaven_search
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
