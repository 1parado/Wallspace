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

export async function revealItem(path: string): Promise<void> {
  return invoke('reveal_item', { path });
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
