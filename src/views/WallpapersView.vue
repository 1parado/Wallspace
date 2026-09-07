<script setup lang="ts">
import { computed } from 'vue';
import { useLibraryStore } from '../stores/library';
import { useUiStore } from '../stores/ui';
import { useI18n } from '../lib/i18n';
import WallpaperGrid from '../components/wallpaper/WallpaperGrid.vue';
import EmptyState from '../components/common/EmptyState.vue';

const lib = useLibraryStore();
const ui = useUiStore();
const { t } = useI18n();

const filtered = computed(() => {
  let items = lib.items;
  if (ui.categoryFilter) {
    items = items.filter((i) => (i.category ?? null) === ui.categoryFilter);
  }
  const q = ui.search.trim().toLowerCase();
  if (q) {
    items = items.filter(
      (i) =>
        i.title.toLowerCase().includes(q) ||
        (i.category ?? '').toLowerCase().includes(q) ||
        (i.prompt ?? '').toLowerCase().includes(q)
    );
  }
  return items;
});

const hasAny = computed(() => lib.items.length > 0);
const isFiltering = computed(() => !!ui.categoryFilter || !!ui.search.trim());
</script>

<template>
  <div class="wallpapers">
    <div class="chips">
      <button :class="{ active: !ui.categoryFilter }" @click="ui.categoryFilter = null">
        {{ t('toolbar.allWallpapers') }}
      </button>
      <button
        v-for="c in ui.categories"
        :key="c"
        :class="{ active: ui.categoryFilter === c }"
        @click="ui.categoryFilter = ui.categoryFilter === c ? null : c"
      >
        {{ t(`cat.${c.toLowerCase()}`) }}
      </button>
    </div>

    <WallpaperGrid v-if="filtered.length" :items="filtered" />

    <EmptyState
      v-else-if="!hasAny"
      :title="t('empty.library.title')"
      :subtitle="t('empty.library.sub')"
      :action-label="t('hero.create')"
      @action="ui.goto('create')"
    />
    <EmptyState
      v-else-if="isFiltering"
      :title="t('empty.nothing.title')"
      :subtitle="t('empty.nothing.sub')"
      :action-label="t('empty.showAll')"
      @action="ui.categoryFilter = null; ui.search = ''"
    />
  </div>
</template>

<style scoped>
.wallpapers {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.chips button {
  font-size: 13px;
  color: var(--text-3);
  border: 1px solid var(--stroke);
  border-radius: 100px;
  padding: 6px 16px;
  transition: all var(--dur-1) var(--ease-out);
}

.chips button:hover {
  color: var(--text-1);
  border-color: var(--stroke-strong);
}

.chips button.active {
  color: var(--text-1);
  background: var(--fill-active);
  border-color: var(--stroke-strong);
}
</style>
