use crate::library;
use crate::models::{CmdResult, MonitorInfo, WallpaperItem};
use crate::paths;
use crate::settings;
use image::DynamicImage;
use std::fs;
use std::path::Path;
use tauri::AppHandle;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, DISPLAY_DEVICEW, HDC, HMONITOR,
    MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Shell::{DesktopWallpaper, IDesktopWallpaper, DWPOS_FILL};
use windows::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

fn widestr(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

struct RawMonitor {
    id: String,
    name: String,
    width: i32,
    height: i32,
    primary: bool,
}

unsafe extern "system" fn monitor_enum_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> windows::core::BOOL {
    let out = &mut *(lparam.0 as *mut Vec<RawMonitor>);
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    let ok = GetMonitorInfoW(
        hmonitor,
        &mut info as *mut MONITORINFOEXW as *mut MONITORINFO,
    )
    .as_bool();
    if !ok {
        return windows::core::BOOL(1);
    }
    let device = widestr(&info.szDevice);
    let (id, name) = match query_display_device(&device) {
        Some(v) => v,
        None => (device.clone(), device.clone()),
    };
    let rect = info.monitorInfo.rcMonitor;
    out.push(RawMonitor {
        id,
        name,
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
        primary: (info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY) != 0,
    });
    windows::core::BOOL(1)
}

/// 二层枚举：把 \\.\DISPLAYN 转成 IDesktopWallpaper 设备路径 + 显示器友好名。
fn query_display_device(device: &str) -> Option<(String, String)> {
    unsafe {
        let mut dd = DISPLAY_DEVICEW::default();
        dd.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
        let dev = HSTRING::from(device);
        if EnumDisplayDevicesW(PCWSTR(dev.as_ptr()), 0, &mut dd, 0).as_bool() {
            let id = widestr(&dd.DeviceID);
            let mut name = widestr(&dd.DeviceString);
            if id.is_empty() {
                return None;
            }
            if name.is_empty() {
                name = device.replace("\\\\.\\", "");
            }
            Some((id, name))
        } else {
            None
        }
    }
}

pub fn list_monitors_impl() -> CmdResult<Vec<MonitorInfo>> {
    let mut raw: Vec<RawMonitor> = Vec::new();
    let ptr = &mut raw as *mut Vec<RawMonitor>;
    unsafe {
        EnumDisplayMonitors(None, None, Some(monitor_enum_proc), LPARAM(ptr as isize))
            .as_bool()
            .then_some(())
            .ok_or_else(|| "枚举显示器失败".to_string())?;
    }
    Ok(raw
        .into_iter()
        .map(|m| MonitorInfo {
            id: m.id,
            name: m.name,
            width: m.width,
            height: m.height,
            primary: m.primary,
        })
        .collect())
}

pub fn set_dpi_awareness() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

/// cover 裁剪：等比缩放至覆盖目标尺寸后居中裁剪。
fn cover_crop(img: DynamicImage, tw: u32, th: u32) -> DynamicImage {
    cover_crop_at(img, tw, th, 0.5, 0.5)
}

/// cover 裁剪 + 归一化取景偏移（0..1，0.5/0.5 = 居中）。
/// 偏移只在缩放后存在溢出的轴上生效，用于导出时自定义构图。
pub fn cover_crop_at(img: DynamicImage, tw: u32, th: u32, ox: f32, oy: f32) -> DynamicImage {
    if img.width() == tw && img.height() == th {
        return img;
    }
    let scale = f32::max(tw as f32 / img.width() as f32, th as f32 / img.height() as f32);
    let nw = ((img.width() as f32 * scale).round() as u32).max(tw);
    let nh = ((img.height() as f32 * scale).round() as u32).max(th);
    let scaled = img.resize_exact(nw, nh, image::imageops::FilterType::Lanczos3);
    let max_x = (nw - tw) as f32;
    let max_y = (nh - th) as f32;
    let x = (max_x * ox.clamp(0.0, 1.0)).round() as u32;
    let y = (max_y * oy.clamp(0.0, 1.0)).round() as u32;
    scaled.crop_imm(x, y, tw, th)
}

/// fit 适配：目标尺寸黑色画布，完整居中显示。
pub(crate) fn fit_canvas(img: DynamicImage, tw: u32, th: u32) -> DynamicImage {
    if img.width() == tw && img.height() == th {
        return img;
    }
    let scale = f32::min(tw as f32 / img.width() as f32, th as f32 / img.height() as f32);
    let nw = ((img.width() as f32 * scale).round() as u32).max(1);
    let nh = ((img.height() as f32 * scale).round() as u32).max(1);
    let scaled = img.resize_exact(nw, nh, image::imageops::FilterType::Lanczos3);
    let mut canvas = DynamicImage::new_rgb8(tw, th);
    image::imageops::overlay(&mut canvas, &scaled, ((tw - nw) / 2) as i64, ((th - nh) / 2) as i64);
    canvas
}

fn write_jpeg(img: &DynamicImage, path: &Path) -> CmdResult<()> {
    let file = fs::File::create(path).map_err(|e| format!("创建输出文件失败: {e}"))?;
    let writer = std::io::BufWriter::new(file);
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(writer, 95);
    img.write_with_encoder(encoder)
        .map_err(|e| format!("编码图片失败: {e}"))?;
    Ok(())
}

fn set_wallpaper_win(monitor: Option<&str>, path: &Path) -> CmdResult<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .map_err(|e| format!("COM 初始化失败: {e}"))?;
        let result = (|| -> CmdResult<()> {
            let dw: IDesktopWallpaper = CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL)
                .map_err(|e| format!("创建壁纸接口失败: {e}"))?;
            let file = HSTRING::from(path.as_os_str());
            let mon = monitor.map(HSTRING::from);
            let monitor_pcw = match &mon {
                Some(m) => PCWSTR(m.as_ptr()),
                None => PCWSTR::null(),
            };
            dw.SetWallpaper(monitor_pcw, PCWSTR(file.as_ptr()))
                .map_err(|e| format!("设置壁纸失败: {e}"))?;
            let _ = dw.SetPosition(DWPOS_FILL);
            Ok(())
        })();
        CoUninitialize();
        result
    }
}

/// 裁剪适配到目标显示器分辨率并设置为系统壁纸。
/// display: None 表示应用到所有显示器（按主屏分辨率适配）。
pub fn adapt_and_apply(
    app: &AppHandle,
    item: &WallpaperItem,
    display: Option<&str>,
) -> CmdResult<()> {
    let cfg = settings::load(app);
    let fill = cfg.fill_mode != "fit";

    let monitors = list_monitors_impl()?;
    if monitors.is_empty() {
        return Err("未检测到显示器".into());
    }
    let (tw, th) = match display {
        Some(id) => {
            let m = monitors
                .iter()
                .find(|m| m.id == id)
                .ok_or_else(|| "未找到所选显示器".to_string())?;
            (m.width.max(1) as u32, m.height.max(1) as u32)
        }
        None => {
            let m = monitors
                .iter()
                .find(|m| m.primary)
                .unwrap_or(&monitors[0]);
            (m.width.max(1) as u32, m.height.max(1) as u32)
        }
    };

    let img = image::open(Path::new(&item.file_path))
        .map_err(|e| format!("读取壁纸图片失败: {e}"))?;
    let adapted = if fill {
        cover_crop(img, tw, th)
    } else {
        fit_canvas(img, tw, th)
    };

    let mode = if fill { "fill" } else { "fit" };
    let out_path = paths::data_root(app)?
        .join("adapted")
        .join(format!("{}_{}x{}_{}.jpg", item.id, tw, th, mode));
    write_jpeg(&adapted, &out_path)?;

    set_wallpaper_win(display, &out_path)?;
    library::touch_applied(app, &item.id)?;
    Ok(())
}
