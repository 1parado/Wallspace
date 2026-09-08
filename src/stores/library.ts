import { computed, ref, shallowRef } from 'vue';
import { defineStore } from 'pinia';
import type { WallpaperItem } from '../types';
import * as api from '../lib/api';
import { useUiStore } from './ui';
import { useI18n } from '../lib/i18n';

/**
 * 媒体库 store（setup store）。
 * items 用 shallowRef：数千张壁纸时不再为每个对象建深度代理，
 * 所有变更都以「整体替换数组」的方式提交，只触发一次响应。
 */
export const useLibraryStore = defineStore('library', () => {
  const items = shallowRef<WallpaperItem[]>([]);
  const loaded = ref(false);
  const generating = ref(false);
  const importingUrl = ref(false);
  const importingFiles = ref(false);
  const applyingId = ref<string | null>(null);

  // —— 派生列表（computed，items 替换时才重算） ——
  function byId(id: string | null): WallpaperItem | null {
    return id ? items.value.find((i) => i.id === id) ?? null : null;
  }

  const favorites = computed(() => items.value.filter((i) => i.favorite));

  /** 最近一次应用的条目 id（托盘/卡片「使用中」标记用） */
  const currentItemId = computed<string | null>(() => {
    let best: string | null = null;
    let bestTime = 0;
    for (const i of items.value) {
      if (i.appliedAt && i.appliedAt > bestTime) {
        bestTime = i.appliedAt;
        best = i.id;
      }
    }
    return best;
  });

  const recentApplied = computed(() =>
    items.value
      .filter((i) => i.appliedAt)
      .sort((a, b) => (b.appliedAt ?? 0) - (a.appliedAt ?? 0))
  );

  const downloads = computed(() => items.value.filter((i) => i.source === 'url'));
  const imports = computed(() => items.value.filter((i) => i.source === 'local'));
  const aiItems = computed(() => items.value.filter((i) => i.source === 'ai'));

  // —— actions：全部走「替换数组」，保持 shallowRef 语义 ——
  async function refresh() {
    const ui = useUiStore();
    const { t } = useI18n();
    try {
      items.value = await api.getLibrary();
    } catch (e) {
      ui.toast('error', t('toast.libraryFail', { e: String(e) }));
    } finally {
      loaded.value = true;
    }
  }

  async function generate(prompt: string, size: string, category: string | null, tags: string[] = []) {
    const ui = useUiStore();
    const { t } = useI18n();
    generating.value = true;
    try {
      const item = await api.generateWallpaper(prompt, size, category, tags);
      items.value = [item, ...items.value];
      ui.toast('success', t('toast.generated'));
      return item;
    } catch (e) {
      ui.toast('error', String(e));
      return null;
    } finally {
      generating.value = false;
    }
  }

  async function generateGrok(prompt: string, model: string, aspectRatio: string, category: string | null, tags: string[] = []) {
    const ui = useUiStore();
    const { t } = useI18n();
    generating.value = true;
    try {
      const item = await api.grokImagine(prompt, model, aspectRatio, category, tags);
      items.value = [item, ...items.value];
      ui.toast('success', t('toast.generated'));
      return item;
    } catch (e) {
      ui.toast('error', String(e));
      return null;
    } finally {
      generating.value = false;
    }
  }

  async function importUrl(url: string, category: string | null = null, tags: string[] = []) {
    const ui = useUiStore();
    const { t } = useI18n();
    importingUrl.value = true;
    try {
      const item = await api.importFromUrl(url, category, tags);
      items.value = [item, ...items.value];
      ui.toast('success', t('toast.downloaded'));
      return item;
    } catch (e) {
      ui.toast('error', String(e));
      return null;
    } finally {
      importingUrl.value = false;
    }
  }

  async function importFiles(paths: string[], category: string | null = null, tags: string[] = []) {
    const ui = useUiStore();
    const { t } = useI18n();
    if (!paths.length) return;
    importingFiles.value = true;
    try {
      const report = await api.importLocalFiles(paths, category, tags);
      items.value = [...report.imported, ...items.value];
      if (report.imported.length) {
        ui.toast('success', t('toast.importedN', { n: report.imported.length }));
      }
      if (report.failed.length) {
        ui.toast('error', t('toast.importFailedN', { n: report.failed.length }));
      }
    } catch (e) {
      ui.toast('error', String(e));
    } finally {
      importingFiles.value = false;
    }
  }

  async function toggleFavorite(item: WallpaperItem) {
    const next = { ...item, favorite: !item.favorite };
    try {
      await api.updateItem(next);
      items.value = items.value.map((i) => (i.id === item.id ? next : i));
    } catch (e) {
      useUiStore().toast('error', String(e));
    }
  }

  async function patch(item: WallpaperItem, changes: Partial<WallpaperItem>) {
    const next = { ...item, ...changes };
    try {
      await api.updateItem(next);
      items.value = items.value.map((i) => (i.id === item.id ? next : i));
    } catch (e) {
      useUiStore().toast('error', String(e));
    }
  }

  async function remove(id: string) {
    const ui = useUiStore();
    const { t } = useI18n();
    try {
      await api.deleteItem(id);
      items.value = items.value.filter((i) => i.id !== id);
      if (ui.previewId === id) ui.previewId = null;
      ui.toast('success', t('toast.deleted'));
    } catch (e) {
      ui.toast('error', String(e));
    }
  }

  async function apply(id: string) {
    const ui = useUiStore();
    const { t } = useI18n();
    const item = byId(id);
    if (!item) return false;
    applyingId.value = id;
    try {
      await api.applyWallpaper(id, ui.selectedDisplay);
      items.value = items.value.map((i) => (i.id === id ? { ...i, appliedAt: Date.now() } : i));
      const target = ui.selectedDisplay
        ? ui.monitors.find((m) => m.id === ui.selectedDisplay)?.name ?? t('toolbar.display')
        : t('toolbar.allDisplays');
      ui.toast('success', t('toast.appliedTo', { name: target }));
      return true;
    } catch (e) {
      ui.toast('error', String(e));
      return false;
    } finally {
      applyingId.value = null;
    }
  }

  return {
    items,
    loaded,
    generating,
    importingUrl,
    importingFiles,
    applyingId,
    byId,
    favorites,
    currentItemId,
    recentApplied,
    downloads,
    imports,
    aiItems,
    refresh,
    generate,
    generateGrok,
    importUrl,
    importFiles,
    toggleFavorite,
    patch,
    remove,
    apply,
  };
});
