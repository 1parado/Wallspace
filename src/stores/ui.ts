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
  },
});
