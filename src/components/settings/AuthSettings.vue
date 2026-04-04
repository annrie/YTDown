<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useSettingsStore } from '../../stores/settings'
import { open } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'

const settingsStore = useSettingsStore()
const safariAccessGranted = ref<boolean | null>(null)

const browsers = [
  { value: 'none', label: '使用しない' },
  { value: 'safari', label: 'Safari' },
  { value: 'chrome', label: 'Google Chrome' },
  { value: 'brave', label: 'Brave' },
  { value: 'edge', label: 'Microsoft Edge' },
  { value: 'firefox', label: 'Firefox' },
  { value: 'chromium', label: 'Chromium' },
  { value: 'opera', label: 'Opera' },
  { value: 'vivaldi', label: 'Vivaldi' },
]

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
    title: 'Cookieファイルを選択',
    filters: [
      { name: 'Cookie files', extensions: ['txt', 'cookies'] },
      { name: 'All files', extensions: ['*'] },
    ],
  })
  if (selected && typeof selected === 'string') {
    settingsStore.updateSetting('cookie_file', selected)
  }
}

async function recheckSafariAccess() {
  try {
    safariAccessGranted.value = await invoke<boolean>('check_safari_access')
  } catch {
    safariAccessGranted.value = false
  }
}

function openFullDiskAccessSettings() {
  openUrl('x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles')
}
</script>

<template>
  <div class="space-y-6">
    <h3 class="text-base font-semibold">認証設定</h3>

    <!-- Cookie browser -->
    <div>
      <label class="block text-sm font-medium mb-1">Cookieの取得元ブラウザ</label>
      <p class="text-xs text-neutral-500 mb-2">
        年齢制限コンテンツやメンバー限定コンテンツのダウンロードに必要です。
        ブラウザでログイン済みのCookieを使用します。
      </p>
      <select :value="settingsStore.settings.cookie_browser"
              @change="settingsStore.updateSetting('cookie_browser', ($event.target as HTMLSelectElement).value)"
              class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm">
        <option v-for="b in browsers" :key="b.value" :value="b.value">{{ b.label }}</option>
      </select>

      <!-- Safari: FDA not granted -->
      <div v-if="settingsStore.settings.cookie_browser === 'safari' && safariAccessGranted === false"
           class="mt-2 p-2 rounded-md bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800">
        <p class="text-xs text-amber-600 dark:text-amber-400">
          ⚠ Safari の Cookie にアクセスできません。フルディスクアクセスに YTDown を追加してください。
        </p>
        <div class="mt-1.5 flex gap-2">
          <button @click="openFullDiskAccessSettings"
                  class="px-2 py-0.5 text-xs rounded bg-amber-500 text-white hover:bg-amber-600 transition-colors">
            システム設定を開く
          </button>
          <button @click="recheckSafariAccess"
                  class="px-2 py-0.5 text-xs rounded bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-300 hover:bg-amber-200 dark:hover:bg-amber-800/50 transition-colors">
            再チェック
          </button>
        </div>
      </div>

      <!-- Safari: FDA granted -->
      <p v-if="settingsStore.settings.cookie_browser === 'safari' && safariAccessGranted === true"
         class="mt-2 text-xs text-green-600 dark:text-green-400">
        ✓ Safari の Cookie にアクセスできます
      </p>
    </div>

    <!-- Cookie file -->
    <div>
      <label class="block text-sm font-medium mb-1">Cookieファイル</label>
      <p class="text-xs text-neutral-500 mb-2">
        ブラウザの代わりにNetscape形式のCookieファイルを使用する場合はパスを指定してください。
      </p>
      <div class="flex gap-2">
        <input :value="settingsStore.settings.cookie_file"
               @input="settingsStore.updateSetting('cookie_file', ($event.target as HTMLInputElement).value)"
               class="flex-1 h-8 px-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm font-mono outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
               placeholder="Cookieファイルのパス（空欄で無効）" />
        <button @click="handleBrowseCookieFile"
                class="px-3 h-8 rounded-md text-sm bg-neutral-200 dark:bg-neutral-700 hover:bg-neutral-300 dark:hover:bg-neutral-600 transition-colors">
          参照...
        </button>
      </div>
    </div>

    <!-- Info box -->
    <div class="p-3 rounded-md bg-blue-50 dark:bg-blue-900/20 text-xs text-blue-700 dark:text-blue-300 space-y-1">
      <p class="font-medium">Cookie使用時の注意</p>
      <ul class="list-disc list-inside space-y-0.5">
        <li>ブラウザCookieを使う場合、対象ブラウザが起動中でも動作します</li>
        <li>Cookieファイルとブラウザの両方を設定した場合、ファイルが優先されます</li>
        <li>セキュリティのため、Cookieは暗号化された状態で読み取ります</li>
      </ul>
    </div>
  </div>
</template>
