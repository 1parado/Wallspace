<script setup lang="ts">
import { computed, ref } from 'vue';
import type { WallpaperItem } from '../../types';
import { assetUrl } from '../../lib/api';
import { useLibraryStore } from '../../stores/library';
import { useUiStore } from '../../stores/ui';
import { useI18n } from '../../lib/i18n';
import { displayCategory } from '../../lib/categoryTree';
import Icon from '../common/Icon.vue';

const props = withDefaults(
  defineProps<{ item: WallpaperItem; featured?: boolean }>(),
  { featured: false }
);

const lib = useLibraryStore();
const ui = useUiStore();
const { t } = useI18n();
const loaded = ref(false);
const failed = ref(false);

const ratio = computed(() => (props.featured ? '21/9' : '16/9'));
const sizeLabel = computed(() => {
  const mb = props.item.fileSize / (1024 * 1024);
  return mb >= 1 ? `${mb.toFixed(1)} MB` : `${Math.round(props.item.fileSize / 1024)} KB`;
});
const metaLabel = computed(
  () =>
    `${displayCategory(props.item.category, t, t('cat.uncategorized'))} · ${props.item.width}×${props.item.height} · ${sizeLabel.value}`
);
</script>

<template>
  <div
    class="card"
    :class="{ featured }"
    role="button"
    tabindex="0"
    @click="ui.previewId = item.id"
    @keydown.enter="ui.previewId = item.id"
  >
    <div class="thumb" :style="{ aspectRatio: ratio }">
      <div v-if="!loaded && !failed" class="skeleton" />
      <img
        v-if="!failed"
        :src="assetUrl(item.filePath)"
        :class="{ visible: loaded }"
        loading="lazy"
        draggable="false"
        @load="loaded = true"
        @error="failed = true"
      />
      <div v-if="failed" class="broken"><Icon name="image" :size="20" /></div>
    </div>

    <!-- 常驻半透明收藏心（右上角，始终可见） -->
    <button
      class="round-btn heart persistent"
      :class="{ loved: item.favorite }"
      :title="t('preview.favorite')"
      @click.stop="lib.toggleFavorite(item)"
    >
      <Icon name="heart" :size="15" />
    </button>

    <div class="overlay">
      <div class="info">
        <p class="name">{{ item.title }}</p>
        <p class="meta">{{ metaLabel }}</p>
      </div>
      <div class="actions" @click.stop>
        <button class="apply-btn" :disabled="lib.applyingId === item.id" @click="lib.apply(item.id)">
          {{ lib.applyingId === item.id ? t('preview.applying') : t('preview.apply') }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.card {
  position: relative;
  border-radius: var(--radius-card);
  overflow: hidden;
  cursor: pointer;
  background: var(--bg-3);
  border: 1px solid var(--stroke);
  transition:
    transform var(--dur-2) var(--ease-out),
    box-shadow var(--dur-2) var(--ease-out);
}

.card:hover {
  transform: scale(1.012);
  z-index: 2;
  box-shadow: 0 10px 26px rgba(17, 17, 17, 0.12);
}

.thumb {
  position: relative;
  width: 100%;
  background: var(--bg-3);
}

.thumb img {
  width: 100%;
  height: 100%;
  position: absolute;
  inset: 0;
  object-fit: cover;
  opacity: 0;
  transition: opacity var(--dur-2) var(--ease-out);
}

.thumb img.visible {
  opacity: 1;
}

.skeleton {
  position: absolute;
  inset: 0;
  background: linear-gradient(100deg, var(--skeleton-a) 40%, var(--skeleton-b) 50%, var(--skeleton-a) 60%);
  background-size: 200% 100%;
  animation: shimmer 1.6s infinite;
}

@keyframes shimmer {
  from {
    background-position: 120% 0;
  }
  to {
    background-position: -80% 0;
  }
}

.broken {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  color: var(--text-3);
}

/* 图片上的浮层：默认浅渐变 + 标题常显；悬停时加深并展示 meta / 操作 */
.overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  padding: 14px;
  gap: 10px;
  background: linear-gradient(
    to top,
    rgba(5, 5, 7, 0.5) 0%,
    rgba(5, 5, 7, 0.12) 40%,
    rgba(5, 5, 7, 0) 70%
  );
  transition: background var(--dur-2) var(--ease-out);
}

.card:hover .overlay,
.card:focus-visible .overlay {
  background: linear-gradient(
    to top,
    rgba(5, 5, 7, 0.78) 0%,
    rgba(5, 5, 7, 0.25) 45%,
    rgba(5, 5, 7, 0.06) 100%
  );
}

.info {
  transform: translateY(0);
  transition: transform var(--dur-2) var(--ease-out);
}

.name {
  font-size: 14px;
  font-weight: 600;
  letter-spacing: -0.01em;
  color: rgba(255, 255, 255, 0.95);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.meta {
  margin-top: 2px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.55);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  opacity: 0;
  transition: opacity var(--dur-2) var(--ease-out);
}

.card:hover .meta,
.card:focus-visible .meta {
  opacity: 1;
}

.actions {
  display: flex;
  align-items: center;
  gap: 8px;
  opacity: 0;
  transition: opacity var(--dur-2) var(--ease-out);
}

.card:hover .actions,
.card:focus-visible .actions {
  opacity: 1;
}

/* 常驻收藏心：右上角，半透明可见 */
.heart.persistent {
  position: absolute;
  top: 10px;
  right: 10px;
  z-index: 2;
  opacity: 0.85;
}

.apply-btn {
  padding: 7px 16px;
  border-radius: 100px;
  background: #f5f5f7;
  color: #0b0b0d;
  font-size: 12.5px;
  font-weight: 620;
  transition:
    transform var(--dur-1) var(--ease-out),
    opacity var(--dur-1) var(--ease-out),
    background var(--dur-1) var(--ease-out);
}

.apply-btn:hover {
  background: #fff;
}

.apply-btn:active {
  transform: scale(0.96);
}

.apply-btn:disabled {
  opacity: 0.6;
}

.round-btn {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.14);
  color: #fff;
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  transition:
    transform var(--dur-1) var(--ease-out),
    background var(--dur-1) var(--ease-out),
    color var(--dur-1) var(--ease-out);
}

.round-btn:hover {
  background: rgba(255, 255, 255, 0.22);
}

.round-btn:active {
  transform: scale(0.9);
}

.heart.loved {
  color: var(--accent-heart);
  background: rgba(255, 90, 110, 0.2);
}

.heart.loved:hover {
  background: rgba(255, 90, 110, 0.3);
}
</style>
