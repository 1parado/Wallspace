<script setup lang="ts">
import { useLibraryStore } from '../stores/library';
import { useUiStore } from '../stores/ui';
import { useI18n } from '../lib/i18n';
import WallpaperGrid from '../components/wallpaper/WallpaperGrid.vue';
import EmptyState from '../components/common/EmptyState.vue';

const lib = useLibraryStore();
const ui = useUiStore();
const { t } = useI18n();
</script>

<template>
  <WallpaperGrid v-if="lib.favorites.length" :items="lib.favorites" />
  <EmptyState
    v-else
    :title="t('empty.favorites.title')"
    :subtitle="t('empty.favorites.sub')"
    :action-label="t('empty.explore')"
    @action="ui.goto('wallpapers')"
  />
</template>
