<script setup lang="ts">
import { computed } from 'vue';
import { useLibraryStore } from '../stores/library';
import { useUiStore } from '../stores/ui';
import { useI18n } from '../lib/i18n';
import WallpaperCard from '../components/wallpaper/WallpaperCard.vue';
import WallpaperGrid from '../components/wallpaper/WallpaperGrid.vue';
import EmptyState from '../components/common/EmptyState.vue';
import Icon from '../components/common/Icon.vue';

const lib = useLibraryStore();
const ui = useUiStore();
const { t } = useI18n();

const featured = computed(() => {
  const favs = lib.favorites;
  const rest = lib.items.filter((i) => !i.favorite);
  return [...favs, ...rest].slice(0, 8);
});

const hasLibrary = computed(() => lib.items.length > 0);
</script>

<template>
  <div class="discover">
    <section class="hero">
      <h1 class="hero-title">{{ t('hero.title') }}</h1>
      <p class="hero-sub">{{ t('hero.sub') }}</p>
      <div class="hero-actions">
        <button class="btn-primary" @click="ui.goto('create')">
          <Icon name="sparkles" :size="15" />
          {{ t('hero.create') }}
        </button>
        <button class="btn-ghost" @click="ui.goto('downloads')">
          <Icon name="link" :size="14" />
          {{ t('hero.paste') }}
        </button>
      </div>
    </section>

    <template v-if="hasLibrary">
      <section v-if="featured.length" class="section">
        <div class="section-head">
          <h2>{{ t('hero.featured') }}</h2>
          <span class="hint">{{ lib.favorites.length ? t('hero.lovedFresh') : t('hero.fresh') }}</span>
        </div>
        <div class="rail">
          <WallpaperCard v-for="f in featured" :key="f.id" :item="f" featured class="rail-card" />
        </div>
      </section>

      <section class="section">
        <div class="section-head">
          <h2>{{ t('hero.recent') }}</h2>
          <button class="more" @click="ui.goto('wallpapers')">
            {{ t('hero.viewAll') }}<Icon name="arrow-right" :size="13" />
          </button>
        </div>
        <WallpaperGrid :items="lib.items.slice(0, 12)" />
      </section>
    </template>

    <EmptyState
      v-else
      :title="t('empty.library.title')"
      :subtitle="t('empty.library.sub')"
      :action-label="t('empty.explore')"
      @action="ui.goto('create')"
    />
  </div>
</template>

<style scoped>
.discover {
  display: flex;
  flex-direction: column;
  gap: 36px;
}

.hero {
  padding: 56px 8px 8px;
}

.hero-title {
  font-size: clamp(32px, 3.4vw, 46px);
  font-weight: 680;
  letter-spacing: -0.03em;
  line-height: 1.08;
}

.hero-sub {
  margin-top: 10px;
  font-size: 16px;
  color: var(--text-2);
}

.hero-actions {
  display: flex;
  gap: 12px;
  margin-top: 26px;
}

.section {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.section-head {
  display: flex;
  align-items: baseline;
  gap: 12px;
}

.section-head h2 {
  font-size: 22px;
  font-weight: 620;
  letter-spacing: -0.02em;
}

.hint {
  font-size: 12.5px;
  color: var(--text-3);
}

.more {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--text-3);
  transition: color var(--dur-1) var(--ease-out);
}

.more:hover {
  color: var(--text-1);
}

.rail {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: minmax(420px, 68%);
  gap: 18px;
  overflow-x: auto;
  padding-bottom: 8px;
  scroll-snap-type: x mandatory;
}

.rail-card {
  scroll-snap-align: start;
}

@media (max-width: 1400px) {
  .rail {
    grid-auto-columns: minmax(360px, 78%);
  }
}
</style>
