<script setup lang="ts">
import { computed, ref } from 'vue';
import type { WallpaperItem } from '../types';
import { useCollectionsStore } from '../stores/collections';
import { useLibraryStore } from '../stores/library';
import { useUiStore } from '../stores/ui';
import { useI18n } from '../lib/i18n';
import { assetUrl } from '../lib/api';
import { sortItems } from '../lib/sortItems';
import WallpaperCard from '../components/wallpaper/WallpaperCard.vue';
import GridToolbar from '../components/common/GridToolbar.vue';
import EmptyState from '../components/common/EmptyState.vue';
import Icon from '../components/common/Icon.vue';

const ui = useUiStore();
const lib = useLibraryStore();
const collections = useCollectionsStore();
const { t } = useI18n();

const collection = computed(() => collections.byId(ui.activeCollectionId));

/** 按 itemIds 顺序取条目（集合顺序即用户整理顺序） */
const items = computed(() => {
  if (!collection.value) return [];
  return collection.value.itemIds
    .map((id) => lib.byId(id))
    .filter((i): i is NonNullable<typeof i> => !!i);
});

// —— 排序 + 随机换一张 ——
type CollSort = 'custom' | 'newest' | 'oldest' | 'name' | 'resolution' | 'random';
const collSort = ref<CollSort>('custom');
const sortSeed = ref(0);

const COLL_SORTS = [
  { id: 'custom', labelKey: 'facets.sortCustom' },
  { id: 'newest', labelKey: 'facets.sortNewest' },
  { id: 'oldest', labelKey: 'facets.sortOldest' },
  { id: 'name', labelKey: 'facets.sortName' },
  { id: 'resolution', labelKey: 'facets.sortResolution' },
  { id: 'random', labelKey: 'facets.sortRandom' },
] as const;

/** 展示顺序：custom = 集合原始顺序，其余走共享排序工具 */
const displayed = computed(() => sortItems(items.value, collSort.value, sortSeed.value));

function setCollSort(m: CollSort) {
  // 重复点「随机」= 重新洗牌
  if (m === 'random' && collSort.value === 'random') {
    sortSeed.value = Date.now();
    return;
  }
  collSort.value = m;
}

/** 随机换一张：从集合当前展示顺序中随机挑一张立即应用 */
async function applyRandom() {
  if (!displayed.value.length || lib.applyingId) return;
  const pick = displayed.value[Math.floor(Math.random() * displayed.value.length)];
  await lib.apply(pick.id);
}

// —— 拖拽排序 ——
const dragIndex = ref<number | null>(null);
const overIndex = ref<number | null>(null);

function onDragStart(i: number) {
  dragIndex.value = i;
}

function onDragOver(i: number) {
  overIndex.value = i;
}

function onDragEnd() {
  dragIndex.value = null;
  overIndex.value = null;
}

async function onDrop(to: number) {
  const from = dragIndex.value;
  onDragEnd();
  if (from == null || from === to || !collection.value) return;
  const ids = [...collection.value.itemIds];
  // from/to 都按「展示列表条目在 itemIds 中的原始下标」换算，避免失效条目错位
  const liveIds = displayed.value.map((i) => i.id);
  const moved = liveIds[from];
  const target = liveIds[to];
  const fromRaw = ids.indexOf(moved);
  const toRaw = ids.indexOf(target);
  if (fromRaw < 0 || toRaw < 0) return;
  ids.splice(fromRaw, 1);
  ids.splice(toRaw, 0, moved);
  await collections.reorder(collection.value.id, ids);
}

// —— 管理模式（批量移除） ——
const managing = ref(false);
const selectedIds = ref<Set<string>>(new Set());

function toggleSelect(id: string) {
  const next = new Set(selectedIds.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  selectedIds.value = next;
}

function exitManage() {
  managing.value = false;
  selectedIds.value = new Set();
}

async function removeSelected() {
  if (!collection.value || !selectedIds.value.size) return;
  for (const id of [...selectedIds.value]) {
    await collections.removeItem(collection.value.id, id);
  }
  selectedIds.value = new Set();
}

/** 管理模式下点击卡片 = 切换选中，而非打开预览 */
function onCellClickCapture(id: string, e: MouseEvent) {
  if (!managing.value) return;
  e.preventDefault();
  e.stopPropagation();
  toggleSelect(id);
}

// —— 封面与快速应用 ——
const cover = computed(() => items.value[0] ?? null);

async function quickApply() {
  const first = items.value[0];
  if (first && lib.applyingId !== first.id) await lib.apply(first.id);
}
</script>

<template>
  <div v-if="collection" class="collection-view">
    <div class="head">
      <img v-if="cover" class="cover" :src="assetUrl(cover.filePath)" alt="" />
      <h2 class="name">{{ collection.name }}</h2>
      <span class="count-label">
        {{ t('collections.itemCount', { n: items.length }) }}
      </span>
      <span v-if="items.length > 1 && !managing && collSort === 'custom'" class="hint">
        {{ t('collections.reorderHint') }}
      </span>
      <div class="head-actions">
        <button
          v-if="managing && selectedIds.size"
          class="pill danger"
          @click="removeSelected"
        >
          {{ t('collections.removeSelected', { n: selectedIds.size }) }}
        </button>
        <button
          class="pill"
          :class="{ active: managing }"
          :disabled="!items.length"
          @click="managing ? exitManage() : (managing = true)"
        >
          {{ managing ? t('collections.manageDone') : t('collections.manage') }}
        </button>
        <button
          class="pill primary"
          :disabled="!items.length || lib.applyingId === items[0].id"
          @click="quickApply"
        >
          {{ lib.applyingId === items[0]?.id ? t('preview.applying') : t('collections.quickApply') }}
        </button>
      </div>
    </div>

    <!-- 结果栏：排序 + 随机换一张 -->
    <GridToolbar
      :count="displayed.length"
      :sorts="COLL_SORTS"
      :model-value="collSort"
      :seed="sortSeed"
      :applying="lib.applyingId"
      @update:model-value="setCollSort($event as never)"
      @reshuffle="setCollSort('random')"
      @apply="applyRandom"
    />

    <div v-if="displayed.length" class="grid" @click.capture="ui.previewIds = displayed.map((i) => i.id)">
      <div
        v-for="(item, i) in displayed"
        :key="item.id"
        class="cell"
        :class="{
          dragging: dragIndex === i,
          'drop-target': overIndex === i && dragIndex !== null && dragIndex !== i,
          selectable: managing,
          selected: managing && selectedIds.has(item.id),
        }"
        :draggable="!managing && collSort === 'custom'"
        @dragstart="onDragStart(i)"
        @dragover.prevent="onDragOver(i)"
        @drop.prevent="onDrop(i)"
        @dragend="onDragEnd"
        @click.capture="onCellClickCapture(item.id, $event)"
      >
        <span
          v-if="managing"
          class="check"
          :class="{ on: selectedIds.has(item.id) }"
        >
          <Icon v-if="selectedIds.has(item.id)" name="check" :size="12" />
        </span>
        <WallpaperCard :item="item as WallpaperItem" />
      </div>
    </div>

    <EmptyState
      v-else
      :title="t('collections.empty.title')"
      :subtitle="t('collections.empty.sub')"
      :action-label="t('nav.wallpapers')"
      @action="ui.goto('wallpapers')"
    />
  </div>
</template>

<style scoped>
.collection-view {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.head {
  display: flex;
  align-items: baseline;
  gap: 12px;
}

.head-actions {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 8px;
}

.pill {
  font-size: 12.5px;
  color: var(--text-2);
  border: 1px solid var(--stroke);
  border-radius: 100px;
  padding: 6px 15px;
  transition: all var(--dur-1) var(--ease-out);
}

.pill:hover:not(:disabled) {
  color: var(--text-1);
  border-color: var(--stroke-strong);
}

.pill:disabled {
  opacity: 0.5;
}

.pill.active {
  color: var(--text-1);
  background: var(--fill-active);
  border-color: var(--stroke-strong);
}

.pill.primary {
  color: #fff;
  background: var(--accent, #3b82f6);
  border-color: transparent;
}

.pill.danger {
  color: var(--accent-heart);
  border-color: var(--accent-heart);
}

.cover {
  width: 44px;
  height: 30px;
  object-fit: cover;
  border-radius: 7px;
  align-self: center;
  border: 1px solid var(--stroke);
}

.cell {
  position: relative;
  border-radius: var(--radius-card);
}

.cell.selectable {
  cursor: pointer;
}

.cell.selected {
  outline: 2px solid var(--accent, #3b82f6);
  outline-offset: 3px;
}

.check {
  position: absolute;
  top: 8px;
  left: 8px;
  z-index: 3;
  display: grid;
  place-items: center;
  width: 20px;
  height: 20px;
  border-radius: 6px;
  border: 1.5px solid rgba(255, 255, 255, 0.75);
  background: rgba(0, 0, 0, 0.3);
  color: #fff;
  transition: background var(--dur-1) var(--ease-out);
}

.check.on {
  background: var(--accent, #3b82f6);
  border-color: transparent;
}

.name {
  font-size: 18px;
  font-weight: 600;
  letter-spacing: -0.02em;
}

.count-label {
  font-size: 12.5px;
  color: var(--text-3);
}

.hint {
  margin-left: auto;
  font-size: 12px;
  color: var(--text-3);
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 18px;
}

@media (min-width: 1900px) {
  .grid {
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  }
}

.cell {
  position: relative;
  border-radius: var(--radius-card);
}

.cell.dragging {
  opacity: 0.4;
}

.cell.drop-target {
  outline: 2px dashed var(--stroke-strong);
  outline-offset: 3px;
}
</style>
