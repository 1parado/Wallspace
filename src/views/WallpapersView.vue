<script setup lang="ts">
import { computed, ref } from 'vue';
import type { WallpaperItem } from '../types';
import { useLibraryStore } from '../stores/library';
import { useUiStore } from '../stores/ui';
import { useI18n } from '../lib/i18n';
import { buildCategoryTree, matchCategory, displayCategory, type CatNode } from '../lib/categoryTree';
import { sortItems } from '../lib/sortItems';
import { COLOR_FAMILIES, countByFamily, familyOfHex } from '../lib/colorFamily';
import { guessCategory, suggestTags } from '../lib/autoTag';
import * as api from '../lib/api';
import { assetUrl } from '../lib/api';import { useSettingsStore } from '../stores/settings';
import WallpaperGrid from '../components/wallpaper/WallpaperGrid.vue';
import GridToolbar from '../components/common/GridToolbar.vue';
import EmptyState from '../components/common/EmptyState.vue';
import Icon from '../components/common/Icon.vue';

const lib = useLibraryStore();
const ui = useUiStore();
const settings = useSettingsStore();
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

/** 颜色 facet：把调色板归入 12 个感知色系，只显示库内有内容的色系 */
const familyCounts = computed(() => countByFamily(lib.items));

const activeFamilies = computed(() =>
  COLOR_FAMILIES.filter((f) => (familyCounts.value.get(f.id) ?? 0) > 0)
);

const familyCount = (id: string) => familyCounts.value.get(id) ?? 0;

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
    items = items.filter((i) =>
      (i.palette ?? []).some((c) => familyOfHex(c) === ui.colorFilter)
    );
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

const sorted = computed(() => sortItems(filtered.value, ui.sortMode, ui.sortSeed));

/** 随机换一张：从当前过滤结果中随机挑一张立即应用 */
async function applyRandom() {
  if (!sorted.value.length || lib.applyingId) return;
  const pick = sorted.value[Math.floor(Math.random() * sorted.value.length)];
  await lib.apply(pick.id);
}

// —— 一键智能整理：规则回填 + 可选 LLM（classify_model）兜底 ——
const retagging = ref(false);
const retagProgress = ref<{ done: number; total: number } | null>(null);

const retaggable = computed(
  () => lib.items.filter((i) => !i.category?.trim() || !(i.tags?.length)).length
);

function textOf(item: WallpaperItem): string {
  // 文本源：标题 + 提示词 + 链接末段（文件名常含关键词）
  const tail = (item.originUrl ?? '').split(/[/?]/).filter(Boolean).pop() ?? '';
  return `${item.title} ${item.prompt ?? ''} ${decodeURIComponent(tail)}`;
}

/** 把分类/标签建议合并进条目：只补空缺、只追加新标签；返回是否实际变更 */
function mergeSuggestion(item: WallpaperItem, cat: string | null, tags: string[]): Partial<WallpaperItem> | null {
  const changes: Partial<WallpaperItem> = {};
  if (!item.category?.trim() && cat) changes.category = cat;
  const have = new Set(item.tags ?? []);
  const fresh = tags.filter((tg) => tg && !have.has(tg));
  if (fresh.length) changes.tags = [...(item.tags ?? []), ...fresh].slice(0, 8);
  return Object.keys(changes).length ? changes : null;
}

async function retagLibrary() {
  if (retagging.value) return;
  retagging.value = true;
  let touched = 0;
  let cats = 0;
  let tgs = 0;
  try {
    // 第一遍：关键词规则（本地、瞬时）
    for (const item of lib.items) {
      const text = textOf(item);
      const s = mergeSuggestion(item, guessCategory(text), suggestTags(text));
      if (s) {
        if (s.category) cats++;
        if (s.tags) tgs++;
        await lib.patch(item, s);
        touched++;
      }
    }

    // 第二遍：规则未解决的交给轻量模型（最多 40 条，失败/未配置即止）
    if ((settings.classifyModel ?? '').trim()) {
      const unresolved = lib.items
        .filter((i) => !i.category?.trim() || !(i.tags?.length))
        .slice(0, 40);
      retagProgress.value = { done: 0, total: unresolved.length };
      for (let n = 0; n < unresolved.length; n++) {
        retagProgress.value = { done: n + 1, total: unresolved.length };
        const r = await api.classifyText(textOf(unresolved[n]));
        if (!r) break; // 未配置或请求失败：停止，保留已有结果
        const item = unresolved[n];
        const s = mergeSuggestion(item, r.category, r.tags);
        if (s) {
          if (s.category) cats++;
          if (s.tags) tgs++;
          await lib.patch(item, s);
          touched++;
        }
      }
      retagProgress.value = null;
    }
  } finally {
    retagging.value = false;
    retagProgress.value = null;
  }
  ui.toast(
    touched ? 'success' : 'info',
    touched ? t('facets.retagDone', { n: touched, c: cats, t: tgs }) : t('facets.retagNone')
  );
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
      label: t(`facets.family.${ui.colorFilter}`),
      clear: () => (ui.colorFilter = null),
    });
  return chips;
});

function toggleSource(s: 'ai' | 'url' | 'local') {
  ui.sourceFilter = ui.sourceFilter.includes(s)
    ? ui.sourceFilter.filter((x) => x !== s)
    : [...ui.sourceFilter, s];
}

// —— 重复 / 相似图片检测 ——
interface GroupRow {
  ids: string[];
  /** 精确重复时为单张文件大小；相似检测为 null */
  fileSize: number | null;
}

const scanBusy = ref(false);
const scanKind = ref<'dup' | 'sim'>('dup');
const simThreshold = ref(8);
/** 弹窗数据：两种检测共用 */
const dupGroups = ref<GroupRow[] | null>(null);

function fmtMb(bytes: number): string {
  const mb = bytes / (1024 * 1024);
  return mb >= 1 ? `${mb.toFixed(1)} MB` : `${Math.round(bytes / 1024)} KB`;
}

const dupWasted = computed(() =>
  (dupGroups.value ?? []).reduce((sum, g) => sum + (g.fileSize ?? 0) * (g.ids.length - 1), 0)
);

function groupItems(g: GroupRow) {
  return g.ids.map((id) => lib.byId(id)).filter((i): i is WallpaperItem => !!i);
}

async function findDups() {
  if (scanBusy.value) return;
  scanBusy.value = true;
  try {
    const groups = await api.findDuplicates();
    scanKind.value = 'dup';
    dupGroups.value = groups.map((g) => ({ ids: g.ids, fileSize: g.fileSize }));
    if (!groups.length) ui.toast('info', t('dup.none'));
  } catch (e) {
    ui.toast('error', String(e));
  } finally {
    scanBusy.value = false;
  }
}

async function findSims() {
  if (scanBusy.value) return;
  scanBusy.value = true;
  try {
    const groups = await api.findSimilar(simThreshold.value);
    scanKind.value = 'sim';
    dupGroups.value = groups.map((g) => ({ ids: g.ids, fileSize: null }));
    if (!groups.length) ui.toast('info', t('dup.none'));
  } catch (e) {
    ui.toast('error', String(e));
  } finally {
    scanBusy.value = false;
  }
}

/** 清理一组：保留第一张，其余移入回收站 */
async function cleanGroup(gi: number) {
  const g = dupGroups.value?.[gi];
  if (!g) return;
  const dupes = g.ids.slice(1);
  for (const id of dupes) {
    await lib.remove(id);
  }
  dupGroups.value = (dupGroups.value ?? []).filter((_, i) => i !== gi);
  if (!dupGroups.value.length) dupGroups.value = null;
  ui.toast('success', t('dup.removed', { n: dupes.length }));
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

        <template v-if="activeFamilies.length">
          <span class="facet-sep" />
          <span class="facet-label">{{ t('facets.color') }}</span>
          <button
            v-for="f in activeFamilies"
            :key="f.id"
            class="swatch"
            :class="{ active: ui.colorFilter === f.id }"
            :style="{ background: f.hex }"
            :title="`${t(f.labelKey)} · ${familyCount(f.id)}`"
            @click="ui.colorFilter = ui.colorFilter === f.id ? null : f.id"
          />
        </template>

        <span class="facet-sep" />
        <button v-if="isFiltering" class="chip reset" @click="ui.resetFacets(); ui.search = ''">
          {{ t('facets.reset') }}
        </button>
      </div>
    </div>

    <!-- 一键智能整理：库里有缺分类/标签的条目时出现 -->
    <div v-if="retaggable > 0" class="retag-row">
      <span class="result-count">{{ t('facets.retagCta', { n: retaggable }) }}</span>
      <button class="chip retag-btn" :disabled="retagging" @click="retagLibrary">
        <Icon name="sparkles" :size="13" />
        {{ retagProgress
          ? t('facets.retaggingN', { done: retagProgress.done, total: retagProgress.total })
          : retagging ? t('facets.retagging') : t('facets.retagRun') }}
      </button>
    </div>

    <!-- 重复 / 相似检测：库维护 -->
    <div v-if="hasAny" class="retag-row">
      <span class="result-count">{{ t('dup.cta') }}</span>
      <button class="chip retag-btn" :disabled="scanBusy" @click="findDups">
        <Icon name="copy" :size="13" />
        {{ scanBusy && scanKind === 'dup' ? t('dup.scanning') : t('dup.scan') }}
      </button>
      <button class="chip retag-btn" :disabled="scanBusy" @click="findSims">
        <Icon name="sparkles" :size="13" />
        {{ scanBusy && scanKind === 'sim' ? t('dup.scanning') : t('sim.scan') }}
      </button>
      <select v-model="simThreshold" class="chip sim-threshold" :title="t('sim.threshold')">
        <option :value="4">{{ t('sim.strict') }}</option>
        <option :value="8">{{ t('sim.normal') }}</option>
        <option :value="14">{{ t('sim.loose') }}</option>
      </select>
    </div>

    <!-- 结果栏：计数 + 排序 + 随机换一张 -->
    <GridToolbar
      v-if="hasAny"
      :count="filtered.length"
      :sorts="SORTS"
      :model-value="ui.sortMode"
      :seed="ui.sortSeed"
      :applying="lib.applyingId"
      @update:model-value="ui.setSort($event as never)"
      @reshuffle="ui.setSort('random')"
      @apply="applyRandom"
    />

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

    <!-- 重复 / 相似检测结果弹窗 -->
    <div v-if="dupGroups" class="dup-backdrop" @click.self="dupGroups = null">
      <div class="dup-modal glass">
        <div class="dup-head">
          <h3>{{ scanKind === 'dup' ? t('dup.title') : t('sim.title') }}</h3>
          <button class="dup-close" @click="dupGroups = null">
            <Icon name="x" :size="14" />
          </button>
        </div>
        <p class="dup-summary">
          {{
            scanKind === 'dup'
              ? t('dup.summary', { g: dupGroups.length, mb: fmtMb(dupWasted) })
              : t('sim.summary', { g: dupGroups.length })
          }}
        </p>
        <p v-if="!dupGroups.length" class="dup-empty">{{ t('dup.none') }}</p>
        <div v-else class="dup-list">
          <div v-for="(g, gi) in dupGroups" :key="g.ids[0]" class="dup-group">
            <div class="dup-thumbs">
              <div v-for="(it, ii) in groupItems(g)" :key="it.id" class="dup-thumb">
                <img :src="assetUrl(it.filePath)" draggable="false" />
                <span class="dup-tag" :class="{ keep: ii === 0 }">
                  {{ ii === 0 ? t('dup.keep') : t('dup.dupe') }}
                </span>
              </div>
            </div>
            <button class="chip dup-clean" @click="cleanGroup(gi)">
              {{
                g.fileSize != null
                  ? t('dup.removeDupes', { n: g.ids.length - 1 })
                    + ' · ' + fmtMb(g.fileSize * (g.ids.length - 1))
                  : t('sim.removeSimilar', { n: g.ids.length - 1 })
              }}
            </button>
          </div>
        </div>
      </div>
    </div>
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

.retag-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.retag-btn {
  color: var(--text-1);
  background: var(--fill-subtle);
}

.retag-btn:hover:not(:disabled) {
  background: var(--fill-hover);
  border-color: var(--stroke-strong);
}

.retag-btn:disabled {
  opacity: 0.55;
  cursor: default;
}

/* 重复检测结果 */
.dup-backdrop {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: grid;
  place-items: center;
  padding: 40px;
  background: var(--veil);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
}

.dup-modal {
  width: min(720px, 100%);
  max-height: calc(100vh - 120px);
  display: flex;
  flex-direction: column;
  padding: 22px 24px;
  border-radius: var(--radius-overlay);
  border: 1px solid var(--stroke-strong);
  box-shadow: 0 40px 120px rgba(0, 0, 0, 0.4);
}

.dup-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.dup-head h3 {
  font-size: 17px;
  font-weight: 640;
}

.dup-close {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  color: var(--text-3);
  transition: all var(--dur-1) var(--ease-out);
}

.dup-close:hover {
  color: var(--text-1);
  background: var(--fill-hover);
}

.dup-summary {
  margin-top: 6px;
  font-size: 12.5px;
  color: var(--text-3);
}

.dup-empty {
  margin-top: 20px;
  font-size: 13px;
  color: var(--text-2);
  text-align: center;
}

.dup-list {
  margin-top: 14px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  overflow-y: auto;
  padding-right: 4px;
}

.dup-group {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding: 10px 12px;
  border: 1px solid var(--stroke);
  border-radius: 12px;
}

.dup-thumbs {
  display: flex;
  gap: 8px;
  min-width: 0;
  overflow: hidden;
}

.dup-thumb {
  position: relative;
  width: 74px;
  height: 46px;
  border-radius: 8px;
  overflow: hidden;
  flex-shrink: 0;
  border: 1px solid var(--stroke);
}

.dup-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.dup-tag {
  position: absolute;
  left: 0;
  bottom: 0;
  font-size: 9.5px;
  color: #fff;
  background: rgba(15, 15, 18, 0.65);
  padding: 1px 6px;
  border-radius: 0 6px 0 0;
}

.dup-tag.keep {
  background: rgba(46, 160, 67, 0.85);
}

.dup-clean {
  flex-shrink: 0;
  color: #fff;
  background: var(--accent-danger);
  border-color: var(--accent-danger);
}

.dup-clean:hover {
  filter: brightness(1.08);
}

.sim-threshold {
  background: transparent;
  color: var(--text-2);
  cursor: pointer;
  outline: none;
}


</style>
