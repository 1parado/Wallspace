use std::fs;
use std::path::PathBuf;
use tauri::Manager;

/// 应用数据根目录：%APPDATA%/com.wallspace.app/data
pub fn data_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法定位应用数据目录: {e}"))?;
    let root = base.join("data");
    ensure_dirs(&root)?;
    Ok(root)
}

pub fn ensure_dirs(root: &PathBuf) -> Result<(), String> {
    fs::create_dir_all(root.join("wallpapers")).map_err(|e| format!("创建目录失败: {e}"))?;
    fs::create_dir_all(root.join("adapted")).map_err(|e| format!("创建目录失败: {e}"))?;
    Ok(())
}

pub fn library_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(data_root(app)?.join("library.json"))
}

pub fn settings_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(data_root(app)?.join("settings.json"))
}
