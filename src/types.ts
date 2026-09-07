export interface WallpaperItem {
  id: string;
  title: string;
  source: 'ai' | 'url' | 'local';
  filePath: string;
  width: number;
  height: number;
  fileSize: number;
  /** 可空分类；「自动」模式下由 tags + palette 承担归类 */
  category?: string;
  tags: string[];
  /** 主色调色板（hex，最多 5 色，按占比排序），用于按颜色过滤 */
  palette?: string[];
  favorite: boolean;
  prompt?: string;
  model?: string;
  originUrl?: string;
  createdAt: number;
  appliedAt?: number;
}

export interface MonitorInfo {
  id: string;
  name: string;
  width: number;
  height: number;
  primary: boolean;
}

export type ThemeMode = 'system' | 'dark' | 'light';

export interface Settings {
  apiBaseUrl: string;
  apiKey: string;
  apiModel: string;
  fillMode: 'fill' | 'fit';
  defaultSize: string;
  locale: 'zh-CN' | 'en';
  theme: ThemeMode;
  sidebarHidden: boolean;
}

export interface ImportReport {
  imported: WallpaperItem[];
  failed: { path: string; reason: string }[];
}

export type ViewId =
  | 'discover'
  | 'create'
  | 'wallpapers'
  | 'favorites'
  | 'downloads'
  | 'imports'
  | 'recent';

export interface Toast {
  id: number;
  kind: 'success' | 'error' | 'info';
  message: string;
}
