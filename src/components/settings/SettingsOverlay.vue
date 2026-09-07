<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useSettingsStore } from '../../stores/settings';
import { useCollectionsStore } from '../../stores/collections';
import { useUiStore } from '../../stores/ui';
import { testConnection, exportBackup, importBackup } from '../../lib/api';
import { isEnabled as autostartEnabled, enable as autostartEnable, disable as autostartDisable } from '@tauri-apps/plugin-autostart';
import { save as saveDialog, open as openDialog } from '@tauri-apps/plugin-dialog';
import { useI18n } from '../../lib/i18n';
import type { ThemeMode } from '../../types';
import Icon from '../common/Icon.vue';

const settings = useSettingsStore();
const collections = useCollectionsStore();
const ui = useUiStore();
const { t } = useI18n();

const testing = ref(false);
const testResult = ref<{ ok: boolean; message: string } | null>(null);
const launchAtLogin = ref(false);

onMounted(async () => {
  settings.load();
  collections.load();
  try {
    launchAtLogin.value = await autostartEnabled();
  } catch {
    /* 插件不可用时忽略 */
  }
});

async function toggleLaunchAtLogin() {
  try {
    if (launchAtLogin.value) {
      await autostartDisable();
      launchAtLogin.value = false;
    } else {
      await autostartEnable();
      launchAtLogin.value = true;
    }
  } catch (e) {
    ui.toast('error', String(e));
  }
}

const SWITCH_INTERVALS = [1, 5, 10, 15, 30, 60, 120];

// —— 备份与恢复 ——
const backingUp = ref(false);

async function runExportBackup() {
  if (backingUp.value) return;
  const target = await saveDialog({
    title: t('settings.backupSaveTitle'),
    defaultPath: `wallspace-backup-${new Date().toISOString().slice(0, 10)}.zip`,
    filters: [{ name: 'Wallspace Backup', extensions: ['zip'] }],
  });
  if (!target) return;
  backingUp.value = true;
  try {
    const n = await exportBackup(target);
    ui.toast('success', t('settings.backupDone', { n }));
  } catch (e) {
    ui.toast('error', String(e));
  } finally {
    backingUp.value = false;
  }
}

async function runImportBackup() {
  if (backingUp.value) return;
  const picked = await openDialog({
    title: t('settings.backupOpenTitle'),
    multiple: false,
    filters: [{ name: 'Wallspace Backup', extensions: ['zip'] }],
  });
  if (!picked || Array.isArray(picked)) return;
  backingUp.value = true;
  try {
    const r = await importBackup(picked);
    if (r.imported) await settings.load();
    await collections.load();
    ui.toast(
      r.imported ? 'success' : 'info',
      t('settings.restoreDone', {
        n: r.imported,
        s: r.skipped,
        c: r.collectionsAdded,
      })
    );
  } catch (e) {
    ui.toast('error', String(e));
  } finally {
    backingUp.value = false;
  }
}

function pickWatchFolder() {
  openDialog({ directory: true, title: t('settings.watchFolderPick') }).then((picked) => {
    if (typeof picked === 'string' && picked) {
      settings.watchFolder = picked;
    }
  });
}

async function runTest() {
  testing.value = true;
  testResult.value = null;
  try {
    const models = await testConnection(settings.apiBaseUrl, settings.apiKey);
    const hint = models.slice(0, 6).join(' · ');
    testResult.value = {
      ok: true,
      message: `${t('settings.ok', { n: models.length })}${hint ? `：${hint}` : ''}`,
    };
  } catch (e) {
    testResult.value = { ok: false, message: `${t('settings.fail')}：${String(e)}` };
  } finally {
    testing.value = false;
  }
}

const THEMES: { id: ThemeMode; labelKey: string }[] = [
  { id: 'system', labelKey: 'settings.themeSystem' },
  { id: 'dark', labelKey: 'settings.themeDark' },
  { id: 'light', labelKey: 'settings.themeLight' },
];

function saveAndClose() {
  settings.save();
  ui.settingsOpen = false;
}
</script>

<template>
  <div class="backdrop" @click.self="ui.settingsOpen = false">
    <div class="panel glass overlay-spring">
      <div class="head">
        <h2>{{ t('settings.title') }}</h2>
        <button class="close" @click="ui.settingsOpen = false">
          <Icon name="x" :size="16" />
        </button>
      </div>

      <div class="body">
        <section>
          <p class="group-label">{{ t('settings.generation') }}</p>
          <label class="field">
            <span>{{ t('settings.baseUrl') }}</span>
            <input v-model="settings.apiBaseUrl" class="text-field" placeholder="https://api.openai.com/v1" spellcheck="false" />
          </label>
          <label class="field">
            <span>{{ t('settings.apiKey') }}</span>
            <input v-model="settings.apiKey" class="text-field" type="password" placeholder="sk-…" spellcheck="false" />
          </label>
          <label class="field">
            <span>{{ t('settings.model') }}</span>
            <input v-model="settings.apiModel" class="text-field" placeholder="gpt-image-1" spellcheck="false" />
          </label>
          <label class="field">
            <span>{{ t('settings.classifyModel') }}</span>
            <input v-model="settings.classifyModel" class="text-field" placeholder="gpt-4o-mini" spellcheck="false" />
          </label>
          <p class="privacy">{{ t('settings.classifyHint') }}</p>
          <label class="field">
            <span>{{ t('settings.defaultSize') }}</span>
            <input v-model="settings.defaultSize" class="text-field" placeholder="1536x1024" spellcheck="false" />
          </label>
          <div class="row">
            <button class="btn-ghost" :disabled="testing" @click="runTest">
              {{ testing ? t('settings.testing') : t('settings.test') }}
            </button>
            <p v-if="testResult" class="test-result" :class="testResult.ok ? 'ok' : 'bad'">
              {{ testResult.message }}
            </p>
          </div>
        </section>

        <section>
          <p class="group-label">{{ t('settings.appearance') }}</p>
          <div class="appearance-grid">
            <div class="appearance-item">
              <span class="appearance-label">{{ t('settings.theme') }}</span>
              <div class="segmented">
                <button
                  v-for="th in THEMES"
                  :key="th.id"
                  :class="{ active: settings.theme === th.id }"
                  @click="settings.setTheme(th.id)"
                >
                  {{ t(th.labelKey) }}
                </button>
              </div>
            </div>
            <div class="appearance-item">
              <span class="appearance-label">{{ t('settings.language') }}</span>
              <div class="segmented">
                <button
                  :class="{ active: settings.locale === 'zh-CN' }"
                  @click="settings.setLocale('zh-CN')"
                >
                  中文
                </button>
                <button
                  :class="{ active: settings.locale === 'en' }"
                  @click="settings.setLocale('en')"
                >
                  English
                </button>
              </div>
            </div>
          </div>
        </section>

        <section>
          <p class="group-label">{{ t('settings.applyMode') }}</p>
          <div class="segmented">
            <button :class="{ active: settings.fillMode === 'fill' }" @click="settings.fillMode = 'fill'">
              {{ t('settings.fill') }}
            </button>
            <button :class="{ active: settings.fillMode === 'fit' }" @click="settings.fillMode = 'fit'">
              {{ t('settings.fit') }}
            </button>
          </div>
        </section>

        <section>
          <p class="group-label">{{ t('settings.autoSwitch') }}</p>
          <div class="appearance-item">
            <span class="appearance-label">{{ t('settings.autoSwitchMode') }}</span>
            <div class="segmented">
              <button
                :class="{ active: settings.autoSwitchMode === 'interval' }"
                @click="settings.autoSwitchMode = 'interval'"
              >
                {{ t('settings.modeInterval') }}
              </button>
              <button
                :class="{ active: settings.autoSwitchMode === 'daynight' }"
                @click="settings.autoSwitchMode = 'daynight'"
              >
                {{ t('settings.modeDayNight') }}
              </button>
            </div>
          </div>

          <template v-if="settings.autoSwitchMode === 'interval'">
            <label class="field">
              <span>{{ t('settings.autoSwitchSource') }}</span>
              <select v-model="settings.autoSwitchCollectionId" class="text-field">
                <option :value="null">{{ t('settings.autoSwitchOff') }}</option>
                <option v-for="c in collections.collections" :key="c.id" :value="c.id">
                  {{ c.name }}（{{ c.itemIds.length }}）
                </option>
              </select>
            </label>
            <label class="field">
              <span>{{ t('settings.autoSwitchInterval') }}</span>
              <select v-model.number="settings.autoSwitchIntervalMin" class="text-field">
                <option v-for="m in SWITCH_INTERVALS" :key="m" :value="m">
                  {{ t('settings.autoSwitchMin', { n: m }) }}
                </option>
              </select>
            </label>
          </template>

          <template v-else>
            <label class="field">
              <span>{{ t('settings.dayCollection') }}</span>
              <select v-model="settings.dayCollectionId" class="text-field">
                <option :value="null">{{ t('settings.autoSwitchOff') }}</option>
                <option v-for="c in collections.collections" :key="c.id" :value="c.id">
                  {{ c.name }}（{{ c.itemIds.length }}）
                </option>
              </select>
            </label>
            <label class="field">
              <span>{{ t('settings.nightCollection') }}</span>
              <select v-model="settings.nightCollectionId" class="text-field">
                <option :value="null">{{ t('settings.autoSwitchOff') }}</option>
                <option v-for="c in collections.collections" :key="c.id" :value="c.id">
                  {{ c.name }}（{{ c.itemIds.length }}）
                </option>
              </select>
            </label>
            <div class="appearance-item">
              <span class="appearance-label">{{ t('settings.dayStart') }}</span>
              <input v-model="settings.dayStart" type="time" class="text-field time-input" />
            </div>
            <div class="appearance-item">
              <span class="appearance-label">{{ t('settings.nightStart') }}</span>
              <input v-model="settings.nightStart" type="time" class="text-field time-input" />
            </div>
          </template>

          <div class="appearance-item">
            <span class="appearance-label">{{ t('settings.autoSwitchRandom') }}</span>
            <div class="segmented">
              <button
                :class="{ active: settings.autoSwitchRandom }"
                @click="settings.autoSwitchRandom = true"
              >
                {{ t('settings.launchOn') }}
              </button>
              <button
                :class="{ active: !settings.autoSwitchRandom }"
                @click="settings.autoSwitchRandom = false"
              >
                {{ t('settings.launchOff') }}
              </button>
            </div>
          </div>
          <div class="appearance-item">
            <span class="appearance-label">{{ t('settings.autoSwitchScope') }}</span>
            <div class="segmented">
              <button
                :class="{ active: settings.autoSwitchScope === 'primary' }"
                @click="settings.autoSwitchScope = 'primary'"
              >
                {{ t('settings.scopePrimary') }}
              </button>
              <button
                :class="{ active: settings.autoSwitchScope === 'all' }"
                @click="settings.autoSwitchScope = 'all'"
              >
                {{ t('settings.scopeAll') }}
              </button>
            </div>
          </div>
          <p class="privacy">
            {{ settings.autoSwitchRandom
              ? t('settings.autoSwitchRandomHint')
              : (settings.autoSwitchMode === 'daynight' ? t('settings.dayNightHint') : t('settings.autoSwitchHint')) }}
          </p>
          <div class="appearance-item">
            <span class="appearance-label">{{ t('settings.launchAtLogin') }}</span>
            <div class="segmented">
              <button
                :class="{ active: launchAtLogin }"
                @click="!launchAtLogin && toggleLaunchAtLogin()"
              >
                {{ t('settings.launchOn') }}
              </button>
              <button
                :class="{ active: !launchAtLogin }"
                @click="launchAtLogin && toggleLaunchAtLogin()"
              >
                {{ t('settings.launchOff') }}
              </button>
            </div>
          </div>
          <div class="appearance-item">
            <span class="appearance-label">{{ t('settings.globalShortcuts') }}</span>
            <div class="segmented">
              <button
                :class="{ active: settings.globalShortcuts }"
                @click="settings.globalShortcuts = true"
              >
                {{ t('settings.launchOn') }}
              </button>
              <button
                :class="{ active: !settings.globalShortcuts }"
                @click="settings.globalShortcuts = false"
              >
                {{ t('settings.launchOff') }}
              </button>
            </div>
          </div>
          <p v-if="settings.globalShortcuts" class="privacy">{{ t('settings.globalShortcutsHint') }}</p>
        </section>

        <section>
          <p class="group-label">{{ t('settings.watchSection') }}</p>
          <label class="field">
            <span>{{ t('settings.watchFolder') }}</span>
            <div class="watch-row">
              <input
                :value="settings.watchFolder ?? ''"
                class="text-field"
                readonly
                :placeholder="t('settings.watchFolderNone')"
              />
              <button class="backup-btn" :disabled="backingUp" @click="pickWatchFolder">
                {{ t('settings.watchFolderPick') }}
              </button>
              <button
                v-if="settings.watchFolder"
                class="backup-btn"
                @click="settings.watchFolder = ''"
              >
                {{ t('settings.watchFolderClear') }}
              </button>
            </div>
          </label>
          <p class="privacy">{{ t('settings.watchFolderHint') }}</p>
        </section>

        <section>
          <p class="group-label">{{ t('settings.backup') }}</p>
          <div class="backup-row">
            <button class="backup-btn" :disabled="backingUp" @click="runExportBackup">
              <Icon name="download" :size="13" />
              {{ backingUp ? t('settings.backupWorking') : t('settings.backupExport') }}
            </button>
            <button class="backup-btn" :disabled="backingUp" @click="runImportBackup">
              <Icon name="restore" :size="13" />
              {{ t('settings.backupRestore') }}
            </button>
          </div>
          <label class="field">
            <span>{{ t('settings.autoBackup') }}</span>
            <select v-model.number="settings.autoBackupDays" class="text-field">
              <option :value="0">{{ t('settings.autoBackupOff') }}</option>
              <option :value="1">{{ t('settings.autoBackupDaily') }}</option>
              <option :value="7">{{ t('settings.autoBackupWeekly') }}</option>
              <option :value="30">{{ t('settings.autoBackupMonthly') }}</option>
            </select>
          </label>
          <p class="privacy">{{ t('settings.backupHint') }}</p>
        </section>
      </div>

      <div class="foot">
        <p class="privacy">{{ t('settings.privacy') }}</p>
        <button class="btn-primary" @click="saveAndClose">{{ t('settings.save') }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.backup-row {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.backup-btn {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  font-size: 13px;
  color: var(--text-1);
  background: var(--fill-subtle);
  border: 1px solid var(--stroke);
  border-radius: var(--radius-btn);
  padding: 8px 16px;
  cursor: pointer;
  transition: all var(--dur-1) var(--ease-out);
}

.backup-btn:hover:not(:disabled) {
  background: var(--fill-hover);
  border-color: var(--stroke-strong);
}

.backup-btn:disabled {
  opacity: 0.55;
  cursor: default;
}

.watch-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.watch-row .text-field {
  flex: 1;
  min-width: 0;
}

.backdrop {
  position: fixed;
  inset: 0;
  z-index: 220;
  display: grid;
  place-items: center;
  padding: 40px;
  background: var(--veil);
  backdrop-filter: blur(30px);
  -webkit-backdrop-filter: blur(30px);
}

.panel {
  width: min(560px, 100%);
  border-radius: var(--radius-overlay);
  box-shadow: 0 32px 90px rgba(0, 0, 0, 0.35);
  overflow: hidden;
}

.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px 0;
}

.head h2 {
  font-size: 19px;
  font-weight: 640;
  letter-spacing: -0.015em;
}

.close {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border-radius: 50%;
  color: var(--text-3);
  transition: all var(--dur-1) var(--ease-out);
}

.close:hover {
  color: var(--text-1);
  background: var(--fill-hover);
}

.body {
  padding: 8px 24px 20px;
  display: flex;
  flex-direction: column;
  gap: 22px;
  max-height: calc(100vh - 300px);
  overflow-y: auto;
}

.group-label {
  font-size: 11px;
  font-weight: 560;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-3);
  margin-bottom: 12px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 12px;
}

.field span {
  font-size: 12.5px;
  color: var(--text-2);
}

.row {
  display: flex;
  align-items: center;
  gap: 14px;
  flex-wrap: wrap;
}

.test-result {
  font-size: 12px;
  color: var(--text-2);
  max-width: 340px;
}

.test-result.ok {
  color: var(--text-1);
}

.test-result.bad {
  color: var(--accent-heart);
}

.appearance-grid {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.appearance-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.appearance-label {
  font-size: 13px;
  color: var(--text-2);
  flex-shrink: 0;
}

.segmented {
  display: flex;
  gap: 4px;
  padding: 4px;
  border-radius: 12px;
  background: var(--fill-subtle);
  border: 1px solid var(--stroke);
}

.segmented button {
  flex: 1;
  padding: 7px 14px;
  border-radius: 9px;
  font-size: 13px;
  color: var(--text-3);
  white-space: nowrap;
  transition: all var(--dur-1) var(--ease-out);
}

.segmented button.active {
  color: var(--text-1);
  background: var(--fill-active);
}

.foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 24px 18px;
  border-top: 1px solid var(--stroke);
}

.privacy {
  font-size: 11.5px;
  color: var(--text-3);
}
</style>
