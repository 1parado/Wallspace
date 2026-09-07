<script setup lang="ts">
import { computed } from 'vue';
import { useLibraryStore } from '../stores/library';
import { useUiStore } from '../stores/ui';
import { useI18n } from '../lib/i18n';
import { sortItems } from '../lib/sortItems';
import WallpaperGrid from '../components/wallpaper/WallpaperGrid.vue';
import GridToolbar from '../components/common/GridToolbar.vue';
import EmptyState from '../components/common/EmptyState.vue';

const lib = useLibraryStore();
const ui = useUiStore();
const { t } = useI18n();

const SORTS = [
  { id: 'newest', labelKey: 'facets.sortNewest' },
  { id: 'oldest', labelKey: 'facets.sortOldest' },
  { id: 'name', labelKey: 'facets.sortName' },
  { id: 'resolution', labelKey: 'facets.sortResolution' },
  { id: 'random', labelKey: 'facets.sortRandom' },
] as const;

const sorted = computed(() => sortItems(lib.favorites, ui.sortMode, ui.sortSeed));

/** 随机换一张：从收藏中随机挑一张立即应用 */
async function applyRandom() {
  if (!sorted.value.length || lib.applyingId) return;
  const pick = sorted.value[Math.floor(Math.random() * sorted.value.length)];
  await lib.apply(pick.id);
}
</script>

<template>
  <div v-if="lib.favorites.length" class="favorites">
    <GridToolbar
      :count="lib.favorites.length"
      :sorts="SORTS"
      :model-value="ui.sortMode"
      :seed="ui.sortSeed"
      :applying="lib.applyingId"
      @update:model-value="ui.setSort($event as never)"
      @reshuffle="ui.setSort('random')"
      @apply="applyRandom"
    />
    <WallpaperGrid :items="sorted" />
  </div>
  <EmptyState
    v-else
    :title="t('empty.favorites.title')"
    :subtitle="t('empty.favorites.sub')"
    :action-label="t('empty.explore')"
    @action="ui.goto('wallpapers')"
  />
</template>

<style scoped>
.favorites {
  display: flex;
  flex-direction: column;
  gap: 20px;
}
</style>
