<script setup lang="ts">
import { ref, watch } from 'vue';
import type { WallpaperItem } from '../../types';
import * as api from '../../lib/api';
import { renderShareCard, canvasToPng, canvasToJpeg, type ShareKind } from '../../lib/shareCard';
import { save as saveDialog } from '@tauri-apps/plugin-dialog';
import { useUiStore } from '../../stores/ui';
import { useI18n } from '../../lib/i18n';
import Icon from './Icon.vue';

const props = defineProps<{
  /** 要生成分享卡的条目 */
  item: WallpaperItem | null;
  /** 弹窗开关（打开时自动渲染卡片） */
  open: boolean;
}>();

const emit = defineEmits<{ (e: 'close'): void }>();

const ui = useUiStore();
const { t } = useI18n();

const busy = ref(false);
const saving = ref(false);
const url = ref('');
const kind = ref<ShareKind>('portrait');
let canvasEl: HTMLCanvasElement | null = null;
let renderToken = 0;

const KINDS: { id: ShareKind; labelKey: string }[] = [
  { id: 'portrait', labelKey: 'share.portrait' },
  { id: 'landscape', labelKey: 'share.landscape' },
  { id: 'square', labelKey: 'share.square' },
];

async function render() {
  if (!props.item) return;
  const token = ++renderToken;
  busy.value = true;
  try {
    const canvas = await renderShareCard(props.item, kind.value);
    const newUrl = URL.createObjectURL(await canvasToPng(canvas));
    if (token !== renderToken) {
      URL.revokeObjectURL(newUrl);
      return;
    }
    if (url.value) URL.revokeObjectURL(url.value);
    canvasEl = canvas;
    url.value = newUrl;
  } catch (e) {
    if (token === renderToken) {
      ui.toast('error', String(e));
      emit('close');
    }
  } finally {
    if (token === renderToken) busy.value = false;
  }
}

function setKind(k: ShareKind) {
  if (kind.value === k) return;
  kind.value = k;
  if (props.open) void render();
}

watch(
  () => props.open,
  (open) => {
    if (open) void render();
    else {
      if (url.value) {
        URL.revokeObjectURL(url.value);
        url.value = '';
      }
      canvasEl = null;
      renderToken++;
      busy.value = false;
    }
  }
);

function safeName(title: string): string {
  return title.replace(/[\\/:*?"<>|]/g, '_').slice(0, 60) || 'wallspace';
}

async function copyShare() {
  if (!canvasEl) return;
  try {
    const blob = await canvasToPng(canvasEl);
    await navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })]);
    ui.toast('success', t('share.copied'));
  } catch (e) {
    ui.toast('error', String(e));
  }
}

async function saveShare() {
  if (!canvasEl || !props.item || saving.value) return;
  const path = await saveDialog({
    defaultPath: `wallspace-${safeName(props.item.title)}.jpg`,
    filters: [{ name: 'JPEG', extensions: ['jpg'] }],
  });
  if (!path || !canvasEl) return;
  saving.value = true;
  try {
    const blob = await canvasToJpeg(canvasEl);
    await api.saveBinaryFile(path, new Uint8Array(await blob.arrayBuffer()));
    ui.toast('success', t('share.saved'));
    emit('close');
  } catch (e) {
    ui.toast('error', String(e));
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div v-if="open" class="share-backdrop" @click.self="emit('close')">
    <div class="share-modal glass">
      <div class="share-head">
        <h3>{{ t('share.title') }}</h3>
        <button class="share-close" @click="emit('close')">
          <Icon name="x" :size="14" />
        </button>
      </div>
      <div class="share-kinds">
        <button
          v-for="k in KINDS"
          :key="k.id"
          :class="{ active: kind === k.id }"
          :disabled="busy"
          @click="setKind(k.id)"
        >
          {{ t(k.labelKey) }}
        </button>
      </div>
      <div class="share-preview">
        <span v-if="busy || !url" class="share-busy">{{ t('share.rendering') }}</span>
        <img v-else :src="url" draggable="false" />
      </div>
      <div class="share-actions">
        <button class="btn-ghost" :disabled="busy || !url" @click="copyShare">
          {{ t('share.copy') }}
        </button>
        <button class="btn-primary" :disabled="saving || busy || !url" @click="saveShare">
          <Icon name="download" :size="14" />
          {{ saving ? t('share.saving') : t('share.save') }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.share-backdrop {
  position: fixed;
  inset: 0;
  z-index: 220;
  display: grid;
  place-items: center;
  padding: 40px;
  background: var(--veil);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
}

.share-modal {
  display: flex;
  flex-direction: column;
  gap: 14px;
  width: min(420px, 90vw);
  max-height: calc(100vh - 100px);
  padding: 18px 20px;
  border-radius: var(--radius-overlay);
  border: 1px solid var(--stroke-strong);
  box-shadow: 0 40px 120px rgba(0, 0, 0, 0.4);
}

.share-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.share-head h3 {
  font-size: 15px;
  font-weight: 620;
}

.share-close {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  color: var(--text-3);
  transition: all var(--dur-1) var(--ease-out);
}

.share-close:hover {
  color: var(--text-1);
  background: var(--fill-hover);
}

.share-kinds {
  display: flex;
  border: 1px solid var(--stroke);
  border-radius: 100px;
  overflow: hidden;
}

.share-kinds button {
  flex: 1;
  font-size: 12px;
  padding: 6px 0;
  color: var(--text-3);
  transition: all var(--dur-1) var(--ease-out);
}

.share-kinds button.active {
  color: var(--text-1);
  background: var(--fill-active);
}

.share-preview {
  flex: 1;
  min-height: 260px;
  display: grid;
  place-items: center;
}

.share-preview img {
  max-width: 100%;
  max-height: min(52vh, 560px);
  object-fit: contain;
  border-radius: 12px;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.35);
}

.share-busy {
  font-size: 13px;
  color: var(--text-3);
}

.share-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
