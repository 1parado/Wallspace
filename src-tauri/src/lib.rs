mod auto_classify;
mod autoswitch;
mod backup;
mod collections;
mod download;
mod duplicates;
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
mod watch;

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
    let title = item.title.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        wallpaper::adapt_and_apply(&app2, &item, display.as_deref())
    })
    .await
    .map_err(|e| format!("任务执行失败: {e}"))??;
    // 更新托盘上的当前壁纸标题
    crate::tray::sync_current_title(&app, &title);
    Ok(())
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
    settings::save(&app, &settings)?;
    // 保存后同步全局快捷键注册状态
    apply_global_shortcuts(&app, settings.global_shortcuts);
    Ok(())
}

/// 全局快捷键注册开关（幂等）：Ctrl+Alt+N 下一张，Ctrl+Alt+P 暂停/恢复轮换
fn apply_global_shortcuts(app: &AppHandle, enable: bool) {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let gs = app.global_shortcut();
    let _ = gs.unregister("ctrl+alt+n");
    let _ = gs.unregister("ctrl+alt+p");
    if enable {
        if let Err(e) = gs.register("ctrl+alt+n") {
            eprintln!("注册 Ctrl+Alt+N 失败: {e}");
        }
        if let Err(e) = gs.register("ctrl+alt+p") {
            eprintln!("注册 Ctrl+Alt+P 失败: {e}");
        }
    }
}

#[tauri::command]
async fn test_connection(base_url: String, api_key: String) -> CmdResult<Vec<String>> {
    settings::test_connection(base_url, api_key).await
}

/// 导出库备份（清单 + 图片 zip）到指定路径，返回导出条目数。
#[tauri::command]
async fn export_backup(app: AppHandle, save_path: String) -> CmdResult<u32> {
    tauri::async_runtime::spawn_blocking(move || {
        backup::export(&app, std::path::Path::new(&save_path))
    })
    .await
    .map_err(|e| format!("任务执行失败: {e}"))?
}

/// 从备份恢复（按 id 合并，已存在跳过）。
#[tauri::command]
async fn import_backup(app: AppHandle, path: String) -> CmdResult<backup::ImportOutcome> {
    tauri::async_runtime::spawn_blocking(move || {
        backup::import(&app, std::path::Path::new(&path))
    })
    .await
    .map_err(|e| format!("任务执行失败: {e}"))?
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateInfo {
    current: String,
    latest: String,
    has_update: bool,
    url: String,
}

/// 检查更新：查询 GitHub Releases 最新版本并与当前版本比较。
#[tauri::command]
async fn check_updates(app: AppHandle) -> CmdResult<UpdateInfo> {
    let current = app.package_info().version.to_string();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let resp = client
        .get("https://api.github.com/repos/1parado/Wallspace/releases/latest")
        .header("User-Agent", "wallspace-app")
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("GitHub API 返回 HTTP {}", resp.status().as_u16()));
    }
    let payload: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {e}"))?;
    let latest = payload["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    let url = payload["html_url"]
        .as_str()
        .unwrap_or("https://github.com/1parado/Wallspace/releases")
        .to_string();
    if latest.is_empty() {
        return Err("未获取到最新版本号".into());
    }
    let has_update = version_gt(&latest, &current);
    Ok(UpdateInfo {
        current,
        latest,
        has_update,
        url,
    })
}

/// 三段式版本号比较：a > b 则 true
fn version_gt(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.').map(|p| p.trim().parse().unwrap_or(0)).collect()
    };
    let (va, vb) = (parse(a), parse(b));
    for i in 0..3 {
        let x = va.get(i).copied().unwrap_or(0);
        let y = vb.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    false
}

/// 单条文本 LLM 打标（「一键智能整理」的规则未命中回退）。
/// 未配置模型/key 或调用失败都返回 None，由前端保留关键词规则结果。
#[tauri::command]
async fn classify_text(
    app: AppHandle,
    text: String,
) -> CmdResult<Option<auto_classify::LlmClassifyDto>> {
    let cfg = settings::load(&app);
    if cfg.classify_model.trim().is_empty() || cfg.api_key.trim().is_empty() {
        return Ok(None);
    }
    Ok(auto_classify::classify(
        &cfg.api_base_url,
        &cfg.api_key,
        &cfg.classify_model,
        &text,
    )
    .await
    .ok()
    .map(Into::into))
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
    color: Option<String>,
) -> CmdResult<Vec<wallhaven::WhThumb>> {
    wallhaven::search(
        query,
        page.unwrap_or(1),
        sorting.unwrap_or_else(|| "relevance".into()),
        atleast.unwrap_or_default(),
        color,
    )
    .await
}

#[tauri::command]
fn reveal_item(path: String) -> CmdResult<()> {
    tauri_plugin_opener::reveal_item_in_dir(std::path::Path::new(&path))
        .map_err(|e| format!("打开资源管理器失败: {e}"))
}

/// 提取图片主色调（调色板）：缩略图采样 → 4bit/通道分桶 → 按占比去重取最多 6 色。
#[tauri::command]
async fn extract_palette(path: String) -> CmdResult<Vec<String>> {
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<String>, String> {
        let img = image::open(&path).map_err(|e| format!("读取图片失败: {e}"))?;
        let thumb = img.thumbnail(64, 64);
        // 分桶统计：key = 4bit/通道的 RGB，记录计数与各通道累加值
        let mut buckets: std::collections::HashMap<u16, (u32, u64, u64, u64)> =
            std::collections::HashMap::new();
        for px in thumb.to_rgba8().pixels() {
            if px[3] < 128 {
                continue; // 跳过透明像素
            }
            let (r, g, b) = (px[0] as u64, px[1] as u64, px[2] as u64);
            let key = (((r >> 4) as u16) << 8) | (((g >> 4) as u16) << 4) | ((b >> 4) as u16);
            let e = buckets.entry(key).or_insert((0, 0, 0, 0));
            e.0 += 1;
            e.1 += r;
            e.2 += g;
            e.3 += b;
        }
        // 按像素占比排序，依次挑取与已选色距离足够远的颜色
        let mut cands: Vec<(u32, [f32; 3])> = buckets
            .into_iter()
            .map(|(_k, e)| {
                let n = e.0 as f32;
                (
                    e.0,
                    [
                        (e.1 as f32 / n).round(),
                        (e.2 as f32 / n).round(),
                        (e.3 as f32 / n).round(),
                    ],
                )
            })
            .collect();
        cands.sort_by(|a, b| b.0.cmp(&a.0));
        let mut picked: Vec<[f32; 3]> = Vec::new();
        for (_, c) in cands {
            let far = picked.iter().all(|p| {
                let d = (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) + (p[2] - c[2]).powi(2);
                d > 48.0 * 48.0
            });
            if far {
                picked.push(c);
                if picked.len() >= 6 {
                    break;
                }
            }
        }
        Ok(picked
            .iter()
            .map(|c| format!("#{:02X}{:02X}{:02X}", c[0] as u8, c[1] as u8, c[2] as u8))
            .collect())
    })
    .await
    .map_err(|e| format!("调色板任务失败: {e}"))?
}

/// 检测库内重复图片（文件大小分桶 + 内容哈希），后台线程执行。
#[tauri::command]
async fn find_duplicates(app: AppHandle) -> CmdResult<Vec<duplicates::DupGroup>> {
    duplicates::find(app).await
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
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    // 仅响应按下瞬间
                    if event.state() != tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        return;
                    }
                    let key = shortcut.into_string().to_lowercase();
                    let app = app.clone();
                    match key.as_str() {
                        "ctrl+alt+n" => std::thread::spawn(move || {
                            autoswitch::force_next(&app);
                        }),
                        "ctrl+alt+p" => std::thread::spawn(move || {
                            let paused = autoswitch::pause_toggle();
                            tray::sync_pause_label(&app, paused);
                        }),
                        _ => return,
                    };
                })
                .build(),
        )
        .setup(|app| {
            store::init(app.handle())?;
            autoswitch::spawn(app.handle().clone());
            tray::init(app)?;
            let cfg = settings::load(app.handle());
            apply_global_shortcuts(app.handle(), cfg.global_shortcuts);
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
            classify_text,
            export_backup,
            import_backup,
            check_updates,
            grok_imagine,
            grok_imagine_status,
            list_collections,
            save_collections,
            create_collection,
            reveal_item,
            extract_palette,
            find_duplicates,
            export_wallpaper,
            wallhaven_search
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
