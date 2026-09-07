<script setup lang="ts">
import { computed, ref } from 'vue';
import type { WallpaperItem } from '../types';
import { useLibraryStore } from '../stores/library';
import { useUiStore } from '../stores/ui';
import { useI18n } from '../lib/i18n';
import { assetUrl } from '../lib/api';
import { seededRandom } from '../lib/sortItems';
import ShareCardModal from '../components/common/ShareCardModal.vue';
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

// —— 每日推荐：按当天日期做种子，同一天内推荐固定，次日更换 ——
const dateLabel = new Date().toLocaleDateString();

const daily = computed(() => {
  if (!lib.items.length) return null;
  const now = new Date();
  const seed = now.getFullYear() * 10000 + (now.getMonth() + 1) * 100 + now.getDate();
  const shuffled = [...lib.items];
  const rnd = seededRandom(seed);
  for (let i = shuffled.length - 1; i > 0; i--) {
    const j = Math.floor(rnd() * (i + 1));
    [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
  }
  return shuffled[0];
});

const dailyApplying = computed(() => daily.value && lib.applyingId === daily.value.id);

// —— 每日精选分享卡 ——
const shareFor = ref<WallpaperItem | null>(null);
const shareOpen = ref(false);
const shareBusy = ref(false);

async function openShare() {
  if (shareBusy.value || !daily.value) return;
  shareFor.value = daily.value;
  shareOpen.value = true;
}
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
      <!-- 每日推荐 -->
      <section v-if="daily" class="section">
        <div class="section-head">
          <h2>{{ t('hero.daily') }}</h2>
          <span class="date-hint">{{ dateLabel }}</span>
        </div>
        <div class="daily-card" :style="{ backgroundImage: `url(${assetUrl(daily.filePath)})` }">
          <div class="daily-info">
            <p class="daily-title">{{ daily.title }}</p>
            <p class="daily-meta">{{ daily.width }} × {{ daily.height }}</p>
            <div class="daily-actions">
              <button
                class="btn-primary"
                :disabled="!!lib.applyingId"
                @click="daily && lib.apply(daily.id)"
              >
                <Icon name="monitor" :size="14" />
                {{ dailyApplying ? t('preview.applying') : t('hero.dailyApply') }}
              </button>
              <button class="btn-ghost" @click="ui.previewId = daily.id">
                {{ t('hero.dailyOpen') }}
              </button>
              <button class="btn-ghost" :disabled="shareBusy" @click="openShare">
                <Icon name="sparkles" :size="13" />
                {{ shareBusy ? t('share.rendering') : t('share.open') }}
              </button>
            </div>
          </div>
        </div>
      </section>

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

    <!-- 分享卡弹窗 -->
    <ShareCardModal :item="shareFor" :open="shareOpen" @close="shareOpen = false" />
  </div>
</template>

<style scoped>
.discover {
  display: flex;
  flex-direction: column;
  gap: 32px;
}

.hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  max-width: 560px;
  margin: 0 auto;
  padding: 72px 8px 18px;
  text-align: center;
}

.hero-title {
  font-size: clamp(28px, 3vw, 38px);
  font-weight: 650;
  letter-spacing: -0.02em;
  line-height: 1.12;
}

.hero-sub {
  margin-top: 9px;
  font-size: 14px;
  color: var(--text-2);
}

.hero-actions {
  display: flex;
  justify-content: center;
  gap: 12px;
  margin-top: 22px;
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
  font-size: 18px;
  font-weight: 600;
  letter-spacing: -0.02em;
}

.date-hint {
  font-size: 12px;
  color: var(--text-3);
}

.daily-card {
  position: relative;
  aspect-ratio: 21 / 9;
  border-radius: var(--radius-overlay);
  overflow: hidden;
  border: 1px solid var(--stroke);
  background-size: cover;
  background-position: center;
  transition: transform var(--dur-2) var(--ease-out);
}

.daily-card:hover {
  transform: translateY(-2px);
}

.daily-info {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  padding: 24px 28px;
  background: linear-gradient(transparent 40%, rgba(0, 0, 0, 0.62));
  color: #fff;
}

.daily-title {
  font-size: 19px;
  font-weight: 640;
  letter-spacing: -0.01em;
  text-shadow: 0 1px 8px rgba(0, 0, 0, 0.4);
}

.daily-meta {
  margin-top: 3px;
  font-size: 12px;
  opacity: 0.85;
}

.daily-actions {
  display: flex;
  gap: 10px;
  margin-top: 14px;
}

@media (max-width: 900px) {
  .daily-card {
    aspect-ratio: 16 / 9;
  }
}

.hint {
  display: none;
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
  gap: 14px;
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
