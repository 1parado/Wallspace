# Wallspace

AI 生图桌面壁纸应用（Windows）· Tauri 2 + Vue 3

生图即壁纸

## 功能

- **AI 生图**
  - **OpenAI 兼容接口** — 设置页配置 Base URL / API Key / 模型（`/v1/images/generations`，b64/url 双回退），支持连接测试
  - **Grok 账号直连** — 与 [grok_switch](../grok_switch) 的 ImagineEngine 同实现
- **一键应用** — 按目标显示器分辨率智能裁剪或完整显示后，通过 `IDesktopWallpaper` 设置系统壁纸；支持每个显示器独立设置或一次应用到全部显示器
- **画廊管理** — 生成 / 链接下载 / 本地导入统一入库；重命名、分类、收藏、删除、搜索（Ctrl+K）
- **第三方壁纸链接** — 粘贴图片直链即可下载入库，本地保存、离线可用
- **本地导入** — 文件选择器或全窗口拖放（PNG / JPG / WebP / GIF / BMP）
- **多显示器** — 工具栏显示器选择器，列出各屏分辨率
- **i18n 中英文** — 设置页切换中文 / English，界面文案全面双语
- **深色 / 浅色主题** — 跟随系统或手动切换，窗口 Acrylic 底色随主题联动；工具栏快捷切换按钮
- **侧边栏收起** — 工具栏按钮或 Ctrl+B 展开/收起侧栏，媒体库全宽浏览
- **快捷键** — Ctrl+K 搜索、Ctrl+B 侧栏、Esc 关闭预览、F 收藏（预览内）

## 预览

### 主界面

![Discover 主页 — 侧边栏分类、Featured 与 Recent 画廊、AI 生图与粘贴链接入口](pic/pic1.png)

### AI 生图预览

![图片详情页 — Prompt 标签、收藏、应用壁纸按钮](pic/pic2.png)

### 设为桌面壁纸

![通过 Wallspace 应用为桌面壁纸后的实际效果](pic/pic3.png)
