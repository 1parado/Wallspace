<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import type { WallpaperItem } from '../../types';
import WallpaperCard from './WallpaperCard.vue';
import { useUiStore } from '../../stores/ui';

const props = defineProps<{ items: WallpaperItem[] }>();
const ui = useUiStore();

// 捕获阶段记录点击时所在列表的顺序，供预览层上一张/下一张导航
function recordList() {
  ui.previewIds = props.items.map((i) => i.id);
}

// —— 增量挂载：大库不一次性渲染全部卡片，滚动临近底部再挂下一批 ——
const CHUNK = 80;
const shown = ref(CHUNK);
const shownItems = computed(() => props.items.slice(0, shown.value));
const sentinelEl = ref<HTMLElement | null>(null);
let io: IntersectionObserver | null = null;

function mountMore(entries: IntersectionObserverEntry[]) {
  if (entries.some((e) => e.isIntersecting)) {
    shown.value = Math.min(props.items.length, shown.value + CHUNK);
  }
}

onMounted(() => {
  if (!('IntersectionObserver' in window)) {
    shown.value = props.items.length;
    return;
  }
  io = new IntersectionObserver(mountMore, { rootMargin: '1200px 0px' });
  if (sentinelEl.value) io.observe(sentinelEl.value);
});

onBeforeUnmount(() => io?.disconnect());

// 列表变化（切视图/筛选/导入）后重置挂载量，避免旧计数越积越大
watch(
  () => props.items,
  () => {
    shown.value = Math.min(CHUNK, props.items.length);
  }
);

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
    <WallpaperCard v-for="item in shownItems" :key="item.id" :item="item" />
    <div ref="sentinelEl" class="sentinel" aria-hidden="true" />
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

.sentinel {
  grid-column: 1 / -1;
  height: 1px;
}
</style>
