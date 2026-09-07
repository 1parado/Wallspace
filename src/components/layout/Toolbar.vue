<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useUiStore } from '../../stores/ui';
import { useSettingsStore } from '../../stores/settings';
import { useI18n } from '../../lib/i18n';
import Icon from '../common/Icon.vue';

const ui = useUiStore();
const settings = useSettingsStore();
const { t } = useI18n();
const win = getCurrentWindow();

const subtitle = computed(() => {
  if (ui.view === 'wallpapers') {
    if (ui.search) return t('toolbar.searchResult', { q: ui.search });
    if (ui.categoryFilter) return t('toolbar.category');
    return t('toolbar.allWallpapers');
  }
  if (ui.view === 'discover') return t('toolbar.curated');
  if (ui.view === 'create') return t('toolbar.generate');
  if (ui.view === 'downloads') return t('toolbar.offlineReady');
  if (ui.view === 'favorites') return t('toolbar.favoritesSub');
  if (ui.view === 'recent') return t('toolbar.recentSub');
  return '';
});

const searchEl = ref<HTMLInputElement | null>(null);

// —— 搜索历史 ——
const historyOpen = ref(false);
let searchTimer: number | undefined;

// 输入停顿 900ms 后记录搜索词（去重置顶）
watch(
  () => ui.search,
  (q) => {
    clearTimeout(searchTimer);
    searchTimer = window.setTimeout(() => ui.rememberSearch(q), 900);
  }
);

function onSearchBlur() {
  // 延迟关闭，给历史项的 mousedown 留时间
  window.setTimeout(() => (historyOpen.value = false), 160);
}

function useHistory(q: string) {
  ui.search = q;
  historyOpen.value = false;
}

function onGlobalKey(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
    e.preventDefault();
    ui.view = 'wallpapers';
    ui.categoryFilter = null;
    nextTick(() => searchEl.value?.focus());
  } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'b') {
    e.preventDefault();
    settings.sidebarHidden = !settings.sidebarHidden;
  }
}

onMounted(() => window.addEventListener('keydown', onGlobalKey));
onUnmounted(() => window.removeEventListener('keydown', onGlobalKey));

const displayOpen = ref(false);

const displayLabel = computed(() => {
  if (!ui.selectedDisplay) return t('toolbar.allDisplays');
  const m = ui.monitors.find((x) => x.id === ui.selectedDisplay);
  return m?.name ?? t('toolbar.display');
});

function pickDisplay(id: string | null) {
  ui.selectedDisplay = id;
  displayOpen.value = false;
}

const maximized = ref(false);
async function syncMaximized() {
  maximized.value = await win.isMaximized();
}
onMounted(() => {
  syncMaximized();
  window.addEventListener('resize', syncMaximized);
});
onUnmounted(() => window.removeEventListener('resize', syncMaximized));

function toggleTheme() {
  settings.setTheme(settings.resolvedDark ? 'light' : 'dark');
}
</script>

<template>
  <header class="toolbar">
    <div class="left" data-tauri-drag-region>
      <button
        class="icon-only"
        :title="t('toolbar.toggleSidebar') + ' (Ctrl+B)'"
        @click="settings.sidebarHidden = !settings.sidebarHidden"
      >
        <Icon name="panel-left" :size="16" />
      </button>
      <span class="subtitle" data-tauri-drag-region>{{ subtitle }}</span>
    </div>

    <div class="center">
      <div class="search-wrap">
        <div class="search" @focusin="historyOpen = true" @focusout="onSearchBlur">
          <Icon name="search" :size="15" />
          <input
            ref="searchEl"
            v-model="ui.search"
            :placeholder="t('toolbar.search')"
            spellcheck="false"
          />
          <kbd>Ctrl K</kbd>
        </div>
        <Transition name="menu">
          <div v-if="historyOpen && ui.searchHistory.length" class="menu glass search-menu">
            <div class="menu-head">
              <span>{{ t('toolbar.searchHistory') }}</span>
              <button @click="ui.clearSearchHistory()">{{ t('toolbar.clearHistory') }}</button>
            </div>
            <button
              v-for="q in ui.searchHistory"
              :key="q"
              class="menu-item"
              @mousedown.prevent="useHistory(q)"
            >
              <Icon name="search" :size="13" />
              <span class="menu-name">{{ q }}</span>
            </button>
          </div>
        </Transition>
      </div>
    </div>

    <div class="right">
      <button
        class="icon-only"
        :title="t('toolbar.toggleTheme')"
        @click="toggleTheme"
      >
        <Icon :name="settings.resolvedDark ? 'sun' : 'moon'" :size="16" />
      </button>

      <div class="display-picker">
        <button class="pill" @click="displayOpen = !displayOpen">
          <Icon name="monitor" :size="15" />
          <span class="pill-label">{{ displayLabel }}</span>
          <Icon name="chevron-down" :size="13" />
        </button>
        <Transition name="menu">
          <div v-if="displayOpen" class="menu glass">
            <button class="menu-item" :class="{ active: !ui.selectedDisplay }" @click="pickDisplay(null)">
              <span class="radio" />
              <span>{{ t('toolbar.allDisplays') }}</span>
            </button>
            <button
              v-for="m in ui.monitors"
              :key="m.id"
              class="menu-item"
              :class="{ active: ui.selectedDisplay === m.id }"
              @click="pickDisplay(m.id)"
            >
              <span class="radio" />
              <span class="menu-name">{{ m.name }}</span>
              <span class="menu-meta">{{ m.width }}×{{ m.height }}</span>
            </button>
          </div>
        </Transition>
      </div>

      <button class="icon-only" :title="t('toolbar.settings')" @click="ui.settingsOpen = true">
        <Icon name="settings" :size="17" />
      </button>
    </div>

    <div class="window-controls" data-tauri-drag-region="false">
      <button class="wc" :title="t('toolbar.minimize')" @click="win.minimize()">
        <Icon name="minus" :size="14" />
      </button>
      <button class="wc" :title="maximized ? t('toolbar.restore') : t('toolbar.maximize')" @click="win.toggleMaximize()">
        <Icon :name="maximized ? 'restore' : 'square'" :size="12" />
      </button>
      <button class="wc close" :title="t('toolbar.close')" @click="win.close()">
        <Icon name="x" :size="14" />
      </button>
    </div>
  </header>
</template>

<style scoped>
.toolbar {
  position: relative;
  /* backdrop-filter 会形成独立层叠上下文，必须显式抬高，
     否则 content 区（DOM 靠后 + 卡片 hover transform）会盖住下拉菜单 */
  z-index: 50;
  height: var(--toolbar-h);
  flex-shrink: 0;
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  padding: 0 8px 0 10px;
  background: var(--glass-top);
  border-bottom: 1px solid var(--stroke);
  backdrop-filter: blur(30px) saturate(1.2);
  -webkit-backdrop-filter: blur(30px) saturate(1.2);
}

.left {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  -webkit-app-region: drag;
}

.left .icon-only {
  -webkit-app-region: no-drag;
  flex-shrink: 0;
}

.subtitle {
  display: none;
}

.center {
  display: flex;
  justify-content: center;
  position: absolute;
  right: 232px;
}

.search-wrap {
  position: relative;
}

.search-menu {
  width: 260px;
}

.menu-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 10px 6px;
  font-size: 11px;
  color: var(--text-3);
}

.menu-head button {
  font-size: 11px;
  color: var(--text-3);
  transition: color var(--dur-1) var(--ease-out);
}

.menu-head button:hover {
  color: var(--text-1);
}

.search {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 38px;
  height: 34px;
  padding: 6px 10px;
  border-radius: 10px;
  background: transparent;
  border: 1px solid transparent;
  overflow: hidden;
  color: var(--text-3);
  transition:
    border-color var(--dur-1) var(--ease-out),
    background var(--dur-1) var(--ease-out);
}

.search:focus-within {
  width: 220px;
  border-color: var(--stroke-strong);
  background: var(--fill-hover);
}

.search:hover {
  width: 220px;
  border-color: var(--stroke);
  background: var(--fill-subtle);
}

.search input {
  flex: 1;
  min-width: 0;
  background: none;
  border: none;
  outline: none;
  width: 0;
  font-size: 13px;
  color: var(--text-1);
}

.search:hover input,
.search:focus-within input {
  width: auto;
}

.search input::placeholder {
  color: var(--text-3);
}

.search kbd {
  font-family: var(--font);
  font-size: 10.5px;
  color: var(--text-3);
  display: none;
  border: 1px solid var(--stroke);
  border-radius: 5px;
  padding: 1px 5px;
}

.search:hover kbd,
.search:focus-within kbd {
  display: inline-block;
}

.right {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 2px;
  padding: 0 122px 0 16px;
}

.pill {
  display: flex;
  align-items: center;
  gap: 7px;
  width: 34px;
  height: 34px;
  justify-content: center;
  padding: 6px;
  border-radius: 10px;
  font-size: 12.5px;
  color: var(--text-2);
  border: 1px solid var(--stroke);
  transition:
    color var(--dur-1) var(--ease-out),
    background var(--dur-1) var(--ease-out);
}

.pill:hover {
  color: var(--text-1);
  background: var(--fill-hover);
}

.pill-label {
  display: none;
}

.pill > :last-child {
  display: none;
}

.display-picker {
  position: relative;
}

.menu {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  min-width: 230px;
  padding: 6px;
  border-radius: 14px;
  z-index: 100;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.35);
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 8px 10px;
  border-radius: 9px;
  font-size: 13px;
  color: var(--text-2);
  transition: background var(--dur-1) var(--ease-out);
}

.menu-item:hover {
  background: var(--fill-hover);
  color: var(--text-1);
}

.menu-item.active {
  color: var(--text-1);
}

.menu-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: left;
}

.menu-meta {
  font-size: 11px;
  color: var(--text-3);
}

.radio {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  border: 1.5px solid var(--text-3);
  flex-shrink: 0;
  transition: all var(--dur-1) var(--ease-out);
}

.menu-item.active .radio {
  background: var(--text-1);
  border-color: var(--text-1);
}

.menu-enter-active,
.menu-leave-active {
  transition:
    opacity var(--dur-1) var(--ease-out),
    transform var(--dur-1) var(--ease-out);
}

.menu-enter-from,
.menu-leave-to {
  opacity: 0;
  transform: translateY(-6px) scale(0.98);
}

.icon-only {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  border-radius: 50%;
  color: var(--text-2);
  transition:
    background var(--dur-1) var(--ease-out),
    color var(--dur-1) var(--ease-out);
}

.icon-only:hover {
  color: var(--text-1);
  background: var(--fill-hover);
}

.window-controls {
  position: absolute;
  top: 0;
  right: 0;
  display: flex;
  height: 100%;
  -webkit-app-region: no-drag;
}

.wc {
  display: grid;
  place-items: center;
  width: 38px;
  height: 34px;
  color: var(--text-3);
  transition:
    background var(--dur-1) var(--ease-out),
    color var(--dur-1) var(--ease-out);
}

.wc:hover {
  color: var(--text-1);
  background: var(--fill-hover);
}

.wc.close:hover {
  color: #fff;
  background: var(--accent-danger);
}

@media (max-width: 760px) {
  .center {
    right: 126px;
  }

  .window-controls {
    display: flex;
  }

  .right {
    padding-right: 116px;
  }
}
</style>
