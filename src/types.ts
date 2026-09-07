export interface WallpaperItem {
  id: string;
  title: string;
  source: 'ai' | 'url' | 'local';
  filePath: string;
  width: number;
  height: number;
  fileSize: number;
  /** 可空分类（Rust Option 序列化为 null）；「自动」模式下由 tags + palette 承担归类。
   *  层级用 '/' 约定，如 'Nature/Mountains' */
  category?: string | null;
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
  /** 侧边栏展开（图标+文字）模式 */
  sidebarExpanded: boolean;
  /** 智能打标模型（chat/completions）；空 = 仅关键词规则 */
  classifyModel?: string;
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
  | 'recent'
  | 'collection';

export interface Collection {
  id: string;
  name: string;
  itemIds: string[];
  createdAt: number;
}

export interface Toast {
  id: number;
  kind: 'success' | 'error' | 'info';
  message: string;
}
