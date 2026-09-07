import { defineStore } from 'pinia';
import type { Settings, ThemeMode } from '../types';
import * as api from '../lib/api';
import { useUiStore } from './ui';
import { setLocale, useI18n, type Locale } from '../lib/i18n';
import { getCurrentWindow, Effect, EffectState } from '@tauri-apps/api/window';

function systemPrefersDark(): boolean {
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
}

export const useSettingsStore = defineStore('settings', {
  state: (): Settings => ({
    apiBaseUrl: 'https://api.openai.com/v1',
    apiKey: '',
    apiModel: 'gpt-image-1',
    fillMode: 'fill',
    defaultSize: '1536x1024',
    locale: 'zh-CN',
    // 与 mini-vedio 保持清晰、低干扰的浅色默认；用户仍可切换深色。
    theme: 'light',
    sidebarHidden: false,
    classifyModel: '',
    sidebarExpanded: false,
  }),
  getters: {
    resolvedDark(state): boolean {
      if (state.theme === 'system') return systemPrefersDark();
      return state.theme === 'dark';
    },
  },
  actions: {
    async load() {
      try {
        Object.assign(this, await api.getSettings());
      } catch {
        /* 使用默认值 */
      }
      const saved = localStorage.getItem('wallspace.sidebarExpanded');
      if (saved != null) this.sidebarExpanded = saved === '1';
      setLocale(this.locale);
      this.applyTheme();
      this.applySidebar();
      window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
        if (this.theme === 'system') this.applyTheme();
      });
    },
    /** 同步 CSS 主题与窗口 Acrylic 底色 */
    applyTheme() {
      const dark = this.resolvedDark;
      document.documentElement.dataset.theme = dark ? 'dark' : 'light';
      getCurrentWindow()
        .setEffects({
          effects: [Effect.Acrylic],
          color: dark ? [11, 11, 13, 205] : [242, 242, 245, 215],
          state: EffectState.Active,
        })
        .catch(() => {});
    },
    /** 同步侧边栏展开模式到根元素（--sidebar-w 由 CSS 变量切换） */
    applySidebar() {
      document.documentElement.dataset.sidebar = this.sidebarExpanded ? 'wide' : 'narrow';
    },
    toggleSidebarExpanded() {
      this.sidebarExpanded = !this.sidebarExpanded;
      localStorage.setItem('wallspace.sidebarExpanded', this.sidebarExpanded ? '1' : '0');
      this.applySidebar();
    },
    setTheme(mode: ThemeMode) {
      this.theme = mode;
      this.applyTheme();
    },
    setLocale(locale: Locale) {
      this.locale = locale;
      setLocale(locale);
    },
    async save() {
      const ui = useUiStore();
      const { t } = useI18n();
      try {
        await api.saveSettings({ ...this.$state });
        ui.toast('success', t('toast.saved'));
      } catch (e) {
        ui.toast('error', String(e));
      }
    },
  },
});
