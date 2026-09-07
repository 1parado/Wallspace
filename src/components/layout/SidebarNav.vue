<script setup lang="ts">
import { computed, nextTick, ref } from 'vue';
import { useUiStore } from '../../stores/ui';
import { useLibraryStore } from '../../stores/library';
import { useCollectionsStore } from '../../stores/collections';
import { useI18n } from '../../lib/i18n';
import { buildCategoryTree, matchCategory, type CatNode } from '../../lib/categoryTree';
import { open } from '@tauri-apps/plugin-dialog';
import Icon from '../common/Icon.vue';

const ui = useUiStore();
const lib = useLibraryStore();
const collections = useCollectionsStore();
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

const CATEGORY_ICONS: Record<string, string> = {
  Nature: 'globe',
  Space: 'sparkles',
  Abstract: 'compass',
  Cinematic: 'play',
  Minimal: 'image',
};

/** 分类树：只显示库内有内容的节点（含「未分类」空档位） */
const catTree = computed(() => buildCategoryTree(lib.items));
const uncategorizedCount = computed(
  () => lib.items.filter((i) => !i.category?.trim()).length
);

/** 已展开的父分类（本地 UI 状态） */
const expanded = ref(new Set<string>());

function toggleExpand(key: string) {
  const next = new Set(expanded.value);
  if (next.has(key)) next.delete(key);
  else next.add(key);
  expanded.value = next;
}

/** 顶层分类是否被「前缀命中」（选中父分类时高亮它） */
function isTreeActive(node: CatNode): boolean {
  return ui.view === 'wallpapers' && ui.categoryFilter !== null
    ? matchCategory(ui.categoryFilter, node.key)
    : false;
}

function catLabel(key: string | null): string {
  if (!key) return t('cat.uncategorized');
  // 子分类（无内置翻译）直接显示最后一段原名
  return key.includes('/')
    ? key.split('/').pop()!
    : t('cat.' + key.toLowerCase());
}

function goCategory(cat: string) {
  ui.view = 'wallpapers';
  // '' = 未分类；null = 全部
  ui.categoryFilter = cat;
  ui.search = '';
}

// —— 集合（用户驱动）——
const creatingCollection = ref(false);
const newNameDraft = ref('');
const editingId = ref<string | null>(null);
const renameDraft = ref('');
const renameInput = ref<HTMLInputElement | null>(null);

async function startCreate() {
  creatingCollection.value = true;
  newNameDraft.value = '';
  await nextTick();
  (document.activeElement as HTMLElement)?.blur?.();
}

async function commitCreate() {
  const name = newNameDraft.value.trim();
  creatingCollection.value = false;
  if (!name) return;
  const c = await collections.create(name);
  if (c) {
    ui.activeCollectionId = c.id;
    ui.view = 'collection';
  }
}

async function startRename(id: string) {
  editingId.value = id;
  renameDraft.value = collections.byId(id)?.name ?? '';
  await nextTick();
  renameInput.value?.focus();
  renameInput.value?.select();
}

async function commitRename() {
  const id = editingId.value;
  editingId.value = null;
  if (id) await collections.rename(id, renameDraft.value);
}

function openCollection(id: string) {
  ui.activeCollectionId = id;
  ui.view = 'collection';
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
        :title="t(n.labelKey)"
        :aria-label="t(n.labelKey)"
        @click="ui.goto(n.id)"
      >
        <Icon :name="n.icon" :size="17" />
        <span class="nav-text">{{ t(n.labelKey) }}</span>
      </button>
    </nav>

    <div class="section">
      <p class="section-label">{{ t('nav.categories') }}</p>
      <template v-for="node in catTree" :key="node.key">
        <button
          class="nav-item sub"
          :class="{ active: ui.view === 'wallpapers' && isTreeActive(node) }"
          :title="catLabel(node.key)"
          :aria-label="catLabel(node.key)"
          @click="goCategory(node.key)"
        >
          <button
            v-if="node.children.length"
            class="twist"
            :class="{ open: expanded.has(node.key) }"
            :title="t('nav.expand')"
            @click.stop="toggleExpand(node.key)"
          >
            <Icon name="chevron-down" :size="12" />
          </button>
          <Icon :name="CATEGORY_ICONS[node.key] ?? 'image'" :size="16" />
          <span class="nav-text">{{ catLabel(node.key) }}</span>
          <span v-if="node.children.length" class="sub-n">{{ node.children.length }}</span>
        </button>
        <button
          v-for="child in expanded.has(node.key) ? node.children : []"
          :key="child.key"
          class="nav-item sub child"
          :class="{ active: ui.view === 'wallpapers' && ui.categoryFilter === child.key }"
          :title="catLabel(child.key)"
          :aria-label="catLabel(child.key)"
          @click="goCategory(child.key)"
        >
          <span class="twist-gap" />
          <Icon name="image" :size="14" />
          <span class="nav-text">{{ catLabel(child.key) }}</span>
        </button>
      </template>
      <button
        v-if="uncategorizedCount"
        class="nav-item sub"
        :class="{ active: ui.view === 'wallpapers' && ui.categoryFilter === '' }"
        :title="t('cat.uncategorized')"
        @click="goCategory('')"
      >
        <Icon name="image" :size="16" />
        <span class="nav-text">{{ t('cat.uncategorized') }}</span>
      </button>
    </div>

    <div class="section">
      <p class="section-label">{{ t('nav.collections') }}</p>
      <template v-for="c in collections.collections" :key="c.id">
        <input
          v-if="editingId === c.id"
          ref="renameInput"
          v-model="renameDraft"
          class="rename-input"
          :maxlength="24"
          @keydown.enter="commitRename"
          @keydown.esc="editingId = null"
          @blur="commitRename"
        />
        <button
          v-else
          class="nav-item sub"
          :class="{ active: ui.view === 'collection' && ui.activeCollectionId === c.id }"
          :title="c.name"
          @click="openCollection(c.id)"
        >
          <Icon name="folder" :size="16" />
          <span class="nav-text coll-name">{{ c.name }}</span>
          <span class="coll-actions" @click.stop>
            <button class="mini-act" :title="t('collections.rename')" @click="startRename(c.id)">
              <Icon name="pencil" :size="12" />
            </button>
            <button class="mini-act danger" :title="t('collections.delete')" @click="collections.remove(c.id)">
              <Icon name="trash" :size="12" />
            </button>
          </span>
        </button>
      </template>
      <button v-if="!creatingCollection" class="nav-item sub add-coll" :title="t('collections.new')" @click="startCreate">
        <Icon name="plus" :size="16" />
        <span class="nav-text">{{ t('collections.new') }}</span>
      </button>
      <input
        v-else
        v-model="newNameDraft"
        class="rename-input"
        :placeholder="t('collections.namePlaceholder')"
        :maxlength="24"
        @keydown.enter="commitCreate"
        @keydown.esc="creatingCollection = false"
        @blur="commitCreate"
      />
    </div>

    <div class="section">
      <p class="section-label">{{ t('nav.personal') }}</p>
      <button
        v-for="p in PERSONAL"
        :key="p.id"
        class="nav-item sub"
        :class="{ active: ui.view === p.id }"
        :title="t(p.labelKey)"
        :aria-label="t(p.labelKey)"
        @click="ui.goto(p.id)"
      >
        <Icon :name="p.icon" :size="16" />
        <span class="nav-text">{{ t(p.labelKey) }}</span>
      </button>
    </div>

    <div class="spacer" />

    <button class="import-cta" :title="t('nav.importOwn')" :aria-label="t('nav.importOwn')" @click="importOwn">
      <Icon name="upload" :size="15" />
      <span class="nav-text">{{ t('nav.importOwn') }}</span>
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
  padding: 14px 8px 12px;
  background: var(--glass-side);
  border-right: 1px solid var(--stroke);
  backdrop-filter: blur(18px) saturate(1.05);
  -webkit-backdrop-filter: blur(18px) saturate(1.05);
}

.brand {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 4px 0 20px;
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
  display: none;
}

.nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-item {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 9px;
  width: 100%;
  padding: 9px 8px;
  border-radius: 10px;
  font-size: 13px;
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
  position: absolute;
  margin: -18px 0 0 20px;
  min-width: 14px;
  text-align: center;
  background: var(--text-1);
  color: var(--bg);
  border-radius: 999px;
  font-size: 11.5px;
  font-weight: 600;
}

.section {
  margin-top: 18px;
}

.section-label {
  font-size: 11px;
  font-weight: 560;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-3);
  padding: 0 0 6px;
  text-align: center;
  font-size: 0;
}

.section-label::before {
  content: '';
  display: block;
  width: 18px;
  height: 1px;
  margin: 0 auto;
  background: var(--stroke);
}

.nav-item.sub {
  padding: 8px;
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
  border-radius: 10px;
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
  display: none;
}

.rename-input {
  width: 100%;
  padding: 7px 10px;
  border-radius: 10px;
  border: 1px solid var(--stroke-strong);
  background: var(--fill-subtle);
  color: var(--text-1);
  font-size: 12.5px;
  outline: none;
}

.rename-input::placeholder {
  color: var(--text-3);
}

.coll-actions {
  display: none;
  position: absolute;
  right: 6px;
  gap: 2px;
}

.nav-item:hover .coll-actions {
  display: inline-flex;
}

.mini-act {
  display: grid;
  place-items: center;
  width: 20px;
  height: 20px;
  border-radius: 6px;
  color: var(--text-3);
  transition:
    background var(--dur-1) var(--ease-out),
    color var(--dur-1) var(--ease-out);
}

.mini-act:hover {
  color: var(--text-1);
  background: var(--fill-hover);
}

.mini-act.danger:hover {
  color: var(--accent-danger, #e5484d);
}

.add-coll {
  color: var(--text-3);
}

.add-coll:hover {
  color: var(--text-1);
}

.nav-text {
  display: none;
}
</style>
