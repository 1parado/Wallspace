import { defineStore } from 'pinia';
import type { Collection } from '../types';
import * as api from '../lib/api';
import { useUiStore } from './ui';
import { useI18n } from '../lib/i18n';

export const useCollectionsStore = defineStore('collections', {
  state: () => ({
    collections: [] as Collection[],
    loaded: false,
  }),
  getters: {
    byId(state) {
      return (id: string | null) =>
        id ? state.collections.find((c) => c.id === id) ?? null : null;
    },
    /** 某壁纸所在的集合 id 列表 */
    collectionsOf(state) {
      return (itemId: string | null) =>
        itemId
          ? state.collections.filter((c) => c.itemIds.includes(itemId))
          : [];
    },
  },
  actions: {
    async load() {
      try {
        this.collections = await api.listCollections();
      } catch {
        this.collections = [];
      } finally {
        this.loaded = true;
      }
    },
    async persist() {
      await api.saveCollections(this.collections);
    },
    async create(name: string): Promise<Collection | null> {
      const ui = useUiStore();
      const n = name.trim();
      if (!n) return null;
      try {
        const c = await api.createCollection(n);
        this.collections.push(c);
        return c;
      } catch (e) {
        ui.toast('error', String(e));
        return null;
      }
    },
    async rename(id: string, name: string) {
      const n = name.trim();
      const c = this.byId(id);
      if (!c || !n || c.name === n) return;
      c.name = n;
      await this.persist();
    },
    async remove(id: string) {
      this.collections = this.collections.filter((c) => c.id !== id);
      await this.persist();
    },
    async addItem(collectionId: string, itemId: string) {
      const ui = useUiStore();
      const { t } = useI18n();
      const c = this.byId(collectionId);
      if (!c) return;
      if (!c.itemIds.includes(itemId)) c.itemIds.unshift(itemId);
      try {
        await this.persist();
        ui.toast('success', t('toast.addedToCollection', { name: c.name }));
      } catch (e) {
        ui.toast('error', String(e));
      }
    },
    async removeItem(collectionId: string, itemId: string) {
      const c = this.byId(collectionId);
      if (!c) return;
      c.itemIds = c.itemIds.filter((id) => id !== itemId);
      await this.persist();
    },
    /** 批量加入集合（自动去重），单条 toast 汇总 */
    async addItems(collectionId: string, itemIds: string[]) {
      const ui = useUiStore();
      const { t } = useI18n();
      const c = this.byId(collectionId);
      if (!c || !itemIds.length) return;
      let added = 0;
      for (const id of itemIds) {
        if (!c.itemIds.includes(id)) {
          c.itemIds.unshift(id);
          added++;
        }
      }
      if (!added) return;
      try {
        await this.persist();
        ui.toast('success', t('toast.batchAdded', { n: added, name: c.name }));
      } catch (e) {
        ui.toast('error', String(e));
      }
    },
    /** 批量移动：加入目标集合并从源集合移除 */
    async moveItems(fromId: string, toId: string, itemIds: string[]) {
      if (fromId === toId || !itemIds.length) return;
      await this.addItems(toId, itemIds);
      const from = this.byId(fromId);
      if (!from) return;
      from.itemIds = from.itemIds.filter((id) => !itemIds.includes(id));
      await this.persist();
    },
    /** 拖拽排序：用新的 id 顺序整体替换并持久化 */
    async reorder(collectionId: string, newItemIds: string[]) {
      const c = this.byId(collectionId);
      if (!c) return;
      c.itemIds = newItemIds;
      await this.persist();
    },
    /** 设为封面：再次设置同一张时取消（恢复默认首图） */
    async setCover(collectionId: string, itemId: string) {
      const ui = useUiStore();
      const { t } = useI18n();
      const c = this.byId(collectionId);
      if (!c) return;
      c.coverItemId = c.coverItemId === itemId ? null : itemId;
      try {
        await this.persist();
        ui.toast('success', t('toast.coverUpdated'));
      } catch (e) {
        ui.toast('error', String(e));
      }
    },
    /** 恢复默认封面（集合首图） */
    async clearCover(collectionId: string) {
      const c = this.byId(collectionId);
      if (!c) return;
      c.coverItemId = null;
      await this.persist();
    },
  },
});
