<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import type { ViewMode, SidebarSection, DownloadOptions } from './types'
import { useDownloadsStore } from './stores/downloads'
import { useLibraryStore } from './stores/library'
import { useSettingsStore } from './stores/settings'

// Layout components
import AppToolbar from './components/layout/AppToolbar.vue'
import AppSidebar from './components/layout/AppSidebar.vue'
import AppStatusBar from './components/layout/AppStatusBar.vue'

// Download components
import DownloadDialog from './components/download/DownloadDialog.vue'
import DownloadQueue from './components/download/DownloadQueue.vue'
import BatchUrlDialog from './components/download/BatchUrlDialog.vue'

// Library view components
import ListView from './components/library/ListView.vue'
import GridView from './components/library/GridView.vue'
import ColumnView from './components/library/ColumnView.vue'

// Image components
import ImageDownloadView from './components/images/ImageDownloadView.vue'
import ImageGalleryView from './components/images/ImageGalleryView.vue'

// Settings components
import GeneralSettings from './components/settings/GeneralSettings.vue'
import FormatSettings from './components/settings/FormatSettings.vue'
import AuthSettings from './components/settings/AuthSettings.vue'
import AdvancedSettings from './components/settings/AdvancedSettings.vue'
import RuleSettings from './components/settings/RuleSettings.vue'

const currentView = ref<ViewMode>('list')
const currentSection = ref<SidebarSection>('library-all')
const searchQuery = ref('')

// Download dialog state
const showDownloadDialog = ref(false)
const showBatchDialog = ref(false)
const downloadUrl = ref('')

const downloadsStore = useDownloadsStore()
const libraryStore = useLibraryStore()
const settingsStore = useSettingsStore()

// Computed: items to display in library views
const displayItems = computed(() => {
  let items = libraryStore.filteredItems
  if (currentSection.value === 'library-video') {
    items = items.filter(d => ['mp4', 'mkv', 'webm'].includes(d.format ?? ''))
  } else if (currentSection.value === 'library-audio') {
    items = items.filter(d => ['mp3', 'm4a', 'flac', 'wav', 'opus'].includes(d.format ?? ''))
  }
  return items
})

// Section label for display
const sectionLabel = computed(() => {
  const labels: Record<SidebarSection, string> = {
    'downloads-active': '進行中のダウンロード',
    'downloads-completed': '完了したダウンロード',
    'library-all': 'すべてのメディア',
    'library-video': '映像',
    'library-audio': '音声',
    'settings': '設定',
    'images-download': '画像ダウンロード',
    'images-gallery': '画像ギャラリー',
  }
  return labels[currentSection.value] ?? currentSection.value
})

// Whether to show library-style views
const isLibrarySection = computed(() =>
  ['library-all', 'library-video', 'library-audio', 'downloads-completed'].includes(currentSection.value)
)

// Dark mode detection & theme application
const isDark = ref(false)
let darkModeQuery: MediaQueryList | null = null

function resolveIsDark(): boolean {
  const theme = settingsStore.settings.theme
  if (theme === 'dark') return true
  if (theme === 'light') return false
  return window.matchMedia('(prefers-color-scheme: dark)').matches
}

function applyTheme() {
  isDark.value = resolveIsDark()
  document.documentElement.classList.toggle('dark', isDark.value)
}

function onDarkModeChange() {
  if (settingsStore.settings.theme === 'system') {
    applyTheme()
  }
}

// Background image (auto-switch by theme)
const activeBackgroundImage = computed(() => {
  return isDark.value
    ? settingsStore.settings.background_image_dark
    : settingsStore.settings.background_image_light
})

const backgroundStyle = computed(() => {
  const bg = activeBackgroundImage.value
  if (!bg) return {}
  const url = bg.startsWith('/') ? convertFileSrc(bg) : bg
  return {
    backgroundImage: `url("${url}")`,
    backgroundSize: 'cover',
    backgroundPosition: 'center',
  }
})
const backgroundOverlayOpacity = computed(() => {
  return (100 - settingsStore.settings.background_opacity) / 100
})
const hasBackground = computed(() => !!activeBackgroundImage.value)

// Handlers
function handleSubmitUrl(url: string) {
  downloadUrl.value = url
  showDownloadDialog.value = true
}

async function handleStartDownload(url: string, options: DownloadOptions) {
  currentSection.value = 'downloads-active'
  invoke('save_url_history', { historyType: 'video', url }).catch(() => {})
  await downloadsStore.startDownload(url, options)
}

async function handleBatchDownload(urls: string[]) {
  const s = settingsStore.settings
  const defaultOptions: DownloadOptions = {
    format: s.default_video_format,
    quality: s.default_video_quality,
    output_dir: s.download_dir,
    embed_thumbnail: s.embed_thumbnail,
    embed_metadata: s.embed_metadata,
    write_subs: s.write_subs,
    embed_subs: s.embed_subs,
    embed_chapters: s.embed_chapters,
    sponsorblock: s.sponsorblock,
    custom_format: null,
    playlist_mode: 'single',
    restrict_filenames: s.restrict_filenames,
    no_overwrites: s.no_overwrites,
    geo_bypass: s.geo_bypass,
    rate_limit: s.rate_limit,
    sub_lang: s.sub_lang,
    convert_subs: s.convert_subs,
    merge_output_format: s.merge_output_format,
    recode_video: s.recode_video,
    retries: s.retries,
    proxy: s.proxy,
    extra_args: s.extra_args,
  }
  // Switch to active downloads immediately so user sees items appear
  currentSection.value = 'downloads-active'
  // Start all downloads in parallel (each runs as an independent yt-dlp process)
  await Promise.allSettled(
    urls.map(url => downloadsStore.startDownload(url, defaultOptions))
  )
}

// Keyboard shortcuts
function handleKeydown(e: KeyboardEvent) {
  if (e.metaKey && e.key === ',') {
    e.preventDefault()
    currentSection.value = 'settings'
  }
  if (e.metaKey && e.key === 'f') {
    e.preventDefault()
    // Toolbar search will handle focus
  }
  if (e.metaKey && e.key === '1') { e.preventDefault(); currentView.value = 'list' }
  if (e.metaKey && e.key === '2') { e.preventDefault(); currentView.value = 'grid' }
  if (e.metaKey && e.key === '3') { e.preventDefault(); currentView.value = 'column' }
}

// Re-apply theme when setting changes
watch(() => settingsStore.settings.theme, () => applyTheme())

onMounted(async () => {
  await settingsStore.loadSettings()
  applyTheme()
  await downloadsStore.setupProgressListener(() => {
    libraryStore.loadItems()
  })
  await libraryStore.loadItems()
  document.addEventListener('keydown', handleKeydown)
  darkModeQuery = window.matchMedia('(prefers-color-scheme: dark)')
  darkModeQuery.addEventListener('change', onDarkModeChange)
})

onUnmounted(() => {
  downloadsStore.cleanup()
  document.removeEventListener('keydown', handleKeydown)
  darkModeQuery?.removeEventListener('change', onDarkModeChange)
})
</script>

<template>
  <div class="flex flex-col h-screen">
    <!-- Toolbar -->
    <AppToolbar
      :currentView="currentView"
      :searchQuery="searchQuery"
      :currentSection="currentSection"
      @update:currentView="currentView = $event"
      @update:searchQuery="searchQuery = $event; libraryStore.searchQuery = $event"
      @submit-url="handleSubmitUrl"
      @open-batch="showBatchDialog = true"
    />

    <div class="flex flex-1 overflow-hidden">
      <!-- Sidebar -->
      <AppSidebar
        :currentSection="currentSection"
        @update:currentSection="currentSection = $event"
      />

      <!-- Main Content -->
      <main class="flex-1 flex flex-col overflow-hidden bg-white dark:bg-neutral-900 relative" :style="backgroundStyle">
        <!-- Background overlay for readability -->
        <div v-if="hasBackground"
             class="absolute inset-0 bg-white dark:bg-neutral-900 pointer-events-none"
             :style="{ opacity: backgroundOverlayOpacity }" />
        <!-- Section header (for library views) -->
        <div v-if="isLibrarySection" class="flex items-center justify-between px-4 py-2 border-b border-[var(--color-separator)] relative z-10">
          <span class="text-sm font-medium text-neutral-600 dark:text-neutral-400">
            {{ sectionLabel }}
          </span>
        </div>

        <!-- Content Area -->
        <div class="flex-1 overflow-auto relative z-10">
          <!-- Active downloads -->
          <template v-if="currentSection === 'downloads-active'">
            <div class="px-4 py-2 border-b border-[var(--color-separator)]">
              <span class="text-sm font-medium text-neutral-600 dark:text-neutral-400">{{ sectionLabel }}</span>
            </div>
            <DownloadQueue />
          </template>

          <!-- Completed downloads / Library views -->
          <template v-else-if="isLibrarySection">
            <div class="flex-1 overflow-auto" :class="currentView !== 'column' ? 'p-4' : 'h-full'">
              <ListView v-if="currentView === 'list'" :items="displayItems" />
              <GridView v-else-if="currentView === 'grid'" :items="displayItems" />
              <ColumnView v-else :items="displayItems" />
            </div>
          </template>

          <!-- Image download -->
          <template v-else-if="currentSection === 'images-download'">
            <ImageDownloadView />
          </template>

          <!-- Image gallery -->
          <template v-else-if="currentSection === 'images-gallery'">
            <ImageGalleryView />
          </template>

          <!-- Settings -->
          <template v-else-if="currentSection === 'settings'">
            <div class="p-6 space-y-8 max-w-2xl overflow-auto">
              <GeneralSettings />
              <hr class="border-[var(--color-separator)]" />
              <FormatSettings />
              <hr class="border-[var(--color-separator)]" />
              <AuthSettings />
              <hr class="border-[var(--color-separator)]" />
              <AdvancedSettings />
              <hr class="border-[var(--color-separator)]" />
              <RuleSettings />
            </div>
          </template>
        </div>
      </main>
    </div>

    <!-- Status Bar -->
    <AppStatusBar />

    <!-- Download Dialog -->
    <DownloadDialog
      :url="downloadUrl"
      :open="showDownloadDialog"
      @close="showDownloadDialog = false"
      @start="handleStartDownload"
    />

    <!-- Batch URL Dialog -->
    <BatchUrlDialog
      :open="showBatchDialog"
      @close="showBatchDialog = false"
      @start-batch="handleBatchDownload"
    />
  </div>
</template>
