import { computed, reactive } from 'vue';
import zhCN from '../locales/zh-CN';
import en from '../locales/en';

export type Locale = 'zh-CN' | 'en';

const state = reactive({ locale: 'zh-CN' as Locale });

const DICTS: Record<Locale, Record<string, string>> = {
  'zh-CN': zhCN,
  en,
};

export function detectLocale(): Locale {
  return navigator.language.toLowerCase().startsWith('zh') ? 'zh-CN' : 'en';
}

export function setLocale(locale: Locale) {
  state.locale = locale;
  document.documentElement.lang = locale;
}

export function useI18n() {
  const t = (key: string, params?: Record<string, string | number>) => {
    const dict = DICTS[state.locale] ?? DICTS['zh-CN'];
    let text = dict[key] ?? DICTS['zh-CN'][key] ?? key.split('.').pop()!;
    if (params) {
      for (const [k, v] of Object.entries(params)) {
        text = text.split(`{${k}}`).join(String(v));
      }
    }
    return text;
  };
  return { t, locale: computed(() => state.locale) };
}
