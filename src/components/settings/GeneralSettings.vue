<script setup lang="ts">
import { useSettingsStore } from '../../stores/settings'
import { ref } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { SUPPORTED_LOCALES, setLocale, type SupportedLocale } from '../../i18n'

const { t } = useI18n()
const settingsStore = useSettingsStore()
// Backup / Restore
const backupBusy = ref(false)
const backupMsg = ref<{ type: 'success' | 'error'; text: string } | null>(null)

async function handleExport() {
  const path = await save({
    title: t('general.export_dialog_title'),
    defaultPath: 'ytdown-backup.json',
    filters: [{ name: 'JSON', extensions: ['json'] }],
  })
  if (!path) return
  backupBusy.value = true
  backupMsg.value = null
  try {
    await invoke('export_settings_to_file', { path })
    backupMsg.value = { type: 'success', text: t('general.export_success') }
  } catch (e) {
    backupMsg.value = { type: 'error', text: String(e) }
  } finally {
    backupBusy.value = false
  }
}

async function handleImport() {
  const path = await open({
    title: t('general.import_dialog_title'),
    multiple: false,
    filters: [{ name: 'JSON', extensions: ['json'] }],
  })
  if (!path || typeof path !== 'string') return
  backupBusy.value = true
  backupMsg.value = null
  try {
    const result = await invoke<{ settings_count: number; presets_count: number; rules_count: number; schedules_count: number }>(
      'import_settings_from_file', { path }
    )
    await settingsStore.loadSettings()
    backupMsg.value = {
      type: 'success',
      text: t('general.import_success', {
        settings: result.settings_count,
        presets: result.presets_count,
        rules: result.rules_count,
        schedules: result.schedules_count,
      }),
    }
  } catch (e) {
    backupMsg.value = { type: 'error', text: String(e) }
  } finally {
    backupBusy.value = false
  }
}

const filenamePresets = [
  { labelKey: 'general.template_title', value: '%(title)s.%(ext)s' },
  { labelKey: 'general.template_title_channel', value: '%(title)s - %(channel)s.%(ext)s' },
  { labelKey: 'general.template_channel_title', value: '%(channel)s/%(title)s.%(ext)s' },
  { labelKey: 'general.template_date_title', value: '%(upload_date)s-%(title)s.%(ext)s' },
]

async function handleBrowseBackground(mode: 'light' | 'dark') {
  const selected = await open({
    multiple: false,
    title: mode === 'light' ? t('general.background_light') : t('general.background_dark'),
    filters: [{ name: t('general.image_filter'), extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp'] }],
  })
  if (selected && typeof selected === 'string') {
    const key = mode === 'light' ? 'background_image_light' : 'background_image_dark'
    settingsStore.updateSetting(key, selected)
  }
}

async function handleBrowseDir() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: t('general.download_dir'),
  })
  if (selected && typeof selected === 'string') {
    settingsStore.updateSetting('download_dir', selected)
  }
}

function handleLanguageChange(locale: string) {
  settingsStore.updateSetting('language', locale)
  setLocale(locale as SupportedLocale)
}
</script>

<template>
  <div class="space-y-6">
    <h3 class="text-base font-semibold">{{ t('general.title') }}</h3>

    <!-- Download directory -->
    <div>
      <label class="block text-sm font-medium mb-1">{{ t('general.download_dir') }}</label>
      <div class="flex gap-2">
        <input :value="settingsStore.settings.download_dir"
               @input="settingsStore.updateSetting('download_dir', ($event.target as HTMLInputElement).value)"
               class="flex-1 h-8 px-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm outline-none focus:ring-1 focus:ring-[var(--color-accent)]" />
        <button class="px-3 h-8 rounded-md text-sm bg-neutral-200 dark:bg-neutral-700" @click="handleBrowseDir">
          {{ t('common.browse') }}
        </button>
      </div>
    </div>

    <!-- Filename template -->
    <div>
      <label class="block text-sm font-medium mb-1">{{ t('general.filename_template') }}</label>
      <select :value="settingsStore.settings.filename_template"
              @change="settingsStore.updateSetting('filename_template', ($event.target as HTMLSelectElement).value)"
              class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm">
        <option v-for="preset in filenamePresets" :key="preset.value" :value="preset.value">
          {{ t(preset.labelKey) }} ({{ preset.value }})
        </option>
      </select>
      <input :value="settingsStore.settings.filename_template"
             @input="settingsStore.updateSetting('filename_template', ($event.target as HTMLInputElement).value)"
             class="mt-1 w-full h-8 px-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm font-mono outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
             :placeholder="t('general.template_placeholder')" />
    </div>

    <!-- Concurrent downloads -->
    <div>
      <label class="block text-sm font-medium mb-1">{{ t('general.concurrent_downloads') }}</label>
      <input type="number" min="1" max="10"
             :value="settingsStore.settings.concurrent_downloads"
             @input="settingsStore.updateSetting('concurrent_downloads', parseInt(($event.target as HTMLInputElement).value) || 3)"
             class="w-20 h-8 px-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm outline-none focus:ring-1 focus:ring-[var(--color-accent)]" />
    </div>

    <!-- Theme -->
    <div>
      <label class="block text-sm font-medium mb-1">{{ t('general.theme') }}</label>
      <div class="flex gap-2">
        <button v-for="theme in (['system', 'light', 'dark'] as const)" :key="theme"
                class="px-3 py-1.5 rounded-md text-sm"
                :class="settingsStore.settings.theme === theme ? 'bg-[var(--color-accent)] text-white' : 'bg-neutral-100 dark:bg-neutral-700'"
                @click="settingsStore.updateSetting('theme', theme)">
          {{ theme === 'system' ? t('general.theme_system') : theme === 'light' ? t('general.theme_light') : t('general.theme_dark') }}
        </button>
      </div>
    </div>

    <!-- Language -->
    <div>
      <label class="block text-sm font-medium mb-1">{{ t('general.language') }}</label>
      <select :value="settingsStore.settings.language"
              @change="handleLanguageChange(($event.target as HTMLSelectElement).value)"
              class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm">
        <option v-for="locale in SUPPORTED_LOCALES" :key="locale.value" :value="locale.value">
          {{ locale.label }}
        </option>
      </select>
    </div>

    <!-- Background image -->
    <div>
      <label class="block text-sm font-medium mb-2">{{ t('general.background_image') }}</label>
      <div class="space-y-3">
        <!-- Light mode -->
        <div class="p-3 rounded-lg bg-neutral-50 dark:bg-neutral-800/50 space-y-1">
          <label class="block text-xs font-medium text-neutral-500">{{ t('general.background_light') }}</label>
          <div class="flex gap-2">
            <input :value="settingsStore.settings.background_image_light"
                   @input="settingsStore.updateSetting('background_image_light', ($event.target as HTMLInputElement).value)"
                   class="flex-1 h-8 px-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
                   :placeholder="t('general.background_placeholder')" />
            <button class="px-3 h-8 rounded-md text-sm bg-neutral-200 dark:bg-neutral-700" @click="handleBrowseBackground('light')">
              {{ t('common.select') }}
            </button>
            <button v-if="settingsStore.settings.background_image_light"
                    class="px-3 h-8 rounded-md text-sm bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400"
                    @click="settingsStore.updateSetting('background_image_light', '')">
              {{ t('common.release') }}
            </button>
          </div>
        </div>
        <!-- Dark mode -->
        <div class="p-3 rounded-lg bg-neutral-50 dark:bg-neutral-800/50 space-y-1">
          <label class="block text-xs font-medium text-neutral-500">{{ t('general.background_dark') }}</label>
          <div class="flex gap-2">
            <input :value="settingsStore.settings.background_image_dark"
                   @input="settingsStore.updateSetting('background_image_dark', ($event.target as HTMLInputElement).value)"
                   class="flex-1 h-8 px-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
                   :placeholder="t('general.background_placeholder')" />
            <button class="px-3 h-8 rounded-md text-sm bg-neutral-200 dark:bg-neutral-700" @click="handleBrowseBackground('dark')">
              {{ t('common.select') }}
            </button>
            <button v-if="settingsStore.settings.background_image_dark"
                    class="px-3 h-8 rounded-md text-sm bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400"
                    @click="settingsStore.updateSetting('background_image_dark', '')">
              {{ t('common.release') }}
            </button>
          </div>
        </div>
        <!-- Opacity slider -->
        <div v-if="settingsStore.settings.background_image_light || settingsStore.settings.background_image_dark">
          <label class="block text-xs text-neutral-500 mb-1">
            {{ t('general.background_opacity', { value: settingsStore.settings.background_opacity }) }}
          </label>
          <input type="range" min="5" max="100" step="5"
                 :value="settingsStore.settings.background_opacity"
                 @input="settingsStore.updateSetting('background_opacity', parseInt(($event.target as HTMLInputElement).value))"
                 class="w-full accent-[var(--color-accent)]" />
          <div class="flex justify-between text-xs text-neutral-400">
            <span>{{ t('general.background_thin') }}</span>
            <span>{{ t('general.background_thick') }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Backup / Restore -->
    <div class="p-3 rounded-lg border border-[var(--color-separator)] space-y-3">
      <h4 class="text-sm font-medium">{{ t('general.backup_title') }}</h4>
      <p class="text-xs text-neutral-500">{{ t('general.backup_description') }}</p>
      <div class="flex gap-2">
        <button @click="handleExport" :disabled="backupBusy"
                class="px-3 py-1.5 text-xs rounded-md bg-neutral-100 dark:bg-neutral-700 hover:bg-neutral-200 dark:hover:bg-neutral-600 disabled:opacity-50 transition-colors">
          {{ t('general.export') }}
        </button>
        <button @click="handleImport" :disabled="backupBusy"
                class="px-3 py-1.5 text-xs rounded-md bg-neutral-100 dark:bg-neutral-700 hover:bg-neutral-200 dark:hover:bg-neutral-600 disabled:opacity-50 transition-colors">
          {{ t('general.import') }}
        </button>
      </div>
      <p v-if="backupMsg"
         :class="backupMsg.type === 'success' ? 'text-green-600 dark:text-green-400' : 'text-red-500'"
         class="text-xs">{{ backupMsg.text }}</p>
    </div>
  </div>
</template>
