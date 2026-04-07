<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { useImagesStore } from '../../stores/images'
import { useSettingsStore } from '../../stores/settings'
import ImagePreviewGrid from './ImagePreviewGrid.vue'
import UrlHistoryDropdown from '../common/UrlHistoryDropdown.vue'

const { t } = useI18n()
const imagesStore = useImagesStore()
const settingsStore = useSettingsStore()

const url = ref('')
const minWidth = ref(100)
const minHeight = ref(100)
const format = ref<string | undefined>(undefined)
const fetchingBrowserUrl = ref(false)
const browserUrlError = ref('')

let unlistenProgress: (() => void) | null = null

onMounted(async () => {
  const unlisten = await imagesStore.setupProgressListener()
  unlistenProgress = unlisten
})

onUnmounted(() => {
  unlistenProgress?.()
})

async function handleScrape() {
  if (!url.value.trim()) return
  invoke('save_url_history', { historyType: 'image', url: url.value.trim() }).catch(() => {})
  await imagesStore.scrapeUrl(url.value.trim(), minWidth.value, minHeight.value)
}

async function handleDownload() {
  if (!imagesStore.hasSelection) return
  const outputDir = settingsStore.settings.download_dir || `~/Downloads/YTDown`
  await imagesStore.startDownload(outputDir, format.value)
}

async function fetchBrowserUrl() {
  fetchingBrowserUrl.value = true
  browserUrlError.value = ''
  try {
    const result = await invoke<string>('get_browser_url')
    if (result) url.value = result
  } catch (e) {
    browserUrlError.value = String(e)
    setTimeout(() => { browserUrlError.value = '' }, 3000)
  } finally {
    fetchingBrowserUrl.value = false
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !imagesStore.scraping) {
    handleScrape()
  }
}
</script>

<template>
  <div class="flex flex-col h-full">
    <!-- Header: URL input + filters (fixed top) -->
    <div class="shrink-0 p-4 pb-0">
      <!-- URL input bar -->
      <div class="flex gap-2 mb-3">
        <input
          v-model="url"
          type="url"
          :placeholder="t('images.fetch_placeholder')"
          class="flex-1 px-3 py-2 rounded-lg bg-neutral-100 dark:bg-neutral-800 border border-neutral-300 dark:border-neutral-600 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          :disabled="imagesStore.scraping"
          @keydown="handleKeydown"
        />
        <UrlHistoryDropdown
          type="image"
          @select="(u: string) => url = u"
        />
        <!-- ブラウザからURL取得ボタン -->
        <button
          @click="fetchBrowserUrl"
          :disabled="fetchingBrowserUrl || imagesStore.scraping"
          class="w-8 h-8 flex items-center justify-center rounded-md hover:bg-neutral-200 dark:hover:bg-neutral-700 transition-colors disabled:opacity-50"
          :class="browserUrlError ? 'text-red-500' : 'text-neutral-500 hover:text-[var(--color-accent)]'"
          :title="t('toolbar.fetch_browser_url')"
        >
          <svg class="w-5 h-5" :class="{ 'animate-spin': fetchingBrowserUrl }" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path v-if="!fetchingBrowserUrl" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 21a9 9 0 100-18 9 9 0 000 18zm0-18v18m-9-9h18M3.6 9h16.8M3.6 15h16.8" />
            <path v-else stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 12a8 8 0 018-8" />
          </svg>
        </button>
        <button
          class="px-4 py-2 rounded-lg bg-blue-500 text-white text-sm font-medium hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="imagesStore.scraping || !url.trim()"
          @click="handleScrape"
        >
          {{ imagesStore.scraping ? t('images.fetching') : t('images.fetch') }}
        </button>
        <button
          v-if="imagesStore.scrapedImages.length > 0"
          class="px-3 py-2 rounded-lg text-sm text-neutral-500 hover:bg-neutral-200 dark:hover:bg-neutral-700"
          :title="t('images.clear_results')"
          @click="url = ''; imagesStore.resetScrape()"
        >
          {{ t('common.clear') }}
        </button>
      </div>

      <!-- Filter settings -->
      <div class="flex gap-4 mb-4 text-sm">
        <label class="flex items-center gap-1.5">
          <span class="text-neutral-500 dark:text-neutral-400">{{ t('images.min_width') }}</span>
          <input
            v-model.number="minWidth"
            type="number"
            min="0"
            class="w-16 px-2 py-1 rounded bg-neutral-100 dark:bg-neutral-800 border border-neutral-300 dark:border-neutral-600"
          />
          <span class="text-neutral-400">px</span>
        </label>
        <label class="flex items-center gap-1.5">
          <span class="text-neutral-500 dark:text-neutral-400">{{ t('images.min_height') }}</span>
          <input
            v-model.number="minHeight"
            type="number"
            min="0"
            class="w-16 px-2 py-1 rounded bg-neutral-100 dark:bg-neutral-800 border border-neutral-300 dark:border-neutral-600"
          />
          <span class="text-neutral-400">px</span>
        </label>
        <label class="flex items-center gap-1.5">
          <span class="text-neutral-500 dark:text-neutral-400">{{ t('images.convert_format') }}</span>
          <select
            v-model="format"
            class="px-2 py-1 rounded bg-neutral-100 dark:bg-neutral-800 border border-neutral-300 dark:border-neutral-600"
          >
            <option :value="undefined">{{ t('images.format_original') }}</option>
            <option value="webp">WebP</option>
            <option value="avif" disabled>{{ t('images.format_avif_pending') }}</option>
          </select>
        </label>
      </div>

      <!-- Error message -->
      <div v-if="imagesStore.error" class="mb-3 px-3 py-2 rounded-lg bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400 text-sm">
        {{ imagesStore.error }}
      </div>
    </div>

    <!-- Scrollable content area -->
    <div class="flex-1 min-h-0 overflow-y-auto px-4">
      <!-- Preview grid -->
      <div v-if="imagesStore.scrapedImages.length > 0">
        <ImagePreviewGrid
          :images="imagesStore.scrapedImages"
          :selected-ids="imagesStore.selectedIds"
          @toggle-select="imagesStore.toggleSelect"
          @select-all="imagesStore.selectAll"
          @deselect-all="imagesStore.deselectAll"
        />
      </div>

      <!-- Empty state -->
      <div
        v-else-if="!imagesStore.scraping && !imagesStore.error"
        class="h-full flex items-center justify-center text-neutral-400 dark:text-neutral-500"
      >
        <div class="text-center">
          <div class="text-4xl mb-2">🖼</div>
          <p>{{ t('images.enter_url_hint') }}</p>
        </div>
      </div>

      <!-- Loading state -->
      <div
        v-if="imagesStore.scraping"
        class="h-full flex items-center justify-center text-neutral-400"
      >
        <div class="text-center">
          <div class="animate-spin text-2xl mb-2">⏳</div>
          <p>{{ t('images.analyzing') }}</p>
        </div>
      </div>
    </div>

    <!-- Download bar (fixed bottom) -->
    <div v-if="imagesStore.scrapedImages.length > 0" class="shrink-0 px-4 py-3 border-t border-neutral-200 dark:border-neutral-700 bg-white dark:bg-neutral-900">
      <!-- Progress bar (during download) -->
      <div v-if="imagesStore.downloading && imagesStore.downloadProgress" class="mb-2">
        <div class="flex justify-between text-xs text-neutral-500 mb-1">
          <span>{{ imagesStore.downloadProgress.image_index + 1 }} / {{ imagesStore.downloadProgress.total_images }}</span>
          <span>{{ Math.round(imagesStore.downloadProgress.percent) }}%</span>
        </div>
        <div class="h-1.5 bg-neutral-200 dark:bg-neutral-700 rounded-full overflow-hidden">
          <div
            class="h-full bg-blue-500 rounded-full transition-all duration-300"
            :style="{ width: `${imagesStore.downloadProgress.percent}%` }"
          />
        </div>
      </div>

      <!-- Completed state -->
      <div v-if="imagesStore.downloadCompleted" class="flex items-center justify-between">
        <span class="text-sm text-green-600 dark:text-green-400 font-medium">
          {{ t('images.download_complete') }}
        </span>
        <button
          class="px-4 py-2 rounded-lg bg-neutral-200 dark:bg-neutral-700 text-sm font-medium hover:bg-neutral-300 dark:hover:bg-neutral-600"
          @click="imagesStore.resetScrape()"
        >
          {{ t('images.fetch_another') }}
        </button>
      </div>

      <!-- Download button -->
      <button
        v-else
        class="w-full py-2 rounded-lg bg-blue-500 text-white text-sm font-medium hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed"
        :disabled="!imagesStore.hasSelection || imagesStore.downloading"
        @click="handleDownload"
      >
        {{ imagesStore.downloading ? t('images.downloading') : t('images.download_n_images', { count: imagesStore.selectedCount }) }}
      </button>
    </div>
  </div>
</template>
