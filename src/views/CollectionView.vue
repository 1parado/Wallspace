<script setup lang="ts">
import { computed } from 'vue';
import { useCollectionsStore } from '../stores/collections';
import { useLibraryStore } from '../stores/library';
import { useUiStore } from '../stores/ui';
import { useI18n } from '../lib/i18n';
import WallpaperGrid from '../components/wallpaper/WallpaperGrid.vue';
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
</script>

<template>
  <div v-if="collection" class="collection-view">
    <div class="head">
      <h2 class="name">{{ collection.name }}</h2>
      <span class="count-label">
        {{ t('collections.itemCount', { n: items.length }) }}
      </span>
    </div>

    <WallpaperGrid v-if="items.length" :items="items" />

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
</style>
