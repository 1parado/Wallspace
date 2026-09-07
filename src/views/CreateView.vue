<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useLibraryStore } from '../stores/library';
import { useSettingsStore } from '../stores/settings';
import { useUiStore } from '../stores/ui';
import { useI18n } from '../lib/i18n';
import { grokImagineStatus } from '../lib/api';
import { guessCategory, suggestTags } from '../lib/autoTag';
import WallpaperGrid from '../components/wallpaper/WallpaperGrid.vue';
import Icon from '../components/common/Icon.vue';

const lib = useLibraryStore();
const settings = useSettingsStore();
const ui = useUiStore();
const { t } = useI18n();

const prompt = ref('');
const LAST_CAT_KEY = 'wallspace.lastCategory';
const remembered = localStorage.getItem(LAST_CAT_KEY);
const category = ref<string | null>(remembered ?? 'Cinematic');
/** 用户手动改过分类后，不再跟随 prompt 自动预选 */
const categoryTouched = ref(remembered != null);

// 智能预选：prompt 变化且用户未手动指定时，按关键词规则推荐分类
watch(prompt, (p) => {
  if (categoryTouched.value) return;
  const guess = guessCategory(p);
  if (guess) category.value = guess;
});

function pickCategory(c: string | null) {
  category.value = c;
  categoryTouched.value = true;
  if (c) localStorage.setItem(LAST_CAT_KEY, c);
}

const autoSuggested = computed(
  () => !categoryTouched.value && !!prompt.value.trim() && !!guessCategory(prompt.value)
);

/** 生图引擎：OpenAI 兼容接口 / Grok 账号直连（与 grok_switch ImagineEngine 一致） */
const engine = ref<'openai' | 'grok'>('openai');

const PRESETS = [
  { id: 'landscape', labelKey: 'create.landscape', size: '1536x1024' },
  { id: 'square', labelKey: 'create.square', size: '1024x1024' },
  { id: 'portrait', labelKey: 'create.portrait', size: '1024x1536' },
  { id: 'custom', labelKey: 'create.custom', size: '' },
] as const;

const presetId = ref('landscape');
const customSize = ref('');

const GROK_MODELS = [
  { id: 'grok-imagine-image', labelKey: 'create.grokStandard' },
  { id: 'grok-imagine-image-quality', labelKey: 'create.grokQuality' },
] as const;
const grokModel = ref<string>('grok-imagine-image');

const GROK_RATIOS = [
  { id: '16:9', label: '16:9' },
  { id: '1:1', label: '1:1' },
  { id: '9:16', label: '9:16' },
  { id: '4:3', label: '4:3' },
] as const;
const grokRatio = ref<string>('16:9');

const size = computed(() =>
  presetId.value === 'custom'
    ? customSize.value.trim() || settings.defaultSize
    : (PRESETS.find((p) => p.id === presetId.value)?.size ?? '1536x1024')
);

const grokStatus = ref<{ total: number; available: number } | null>(null);
onMounted(async () => {
  try {
    grokStatus.value = await grokImagineStatus();
  } catch {
    grokStatus.value = { total: 0, available: 0 };
  }
});

const canGenerate = computed(() => {
  if (lib.generating || prompt.value.trim().length === 0) return false;
  return engine.value === 'grok' ? !!grokRatio.value : !!size.value;
});

async function generate() {
  if (!canGenerate.value) return;
  const p = prompt.value.trim();
  const tags = suggestTags(p);
  const item =
    engine.value === 'grok'
      ? await lib.generateGrok(p, grokModel.value, grokRatio.value, category.value, tags)
      : await lib.generate(p, size.value, category.value, tags);
  if (item) ui.previewId = item.id;
}

const IDEA_KEYS = ['create.idea1', 'create.idea2', 'create.idea3', 'create.idea4'] as const;

function useIdea(key: string) {
  prompt.value = t(key);
}

const recent = computed(() => lib.aiItems.slice(0, 8));
</script>

<template>
  <div class="create">
    <div class="composer glass">
      <div class="engine-row">
        <p class="label">{{ t('create.engine') }}</p>
        <div class="segmented engine">
          <button :class="{ active: engine === 'openai' }" @click="engine = 'openai'">
            {{ t('create.engineOpenai') }}
          </button>
          <button :class="{ active: engine === 'grok' }" @click="engine = 'grok'">
            {{ t('create.engineGrok') }}
          </button>
        </div>
        <span v-if="engine === 'grok'" class="accounts" :class="{ none: !grokStatus?.available }">
          {{
            grokStatus && grokStatus.available > 0
              ? t('create.accounts', { n: grokStatus.available })
              : t('create.accountsNone')
          }}
        </span>
      </div>

      <p class="label">{{ t('create.prompt') }}</p>
      <textarea
        v-model="prompt"
        class="prompt-input"
        rows="3"
        :placeholder="t('create.placeholder')"
        spellcheck="false"
      />

      <div class="row">
        <div v-if="engine === 'openai'" class="col">
          <p class="label">{{ t('create.aspect') }}</p>
          <div class="segmented">
            <button
              v-for="p in PRESETS"
              :key="p.id"
              :class="{ active: presetId === p.id }"
              @click="presetId = p.id"
            >
              {{ t(p.labelKey) }}
            </button>
          </div>
          <input
            v-if="presetId === 'custom'"
            v-model="customSize"
            class="text-field size-input"
            :placeholder="t('create.customPlaceholder')"
            spellcheck="false"
          />
        </div>

        <div v-else class="col">
          <p class="label">{{ t('create.grokModel') }}</p>
          <div class="segmented">
            <button
              v-for="m in GROK_MODELS"
              :key="m.id"
              :class="{ active: grokModel === m.id }"
              @click="grokModel = m.id"
            >
              {{ t(m.labelKey) }}
            </button>
          </div>
          <p class="label ratio-label">{{ t('create.aspect') }}</p>
          <div class="segmented">
            <button
              v-for="r in GROK_RATIOS"
              :key="r.id"
              :class="{ active: grokRatio === r.id }"
              @click="grokRatio = r.id"
            >
              {{ r.label }}
            </button>
          </div>
        </div>

        <div class="col">
          <p class="label">
            {{ t('create.category') }}
            <span v-if="autoSuggested" class="auto-badge">{{ t('create.autoSuggested') }}</span>
          </p>
          <div class="segmented wrap">
            <button :class="{ active: category === null }" @click="pickCategory(null)">
              {{ t('create.auto') }}
            </button>
            <button
              v-for="c in ui.categories"
              :key="c"
              :class="{ active: category === c }"
              @click="pickCategory(c)"
            >
              {{ t(`cat.${c.toLowerCase()}`) }}
            </button>
          </div>
        </div>
      </div>

      <div class="composer-foot">
        <div class="ideas">
          <button v-for="k in IDEA_KEYS" :key="k" @click="useIdea(k)">{{ t(k).slice(0, 34) }}…</button>
        </div>
        <button class="btn-primary generate" :disabled="!canGenerate" @click="generate">
          <Icon name="sparkles" :size="15" />
          {{ lib.generating ? t('create.generating') : t('create.generate') }}
        </button>
      </div>
    </div>

    <div v-if="lib.generating" class="grid-skeleton">
      <div class="gen-card"><div class="skeleton-wave" /></div>
    </div>

    <section v-if="recent.length" class="section">
      <div class="section-head">
        <h2>{{ t('create.yours') }}</h2>
      </div>
      <WallpaperGrid :items="recent" />
    </section>
  </div>
</template>

<style scoped>
.create {
  display: flex;
  flex-direction: column;
  gap: 30px;
}

.composer {
  border-radius: var(--radius-overlay);
  padding: 22px 24px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.auto-badge {
  display: inline-block;
  margin-left: 6px;
  font-size: 10px;
  font-weight: 560;
  letter-spacing: 0.04em;
  color: var(--text-2);
  border: 1px solid var(--stroke-strong);
  border-radius: 100px;
  padding: 1px 7px;
  vertical-align: 1px;
}

.label {
  font-size: 11px;
  font-weight: 560;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-3);
}

.engine-row {
  display: flex;
  align-items: center;
  gap: 14px;
}

.engine-row .label {
  flex-shrink: 0;
}

.engine {
  flex: 0 1 auto;
}

.accounts {
  font-size: 12px;
  color: var(--text-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.accounts.none {
  color: var(--accent-heart);
}

.prompt-input {
  width: 100%;
  resize: none;
  background: var(--fill-subtle);
  border: 1px solid var(--stroke);
  border-radius: 12px;
  padding: 12px 14px;
  font-size: 15px;
  line-height: 1.55;
  color: var(--text-1);
  outline: none;
  transition:
    border-color var(--dur-1) var(--ease-out),
    background var(--dur-1) var(--ease-out);
}

.prompt-input::placeholder {
  color: var(--text-3);
}

.prompt-input:focus {
  border-color: var(--stroke-strong);
  background: var(--fill-hover);
}

.row {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 28px;
}

.col {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.ratio-label {
  margin-top: 6px;
}

.segmented {
  display: flex;
  gap: 4px;
  padding: 4px;
  border-radius: 11px;
  background: var(--fill-subtle);
  border: 1px solid var(--stroke);
}

.segmented.wrap {
  flex-wrap: wrap;
}

.segmented button {
  padding: 7px 14px;
  border-radius: 8px;
  font-size: 13px;
  color: var(--text-3);
  white-space: nowrap;
  transition: all var(--dur-1) var(--ease-out);
}

.segmented button.active {
  color: var(--text-1);
  background: var(--fill-active);
}

.size-input {
  max-width: 260px;
}

.composer-foot {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 20px;
}

.ideas {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.ideas button {
  font-size: 11.5px;
  color: var(--text-3);
  border: 1px solid var(--stroke);
  border-radius: 100px;
  padding: 4px 12px;
  transition: all var(--dur-1) var(--ease-out);
}

.ideas button:hover {
  color: var(--text-1);
  border-color: var(--stroke-strong);
}

.generate {
  flex-shrink: 0;
  padding: 10px 26px;
}

.grid-skeleton {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 18px;
}

.gen-card {
  aspect-ratio: 16/9;
  border-radius: var(--radius-card);
  overflow: hidden;
  background: var(--bg-3);
}

.skeleton-wave {
  width: 100%;
  height: 100%;
  background: linear-gradient(100deg, var(--skeleton-a) 40%, var(--skeleton-b) 50%, var(--skeleton-a) 60%);
  background-size: 200% 100%;
  animation: wave 1.6s infinite;
}

@keyframes wave {
  from {
    background-position: 120% 0;
  }
  to {
    background-position: -80% 0;
  }
}

.section {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.section-head h2 {
  font-size: 22px;
  font-weight: 620;
  letter-spacing: -0.02em;
}
</style>
