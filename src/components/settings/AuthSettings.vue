<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useSettingsStore } from '../../stores/settings'
import { open } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const settingsStore = useSettingsStore()
const safariAccessGranted = ref<boolean | null>(null)

const BROWSER_NAMES: Record<string, string> = {
  safari: 'Safari', chrome: 'Google Chrome', brave: 'Brave', edge: 'Microsoft Edge',
  firefox: 'Firefox', chromium: 'Chromium', opera: 'Opera', vivaldi: 'Vivaldi',
}
const browserValues = ['none', 'safari', 'chrome', 'brave', 'edge', 'firefox', 'chromium', 'opera', 'vivaldi']

// Check Safari FDA when Safari is selected
watch(() => settingsStore.settings.cookie_browser, async (browser) => {
  if (browser === 'safari') {
    try {
      safariAccessGranted.value = await invoke<boolean>('check_safari_access')
    } catch {
      safariAccessGranted.value = false
    }
  } else {
    safariAccessGranted.value = null
  }
}, { immediate: true })

async function handleBrowseCookieFile() {
  const selected = await open({
    multiple: false,
    title: t('auth.cookie_file'),
    filters: [
      { name: 'Cookie files', extensions: ['txt', 'cookies'] },
      { name: 'All files', extensions: ['*'] },
    ],
  })
  if (selected && typeof selected === 'string') {
    settingsStore.updateSetting('cookie_file', selected)
  }
}

function openFullDiskAccessSettings() {
  openUrl('x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles')
}
</script>

<template>
  <div class="space-y-6">
    <h3 class="text-base font-semibold">{{ t('auth.title') }}</h3>

    <!-- Cookie browser -->
    <div>
      <label class="block text-sm font-medium mb-1">{{ t('auth.cookie_browser') }}</label>
      <p class="text-xs text-neutral-500 mb-2">{{ t('auth.cookie_browser_description') }}</p>
      <select :value="settingsStore.settings.cookie_browser"
              @change="settingsStore.updateSetting('cookie_browser', ($event.target as HTMLSelectElement).value)"
              class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm">
        <option v-for="v in browserValues" :key="v" :value="v">
          {{ v === 'none' ? t('auth.no_cookie') : BROWSER_NAMES[v] }}
        </option>
      </select>

      <!-- Safari: FDA not granted -->
      <div v-if="settingsStore.settings.cookie_browser === 'safari' && safariAccessGranted === false"
           class="mt-2 p-2 rounded-md bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800">
        <p class="text-xs text-amber-600 dark:text-amber-400">{{ t('auth.safari_fda_denied') }}</p>
        <div class="mt-1.5">
          <button @click="openFullDiskAccessSettings"
                  class="px-2 py-0.5 text-xs rounded bg-amber-500 text-white hover:bg-amber-600 transition-colors">
            {{ t('auth.open_system_settings') }}
          </button>
        </div>
      </div>

      <!-- Safari: FDA granted -->
      <p v-if="settingsStore.settings.cookie_browser === 'safari' && safariAccessGranted === true"
         class="mt-2 text-xs text-green-600 dark:text-green-400">
        {{ t('auth.safari_fda_granted') }}
      </p>
    </div>

    <!-- Cookie file -->
    <div>
      <label class="block text-sm font-medium mb-1">{{ t('auth.cookie_file') }}</label>
      <p class="text-xs text-neutral-500 mb-2">{{ t('auth.cookie_file_description') }}</p>
      <div class="flex gap-2">
        <input :value="settingsStore.settings.cookie_file"
               @input="settingsStore.updateSetting('cookie_file', ($event.target as HTMLInputElement).value)"
               class="flex-1 h-8 px-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm font-mono outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
               :placeholder="t('auth.cookie_file_placeholder')" />
        <button @click="handleBrowseCookieFile"
                class="px-3 h-8 rounded-md text-sm bg-neutral-200 dark:bg-neutral-700 hover:bg-neutral-300 dark:hover:bg-neutral-600 transition-colors">
          {{ t('common.browse') }}
        </button>
      </div>
    </div>

    <!-- Info box -->
    <div class="p-3 rounded-md bg-blue-50 dark:bg-blue-900/20 text-xs text-blue-700 dark:text-blue-300 space-y-1">
      <p class="font-medium">{{ t('auth.cookie_note_title') }}</p>
      <ul class="list-disc list-inside space-y-0.5">
        <li>{{ t('auth.cookie_note_1') }}</li>
        <li>{{ t('auth.cookie_note_2') }}</li>
        <li>{{ t('auth.cookie_note_3') }}</li>
      </ul>
    </div>
  </div>
</template>
