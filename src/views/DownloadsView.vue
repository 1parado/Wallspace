<script setup lang="ts">
import { ref } from 'vue';
import { useLibraryStore } from '../stores/library';
import { useUiStore } from '../stores/ui';
import { useI18n } from '../lib/i18n';
import { wallhavenSearch, type WhThumb } from '../lib/api';
import WallpaperGrid from '../components/wallpaper/WallpaperGrid.vue';
import EmptyState from '../components/common/EmptyState.vue';
import Icon from '../components/common/Icon.vue';

const lib = useLibraryStore();
const ui = useUiStore();
const { t } = useI18n();

const url = ref('');
// 「自动」(空值) 为默认：不强制选分类，交给标签与主色体系
const category = ref<string>('');

async function download() {
  const u = url.value.trim();
  if (!u || lib.importingUrl) return;
  const item = await lib.importUrl(u, category.value || null);
  if (item) {
    url.value = '';
    ui.previewId = item.id;
  }
}

// —— Wallhaven 搜索（SFW 公开接口） ——
const whQuery = ref('');
const whSorting = ref('relevance');
const whRes = ref('');
const whResults = ref<WhThumb[]>([]);
const whPage = ref(1);
const whSearching = ref(false);
const whError = ref('');
const whSearched = ref(false);
const importingId = ref<string | null>(null);

const WH_SORTS = [
  { id: 'relevance', labelKey: 'downloads.sortRelevance' },
  { id: 'toplist', labelKey: 'downloads.sortToplist' },
  { id: 'views', labelKey: 'downloads.sortViews' },
  { id: 'random', labelKey: 'downloads.sortRandom' },
];

const WH_RESOLUTIONS = [
  { id: '', labelKey: 'downloads.resAny' },
  { id: '1920x1080', label: '1920×1080' },
  { id: '2560x1440', label: '2560×1440' },
  { id: '3840x2160', label: '3840×2160' },
];

async function whSearch(reset = true) {
  if (whSearching.value) return;
  whSearching.value = true;
  whError.value = '';
  try {
    const page = reset ? 1 : whPage.value + 1;
    const res = await wallhavenSearch({
      query: whQuery.value.trim(),
      page,
      sorting: whSorting.value,
      atleast: whRes.value,
    });
    whPage.value = page;
    whResults.value = reset ? res : [...whResults.value, ...res];
    whSearched.value = true;
  } catch (e) {
    whError.value = String(e);
  } finally {
    whSearching.value = false;
  }
}

async function importWh(w: WhThumb) {
  if (importingId.value) return;
  importingId.value = w.id;
  try {
    const item = await lib.importUrl(w.path, category.value || null);
    if (item) {
      ui.toast('success', t('downloads.whImported'));
      ui.previewId = item.id;
    }
  } catch (e) {
    ui.toast('error', String(e));
  } finally {
    importingId.value = null;
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
            <option value="">{{ t('downloads.auto') }}</option>
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

    <!-- Wallhaven 搜索（SFW 公开接口） -->
    <div class="wh-panel glass">
      <p class="wh-title">
        <Icon name="globe" :size="14" />
        {{ t('downloads.whTitle') }}
      </p>
      <div class="url-row">
        <div class="url-field">
          <Icon name="search" :size="15" />
          <input
            v-model="whQuery"
            :placeholder="t('downloads.whPlaceholder')"
            spellcheck="false"
            @keydown.enter="whSearch()"
          />
        </div>
        <div class="cat-select">
          <select v-model="whSorting">
            <option v-for="s in WH_SORTS" :key="s.id" :value="s.id">{{ t(s.labelKey) }}</option>
          </select>
        </div>
        <div class="cat-select">
          <select v-model="whRes">
            <option v-for="r in WH_RESOLUTIONS" :key="r.id" :value="r.id">
              {{ 'label' in r ? r.label : t(r.labelKey as never) }}
            </option>
          </select>
        </div>
        <button class="btn-primary" :disabled="whSearching" @click="whSearch()">
          <Icon name="search" :size="14" />
          {{ whSearching ? t('downloads.whSearching') : t('downloads.whSearch') }}
        </button>
      </div>
      <p v-if="whError" class="wh-error">{{ whError }}</p>
      <p class="url-hint">{{ t('downloads.whSfw') }}</p>
    </div>

    <!-- 搜索结果 -->
    <div v-if="whResults.length" class="wh-results">
      <div class="wh-grid">
        <div v-for="w in whResults" :key="w.id" class="wh-card">
          <img :src="w.thumb" loading="lazy" draggable="false" />
          <div class="wh-overlay">
            <button class="wh-import" :disabled="importingId === w.id" @click="importWh(w)">
              <Icon name="download" :size="14" />
              {{ importingId === w.id ? t('downloads.downloading') : t('downloads.whImport') }}
            </button>
          </div>
          <div class="wh-meta">
            <span>{{ w.resolution }}</span>
            <span class="wh-dots">
              <i v-for="c in w.colors.slice(0, 4)" :key="c" :style="{ background: c }" />
            </span>
          </div>
        </div>
      </div>
      <div class="wh-more">
        <button class="pill" :disabled="whSearching" @click="whSearch(false)">
          {{ whSearching ? t('downloads.whSearching') : t('downloads.whLoadMore') }}
        </button>
      </div>
    </div>
    <p v-else-if="whSearched && !whSearching" class="wh-empty">{{ t('downloads.whNone') }}</p>

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

.wh-panel {
  border-radius: var(--radius-overlay);
  padding: 20px 24px;
}

.wh-title {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 13.5px;
  font-weight: 600;
  color: var(--text-1);
  margin-bottom: 14px;
}

.wh-error {
  margin-top: 10px;
  font-size: 12.5px;
  color: var(--danger, #e5484d);
}

.wh-results {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.wh-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 14px;
}

.wh-card {
  position: relative;
  border-radius: var(--radius-card, 14px);
  overflow: hidden;
  background: var(--fill-subtle);
  border: 1px solid var(--stroke);
  aspect-ratio: 16 / 10;
  transition:
    transform var(--dur-1) var(--ease-out),
    border-color var(--dur-1) var(--ease-out);
}

.wh-card:hover {
  transform: translateY(-2px);
  border-color: var(--stroke-strong);
}

.wh-card img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.wh-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.35);
  opacity: 0;
  transition: opacity var(--dur-1) var(--ease-out);
}

.wh-card:hover .wh-overlay {
  opacity: 1;
}

.wh-import {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border-radius: 100px;
  border: none;
  background: rgba(255, 255, 255, 0.92);
  color: #111;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: transform var(--dur-1) var(--ease-out);
}

.wh-import:hover {
  transform: scale(1.04);
}

.wh-import:disabled {
  opacity: 0.6;
  cursor: default;
}

.wh-meta {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  font-size: 11px;
  color: rgba(255, 255, 255, 0.92);
  background: linear-gradient(transparent, rgba(0, 0, 0, 0.55));
}

.wh-dots {
  display: inline-flex;
  gap: 4px;
}

.wh-dots i {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  border: 1px solid rgba(255, 255, 255, 0.4);
  display: inline-block;
}

.wh-more {
  display: flex;
  justify-content: center;
}

.pill {
  padding: 8px 20px;
  border-radius: 100px;
  border: 1px solid var(--stroke);
  background: var(--fill-subtle);
  color: var(--text-1);
  font-size: 13px;
  cursor: pointer;
  transition: background var(--dur-1) var(--ease-out);
}

.pill:hover {
  background: var(--fill-hover);
}

.pill:disabled {
  opacity: 0.6;
  cursor: default;
}

.wh-empty {
  text-align: center;
  font-size: 13px;
  color: var(--text-3);
  padding: 20px 0;
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
