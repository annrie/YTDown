<script setup lang="ts">
import { useSettingsStore } from '../../stores/settings'
import { useYtdlp } from '../../composables/useYtdlp'
import { onMounted } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'

const settingsStore = useSettingsStore()
const { info: ytdlpInfo, loading: ytdlpLoading, loadInfo: loadYtdlpInfo } = useYtdlp()

onMounted(() => {
  loadYtdlpInfo()
})

const filenamePresets = [
  { label: 'タイトル', value: '%(title)s.%(ext)s' },
  { label: 'タイトル - チャンネル', value: '%(title)s - %(channel)s.%(ext)s' },
  { label: 'チャンネル/タイトル', value: '%(channel)s/%(title)s.%(ext)s' },
  { label: '日付-タイトル', value: '%(upload_date)s-%(title)s.%(ext)s' },
]

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
