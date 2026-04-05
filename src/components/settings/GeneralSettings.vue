<script setup lang="ts">
import { useSettingsStore } from '../../stores/settings'
import { useYtdlp } from '../../composables/useYtdlp'
import { onMounted, ref } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'

const settingsStore = useSettingsStore()
const { info: ytdlpInfo, loading: ytdlpLoading, loadInfo: loadYtdlpInfo } = useYtdlp()

onMounted(() => {
  loadYtdlpInfo()
})

// Backup / Restore
const backupBusy = ref(false)
const backupMsg = ref<{ type: 'success' | 'error'; text: string } | null>(null)

async function handleExport() {
  const path = await save({
    title: '設定をエクスポート',
    defaultPath: 'ytdown-backup.json',
    filters: [{ name: 'JSON', extensions: ['json'] }],
  })
  if (!path) return
  backupBusy.value = true
  backupMsg.value = null
  try {
    await invoke('export_settings_to_file', { path })
    backupMsg.value = { type: 'success', text: 'エクスポートしました' }
  } catch (e) {
    backupMsg.value = { type: 'error', text: String(e) }
  } finally {
    backupBusy.value = false
  }
}

async function handleImport() {
  const path = await open({
    title: '設定をインポート',
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
      text: `インポート完了: 設定 ${result.settings_count} 件、プリセット ${result.presets_count} 件、ルール ${result.rules_count} 件、スケジュール ${result.schedules_count} 件`,
    }
  } catch (e) {
    backupMsg.value = { type: 'error', text: String(e) }
  } finally {
    backupBusy.value = false
  }
}

const filenamePresets = [
  { label: 'タイトル', value: '%(title)s.%(ext)s' },
  { label: 'タイトル - チャンネル', value: '%(title)s - %(channel)s.%(ext)s' },
  { label: 'チャンネル/タイトル', value: '%(channel)s/%(title)s.%(ext)s' },
  { label: '日付-タイトル', value: '%(upload_date)s-%(title)s.%(ext)s' },
]

async function handleBrowseBackground(mode: 'light' | 'dark') {
  const selected = await open({
    multiple: false,
    title: mode === 'light' ? 'ライトモード用背景画像を選択' : 'ダークモード用背景画像を選択',
    filters: [{ name: '画像', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp'] }],
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
    title: 'ダウンロードフォルダを選択',
  })
  if (selected && typeof selected === 'string') {
    settingsStore.updateSetting('download_dir', selected)
  }
}
</script>

<template>
  <div class="space-y-6">
    <h3 class="text-base font-semibold">一般設定</h3>

    <!-- Download directory -->
    <div>
      <label class="block text-sm font-medium mb-1">ダウンロードフォルダ</label>
      <div class="flex gap-2">
        <input :value="settingsStore.settings.download_dir"
               @input="settingsStore.updateSetting('download_dir', ($event.target as HTMLInputElement).value)"
               class="flex-1 h-8 px-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm outline-none focus:ring-1 focus:ring-[var(--color-accent)]" />
        <button class="px-3 h-8 rounded-md text-sm bg-neutral-200 dark:bg-neutral-700" @click="handleBrowseDir">
          参照...
        </button>
      </div>
    </div>

    <!-- Filename template -->
    <div>
      <label class="block text-sm font-medium mb-1">ファイル名テンプレート</label>
      <select :value="settingsStore.settings.filename_template"
              @change="settingsStore.updateSetting('filename_template', ($event.target as HTMLSelectElement).value)"
              class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm">
        <option v-for="preset in filenamePresets" :key="preset.value" :value="preset.value">
          {{ preset.label }} ({{ preset.value }})
        </option>
      </select>
      <input :value="settingsStore.settings.filename_template"
             @input="settingsStore.updateSetting('filename_template', ($event.target as HTMLInputElement).value)"
             class="mt-1 w-full h-8 px-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm font-mono outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
             placeholder="カスタムテンプレート" />
    </div>

    <!-- Concurrent downloads -->
    <div>
      <label class="block text-sm font-medium mb-1">同時ダウンロード数</label>
      <input type="number" min="1" max="10"
             :value="settingsStore.settings.concurrent_downloads"
             @input="settingsStore.updateSetting('concurrent_downloads', parseInt(($event.target as HTMLInputElement).value) || 3)"
             class="w-20 h-8 px-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm outline-none focus:ring-1 focus:ring-[var(--color-accent)]" />
    </div>

    <!-- Theme -->
    <div>
      <label class="block text-sm font-medium mb-1">テーマ</label>
      <div class="flex gap-2">
        <button v-for="theme in (['system', 'light', 'dark'] as const)" :key="theme"
                class="px-3 py-1.5 rounded-md text-sm"
                :class="settingsStore.settings.theme === theme ? 'bg-[var(--color-accent)] text-white' : 'bg-neutral-100 dark:bg-neutral-700'"
                @click="settingsStore.updateSetting('theme', theme)">
          {{ theme === 'system' ? 'システム' : theme === 'light' ? 'ライト' : 'ダーク' }}
        </button>
      </div>
    </div>

    <!-- Background image -->
    <div>
      <label class="block text-sm font-medium mb-2">背景画像</label>
      <div class="space-y-3">
        <!-- Light mode -->
        <div class="p-3 rounded-lg bg-neutral-50 dark:bg-neutral-800/50 space-y-1">
          <label class="block text-xs font-medium text-neutral-500">ライトモード用</label>
          <div class="flex gap-2">
            <input :value="settingsStore.settings.background_image_light"
                   @input="settingsStore.updateSetting('background_image_light', ($event.target as HTMLInputElement).value)"
                   class="flex-1 h-8 px-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
                   placeholder="画像ファイルのパスまたはURL" />
            <button class="px-3 h-8 rounded-md text-sm bg-neutral-200 dark:bg-neutral-700" @click="handleBrowseBackground('light')">
              選択...
            </button>
            <button v-if="settingsStore.settings.background_image_light"
                    class="px-3 h-8 rounded-md text-sm bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400"
                    @click="settingsStore.updateSetting('background_image_light', '')">
              解除
            </button>
          </div>
        </div>
        <!-- Dark mode -->
        <div class="p-3 rounded-lg bg-neutral-50 dark:bg-neutral-800/50 space-y-1">
          <label class="block text-xs font-medium text-neutral-500">ダークモード用</label>
          <div class="flex gap-2">
            <input :value="settingsStore.settings.background_image_dark"
                   @input="settingsStore.updateSetting('background_image_dark', ($event.target as HTMLInputElement).value)"
                   class="flex-1 h-8 px-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
                   placeholder="画像ファイルのパスまたはURL" />
            <button class="px-3 h-8 rounded-md text-sm bg-neutral-200 dark:bg-neutral-700" @click="handleBrowseBackground('dark')">
              選択...
            </button>
            <button v-if="settingsStore.settings.background_image_dark"
                    class="px-3 h-8 rounded-md text-sm bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400"
                    @click="settingsStore.updateSetting('background_image_dark', '')">
              解除
            </button>
          </div>
        </div>
        <!-- Opacity slider -->
        <div v-if="settingsStore.settings.background_image_light || settingsStore.settings.background_image_dark">
          <label class="block text-xs text-neutral-500 mb-1">
            背景の濃さ: {{ settingsStore.settings.background_opacity }}%
          </label>
          <input type="range" min="5" max="100" step="5"
                 :value="settingsStore.settings.background_opacity"
                 @input="settingsStore.updateSetting('background_opacity', parseInt(($event.target as HTMLInputElement).value))"
                 class="w-full accent-[var(--color-accent)]" />
          <div class="flex justify-between text-xs text-neutral-400">
            <span>薄い</span>
            <span>濃い</span>
          </div>
        </div>
      </div>
    </div>

    <!-- yt-dlp info -->
    <div>
      <label class="block text-sm font-medium mb-1">yt-dlp</label>
      <div class="p-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm space-y-1">
        <div v-if="ytdlpLoading" class="text-neutral-500">読み込み中...</div>
        <template v-else-if="ytdlpInfo">
          <div class="flex justify-between">
            <span class="text-neutral-500">バージョン</span>
            <span>{{ ytdlpInfo.version }}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-neutral-500">パス</span>
            <span class="font-mono text-xs truncate ml-4">{{ ytdlpInfo.path }}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-neutral-500">管理</span>
            <span>{{ ytdlpInfo.managed_by === 'homebrew' ? 'Homebrew' : ytdlpInfo.managed_by === 'bundled' ? 'バンドル版' : '手動' }}</span>
          </div>
          <div v-if="ytdlpInfo.update_available" class="mt-2 text-xs text-orange-600 dark:text-orange-400">
            アップデートが利用可能です
          </div>
        </template>
        <div v-else class="text-red-500">yt-dlp が見つかりません</div>
      </div>
    </div>

    <!-- Backup / Restore -->
    <div class="p-3 rounded-lg border border-[var(--color-separator)] space-y-3">
      <h4 class="text-sm font-medium">バックアップ / リストア</h4>
      <p class="text-xs text-neutral-500">設定・プリセット・自動分類ルール・スケジュールをJSONファイルに保存・復元します。インポート時は既存データが上書きされます。</p>
      <div class="flex gap-2">
        <button @click="handleExport" :disabled="backupBusy"
                class="px-3 py-1.5 text-xs rounded-md bg-neutral-100 dark:bg-neutral-700 hover:bg-neutral-200 dark:hover:bg-neutral-600 disabled:opacity-50 transition-colors">
          エクスポート
        </button>
        <button @click="handleImport" :disabled="backupBusy"
                class="px-3 py-1.5 text-xs rounded-md bg-neutral-100 dark:bg-neutral-700 hover:bg-neutral-200 dark:hover:bg-neutral-600 disabled:opacity-50 transition-colors">
          インポート
        </button>
      </div>
      <p v-if="backupMsg"
         :class="backupMsg.type === 'success' ? 'text-green-600 dark:text-green-400' : 'text-red-500'"
         class="text-xs">{{ backupMsg.text }}</p>
    </div>

    <!-- yt-dlp path override -->
    <div>
      <label class="block text-sm font-medium mb-1">yt-dlp パス</label>
      <input :value="settingsStore.settings.ytdlp_path"
             @input="settingsStore.updateSetting('ytdlp_path', ($event.target as HTMLInputElement).value)"
             class="w-full h-8 px-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm font-mono outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
             placeholder="auto（自動検出）" />
      <p class="text-xs text-neutral-400 mt-1">「auto」で自動検出、またはフルパスを入力</p>
    </div>
  </div>
</template>
