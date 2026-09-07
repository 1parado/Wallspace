import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import type { Collection, ImportReport, MonitorInfo, Settings, WallpaperItem } from '../types';

export function assetUrl(path: string): string {
  return convertFileSrc(path);
}

export async function getLibrary(): Promise<WallpaperItem[]> {
  return invoke('get_library');
}

export async function importLocalFiles(
  paths: string[],
  category: string | null,
  tags: string[] = []
): Promise<ImportReport> {
  return invoke('import_local_files', { paths, category, tags });
}

export async function importFromUrl(
  url: string,
  category: string | null,
  tags: string[] = []
): Promise<WallpaperItem> {
  return invoke('import_from_url', { url, category, tags });
}

export async function generateWallpaper(
  prompt: string,
  size: string,
  category: string | null,
  tags: string[] = []
): Promise<WallpaperItem> {
  return invoke('generate_wallpaper', { prompt, size, category, tags });
}

export interface GrokImagineStatus {
  total: number;
  available: number;
}

/** Grok（grok.com 账号池）直连生图，协议与 grok_switch ImagineEngine 一致 */
export async function grokImagine(
  prompt: string,
  model: string,
  aspectRatio: string,
  category: string | null,
  tags: string[] = []
): Promise<WallpaperItem> {
  return invoke('grok_imagine', { prompt, model, aspectRatio, category, tags });
}

export async function grokImagineStatus(): Promise<GrokImagineStatus> {
  return invoke('grok_imagine_status');
}

export async function updateItem(item: WallpaperItem): Promise<WallpaperItem> {
  return invoke('update_item', { item });
}

export async function deleteItem(id: string): Promise<void> {
  return invoke('delete_item', { id });
}

/** display 为 null 表示应用到所有显示器 */
export async function applyWallpaper(
  id: string,
  display: string | null
): Promise<void> {
  return invoke('apply_wallpaper', { id, display });
}

export async function listMonitors(): Promise<MonitorInfo[]> {
  return invoke('list_monitors');
}

export async function getSettings(): Promise<Settings> {
  return invoke('get_settings');
}

export async function saveSettings(settings: Settings): Promise<void> {
  return invoke('save_settings', { settings });
}

export async function testConnection(
  baseUrl: string,
  apiKey: string
): Promise<string[]> {
  return invoke('test_connection', { baseUrl, apiKey });
}

export interface LlmClassify {
  category: string | null;
  tags: string[];
}

/** 单条文本 LLM 打标；未配置模型或调用失败返回 null（前端回退关键词规则） */
export async function classifyText(text: string): Promise<LlmClassify | null> {
  return invoke('classify_text', { text });
}

/** 导出库备份（清单 + 图片 zip），返回导出条目数 */
export async function exportBackup(savePath: string): Promise<number> {
  return invoke('export_backup', { savePath });
}

export interface BackupImportResult {
  imported: number;
  skipped: number;
  collectionsAdded: number;
}

/** 从备份恢复（按 id 合并，已存在跳过） */
export async function importBackup(path: string): Promise<BackupImportResult> {
  return invoke('import_backup', { path });
}

export interface UpdateInfo {
  current: string;
  latest: string;
  hasUpdate: boolean;
  url: string;
}

/** 检查更新：查询 GitHub Releases 最新版本 */
export async function checkUpdates(): Promise<UpdateInfo> {
  return invoke('check_updates');
}

export async function revealItem(path: string): Promise<void> {
  return invoke('reveal_item', { path });
}

/** 提取图片主色调（调色板，最多 6 个 hex 色值） */
export async function extractPalette(path: string): Promise<string[]> {
  return invoke('extract_palette', { path });
}

export async function listCollections(): Promise<Collection[]> {
  return invoke('list_collections');
}

export async function saveCollections(collections: Collection[]): Promise<void> {
  return invoke('save_collections', { collections });
}

export async function createCollection(name: string): Promise<Collection> {
  return invoke('create_collection', { name });
}

export interface WhThumb {
  id: string;
  path: string;
  thumb: string;
  resolution: string;
  purity: string;
  colors: string[];
}

/** Wallhaven 搜索（SFW 公开接口，无需 Key） */
export async function wallhavenSearch(params: {
  query: string;
  page?: number;
  sorting?: string;
  atleast?: string;
}): Promise<WhThumb[]> {
  return invoke('wallhaven_search', {
    query: params.query,
    page: params.page ?? 1,
    sorting: params.sorting ?? 'relevance',
    atleast: params.atleast ?? '',
  });
}

export interface ExportResult {
  path: string;
  item: WallpaperItem | null;
}

export interface ExportAdjust {
  /** 加性亮度偏移（-128..128） */
  brightness?: number;
  /** 对比度系数（0.5..1.5） */
  contrast?: number;
  /** 饱和度系数（0..2） */
  saturation?: number;
  /** 高斯模糊 sigma（0..8） */
  blur?: number;
}

/** 按预设尺寸裁剪导出：mode 'cover'（带取景偏移）| 'fit'；format 'jpg' | 'png'（png 的 fit 模式四边透明） */
export async function exportWallpaper(params: {
  id: string;
  width: number;
  height: number;
  mode?: 'cover' | 'fit';
  offsetX?: number;
  offsetY?: number;
  addToLibrary?: boolean;
  savePath?: string;
  title?: string;
  format?: 'jpg' | 'png';
  adjust?: ExportAdjust;
}): Promise<ExportResult> {
  const adj = params.adjust ?? {};
  return invoke('export_wallpaper', {
    id: params.id,
    width: params.width,
    height: params.height,
    mode: params.mode ?? 'cover',
    offsetX: params.offsetX ?? 0.5,
    offsetY: params.offsetY ?? 0.5,
    addToLibrary: params.addToLibrary ?? false,
    savePath: params.savePath ?? null,
    title: params.title ?? null,
    format: params.format ?? 'jpg',
    brightness: adj.brightness ?? 0,
    contrast: adj.contrast ?? 1,
    saturation: adj.saturation ?? 1,
    blur: adj.blur ?? 0,
  });
}
