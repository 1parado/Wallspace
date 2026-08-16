<script setup lang="ts">
import { open } from '@tauri-apps/plugin-dialog';
import { useLibraryStore } from '../stores/library';
import { useI18n } from '../lib/i18n';
import WallpaperGrid from '../components/wallpaper/WallpaperGrid.vue';
import EmptyState from '../components/common/EmptyState.vue';
import Icon from '../components/common/Icon.vue';

const lib = useLibraryStore();
const { t } = useI18n();

async function pickFiles() {
  const picked = await open({
    multiple: true,
    filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp'] }],
  });
  if (!picked) return;
  const paths = Array.isArray(picked) ? picked : [picked];
  await lib.importFiles(paths);
}
</script>

<template>
  <div class="imports">
    <div class="drop-panel glass" role="button" tabindex="0" @click="pickFiles" @keydown.enter="pickFiles">
      <div class="drop-inner">
        <div class="drop-icon"><Icon name="upload" :size="18" /></div>
        <div>
          <p class="drop-title">{{ t('imports.title') }}</p>
          <p class="drop-sub">{{ t('imports.sub') }}</p>
        </div>
        <span v-if="lib.importingFiles" class="busy">{{ t('imports.importing') }}</span>
      </div>
    </div>

    <WallpaperGrid v-if="lib.imports.length" :items="lib.imports" />
    <EmptyState
      v-else
      :title="t('empty.imports.title')"
      :subtitle="t('empty.imports.sub')"
      :action-label="t('empty.browse')"
      @action="pickFiles"
    />
  </div>
</template>

<style scoped>
.imports {
  display: flex;
  flex-direction: column;
  gap: 22px;
}

.drop-panel {
  border-radius: var(--radius-overlay);
  padding: 20px 24px;
  cursor: pointer;
  transition:
    border-color var(--dur-1) var(--ease-out),
    background var(--dur-1) var(--ease-out);
}

.drop-panel:hover {
  border-color: var(--stroke-strong);
}

.drop-inner {
  display: flex;
  align-items: center;
  gap: 16px;
}

.drop-icon {
  width: 42px;
  height: 42px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  background: var(--fill-subtle);
  border: 1px solid var(--stroke-strong);
  color: var(--text-1);
  flex-shrink: 0;
}

.drop-title {
  font-size: 15px;
  font-weight: 600;
}

.drop-sub {
  font-size: 12.5px;
  color: var(--text-3);
  margin-top: 2px;
}

.busy {
  margin-left: auto;
  font-size: 12.5px;
  color: var(--text-2);
}
</style>
