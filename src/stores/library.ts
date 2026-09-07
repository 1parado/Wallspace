import { defineStore } from 'pinia';
import type { WallpaperItem } from '../types';
import * as api from '../lib/api';
import { useUiStore } from './ui';
import { useI18n } from '../lib/i18n';

export const useLibraryStore = defineStore('library', {
  state: () => ({
    items: [] as WallpaperItem[],
    loaded: false,
    generating: false,
    importingUrl: false,
    importingFiles: false,
    applyingId: null as string | null,
  }),
  getters: {
    byId(state) {
      return (id: string | null) =>
        id ? state.items.find((i) => i.id === id) ?? null : null;
    },
    favorites(): WallpaperItem[] {
      return this.items.filter((i) => i.favorite);
    },
    /** 最近一次应用的条目 id（托盘/卡片「使用中」标记用） */
    currentItemId(state): string | null {
      let best: string | null = null;
      let bestTime = 0;
      for (const i of state.items) {
        if (i.appliedAt && i.appliedAt > bestTime) {
          bestTime = i.appliedAt;
          best = i.id;
        }
      }
      return best;
    },
    recentApplied(): WallpaperItem[] {
      return this.items
        .filter((i) => i.appliedAt)
        .sort((a, b) => (b.appliedAt ?? 0) - (a.appliedAt ?? 0));
    },
    downloads(): WallpaperItem[] {
      return this.items.filter((i) => i.source === 'url');
    },
    imports(): WallpaperItem[] {
      return this.items.filter((i) => i.source === 'local');
    },
    aiItems(): WallpaperItem[] {
      return this.items.filter((i) => i.source === 'ai');
    },
  },
  actions: {
    async refresh() {
      const ui = useUiStore();
      const { t } = useI18n();
      try {
        this.items = await api.getLibrary();
      } catch (e) {
        ui.toast('error', t('toast.libraryFail', { e: String(e) }));
      } finally {
        this.loaded = true;
      }
    },
    async generate(prompt: string, size: string, category: string | null, tags: string[] = []) {
      const ui = useUiStore();
      const { t } = useI18n();
      this.generating = true;
      try {
        const item = await api.generateWallpaper(prompt, size, category, tags);
        this.items.unshift(item);
        ui.toast('success', t('toast.generated'));
        return item;
      } catch (e) {
        ui.toast('error', String(e));
        return null;
      } finally {
        this.generating = false;
      }
    },
    async generateGrok(prompt: string, model: string, aspectRatio: string, category: string | null, tags: string[] = []) {
      const ui = useUiStore();
      const { t } = useI18n();
      this.generating = true;
      try {
        const item = await api.grokImagine(prompt, model, aspectRatio, category, tags);
        this.items.unshift(item);
        ui.toast('success', t('toast.generated'));
        return item;
      } catch (e) {
        ui.toast('error', String(e));
        return null;
      } finally {
        this.generating = false;
      }
    },
    async importUrl(url: string, category: string | null = null, tags: string[] = []) {
      const ui = useUiStore();
      const { t } = useI18n();
      this.importingUrl = true;
      try {
        const item = await api.importFromUrl(url, category, tags);
        this.items.unshift(item);
        ui.toast('success', t('toast.downloaded'));
        return item;
      } catch (e) {
        ui.toast('error', String(e));
        return null;
      } finally {
        this.importingUrl = false;
      }
    },
    async importFiles(paths: string[], category: string | null = null, tags: string[] = []) {
      const ui = useUiStore();
      const { t } = useI18n();
      if (!paths.length) return;
      this.importingFiles = true;
      try {
        const report = await api.importLocalFiles(paths, category, tags);
        this.items.unshift(...report.imported);
        if (report.imported.length) {
          ui.toast('success', t('toast.importedN', { n: report.imported.length }));
        }
        if (report.failed.length) {
          ui.toast('error', t('toast.importFailedN', { n: report.failed.length }));
        }
      } catch (e) {
        ui.toast('error', String(e));
      } finally {
        this.importingFiles = false;
      }
    },
    async toggleFavorite(item: WallpaperItem) {
      const next = { ...item, favorite: !item.favorite };
      try {
        await api.updateItem(next);
        const idx = this.items.findIndex((i) => i.id === item.id);
        if (idx >= 0) this.items[idx] = next;
      } catch (e) {
        useUiStore().toast('error', String(e));
      }
    },
    async patch(item: WallpaperItem, changes: Partial<WallpaperItem>) {
      const next = { ...item, ...changes };
      try {
        await api.updateItem(next);
        const idx = this.items.findIndex((i) => i.id === item.id);
        if (idx >= 0) this.items[idx] = next;
      } catch (e) {
        useUiStore().toast('error', String(e));
      }
    },
    async remove(id: string) {
      const ui = useUiStore();
      const { t } = useI18n();
      try {
        await api.deleteItem(id);
        this.items = this.items.filter((i) => i.id !== id);
        if (ui.previewId === id) ui.previewId = null;
        ui.toast('success', t('toast.deleted'));
      } catch (e) {
        ui.toast('error', String(e));
      }
    },
    async apply(id: string) {
      const ui = useUiStore();
      const { t } = useI18n();
      const item = this.byId(id);
      if (!item) return false;
      this.applyingId = id;
      try {
        await api.applyWallpaper(id, ui.selectedDisplay);
        const idx = this.items.findIndex((i) => i.id === id);
        if (idx >= 0) this.items[idx] = { ...this.items[idx], appliedAt: Date.now() };
        const target = ui.selectedDisplay
          ? ui.monitors.find((m) => m.id === ui.selectedDisplay)?.name ?? t('toolbar.display')
          : t('toolbar.allDisplays');
        ui.toast('success', t('toast.appliedTo', { name: target }));
        return true;
      } catch (e) {
        ui.toast('error', String(e));
        return false;
      } finally {
        this.applyingId = null;
      }
    },
  },
});
