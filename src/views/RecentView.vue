<script setup lang="ts">
import { computed, ref } from 'vue';
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
  { id: 'recent', labelKey: 'facets.sortRecentApplied' },
  { id: 'name', labelKey: 'facets.sortName' },
  { id: 'resolution', labelKey: 'facets.sortResolution' },
  { id: 'random', labelKey: 'facets.sortRandom' },
] as const;

const recentSort = ref('recent');
const sortSeed = ref(0);

const sorted = computed(() => sortItems(lib.recentApplied, recentSort.value as never, sortSeed.value));

/** 随机换一张：从最近应用过的壁纸中随机挑一张立即应用 */
async function applyRandom() {
  if (!sorted.value.length || lib.applyingId) return;
  const pick = sorted.value[Math.floor(Math.random() * sorted.value.length)];
  await lib.apply(pick.id);
}
</script>

<template>
  <div v-if="lib.recentApplied.length" class="recent">
    <GridToolbar
      :count="lib.recentApplied.length"
      :sorts="SORTS"
      :model-value="recentSort"
      :seed="sortSeed"
      :applying="lib.applyingId"
      @update:model-value="recentSort = $event"
      @reshuffle="sortSeed = Date.now()"
      @apply="applyRandom"
    />
    <WallpaperGrid :items="sorted" />
  </div>
  <EmptyState
    v-else
    :title="t('empty.recent.title')"
    :subtitle="t('empty.recent.sub')"
    :action-label="t('empty.explore')"
    @action="ui.goto('wallpapers')"
  />
</template>

<style scoped>
.recent {
  display: flex;
  flex-direction: column;
  gap: 20px;
}
</style>
