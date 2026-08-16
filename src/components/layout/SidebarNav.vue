<script setup lang="ts">
import { useUiStore } from '../../stores/ui';
import { useLibraryStore } from '../../stores/library';
import { useI18n } from '../../lib/i18n';
import { open } from '@tauri-apps/plugin-dialog';
import Icon from '../common/Icon.vue';

const ui = useUiStore();
const lib = useLibraryStore();
const { t } = useI18n();

const NAV = [
  { id: 'discover', labelKey: 'nav.discover', icon: 'compass' },
  { id: 'create', labelKey: 'nav.create', icon: 'sparkles' },
  { id: 'wallpapers', labelKey: 'nav.wallpapers', icon: 'image' },
  { id: 'favorites', labelKey: 'nav.favorites', icon: 'heart' },
  { id: 'downloads', labelKey: 'nav.downloads', icon: 'download' },
] as const;

const PERSONAL = [
  { id: 'imports', labelKey: 'nav.imports', icon: 'folder' },
  { id: 'recent', labelKey: 'nav.recent', icon: 'clock' },
] as const;

function goCategory(cat: string) {
  ui.view = 'wallpapers';
  ui.categoryFilter = cat;
  ui.search = '';
}

async function importOwn() {
  const picked = await open({
    multiple: true,
    filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp'] }],
  });
  if (!picked) return;
  const paths = Array.isArray(picked) ? picked : [picked];
  await lib.importFiles(paths);
  ui.view = 'imports';
}
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <div class="brand-mark"><Icon name="film" :size="16" /></div>
      <span class="brand-name">Wallspace</span>
    </div>

    <nav class="nav">
      <button
        v-for="n in NAV"
        :key="n.id"
        class="nav-item"
        :class="{ active: ui.view === n.id }"
        @click="ui.goto(n.id)"
      >
        <Icon :name="n.icon" :size="17" />
        <span>{{ t(n.labelKey) }}</span>
        <span v-if="n.id === 'favorites' && lib.favorites.length" class="count">
          {{ lib.favorites.length }}
        </span>
      </button>
    </nav>

    <div class="section">
      <p class="section-label">{{ t('nav.categories') }}</p>
      <button
        v-for="c in ui.categories"
        :key="c"
        class="nav-item sub"
        :class="{ active: ui.view === 'wallpapers' && ui.categoryFilter === c }"
        @click="goCategory(c)"
      >
        <span>{{ t(`cat.${c.toLowerCase()}`) }}</span>
      </button>
    </div>

    <div class="section">
      <p class="section-label">{{ t('nav.personal') }}</p>
      <button
        v-for="p in PERSONAL"
        :key="p.id"
        class="nav-item sub"
        :class="{ active: ui.view === p.id }"
        @click="ui.goto(p.id)"
      >
        <Icon :name="p.icon" :size="16" />
        <span>{{ t(p.labelKey) }}</span>
      </button>
    </div>

    <div class="spacer" />

    <button class="import-cta" @click="importOwn">
      <Icon name="upload" :size="15" />
      <span>{{ t('nav.importOwn') }}</span>
    </button>
    <p class="footnote">{{ t('nav.footnote') }}</p>
  </aside>
</template>

<style scoped>
.sidebar {
  width: var(--sidebar-w);
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: 16px 12px 14px;
  background: var(--glass-side);
  border-right: 1px solid var(--stroke);
  backdrop-filter: blur(40px) saturate(1.25);
  -webkit-backdrop-filter: blur(40px) saturate(1.25);
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 10px 18px;
}

.brand-mark {
  width: 26px;
  height: 26px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  background: var(--fill-active);
  border: 1px solid var(--stroke-strong);
  color: var(--text-1);
}

.brand-name {
  font-size: 15px;
  font-weight: 650;
  letter-spacing: -0.01em;
}

.nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 8px 10px;
  border-radius: 9px;
  font-size: 13.5px;
  font-weight: 480;
  color: var(--text-2);
  transition:
    background var(--dur-1) var(--ease-out),
    color var(--dur-1) var(--ease-out);
}

.nav-item:hover {
  color: var(--text-1);
  background: var(--fill-hover);
}

.nav-item.active {
  color: var(--text-1);
  background: var(--fill-active);
}

.count {
  margin-left: auto;
  font-size: 11.5px;
  color: var(--text-3);
}

.section {
  margin-top: 20px;
}

.section-label {
  font-size: 11px;
  font-weight: 560;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-3);
  padding: 0 10px 6px;
}

.nav-item.sub {
  padding: 6.5px 10px;
  font-size: 13px;
}

.spacer {
  flex: 1;
}

.import-cta {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  width: 100%;
  padding: 9px 12px;
  border-radius: 11px;
  border: 1px dashed var(--stroke-strong);
  color: var(--text-2);
  font-size: 13px;
  transition:
    color var(--dur-1) var(--ease-out),
    border-color var(--dur-1) var(--ease-out),
    background var(--dur-1) var(--ease-out);
}

.import-cta:hover {
  color: var(--text-1);
  border-color: var(--stroke-strong);
  background: var(--fill-hover);
}

.footnote {
  margin-top: 10px;
  text-align: center;
  font-size: 10.5px;
  color: var(--text-3);
}
</style>
