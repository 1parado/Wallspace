<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useUiStore, CATEGORIES } from '../../stores/ui';
import { useLibraryStore } from '../../stores/library';
import { useCollectionsStore } from '../../stores/collections';
import { useI18n } from '../../lib/i18n';
import { assetUrl, revealItem } from '../../lib/api';
import Icon from '../common/Icon.vue';

const ui = useUiStore();
const lib = useLibraryStore();
const collections = useCollectionsStore();
const { t } = useI18n();

const item = computed(() => lib.byId(ui.previewId));

const addToOpen = ref(false);
const editingTitle = ref(false);
const titleDraft = ref('');
const confirmingDelete = ref(false);
const editCategory = ref(false);

function startEdit() {
  if (!item.value) return;
  titleDraft.value = item.value.title;
  editingTitle.value = true;
}

async function commitTitle() {
  editingTitle.value = false;
  const v = titleDraft.value.trim();
  if (item.value && v && v !== item.value.title) {
    await lib.patch(item.value, { title: v });
  }
}

function toggleFav() {
  if (item.value) lib.toggleFavorite(item.value);
}

async function doDelete() {
  if (!item.value) return;
  if (!confirmingDelete.value) {
    confirmingDelete.value = true;
    setTimeout(() => (confirmingDelete.value = false), 3000);
    return;
  }
  await lib.remove(item.value.id);
}

function onKey(e: KeyboardEvent) {
  if (ui.previewId == null) return;
  if (e.key === 'Escape') {
    if (addToOpen.value) {
      addToOpen.value = false;
      return;
    }
    if (editingTitle.value) {
      editingTitle.value = false;
      return;
    }
    ui.previewId = null;
  } else if (e.key.toLowerCase() === 'f' && !editingTitle.value) {
    toggleFav();
  }
}

onMounted(() => window.addEventListener('keydown', onKey));
onUnmounted(() => window.removeEventListener('keydown', onKey));

const sourceLabel = computed(() => {
  if (!item.value) return '';
  if (item.value.source === 'ai') return `${t('preview.ai')} · ${item.value.model ?? ''}`;
  if (item.value.source === 'url') return t('preview.fromLink');
  return t('preview.imported');
});

const sizeLabel = computed(() => {
  if (!item.value) return '';
  const mb = item.value.fileSize / (1024 * 1024);
  return mb >= 1 ? `${mb.toFixed(1)} MB` : `${Math.round(item.value.fileSize / 1024)} KB`;
});

// —— 加入集合 ——
function toggleAddTo() {
  addToOpen.value = !addToOpen.value;
}

function inCollection(collectionId: string): boolean {
  return !!item.value && collections.byId(collectionId)?.itemIds.includes(item.value.id) === true;
}

async function toggleInCollection(collectionId: string) {
  if (!item.value) return;
  if (inCollection(collectionId)) {
    await collections.removeItem(collectionId, item.value.id);
  } else {
    await collections.addItem(collectionId, item.value.id);
  }
}

async function createAndAdd() {
  const name = prompt(t('collections.namePlaceholder'));
  if (!name || !item.value) return;
  const c = await collections.create(name);
  if (c && item.value) await collections.addItem(c.id, item.value.id);
}

async function copyPrompt() {
  const p = item.value?.prompt;
  if (!p) return;
  try {
    await navigator.clipboard.writeText(p);
    useUiStore().toast('success', t('toast.promptCopied'));
  } catch (e) {
    useUiStore().toast('error', String(e));
  }
}

async function openInExplorer() {
  if (item.value) await revealItem(item.value.filePath);
}

function openSource() {
  const url = item.value?.originUrl;
  if (url) window.open(url, '_blank');
}
</script>

<template>
  <div v-if="item" class="preview-backdrop" @click.self="ui.previewId = null">
    <div class="preview overlay-spring">
      <button class="close-btn" :title="t('preview.close') + ' (Esc)'" @click="ui.previewId = null">
        <Icon name="x" :size="16" />
      </button>

      <div class="stage">
        <img :src="assetUrl(item.filePath)" draggable="false" @dblclick="openInExplorer" />
      </div>

      <div class="bar">
        <div class="bar-info">
          <div v-if="!editingTitle" class="title-row" @dblclick="startEdit">
            <h2 class="title">{{ item.title }}</h2>
            <button class="mini-icon" :title="t('preview.rename')" @click="startEdit">
              <Icon name="pencil" :size="13" />
            </button>
          </div>
          <input
            v-else
            v-model="titleDraft"
            class="title-input"
            @keydown.enter="commitTitle"
            @keydown.esc="editingTitle = false"
            @blur="commitTitle"
            autofocus
          />
          <p class="meta">
            <button class="cat" :class="{ editing: editCategory }" @click="editCategory = !editCategory">
              {{ item.category ? t(`cat.${item.category.toLowerCase()}`) : t('cat.uncategorized') }}
            </button>
            <span class="dot">·</span>
            <span>{{ item.width }}×{{ item.height }}</span>
            <span class="dot">·</span>
            <span>{{ sizeLabel }}</span>
            <span class="dot">·</span>
            <span>{{ sourceLabel }}</span>
          </p>
          <div v-if="editCategory" class="cat-chips">
            <button
              v-for="c in CATEGORIES"
              :key="c"
              :class="{ active: item.category === c }"
              @click="lib.patch(item, { category: c }); editCategory = false"
            >
              {{ t(`cat.${c.toLowerCase()}`) }}
            </button>
          </div>
          <div v-if="item.prompt" class="prompt-row">
            <p class="prompt">“{{ item.prompt }}”</p>
            <button class="mini-icon" :title="t('preview.copyPrompt')" @click="copyPrompt">
              <Icon name="copy" :size="13" />
            </button>
          </div>
        </div>

        <div class="bar-actions">
          <button
            v-if="item.originUrl"
            class="icon-btn"
            :title="t('preview.link')"
            @click="openSource"
          >
            <Icon name="link" :size="16" />
          </button>
          <div class="add-to">
            <button
              class="icon-btn"
              :class="{ active: addToOpen }"
              :title="t('collections.addTo')"
              @click="toggleAddTo"
            >
              <Icon name="plus" :size="16" />
            </button>
            <div v-if="addToOpen" class="add-pop glass">
              <button class="add-new" @click="createAndAdd">
                <Icon name="plus" :size="13" />
                {{ t('collections.new') }}
              </button>
              <template v-if="collections.collections.length">
                <button
                  v-for="c in collections.collections"
                  :key="c.id"
                  class="add-row"
                  @click="toggleInCollection(c.id)"
                >
                  <Icon
                    :name="inCollection(c.id) ? 'check' : 'folder'"
                    :size="13"
                  />
                  <span class="add-name">{{ c.name }}</span>
                  <span class="add-count">{{ c.itemIds.length }}</span>
                </button>
              </template>
              <p v-else class="add-empty">{{ t('collections.noneYet') }}</p>
            </div>
          </div>
          <button class="icon-btn" :title="t('preview.folder')" @click="openInExplorer">
            <Icon name="external" :size="16" />
          </button>
          <button
            class="icon-btn heart"
            :class="{ loved: item.favorite }"
            :title="t('preview.favorite') + ' (F)'"
            @click="toggleFav"
          >
            <Icon name="heart" :size="17" />
          </button>
          <button
            class="icon-btn danger"
            :class="{ confirming: confirmingDelete }"
            :title="t('preview.confirmDelete')"
            @click="doDelete"
          >
            <Icon name="trash" :size="16" />
          </button>
          <button class="btn-primary apply" :disabled="lib.applyingId === item.id" @click="lib.apply(item.id)">
            {{ lib.applyingId === item.id ? t('preview.applying') : t('preview.apply') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.preview-backdrop {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: grid;
  place-items: center;
  padding: 40px;
  background: var(--veil);
  backdrop-filter: blur(42px) saturate(1.15);
  -webkit-backdrop-filter: blur(42px) saturate(1.15);
}

.preview {
  position: relative;
  width: min(1100px, 100%);
  max-height: calc(100vh - 80px);
  display: flex;
  flex-direction: column;
  border-radius: var(--radius-overlay);
  overflow: hidden;
  background: var(--glass-strong);
  border: 1px solid var(--stroke-strong);
  box-shadow: 0 40px 120px rgba(0, 0, 0, 0.4);
}

.close-btn {
  position: absolute;
  top: 14px;
  left: 14px;
  z-index: 5;
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: var(--glass-strong);
  border: 1px solid var(--stroke);
  color: var(--text-2);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  transition:
    color var(--dur-1) var(--ease-out),
    background var(--dur-1) var(--ease-out);
}

.close-btn:hover {
  color: var(--text-1);
  background: var(--fill-hover);
}

.stage {
  flex: 1;
  min-height: 0;
  display: grid;
  place-items: center;
  background:
    radial-gradient(120% 120% at 50% 0%, var(--glow) 0%, transparent 60%),
    var(--bg-stage);
}

.stage img {
  max-width: 100%;
  max-height: min(72vh, 780px);
  object-fit: contain;
}

.bar {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 24px;
  padding: 18px 24px 20px;
  border-top: 1px solid var(--stroke);
  background: var(--glass-strong);
  backdrop-filter: blur(30px);
  -webkit-backdrop-filter: blur(30px);
}

.bar-info {
  min-width: 0;
  flex: 1;
}

.title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.title {
  font-size: 20px;
  font-weight: 640;
  letter-spacing: -0.015em;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mini-icon {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  border-radius: 6px;
  color: var(--text-3);
  flex-shrink: 0;
  transition: color var(--dur-1) var(--ease-out);
}

.mini-icon:hover {
  color: var(--text-1);
}

.title-input {
  width: 100%;
  max-width: 420px;
  background: var(--fill-hover);
  border: 1px solid var(--stroke-strong);
  border-radius: 8px;
  padding: 5px 10px;
  font-size: 17px;
  font-weight: 600;
  color: var(--text-1);
  outline: none;
}

.meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 5px;
  font-size: 13px;
  color: var(--text-3);
  flex-wrap: wrap;
}

.dot {
  opacity: 0.5;
}

.cat {
  font-size: 12px;
  color: var(--text-2);
  border: 1px solid var(--stroke);
  border-radius: 100px;
  padding: 2px 10px;
  transition: all var(--dur-1) var(--ease-out);
}

.cat:hover,
.cat.editing {
  color: var(--text-1);
  border-color: var(--stroke-strong);
}

.cat-chips {
  display: flex;
  gap: 6px;
  margin-top: 10px;
  flex-wrap: wrap;
}

.cat-chips button {
  font-size: 12px;
  color: var(--text-3);
  border: 1px solid var(--stroke);
  border-radius: 100px;
  padding: 3px 12px;
  transition: all var(--dur-1) var(--ease-out);
}

.cat-chips button:hover {
  color: var(--text-1);
}

.cat-chips button.active {
  color: var(--text-1);
  background: var(--fill-active);
  border-color: var(--stroke-strong);
}

.prompt {
  margin-top: 10px;
  font-size: 12.5px;
  color: var(--text-3);
  line-height: 1.55;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  max-width: 560px;
}

.bar-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.add-to {
  position: relative;
}

.add-pop {
  position: absolute;
  bottom: calc(100% + 10px);
  right: 0;
  min-width: 220px;
  max-height: 260px;
  overflow-y: auto;
  padding: 6px;
  border-radius: 14px;
  z-index: 20;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.35);
}

.add-new {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  border-radius: 9px;
  font-size: 13px;
  color: var(--text-1);
  transition: background var(--dur-1) var(--ease-out);
}

.add-new:hover {
  background: var(--fill-hover);
}

.add-row {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  border-radius: 9px;
  font-size: 13px;
  color: var(--text-2);
  transition: background var(--dur-1) var(--ease-out), color var(--dur-1) var(--ease-out);
}

.add-row:hover {
  background: var(--fill-hover);
  color: var(--text-1);
}

.add-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: left;
}

.add-count {
  font-size: 11px;
  color: var(--text-3);
}

.add-empty {
  padding: 10px;
  font-size: 12.5px;
  color: var(--text-3);
  text-align: center;
}

.prompt-row {
  display: flex;
  align-items: flex-start;
  gap: 6px;
}

.prompt-row .prompt {
  flex: 1;
  min-width: 0;
}

.prompt-row .mini-icon {
  flex-shrink: 0;
  margin-top: 2px;
}

.icon-btn {
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  border-radius: 50%;
  border: 1px solid var(--stroke);
  color: var(--text-2);
  transition: all var(--dur-1) var(--ease-out);
}

.icon-btn:hover {
  color: var(--text-1);
  background: var(--fill-hover);
}

.icon-btn.heart.loved {
  color: var(--accent-heart);
  border-color: rgba(255, 90, 110, 0.35);
  background: rgba(255, 90, 110, 0.1);
}

.icon-btn.danger:hover,
.icon-btn.danger.confirming {
  color: #fff;
  background: var(--accent-danger);
  border-color: var(--accent-danger);
}

.apply {
  margin-left: 8px;
  padding: 10px 24px;
}
</style>
