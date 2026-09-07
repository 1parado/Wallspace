<script setup lang="ts">
/**
 * ExportModal —— 「导出为…」裁剪导出弹窗。
 *
 * - 通用预设：头像 1:1（圆形预览）/ 社交竖图 4:5 / 手机 9:16 / 桌面 16:9 / 平板 4:3
 * - 平台预设（阶段 2 第一步）：GitHub / B站 / 微博 / 抖音 头像与 Cover，
 *   导出后可一键打开对应平台上传页（半自动流），并支持整套图包批量导出
 * - cover 模式：拖拽取景（归一化偏移交给后端 cover_crop_at）；fit 模式：黑边完整显示
 * - 出口：加入媒体库（source='export'）或系统「另存为…」
 */
import { computed, ref } from 'vue';
import type { WallpaperItem } from '../../types';
import { exportWallpaper, assetUrl } from '../../lib/api';
import { save, open } from '@tauri-apps/plugin-dialog';
import { useLibraryStore } from '../../stores/library';
import { useUiStore } from '../../stores/ui';
import { useI18n } from '../../lib/i18n';
import Icon from '../common/Icon.vue';

const props = defineProps<{ item: WallpaperItem }>();
const emit = defineEmits<{ close: []; exported: [] }>();

const lib = useLibraryStore();
const ui = useUiStore();
const { t } = useI18n();

interface Preset {
  id: string;
  labelKey: string;
  w: number;
  h: number;
  circle?: boolean;
  group: 'general' | 'platform';
  uploadUrl?: string;
}

const GENERAL_PRESETS: Preset[] = [
  { id: 'avatar', labelKey: 'export.preset.avatar', w: 800, h: 800, circle: true, group: 'general' },
  { id: 'social', labelKey: 'export.preset.social', w: 1080, h: 1350, group: 'general' },
  { id: 'phone', labelKey: 'export.preset.phone', w: 1080, h: 1920, group: 'general' },
  { id: 'desktop', labelKey: 'export.preset.desktop', w: 1920, h: 1080, group: 'general' },
  { id: 'tablet', labelKey: 'export.preset.tablet', w: 1024, h: 768, group: 'general' },
];

/** 平台预设：尺寸为常用推荐值，实际要求以上传页提示为准 */
const PLATFORM_PRESETS: Preset[] = [
  { id: 'github', labelKey: 'export.preset.github', w: 500, h: 500, circle: true, group: 'platform', uploadUrl: 'https://github.com/settings/profile' },
  { id: 'bilibili', labelKey: 'export.preset.bilibili', w: 512, h: 512, circle: true, group: 'platform', uploadUrl: 'https://account.bilibili.com/face' },
  { id: 'weibo', labelKey: 'export.preset.weibo', w: 180, h: 180, circle: true, group: 'platform', uploadUrl: 'https://account.weibo.com/set' },
  { id: 'douyin', labelKey: 'export.preset.douyin', w: 512, h: 512, circle: true, group: 'platform', uploadUrl: 'https://www.douyin.com/' },
  { id: 'bili-cover', labelKey: 'export.preset.biliCover', w: 2560, h: 400, group: 'platform', uploadUrl: 'https://space.bilibili.com/' },
  { id: 'weibo-cover', labelKey: 'export.preset.weiboCover', w: 980, h: 300, group: 'platform', uploadUrl: 'https://account.weibo.com/set' },
];

const ALL_PRESETS: Preset[] = [...GENERAL_PRESETS, ...PLATFORM_PRESETS];

const group = ref<'general' | 'platform'>('general');
const shownPresets = computed(() =>
  group.value === 'general' ? GENERAL_PRESETS : PLATFORM_PRESETS
);

const presetId = ref('desktop');
const mode = ref<'cover' | 'fit'>('cover');
const offset = ref({ x: 0.5, y: 0.5 });
const busy = ref(false);
const exportFormat = ref<'jpg' | 'png'>('jpg');

// 记住上次导出设置（预设 / 模式 / 格式）
const LAST_EXPORT_KEY = 'wallspace.lastExport';
try {
  const last = JSON.parse(localStorage.getItem(LAST_EXPORT_KEY) ?? 'null');
  if (last && typeof last === 'object') {
    if (ALL_PRESETS.some((p) => p.id === last.presetId)) presetId.value = last.presetId;
    if (last.mode === 'fit') mode.value = 'fit';
    if (last.format === 'png') exportFormat.value = 'png';
  }
} catch {
  /* 忽略损坏的历史记录 */
}

function rememberLast() {
  localStorage.setItem(
    LAST_EXPORT_KEY,
    JSON.stringify({ presetId: presetId.value, mode: mode.value, format: exportFormat.value })
  );
}

const preset = computed(() => ALL_PRESETS.find((p) => p.id === presetId.value)!);
const ratioStyle = computed(() => ({ aspectRatio: `${preset.value.w} / ${preset.value.h}` }));
const positionStyle = computed(
  () =>
    mode.value === 'fit'
      ? { objectFit: 'contain' as const }
      : { objectFit: 'cover' as const, objectPosition: `${offset.value.x * 100}% ${offset.value.y * 100}%` }
);

function pickPreset(id: string) {
  presetId.value = id;
  offset.value = { x: 0.5, y: 0.5 };
}

// —— 拖拽取景：把位移换算成归一化偏移（只在有溢出的轴生效） ——
const boxEl = ref<HTMLElement | null>(null);
let dragStart: { px: number; py: number; ox: number; oy: number } | null = null;

function displayedSpans(box: HTMLElement): { sx: number; sy: number } {
  const img = box.querySelector('img');
  if (!img) return { sx: 0, sy: 0 };
  const nw = img.naturalWidth || 1;
  const nh = img.naturalHeight || 1;
  const scale = Math.max(box.clientWidth / nw, box.clientHeight / nh);
  return { sx: nw * scale - box.clientWidth, sy: nh * scale - box.clientHeight };
}

function onPointerDown(e: PointerEvent) {
  if (mode.value !== 'fit') {
    const { sx, sy } = displayedSpans(boxEl.value!);
    if (sx > 1 || sy > 1) {
      dragStart = { px: e.clientX, py: e.clientY, ox: offset.value.x, oy: offset.value.y };
      (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    }
  }
}

function onPointerMove(e: PointerEvent) {
  if (!dragStart || !boxEl.value) return;
  const { sx, sy } = displayedSpans(boxEl.value);
  const dx = e.clientX - dragStart.px;
  const dy = e.clientY - dragStart.py;
  // 拖拽内容向右 = 取景窗口向左 = offset.x 减小
  offset.value = {
    x: sx > 1 ? Math.min(1, Math.max(0, dragStart.ox - dx / sx)) : 0.5,
    y: sy > 1 ? Math.min(1, Math.max(0, dragStart.oy - dy / sy)) : 0.5,
  };
}

function onPointerUp() {
  dragStart = null;
}

const fileName = computed(
  () => `${props.item.title}_${preset.value.w}x${preset.value.h}.${exportFormat.value}`
);

async function addToLibrary() {
  if (busy.value) return;
  busy.value = true;
  try {
    await exportWallpaper({
      id: props.item.id,
      width: preset.value.w,
      height: preset.value.h,
      mode: mode.value,
      offsetX: offset.value.x,
      offsetY: offset.value.y,
      addToLibrary: true,
      format: exportFormat.value,
    });
    rememberLast();
    await lib.refresh();
    ui.toast('success', t('export.toast.added'));
    emit('exported');
    emit('close');
  } catch (e) {
    ui.toast('error', String(e));
  } finally {
    busy.value = false;
  }
}

async function saveAs() {
  if (busy.value) return;
  const dest = await save({
    title: t('export.saveTitle'),
    defaultPath: fileName.value,
    filters: [
      {
        name: exportFormat.value === 'png' ? 'PNG' : 'JPEG',
        extensions: [exportFormat.value],
      },
    ],
  });
  if (!dest) return;
  busy.value = true;
  try {
    await exportWallpaper({
      id: props.item.id,
      width: preset.value.w,
      height: preset.value.h,
      mode: mode.value,
      offsetX: offset.value.x,
      offsetY: offset.value.y,
      savePath: dest,
      format: exportFormat.value,
    });
    rememberLast();
    ui.toast('success', t('export.toast.saved'));
    emit('close');
  } catch (e) {
    ui.toast('error', String(e));
  } finally {
    busy.value = false;
  }
}

/** 半自动流：导出后跳转平台上传页 */
function openUploadPage() {
  if (preset.value.uploadUrl) window.open(preset.value.uploadUrl, '_blank');
}

/** 整套图包：把全部平台预设一次性导出到所选文件夹 */
async function exportPack() {
  if (busy.value) return;
  const dir = await open({ directory: true, title: t('export.saveTitle') });
  if (!dir || typeof dir !== 'string') return;
  const sep = dir.includes('\\') ? '\\' : '/';
  const base = dir.replace(/[\\/]+$/, '');
  busy.value = true;
  try {
    for (const p of PLATFORM_PRESETS) {
      await exportWallpaper({
        id: props.item.id,
        width: p.w,
        height: p.h,
        mode: 'cover',
        offsetX: 0.5,
        offsetY: 0.5,
        savePath: `${base}${sep}Wallspace_${p.id}_${p.w}x${p.h}.jpg`,
      });
    }
    ui.toast('success', t('export.toast.batch', { n: PLATFORM_PRESETS.length }));
  } catch (e) {
    ui.toast('error', String(e));
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="export-mask" @click.self="emit('close')">
    <div class="export-modal glass">
      <div class="modal-head">
        <h3>{{ t('export.title') }}</h3>
        <button class="close-btn" :title="t('preview.close')" @click="emit('close')">
          <Icon name="x" :size="16" />
        </button>
      </div>

      <div class="modal-body">
        <!-- 取景预览 -->
        <div
          ref="boxEl"
          class="crop-box"
          :class="{ pannable: mode === 'cover' }"
          :style="ratioStyle"
          @pointerdown="onPointerDown"
          @pointermove="onPointerMove"
          @pointerup="onPointerUp"
          @pointercancel="onPointerUp"
        >
          <img
            :src="assetUrl(item.filePath)"
            :style="positionStyle"
            draggable="false"
          />
          <span v-if="preset.circle" class="circle-guide" />
          <span v-if="mode === 'cover'" class="pan-hint">{{ t('export.panHint') }}</span>
        </div>

        <!-- 预设与模式 -->
        <div class="controls">
          <div class="segmented group-tabs">
            <button :class="{ active: group === 'general' }" @click="group = 'general'">
              {{ t('export.groupGeneral') }}
            </button>
            <button :class="{ active: group === 'platform' }" @click="group = 'platform'">
              {{ t('export.groupPlatform') }}
            </button>
          </div>
          <p v-if="group === 'platform'" class="spec-note">{{ t('export.specNote') }}</p>
          <p class="label">{{ t('export.presets') }}</p>
          <div class="preset-grid">
            <button
              v-for="p in shownPresets"
              :key="p.id"
              class="preset-chip"
              :class="{ active: presetId === p.id }"
              @click="pickPreset(p.id)"
            >
              {{ t(p.labelKey) }}
              <span class="dim">{{ p.w }}×{{ p.h }}</span>
            </button>
          </div>

          <p class="label">{{ t('export.mode') }}</p>
          <div class="segmented">
            <button :class="{ active: mode === 'cover' }" @click="mode = 'cover'">
              {{ t('export.modeCover') }}
            </button>
            <button :class="{ active: mode === 'fit' }" @click="mode = 'fit'">
              {{ t('export.modeFit') }}
            </button>
          </div>

          <p class="label">{{ t('export.format') }}</p>
          <div class="segmented">
            <button :class="{ active: exportFormat === 'jpg' }" @click="exportFormat = 'jpg'">
              {{ t('export.fmtJpg') }}
            </button>
            <button :class="{ active: exportFormat === 'png' }" @click="exportFormat = 'png'">
              {{ t('export.fmtPng') }}
            </button>
          </div>
        </div>
      </div>

      <div class="modal-foot">
        <span class="file-name">{{ fileName }}</span>
        <div class="foot-actions">
          <button
            v-if="group === 'platform'"
            class="pill"
            :disabled="busy"
            :title="t('export.specNote')"
            @click="exportPack"
          >
            {{ t('export.batch', { n: PLATFORM_PRESETS.length }) }}
          </button>
          <button
            v-if="preset.uploadUrl"
            class="pill"
            :disabled="busy"
            @click="openUploadPage"
          >
            {{ t('export.uploadPage') }}
          </button>
          <button class="pill" :disabled="busy" @click="saveAs">{{ t('export.saveAs') }}</button>
          <button class="btn-primary" :disabled="busy" @click="addToLibrary">
            <Icon name="plus" :size="14" />
            {{ t('export.addToLibrary') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.export-mask {
  position: fixed;
  inset: 0;
  z-index: 120;
  display: grid;
  place-items: center;
  background: rgba(10, 10, 12, 0.55);
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
}

.export-modal {
  width: min(720px, 92vw);
  max-height: 88vh;
  display: flex;
  flex-direction: column;
  border-radius: var(--radius-overlay, 20px);
  overflow: hidden;
}

.modal-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px 10px;
}

.modal-head h3 {
  font-size: 16px;
  font-weight: 620;
}

.close-btn {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  color: var(--text-3);
  transition: all var(--dur-1) var(--ease-out);
}

.close-btn:hover {
  color: var(--text-1);
  background: var(--fill-hover);
}

.modal-body {
  display: flex;
  gap: 18px;
  padding: 4px 20px 14px;
  overflow: auto;
  align-items: flex-start;
}

.crop-box {
  position: relative;
  flex: 0 0 46%;
  max-height: 46vh;
  overflow: hidden;
  border-radius: 12px;
  background: var(--bg-3);
  border: 1px solid var(--stroke);
  touch-action: none;
  user-select: none;
}

.crop-box.pannable {
  cursor: grab;
}

.crop-box:active {
  cursor: grabbing;
}

.crop-box img {
  width: 100%;
  height: 100%;
  display: block;
  pointer-events: none;
}

.circle-guide {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  box-shadow: 0 0 0 200px rgba(0, 0, 0, 0.42);
  pointer-events: none;
}

.pan-hint {
  position: absolute;
  left: 50%;
  bottom: 8px;
  transform: translateX(-50%);
  font-size: 10.5px;
  color: rgba(255, 255, 255, 0.8);
  background: rgba(0, 0, 0, 0.4);
  border-radius: 100px;
  padding: 2px 10px;
  pointer-events: none;
}

.controls {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
}

.group-tabs {
  margin-bottom: 2px;
}

.spec-note {
  font-size: 11px;
  color: var(--text-3);
  line-height: 1.5;
}

.label {
  font-size: 11px;
  font-weight: 560;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-3);
}

.preset-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.preset-chip {
  display: inline-flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 1px;
  font-size: 12.5px;
  color: var(--text-2);
  border: 1px solid var(--stroke);
  border-radius: 10px;
  padding: 6px 12px;
  transition: all var(--dur-1) var(--ease-out);
}

.preset-chip:hover {
  border-color: var(--stroke-strong);
}

.preset-chip.active {
  color: var(--text-1);
  background: var(--fill-active);
  border-color: var(--stroke-strong);
}

.preset-chip .dim {
  font-size: 10px;
  color: var(--text-3);
}

.segmented {
  display: flex;
  gap: 4px;
  padding: 4px;
  border-radius: 11px;
  background: var(--fill-subtle);
  border: 1px solid var(--stroke);
}

.segmented button {
  flex: 1;
  padding: 7px 12px;
  border-radius: 8px;
  font-size: 12.5px;
  color: var(--text-3);
  white-space: nowrap;
  transition: all var(--dur-1) var(--ease-out);
}

.segmented button.active {
  color: var(--text-1);
  background: var(--fill-active);
}

.modal-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding: 12px 20px 16px;
  border-top: 1px solid var(--stroke);
}

.file-name {
  font-size: 11.5px;
  color: var(--text-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.foot-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.pill {
  font-size: 12.5px;
  color: var(--text-2);
  border: 1px solid var(--stroke);
  border-radius: 100px;
  padding: 7px 16px;
  transition: all var(--dur-1) var(--ease-out);
}

.pill:hover:not(:disabled) {
  color: var(--text-1);
  border-color: var(--stroke-strong);
}

.pill:disabled,
.btn-primary:disabled {
  opacity: 0.55;
}

.btn-primary {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12.5px;
  font-weight: 600;
  color: #fff;
  background: var(--accent, #3b82f6);
  border-radius: 100px;
  padding: 7px 16px;
  transition: all var(--dur-1) var(--ease-out);
}
</style>
