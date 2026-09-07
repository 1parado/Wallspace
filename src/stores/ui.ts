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

export const useUiStore = defineStore('ui', {
  state: () => ({
    view: 'discover' as ViewId,
    /** Wallpapers 视图内的分类过滤，null = 全部 */
    categoryFilter: null as string | null,
    search: '',
    previewId: null as string | null,
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
  }),
  actions: {
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
