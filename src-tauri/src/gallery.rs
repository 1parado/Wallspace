//! gallery.rs —— 媒体库导出为离线 HTML 画廊。
//!
//! 选定目标目录后生成 `wallspace-gallery/`：
//! - `images/`：按条目 id 重命名拷贝全部图片（拷贝失败的跳过）
//! - `index.html`：单文件深色画廊（CSS columns 瀑布流 + 原生 JS 灯箱），零外部依赖，
//!   整个文件夹打包发给别人即可离线浏览。

use crate::library;
use crate::models::CmdResult;
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;

/// HTML 转义
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn build_html(count: u32, cards: &str, date: &str) -> String {
    let tpl = r#"<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Wallspace Gallery · __COUNT__ 张</title>
<style>
  :root { color-scheme: dark; }
  body { margin:0; background:#0b0b0d; color:#e8e8ea; font-family:"Segoe UI","Microsoft YaHei","PingFang SC",sans-serif; }
  header { padding:32px 24px 8px; }
  h1 { margin:0; font-size:22px; letter-spacing:-0.01em; }
  p.sub { margin:6px 0 0; color:#8a8a92; font-size:13px; }
  .grid { columns: 4 230px; column-gap:14px; padding:20px 24px 48px; }
  figure { margin:0 0 14px; break-inside:avoid; border-radius:12px; overflow:hidden; background:#141418; border:1px solid #232329; cursor:zoom-in; }
  figure img { width:100%; display:block; }
  figcaption { padding:9px 12px 11px; font-size:12.5px; display:flex; flex-direction:column; gap:3px; }
  figcaption span { color:#8a8a92; font-size:11.5px; }
  #lightbox { position:fixed; inset:0; background:rgba(0,0,0,.93); display:none; align-items:center; justify-content:center; flex-direction:column; gap:14px; cursor:zoom-out; z-index:10; }
  #lightbox.on { display:flex; }
  #lightbox img { max-width:92vw; max-height:82vh; border-radius:10px; }
  #lightbox .cap { color:#cfcfd6; font-size:14px; }
</style>
</head>
<body>
<header><h1>Wallspace Gallery</h1><p class="sub">__COUNT__ 张壁纸 · 由 Wallspace 导出于 __DATE__</p></header>
<div class="grid">
__CARDS__
</div>
<div id="lightbox"><img id="lb-img" alt=""><div class="cap" id="lb-cap"></div></div>
<script>
const lb = document.getElementById('lightbox'), img = document.getElementById('lb-img'), cap = document.getElementById('lb-cap');
document.querySelectorAll('figure').forEach(f => f.addEventListener('click', () => {
  img.src = f.dataset.src;
  cap.textContent = f.querySelector('strong').textContent;
  lb.classList.add('on');
}));
lb.addEventListener('click', () => lb.classList.remove('on'));
document.addEventListener('keydown', e => { if (e.key === 'Escape') lb.classList.remove('on'); });
</script>
</body>
</html>
"#;
    tpl
        .replace("__COUNT__", &count.to_string())
        .replace("__DATE__", date)
        .replace("__CARDS__", cards)
}

/// 导出媒体库为离线画廊，返回成功拷贝的图片数。
pub async fn export(app: AppHandle, dir: String) -> CmdResult<u32> {
    tauri::async_runtime::spawn_blocking(move || -> Result<u32, String> {
        let root = PathBuf::from(dir.trim()).join("wallspace-gallery");
        let images = root.join("images");
        fs::create_dir_all(&images).map_err(|e| format!("创建目录失败: {e}"))?;

        let items = library::load(&app);
        let mut cards: Vec<String> = Vec::new();
        let mut n = 0u32;
        for it in &items {
            let src = std::path::Path::new(&it.file_path);
            if !src.is_file() {
                continue;
            }
            let ext = src
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .unwrap_or_else(|| "jpg".into());
            let fname = format!("{}.{}", it.id, ext);
            if fs::copy(src, images.join(&fname)).is_err() {
                continue; // 源文件丢失/被占用：跳过该张
            }
            let title = esc(&it.title);
            let cat = esc(it.category.as_deref().unwrap_or("-"));
            let tags = it
                .tags
                .iter()
                .map(|t| format!("#{}", esc(t)))
                .collect::<Vec<_>>()
                .join(" ");
            cards.push(format!(
                "<figure data-src=\"images/{fname}\"><img loading=\"lazy\" src=\"images/{fname}\" \
                 alt=\"{title}\"><figcaption><strong>{title}</strong><span>{w}×{h} · {cat} {tags}</span></figcaption></figure>",
                fname = fname,
                title = title,
                w = it.width,
                h = it.height,
                cat = cat,
                tags = tags,
            ));
            n += 1;
        }

        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let html = build_html(n, &cards.join("\n"), &date);
        fs::write(root.join("index.html"), html)
            .map_err(|e| format!("写入 index.html 失败: {e}"))?;
        Ok(n)
    })
    .await
    .map_err(|e| format!("画廊导出任务失败: {e}"))?
}
