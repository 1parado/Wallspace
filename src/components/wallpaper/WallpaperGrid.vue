<script setup lang="ts">
import { computed } from 'vue';
import type { WallpaperItem } from '../../types';
import WallpaperCard from './WallpaperCard.vue';
import { useUiStore } from '../../stores/ui';

const props = defineProps<{ items: WallpaperItem[] }>();
const ui = useUiStore();

// 捕获阶段记录点击时所在列表的顺序，供预览层上一张/下一张导航
function recordList() {
  ui.previewIds = props.items.map((i) => i.id);
}

// 网格密度 → CSS 变量（常规屏 / 超宽屏两档）
const DENSITY = {
  compact: ['200px', '240px'],
  cozy: ['280px', '340px'],
  roomy: ['380px', '440px'],
} as const;

const gridStyle = computed(() => {
  const [min, minWide] = DENSITY[ui.gridDensity];
  return { '--grid-min': min, '--grid-min-wide': minWide };
});
</script>

<template>
  <div class="grid" :style="gridStyle" @click.capture="recordList">
    <WallpaperCard v-for="item in items" :key="item.id" :item="item" />
  </div>
</template>

<style scoped>
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(var(--grid-min, 280px), 1fr));
  gap: 18px;
}

@media (min-width: 1900px) {
  .grid {
    grid-template-columns: repeat(auto-fill, minmax(var(--grid-min-wide, 340px), 1fr));
  }
}
</style>

