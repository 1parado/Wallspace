<script setup lang="ts">
import { computed, ref } from 'vue';
import type { WallpaperItem } from '../types';
import { useLibraryStore } from '../stores/library';
import { useUiStore } from '../stores/ui';
import { useI18n } from '../lib/i18n';
import { buildCategoryTree, matchCategory, displayCategory, type CatNode } from '../lib/categoryTree';
import WallpaperGrid from '../components/wallpaper/WallpaperGrid.vue';
import EmptyState from '../components/common/EmptyState.vue';
import Icon from '../components/common/Icon.vue';

const lib = useLibraryStore();
const ui = useUiStore();
const { t } = useI18n();

function ratioOf(i: WallpaperItem): string {
  const r = i.width / i.height;
  if (r >= 2) return 'wide';
  if (r > 1.2) return 'landscape';
  if (r >= 0.8) return 'square';
  return 'portrait';
}

const RATIO_KEYS = ['wide', 'landscape', 'square', 'portrait'] as const;
const SOURCES = ['ai', 'url', 'local'] as const;

/** 分类树：只显示库内有内容的节点 */
const catTree = computed(() => buildCategoryTree(lib.items));

const uncategorizedCount = computed(
  () => lib.items.filter((i) => !i.category?.trim()).length
);

/** 选中分类对应树节点（用于展开其子分类 chips 行） */
function findNode(nodes: CatNode[], key: string): CatNode | null {
  for (const n of nodes) {
    if (n.key === key) return n;
    const hit = findNode(n.children, key);
    if (hit) return hit;
  }
  return null;
}

const selectedNode = computed(() =>
  ui.categoryFilter ? findNode(catTree.value, ui.categoryFilter) : null
);

/** 标签 facet：按出现次数排序；默认展示前 14 个，可展开全部 */
const TAG_PREVIEW = 14;
const tagsExpanded = ref(false);

const allTags = computed(() => {
  const counts = new Map<string, number>();
  for (const i of lib.items) {
    for (const tg of i.tags ?? []) counts.set(tg, (counts.get(tg) ?? 0) + 1);
  }
  return [...counts.entries()]
    .sort((a, b) => b[1] - a[1])
    .map(([tag, count]) => ({ tag, count }));
});

const topTags = computed(() =>
  tagsExpanded.value ? allTags.value : allTags.value.slice(0, TAG_PREVIEW)
);

const hiddenTagCount = computed(() => Math.max(0, allTags.value.length - TAG_PREVIEW));

/** 颜色 facet：聚合所有调色板，按占比取前 8 */
const paletteColors = computed(() => {
  const counts = new Map<string, number>();
  for (const i of lib.items) {
    for (const c of i.palette ?? []) counts.set(c, (counts.get(c) ?? 0) + 1);
  }
  return [...counts.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, 8)
    .map(([hex, count]) => ({ hex, count }));
});

const sourceCount = (s: string) => lib.items.filter((i) => i.source === s).length;

const ratioCount = (r: string) => lib.items.filter((i) => ratioOf(i) === r).length;

const filtered = computed(() => {
  let items = lib.items;
  if (ui.categoryFilter !== null) {
    items = items.filter((i) => matchCategory(i.category, ui.categoryFilter));
  }
  if (ui.sourceFilter.length) {
    items = items.filter((i) => ui.sourceFilter.includes(i.source));
  }
  if (ui.ratioFilter) {
    items = items.filter((i) => ratioOf(i) === ui.ratioFilter);
  }
  if (ui.colorFilter) {
    items = items.filter((i) => i.palette?.includes(ui.colorFilter!));
  }
  if (ui.tagFilter.length) {
    items =
      ui.tagMode === 'all'
        ? items.filter((i) => ui.tagFilter.every((tg) => i.tags?.includes(tg)))
        : items.filter((i) => ui.tagFilter.some((tg) => i.tags?.includes(tg)));
  }
  const q = ui.search.trim().toLowerCase();
  if (q) {
    items = items.filter(
      (i) =>
        i.title.toLowerCase().includes(q) ||
        (i.category ?? '').toLowerCase().includes(q) ||
        (i.prompt ?? '').toLowerCase().includes(q) ||
        i.tags?.some((tg) => tg.toLowerCase().includes(q)) ||
        `${i.width}x${i.height}`.includes(q)
    );
  }
  return items;
});

const hasAny = computed(() => lib.items.length > 0);
const isFiltering = computed(
  () =>
    ui.categoryFilter !== null ||
    !!ui.search.trim() ||
    ui.tagFilter.length > 0 ||
    ui.sourceFilter.length > 0 ||
    !!ui.ratioFilter ||
    !!ui.colorFilter
);

// —— 排序 + 随机应用 ——
const SORTS = [
  { id: 'newest', labelKey: 'facets.sortNewest' },
  { id: 'oldest', labelKey: 'facets.sortOldest' },
  { id: 'name', labelKey: 'facets.sortName' },
  { id: 'resolution', labelKey: 'facets.sortResolution' },
  { id: 'random', labelKey: 'facets.sortRandom' },
] as const;

/** 种子随机数（mulberry32）：同一种子序列稳定，换种子即重新洗牌 */
function seededRandom(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const sorted = computed(() => {
  const arr = [...filtered.value];
  switch (ui.sortMode) {
    case 'oldest':
      return arr.sort((a, b) => a.createdAt - b.createdAt);
    case 'name':
      return arr.sort((a, b) => a.title.localeCompare(b.title, undefined, { sensitivity: 'base' }));
    case 'resolution':
      return arr.sort((a, b) => b.width * b.height - a.width * a.height);
    case 'random': {
      const rnd = seededRandom(ui.sortSeed || 1);
      for (let i = arr.length - 1; i > 0; i--) {
        const j = Math.floor(rnd() * (i + 1));
        [arr[i], arr[j]] = [arr[j], arr[i]];
      }
      return arr;
    }
    default:
      return arr.sort((a, b) => b.createdAt - a.createdAt);
  }
});

/** 随机换一张：从当前过滤结果中随机挑一张立即应用 */
async function applyRandom() {
  if (!sorted.value.length || lib.applyingId) return;
  const pick = sorted.value[Math.floor(Math.random() * sorted.value.length)];
  await lib.apply(pick.id);
}

function catLabel(key: string): string {
  return displayCategory(key, t, t('cat.uncategorized'));
}

/** 已选过滤器汇总条：每个激活条件一个可单独移除的 chip */
interface FilterChip {
  key: string;
  label: string;
  clear: () => void;
}

const activeChips = computed<FilterChip[]>(() => {
  const chips: FilterChip[] = [];
  const q = ui.search.trim();
  if (q) chips.push({ key: 'search', label: `"${q}"`, clear: () => (ui.search = '') });
  if (ui.categoryFilter !== null)
    chips.push({
      key: 'cat',
      label: catLabel(ui.categoryFilter),
      clear: () => (ui.categoryFilter = null),
    });
  for (const tg of ui.tagFilter)
    chips.push({ key: `tag:${tg}`, label: `#${tg}`, clear: () => ui.toggleTag(tg) });
  for (const s of ui.sourceFilter)
    chips.push({
      key: `src:${s}`,
      label: t(`facets.src.${s}`),
      clear: () => toggleSource(s as 'ai' | 'url' | 'local'),
    });
  if (ui.ratioFilter)
    chips.push({
      key: 'ratio',
      label: t(`facets.ratio.${ui.ratioFilter}`),
      clear: () => (ui.ratioFilter = null),
    });
  if (ui.colorFilter)
    chips.push({
      key: 'color',
      label: ui.colorFilter.toUpperCase(),
      clear: () => (ui.colorFilter = null),
    });
  return chips;
});

function toggleSource(s: 'ai' | 'url' | 'local') {
  ui.sourceFilter = ui.sourceFilter.includes(s)
    ? ui.sourceFilter.filter((x) => x !== s)
    : [...ui.sourceFilter, s];
}
</script>

<template>
  <div class="wallpapers">
    <div class="facets">
      <!-- 已选过滤器汇总条：逐项移除 -->
      <div v-if="activeChips.length" class="facet-row summary-row">
        <span class="facet-label">{{ t('facets.active') }}</span>
        <button
          v-for="chip in activeChips"
          :key="chip.key"
          class="chip summary-chip"
          :title="t('facets.removeFilter')"
          @click="chip.clear()"
        >
          {{ chip.label }}
          <Icon name="x" :size="11" />
        </button>
        <button class="chip reset" @click="ui.resetFacets(); ui.search = ''">
          {{ t('facets.reset') }}
        </button>
      </div>

      <!-- 分类：动态树（只显示库内有内容的），选中父分类时展开其子分类 -->
      <div class="facet-row">
        <span class="facet-label">{{ t('facets.category') }}</span>
        <button
          class="chip"
          :class="{ active: ui.categoryFilter === null }"
          @click="ui.categoryFilter = null"
        >
          {{ t('toolbar.allWallpapers') }}
        </button>
        <button
          v-for="node in catTree"
          :key="node.key"
          class="chip"
          :class="{ active: ui.categoryFilter !== null && matchCategory(ui.categoryFilter, node.key) }"
          @click="ui.categoryFilter = ui.categoryFilter === node.key ? null : node.key"
        >
          {{ catLabel(node.key) }}
          <span class="chip-count">{{ node.count }}</span>
        </button>
        <button
          v-if="uncategorizedCount"
          class="chip"
          :class="{ active: ui.categoryFilter === '' }"
          @click="ui.categoryFilter = ui.categoryFilter === '' ? null : ''"
        >
          {{ t('cat.uncategorized') }}
          <span class="chip-count">{{ uncategorizedCount }}</span>
        </button>
      </div>

      <!-- 子分类行：选中某分类后展开 -->
      <div v-if="selectedNode && selectedNode.children.length" class="facet-row sub-row">
        <span class="facet-label">{{ t('facets.subcategory') }}</span>
        <button
          class="chip"
          :class="{ active: ui.categoryFilter === selectedNode.key }"
          @click="ui.categoryFilter = selectedNode.key"
        >
          {{ t('facets.allSub') }}
          <span class="chip-count">{{ selectedNode.count }}</span>
        </button>
        <button
          v-for="child in selectedNode.children"
          :key="child.key"
          class="chip"
          :class="{ active: ui.categoryFilter === child.key }"
          @click="ui.categoryFilter = ui.categoryFilter === child.key ? selectedNode.key : child.key"
        >
          {{ catLabel(child.key) }}
          <span class="chip-count">{{ child.count }}</span>
        </button>
      </div>

      <!-- 标签：多选 + AND/OR -->
      <div v-if="topTags.length" class="facet-row">
        <span class="facet-label">{{ t('facets.tags') }}</span>
        <button
          v-for="tg in topTags"
          :key="tg.tag"
          class="chip"
          :class="{ active: ui.tagFilter.includes(tg.tag) }"
          @click="ui.toggleTag(tg.tag)"
        >
          #{{ tg.tag }}
          <span class="chip-count">{{ tg.count }}</span>
        </button>
        <span class="tag-mode">
          <button
            :class="{ active: ui.tagMode === 'any' }"
            :title="t('facets.modeAny')"
            @click="ui.tagMode = 'any'"
          >{{ t('facets.modeAny') }}</button>
          <button
            :class="{ active: ui.tagMode === 'all' }"
            :title="t('facets.modeAll')"
            @click="ui.tagMode = 'all'"
          >{{ t('facets.modeAll') }}</button>
        </span>
        <button
          v-if="!tagsExpanded && hiddenTagCount > 0"
          class="chip more"
          @click="tagsExpanded = true"
        >
          {{ t('facets.moreTags', { n: hiddenTagCount }) }}
        </button>
        <button v-else-if="tagsExpanded && hiddenTagCount > 0" class="chip more" @click="tagsExpanded = false">
          {{ t('facets.lessTags') }}
        </button>
      </div>

      <!-- 来源 / 比例 / 颜色 -->
      <div class="facet-row">
        <span class="facet-label">{{ t('facets.source') }}</span>
        <button
          v-for="s in SOURCES"
          :key="s"
          class="chip"
          :class="{ active: ui.sourceFilter.includes(s) }"
          @click="toggleSource(s)"
        >
          {{ t(`facets.src.${s}`) }}
          <span class="chip-count">{{ sourceCount(s) }}</span>
        </button>

        <span class="facet-sep" />
        <span class="facet-label">{{ t('facets.ratio') }}</span>
        <button
          v-for="r in RATIO_KEYS"
          :key="r"
          class="chip"
          :class="{ active: ui.ratioFilter === r }"
          @click="ui.ratioFilter = ui.ratioFilter === r ? null : r"
        >
          {{ t(`facets.ratio.${r}`) }}
          <span class="chip-count">{{ ratioCount(r) }}</span>
        </button>

        <template v-if="paletteColors.length">
          <span class="facet-sep" />
          <span class="facet-label">{{ t('facets.color') }}</span>
          <button
            v-for="c in paletteColors"
            :key="c.hex"
            class="swatch"
            :class="{ active: ui.colorFilter === c.hex }"
            :style="{ background: c.hex }"
            :title="`${c.hex} · ${c.count}`"
            @click="ui.colorFilter = ui.colorFilter === c.hex ? null : c.hex"
          />
        </template>

        <span class="facet-sep" />
        <button v-if="isFiltering" class="chip reset" @click="ui.resetFacets(); ui.search = ''">
          {{ t('facets.reset') }}
        </button>
      </div>
    </div>

    <!-- 结果栏：计数 + 排序 + 随机换一张 -->
    <div v-if="hasAny" class="grid-bar">
      <span class="result-count">{{ t('facets.resultCount', { n: filtered.length }) }}</span>
      <span class="bar-spacer" />
      <span class="facet-label">{{ t('facets.sort') }}</span>
      <button
        v-for="s in SORTS"
        :key="s.id"
        class="chip"
        :class="{ active: ui.sortMode === s.id }"
        @click="ui.setSort(s.id)"
      >
        {{ t(s.labelKey) }}
      </button>
      <span class="facet-sep" />
      <button
        class="chip shuffle-apply"
        :disabled="!!lib.applyingId || !filtered.length"
        :title="t('facets.randomApplyTip')"
        @click="applyRandom"
      >
        <Icon name="shuffle" :size="13" />
        {{ lib.applyingId ? t('preview.applying') : t('facets.randomApply') }}
      </button>
    </div>

    <WallpaperGrid v-if="filtered.length" :items="sorted" />

    <EmptyState
      v-else-if="!hasAny"
      :title="t('empty.library.title')"
      :subtitle="t('empty.library.sub')"
      :action-label="t('hero.create')"
      @action="ui.goto('create')"
    />
    <EmptyState
      v-else-if="isFiltering"
      :title="t('empty.nothing.title')"
      :subtitle="t('empty.nothing.sub')"
      :action-label="t('empty.showAll')"
      @action="ui.resetFacets(); ui.search = ''"
    />
  </div>
</template>

<style scoped>
.wallpapers {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.facets {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.facet-row.sub-row {
  padding-left: 14px;
  border-left: 2px solid var(--stroke);
  margin-left: 4px;
}

.facet-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

.facet-label {
  font-size: 11.5px;
  font-weight: 560;
  letter-spacing: 0.06em;
  color: var(--text-3);
  margin-right: 2px;
}

.facet-sep {
  width: 1px;
  height: 16px;
  background: var(--stroke);
  margin: 0 6px;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--text-3);
  border: 1px solid var(--stroke);
  border-radius: 100px;
  padding: 5px 13px;
  transition: all var(--dur-1) var(--ease-out);
}

.chip:hover {
  color: var(--text-1);
  border-color: var(--stroke-strong);
}

.chip.active {
  color: var(--text-1);
  background: var(--fill-active);
  border-color: var(--stroke-strong);
}

.chip.reset {
  color: var(--text-2);
  border-style: dashed;
}

.chip-count {
  font-size: 10.5px;
  color: var(--text-3);
  background: var(--fill-subtle);
  border-radius: 100px;
  padding: 1px 6px;
}

.chip.active .chip-count {
  color: var(--text-1);
  background: var(--fill-hover);
}

.tag-mode {
  display: inline-flex;
  border: 1px solid var(--stroke);
  border-radius: 100px;
  overflow: hidden;
}

.tag-mode button {
  font-size: 11px;
  padding: 4px 10px;
  color: var(--text-3);
  transition: all var(--dur-1) var(--ease-out);
}

.tag-mode button.active {
  color: var(--text-1);
  background: var(--fill-active);
}

.swatch {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: 2px solid var(--stroke);
  /* 中灰细内圈：亮/暗主题下都能与明暗色块形成对比 */
  box-shadow: inset 0 0 0 1px rgba(128, 128, 128, 0.45);
  cursor: pointer;
  transition:
    transform var(--dur-1) var(--ease-out),
    border-color var(--dur-1) var(--ease-out),
    box-shadow var(--dur-1) var(--ease-out);
}

.swatch:hover {
  transform: scale(1.12);
}

.swatch.active {
  border-color: var(--text-1);
  box-shadow:
    inset 0 0 0 1px rgba(128, 128, 128, 0.45),
    0 0 0 2px var(--fill-active);
  transform: scale(1.12);
}

.summary-row {
  padding-bottom: 8px;
  border-bottom: 1px dashed var(--stroke);
}

.summary-chip {
  color: var(--text-1);
  background: var(--fill-active);
  border-color: var(--stroke-strong);
  gap: 5px;
}

.summary-chip:hover {
  color: var(--text-1);
  border-color: var(--text-3);
}

.chip.more {
  color: var(--text-2);
  border-style: dashed;
}

.chip.more:hover {
  color: var(--text-1);
}

.grid-bar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.result-count {
  font-size: 12.5px;
  color: var(--text-3);
  margin-right: 6px;
}

.bar-spacer {
  flex: 1;
}

.shuffle-apply {
  color: var(--text-1);
  background: var(--fill-subtle);
}

.shuffle-apply:hover:not(:disabled) {
  background: var(--fill-hover);
  border-color: var(--stroke-strong);
}

.shuffle-apply:disabled {
  opacity: 0.55;
  cursor: default;
}
</style>
