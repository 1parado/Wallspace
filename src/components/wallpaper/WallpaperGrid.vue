<script setup lang="ts">
import type { WallpaperItem } from '../../types';
import WallpaperCard from './WallpaperCard.vue';
import { useUiStore } from '../../stores/ui';

const props = defineProps<{ items: WallpaperItem[] }>();
const ui = useUiStore();

// 捕获阶段记录点击时所在列表的顺序，供预览层上一张/下一张导航
function recordList() {
  ui.previewIds = props.items.map((i) => i.id);
}
</script>

<template>
  <div class="grid" @click.capture="recordList">
    <WallpaperCard v-for="item in items" :key="item.id" :item="item" />
  </div>
</template>

<style scoped>
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
</style>
