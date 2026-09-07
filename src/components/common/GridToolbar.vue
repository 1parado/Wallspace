<script setup lang="ts">
import Icon from './Icon.vue';
import { useI18n } from '../../lib/i18n';

const props = defineProps<{
  /** 条目数量 */
  count: number;
  /** 排序选项（含 labelKey） */
  sorts: ReadonlyArray<{ id: string; labelKey: string }>;
  /** 当前排序模式 */
  modelValue: string;
  /** 随机洗牌种子（重复点随机时由父级更新） */
  seed: number;
  /** 正在应用的条目 id（防抖 + 文案切换） */
  applying?: string | null;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', v: string): void;
  (e: 'reshuffle'): void;
  (e: 'apply'): void;
}>();

const { t } = useI18n();

function pick(id: string) {
  // 重复点「随机」= 重新洗牌
  if (id === 'random' && props.modelValue === 'random') emit('reshuffle');
  else emit('update:modelValue', id);
}
</script>

<template>
  <div class="grid-bar">
    <span class="result-count">{{ t('facets.resultCount', { n: count }) }}</span>
    <span class="bar-spacer" />
    <span class="facet-label">{{ t('facets.sort') }}</span>
    <button
      v-for="s in sorts"
      :key="s.id"
      class="chip"
      :class="{ active: modelValue === s.id }"
      @click="pick(s.id)"
    >
      {{ t(s.labelKey) }}
    </button>
    <span class="facet-sep" />
    <button
      class="chip shuffle-apply"
      :disabled="!!applying || !count"
      :title="t('facets.randomApplyTip')"
      @click="emit('apply')"
    >
      <Icon name="shuffle" :size="13" />
      {{ applying ? t('preview.applying') : t('facets.randomApply') }}
    </button>
  </div>
</template>

<style scoped>
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
