<script setup lang="ts">
import { ref } from 'vue';
import { useLibraryStore } from '../stores/library';
import { useUiStore } from '../stores/ui';
import { useI18n } from '../lib/i18n';
import WallpaperGrid from '../components/wallpaper/WallpaperGrid.vue';
import EmptyState from '../components/common/EmptyState.vue';
import Icon from '../components/common/Icon.vue';

const lib = useLibraryStore();
const ui = useUiStore();
const { t } = useI18n();

const url = ref('');
const category = ref<string>('Nature');

async function download() {
  const u = url.value.trim();
  if (!u || lib.importingUrl) return;
  const item = await lib.importUrl(u, category.value);
  if (item) {
    url.value = '';
    ui.previewId = item.id;
  }
}
</script>

<template>
  <div class="downloads">
    <div class="url-panel glass">
      <div class="url-row">
        <div class="url-field">
          <Icon name="link" :size="15" />
          <input
            v-model="url"
            :placeholder="t('downloads.placeholder')"
            spellcheck="false"
            @keydown.enter="download"
          />
        </div>
        <div class="cat-select">
          <select v-model="category">
            <option v-for="c in ui.categories" :key="c" :value="c">
              {{ t(`cat.${c.toLowerCase()}`) }}
            </option>
          </select>
        </div>
        <button class="btn-primary" :disabled="!url.trim() || lib.importingUrl" @click="download">
          <Icon name="download" :size="14" />
          {{ lib.importingUrl ? t('downloads.downloading') : t('downloads.import') }}
        </button>
      </div>
      <p class="url-hint">{{ t('downloads.hint') }}</p>
    </div>

    <div v-if="lib.downloads.length" class="badges">
      <span class="badge"><Icon name="check" :size="12" /> {{ t('downloads.offline') }}</span>
      <span class="badge"><Icon name="folder" :size="12" /> {{ t('downloads.local') }}</span>
      <span class="badge"><Icon name="globe" :size="12" /> {{ t('downloads.noAccount') }}</span>
    </div>

    <WallpaperGrid v-if="lib.downloads.length" :items="lib.downloads" />
    <EmptyState
      v-else
      :title="t('empty.library.title')"
      :subtitle="t('empty.downloads.sub')"
      :action-label="t('empty.explore')"
      @action="ui.goto('wallpapers')"
    />
  </div>
</template>

<style scoped>
.downloads {
  display: flex;
  flex-direction: column;
  gap: 22px;
}

.url-panel {
  border-radius: var(--radius-overlay);
  padding: 20px 24px;
}

.url-row {
  display: flex;
  gap: 12px;
  align-items: center;
}

.url-field {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 14px;
  border-radius: var(--radius-btn);
  background: var(--fill-subtle);
  border: 1px solid var(--stroke);
  color: var(--text-3);
  transition:
    border-color var(--dur-1) var(--ease-out),
    background var(--dur-1) var(--ease-out);
}

.url-field:focus-within {
  border-color: var(--stroke-strong);
  background: var(--fill-hover);
}

.url-field input {
  flex: 1;
  min-width: 0;
  background: none;
  border: none;
  outline: none;
  font-size: 13.5px;
  color: var(--text-1);
}

.url-field input::placeholder {
  color: var(--text-3);
}

.cat-select select {
  appearance: none;
  background: var(--fill-subtle);
  border: 1px solid var(--stroke);
  border-radius: var(--radius-btn);
  padding: 9px 30px 9px 14px;
  font-size: 13px;
  color: var(--text-1);
  outline: none;
  cursor: pointer;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%23888888' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 10px center;
}

.cat-select select option {
  background: var(--bg-3);
  color: var(--text-1);
}

.url-hint {
  margin-top: 10px;
  font-size: 12px;
  color: var(--text-3);
}

.badges {
  display: flex;
  gap: 8px;
}

.badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-3);
  border: 1px solid var(--stroke);
  border-radius: 100px;
  padding: 4px 12px;
}
</style>
