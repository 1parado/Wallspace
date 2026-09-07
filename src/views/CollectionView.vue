<script setup lang="ts">
import { computed, ref } from 'vue';
import type { WallpaperItem } from '../types';
import { useCollectionsStore } from '../stores/collections';
import { useLibraryStore } from '../stores/library';
import { useUiStore } from '../stores/ui';
import { useI18n } from '../lib/i18n';
import WallpaperCard from '../components/wallpaper/WallpaperCard.vue';
import EmptyState from '../components/common/EmptyState.vue';

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
  // from/to 都按「过滤后条目在 itemIds 中的原始下标」换算，避免失效条目错位
  const liveIds = items.value.map((i) => i.id);
  const moved = liveIds[from];
  const target = liveIds[to];
  const fromRaw = ids.indexOf(moved);
  const toRaw = ids.indexOf(target);
  if (fromRaw < 0 || toRaw < 0) return;
  ids.splice(fromRaw, 1);
  ids.splice(toRaw, 0, moved);
  await collections.reorder(collection.value.id, ids);
}
</script>

<template>
  <div v-if="collection" class="collection-view">
    <div class="head">
      <h2 class="name">{{ collection.name }}</h2>
      <span class="count-label">
        {{ t('collections.itemCount', { n: items.length }) }}
      </span>
      <span v-if="items.length > 1" class="hint">{{ t('collections.reorderHint') }}</span>
    </div>

    <div v-if="items.length" class="grid">
      <div
        v-for="(item, i) in items"
        :key="item.id"
        class="cell"
        :class="{
          dragging: dragIndex === i,
          'drop-target': overIndex === i && dragIndex !== null && dragIndex !== i,
        }"
        draggable="true"
        @dragstart="onDragStart(i)"
        @dragover.prevent="onDragOver(i)"
        @drop.prevent="onDrop(i)"
        @dragend="onDragEnd"
      >
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
