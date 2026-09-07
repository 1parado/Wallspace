import { defineStore } from 'pinia';
import type { MonitorInfo, Toast, ViewId } from '../types';
import { listMonitors } from '../lib/api';

export const CATEGORIES = [
  'Nature',
  'Space',
  'Abstract',
  'Cinematic',
  'Minimal',
] as const;

let toastSeq = 0;

export type SortMode = 'newest' | 'oldest' | 'name' | 'resolution' | 'random';

function readSortMode(): string {
  try {
    return localStorage.getItem('wallspace.sortMode') || 'newest';
  } catch {
    return 'newest';
  }
}

export const useUiStore = defineStore('ui', {
  state: () => ({
    view: 'discover' as ViewId,
    /** Wallpapers 视图内的分类过滤，null = 全部 */
    categoryFilter: null as string | null,
    search: '',
    previewId: null as string | null,
    /** 打开预览时所在列表的条目顺序（预览层上一张/下一张导航用）；空 = 退回整个库顺序 */
    previewIds: [] as string[],
    settingsOpen: false,
    /** 目标显示器：null = 所有显示器 */
    selectedDisplay: null as string | null,
    monitors: [] as MonitorInfo[],
    toasts: [] as Toast[],
    categories: CATEGORIES as readonly string[],

    // —— 分面过滤（Wallpapers 视图）——
    /** 选中的标签，多选 */
    tagFilter: [] as string[],
    /** 多标签命中模式：any = OR，all = AND */
    tagMode: 'any' as 'any' | 'all',
    /** 来源过滤，空数组 = 全部 */
    sourceFilter: [] as Array<'ai' | 'url' | 'local'>,
    /** 宽高比档位：wide(超宽) | landscape(横) | square(方) | portrait(竖)，null = 全部 */
    ratioFilter: null as string | null,
    /** 主色过滤（hex），null = 全部 */
    colorFilter: null as string | null,
    /** 当前打开的集合（view = 'collection' 时有效） */
    activeCollectionId: null as string | null,

    // —— 排序（Wallpapers 视图）——
    /** newest | oldest | name | resolution | random */
    sortMode: (readSortMode() as SortMode) || 'newest',
    /** 随机排序的洗牌种子：重复点「随机」时更新 */
    sortSeed: 0 as number,
  }),
  actions: {
    /** 设置排序；重复点「随机」时重新洗牌 */
    setSort(mode: SortMode) {
      if (mode === 'random' && this.sortMode === 'random') {
        this.sortSeed = Date.now();
        return;
      }
      this.sortMode = mode;
      try {
        localStorage.setItem('wallspace.sortMode', mode);
      } catch {
        /* 隐私模式等场景忽略 */
      }
    },
    toast(kind: Toast['kind'], message: string) {
      const id = ++toastSeq;
      this.toasts.push({ id, kind, message });
      setTimeout(() => {
        this.toasts = this.toasts.filter((t) => t.id !== id);
      }, kind === 'error' ? 5200 : 3200);
    },
    async loadMonitors() {
      try {
        this.monitors = await listMonitors();
      } catch (e) {
        this.toast('error', String(e));
      }
    },
    goto(view: ViewId) {
      this.view = view;
      this.categoryFilter = null;
      if (view !== 'collection') this.activeCollectionId = null;
      if (view !== 'wallpapers') this.search = '';
    },
    /** 重置全部分面 */
    resetFacets() {
      this.categoryFilter = null;
      this.tagFilter = [];
      this.sourceFilter = [];
      this.ratioFilter = null;
      this.colorFilter = null;
    },
    toggleTag(tag: string) {
      this.tagFilter = this.tagFilter.includes(tag)
        ? this.tagFilter.filter((t) => t !== tag)
        : [...this.tagFilter, tag];
    },
  },
});
