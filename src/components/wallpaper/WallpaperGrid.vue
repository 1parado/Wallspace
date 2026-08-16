<script setup lang="ts">
import type { WallpaperItem } from '../../types';
import WallpaperCard from './WallpaperCard.vue';

defineProps<{ items: WallpaperItem[] }>();
</script>

<template>
  <div class="grid">
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
