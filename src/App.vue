<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { useUiStore } from './stores/ui';
import { useLibraryStore } from './stores/library';
import { useSettingsStore } from './stores/settings';
import { useI18n } from './lib/i18n';
import SidebarNav from './components/layout/SidebarNav.vue';
import Toolbar from './components/layout/Toolbar.vue';
import PreviewOverlay from './components/wallpaper/PreviewOverlay.vue';
import SettingsOverlay from './components/settings/SettingsOverlay.vue';
import Toasts from './components/common/Toasts.vue';
import DiscoverView from './views/DiscoverView.vue';
import CreateView from './views/CreateView.vue';
import WallpapersView from './views/WallpapersView.vue';
import FavoritesView from './views/FavoritesView.vue';
import DownloadsView from './views/DownloadsView.vue';
import ImportsView from './views/ImportsView.vue';
import RecentView from './views/RecentView.vue';

const ui = useUiStore();
const lib = useLibraryStore();
const settings = useSettingsStore();
const { t } = useI18n();

const VIEWS = {
  discover: DiscoverView,
  create: CreateView,
  wallpapers: WallpapersView,
  favorites: FavoritesView,
  downloads: DownloadsView,
  imports: ImportsView,
  recent: RecentView,
} as const;

const current = computed(() => VIEWS[ui.view]);

const dragging = ref(false);
const IMAGE_EXT = /\.(png|jpe?g|webp|gif|bmp)$/i;

onMounted(async () => {
  await Promise.all([settings.load(), lib.refresh(), ui.loadMonitors()]);

  try {
    const webview = getCurrentWebview();
    await webview.onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === 'enter' || payload.type === 'over') {
        dragging.value = payload.type === 'enter'
          ? payload.paths.some((p: string) => IMAGE_EXT.test(p))
          : dragging.value;
      } else if (payload.type === 'drop') {
        dragging.value = false;
        const images = payload.paths.filter((p: string) => IMAGE_EXT.test(p));
        if (images.length) {
          lib.importFiles(images).then(() => ui.goto('imports'));
        }
      } else if (payload.type === 'leave') {
        dragging.value = false;
      }
    });
  } catch {
    /* 拖放监听失败不影响主流程 */
  }
});
</script>

<template>
  <div class="app">
    <div class="sidebar-slot" :class="{ hidden: settings.sidebarHidden }">
      <SidebarNav />
    </div>
    <div class="main">
      <Toolbar />
      <main class="content">
        <Transition name="view" mode="out-in">
          <component :is="current" :key="ui.view + (ui.categoryFilter ?? '') + settings.locale" />
        </Transition>
      </main>
    </div>

    <Transition name="overlay">
      <PreviewOverlay v-if="ui.previewId" />
    </Transition>
    <Transition name="overlay">
      <SettingsOverlay v-if="ui.settingsOpen" />
    </Transition>
    <Toasts />

    <Transition name="fade">
      <div v-if="dragging" class="drop-veil">
        <div class="drop-card glass">
          <p>{{ t('drop.title') }}</p>
          <span>{{ t('drop.formats') }}</span>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  height: 100vh;
  background: var(--bg);
}

.sidebar-slot {
  flex-shrink: 0;
  width: var(--sidebar-w);
  overflow: hidden;
  transition:
    width var(--dur-2) var(--ease-out),
    opacity var(--dur-2) var(--ease-out);
}

.sidebar-slot.hidden {
  width: 0;
  opacity: 0;
}

.main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 28px 36px 48px;
  scrollbar-gutter: stable;
}

@media (max-width: 760px) {
  .content {
    padding: 22px 18px 36px;
  }
}

.drop-veil {
  position: fixed;
  inset: 0;
  z-index: 400;
  display: grid;
  place-items: center;
  background: var(--veil);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  pointer-events: none;
}

.drop-card {
  padding: 28px 48px;
  border-radius: var(--radius-overlay);
  text-align: center;
  border-style: dashed;
}

.drop-card p {
  font-size: 20px;
  font-weight: 640;
}

.drop-card span {
  font-size: 12px;
  color: var(--text-3);
  letter-spacing: 0.12em;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity var(--dur-1) var(--ease-out);
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
