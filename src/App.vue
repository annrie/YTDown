<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { ViewMode, SidebarSection, DownloadOptions } from './types'
import { useDownloadsStore } from './stores/downloads'
import { useLibraryStore } from './stores/library'
import { useSchedulesStore } from './stores/schedules'
import { useSettingsStore } from './stores/settings'
import { useI18n } from 'vue-i18n'
import { setLocale, type SupportedLocale } from './i18n'

// Layout components
import AppToolbar from './components/layout/AppToolbar.vue'
import AppSidebar from './components/layout/AppSidebar.vue'
import AppStatusBar from './components/layout/AppStatusBar.vue'

// Download components
import DownloadDialog from './components/download/DownloadDialog.vue'
import DownloadQueue from './components/download/DownloadQueue.vue'
import BatchUrlDialog from './components/download/BatchUrlDialog.vue'
import ChannelMonitorDialog from './components/schedules/ChannelMonitorDialog.vue'

// Library view components
import ListView from './components/library/ListView.vue'
import GridView from './components/library/GridView.vue'
import ColumnView from './components/library/ColumnView.vue'

// Image components
import ImageDownloadView from './components/images/ImageDownloadView.vue'
import ImageGalleryView from './components/images/ImageGalleryView.vue'

// Schedule components
import ScheduleView from './components/schedules/ScheduleView.vue'

// Settings components
import GeneralSettings from './components/settings/GeneralSettings.vue'
import FormatSettings from './components/settings/FormatSettings.vue'
import AuthSettings from './components/settings/AuthSettings.vue'
import AdvancedSettings from './components/settings/AdvancedSettings.vue'
import RuleSettings from './components/settings/RuleSettings.vue'
import PresetSettings from './components/settings/PresetSettings.vue'

const { t } = useI18n()
const currentView = ref<ViewMode>('list')
const currentSection = ref<SidebarSection>('library-all')
const searchQuery = ref('')

// Download dialog state
const showDownloadDialog = ref(false)
const showBatchDialog = ref(false)
const showChannelMonitorDialog = ref(false)
const downloadUrl = ref('')
const droppedBatchUrls = ref<string[]>([])
const isDraggingUrl = ref(false)
const dropTextCapture = ref<HTMLTextAreaElement | null>(null)
let dragDepth = 0
let unlistenNativeDragDrop: null | (() => void) = null

const downloadsStore = useDownloadsStore()
const libraryStore = useLibraryStore()
const schedulesStore = useSchedulesStore()
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
    'downloads-active': t('sidebar.active'),
    'downloads-completed': t('sidebar.completed'),
    'library-all': t('sidebar.library'),
    'library-video': t('sidebar.library_video'),
    'library-audio': t('sidebar.library_audio'),
    'images-download': t('images.download_title'),
    'images-gallery': t('images.gallery_title'),
    'schedules': t('sidebar.section_schedule'),
    'settings': t('settings.title'),
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

function closeBatchDialog() {
  showBatchDialog.value = false
  droppedBatchUrls.value = []
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

function normalizeDroppedUrl(value: string): string | null {
  const candidates = [value.trim(), value.trim().replace(/[),.;]+$/g, '')]
  for (const candidate of candidates) {
    if (!candidate) continue
    try {
      const url = new URL(candidate)
      if (!['http:', 'https:'].includes(url.protocol)) continue
      return url.toString()
    } catch {
      continue
    }
  }
  return null
}

function extractUrlsFromText(text: string): string[] {
  const urls = new Set<string>()
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith('#')) continue
    const matches = trimmed.match(/https?:\/\/[^\s"'<>]+/g) ?? []
    for (const match of matches) {
      const normalized = normalizeDroppedUrl(match)
      if (normalized) urls.add(normalized)
    }
  }
  return [...urls]
}

function extractUrlsFromHtml(html: string): string[] {
  const urls = new Set<string>()
  const doc = new DOMParser().parseFromString(html, 'text/html')
  for (const element of doc.querySelectorAll('[href], [src]')) {
    const raw = element.getAttribute('href') ?? element.getAttribute('src')
    if (!raw) continue
    const normalized = normalizeDroppedUrl(raw)
    if (normalized) urls.add(normalized)
  }
  return [...urls]
}

function extractUrlsFromWebloc(text: string): string[] {
  const urls = new Set<string>()
  const doc = new DOMParser().parseFromString(text, 'application/xml')
  const keys = Array.from(doc.getElementsByTagName('key'))
  for (const key of keys) {
    if (key.textContent?.trim() !== 'URL') continue
    const sibling = key.nextElementSibling
    if (!sibling || sibling.tagName.toLowerCase() !== 'string') continue
    const normalized = normalizeDroppedUrl(sibling.textContent ?? '')
    if (normalized) urls.add(normalized)
  }
  return [...urls]
}

function extractUrlsFromInternetShortcut(text: string): string[] {
  const urls = new Set<string>()
  const match = text.match(/^URL=(.+)$/im)
  if (!match) return []
  const normalized = normalizeDroppedUrl(match[1])
  if (normalized) urls.add(normalized)
  return [...urls]
}

async function readDroppedStringItems(dataTransfer: DataTransfer): Promise<string[]> {
  const items = Array.from(dataTransfer.items ?? [])
    .filter((item) => item.kind === 'string')

  const values = await Promise.all(items.map((item) => new Promise<string>((resolve) => {
    item.getAsString((value) => resolve(value ?? ''))
  })))

  return values.filter(Boolean)
}

async function extractUrlsFromDroppedFile(file: File): Promise<string[]> {
  const ext = file.name.toLowerCase().split('.').pop() ?? ''
  const isPlainTextFile = ext === 'txt' || file.type.startsWith('text/')
  if (!['txt', 'webloc', 'url'].includes(ext) && !isPlainTextFile) {
    return []
  }

  let text = ''
  try {
    text = await file.text()
  } catch {
    return []
  }

  if (ext === 'webloc') {
    return extractUrlsFromWebloc(text)
  }
  if (ext === 'url') {
    return extractUrlsFromInternetShortcut(text)
  }
  return extractUrlsFromText(text)
}

async function extractUrlsFromDroppedPath(path: string): Promise<string[]> {
  const ext = path.toLowerCase().split('.').pop() ?? ''
  let text = ''
  try {
    text = await invoke<string>('read_text_file', { path })
  } catch {
    try {
      const response = await fetch(convertFileSrc(path))
      if (!response.ok) return []
      text = await response.text()
    } catch {
      return []
    }
  }

  const urls = new Set<string>()
  if (ext === 'webloc') {
    extractUrlsFromWebloc(text).forEach((url) => urls.add(url))
  }
  if (ext === 'url') {
    extractUrlsFromInternetShortcut(text).forEach((url) => urls.add(url))
  }
  extractUrlsFromText(text).forEach((url) => urls.add(url))
  extractUrlsFromHtml(text).forEach((url) => urls.add(url))
  return [...urls]
}

async function extractUrlsFromFiles(files: FileList | File[]): Promise<string[]> {
  const urls = new Set<string>()
  for (const file of Array.from(files)) {
    for (const url of await extractUrlsFromDroppedFile(file)) {
      urls.add(url)
    }
  }
  return [...urls]
}

async function extractUrlsFromPaths(paths: string[]): Promise<string[]> {
  try {
    const urls = await invoke<string[]>('extract_urls_from_paths', { paths })
    if (urls.length > 0) {
      return urls
    }
  } catch {
    // Fall back to frontend-side file reads below.
  }

  const urls = new Set<string>()
  for (const path of paths) {
    for (const url of await extractUrlsFromDroppedPath(path)) {
      urls.add(url)
    }
  }
  return [...urls]
}

async function extractDroppedUrls(dataTransfer: DataTransfer | null): Promise<string[]> {
  if (!dataTransfer) return []

  const urls = new Set<string>()
  const types = Array.from(dataTransfer.types ?? [])
  const urlLikeTypes = types.filter((type) =>
    /url|uri|plain|text|html|moz/i.test(type)
  )

  if (types.includes('text/uri-list')) {
    for (const url of extractUrlsFromText(dataTransfer.getData('text/uri-list'))) {
      urls.add(url)
    }
  }
  if (types.includes('text/plain')) {
    for (const url of extractUrlsFromText(dataTransfer.getData('text/plain'))) {
      urls.add(url)
    }
  }
  if (types.includes('text/html')) {
    for (const url of extractUrlsFromHtml(dataTransfer.getData('text/html'))) {
      urls.add(url)
    }
  }
  for (const type of urlLikeTypes) {
    const text = dataTransfer.getData(type)
    if (!text) continue
    for (const url of extractUrlsFromText(text)) {
      urls.add(url)
    }
    for (const url of extractUrlsFromHtml(text)) {
      urls.add(url)
    }
  }
  for (const text of await readDroppedStringItems(dataTransfer)) {
    for (const url of extractUrlsFromText(text)) {
      urls.add(url)
    }
    for (const url of extractUrlsFromHtml(text)) {
      urls.add(url)
    }
  }
  if (dataTransfer.files.length > 0) {
    for (const url of await extractUrlsFromFiles(dataTransfer.files)) {
      urls.add(url)
    }
  }

  return [...urls]
}

function openDroppedUrls(urls: string[]) {
  const uniqueUrls = Array.from(new Set(urls)).slice(0, 10)
  if (uniqueUrls.length === 0) return

  if (uniqueUrls.length === 1) {
    handleSubmitUrl(uniqueUrls[0])
    return
  }

  droppedBatchUrls.value = uniqueUrls
  showBatchDialog.value = true
}

function resetUrlDragState() {
  dragDepth = 0
  isDraggingUrl.value = false
}

function focusDropCapture() {
  nextTick(() => {
    dropTextCapture.value?.focus()
  })
}

function handleDragEnter(event: DragEvent) {
  event.preventDefault()
  dragDepth += 1
  isDraggingUrl.value = true
  focusDropCapture()
}

function handleDragOver(event: DragEvent) {
  event.preventDefault()
  isDraggingUrl.value = true
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'copy'
  }
}

function preventNativeDropNavigation(event: DragEvent) {
  event.preventDefault()
}

function handleDragLeave() {
  if (!isDraggingUrl.value) return
  dragDepth = Math.max(0, dragDepth - 1)
  if (dragDepth === 0) {
    resetUrlDragState()
  }
}

async function handleTextCaptureDrop(event: DragEvent) {
  const urls = await extractDroppedUrls(event.dataTransfer)
  if (urls.length > 0) {
    event.preventDefault()
    resetUrlDragState()
    openDroppedUrls(urls)
    return
  }

  setTimeout(() => {
    const fallbackText = dropTextCapture.value?.value ?? ''
    if (dropTextCapture.value) {
      dropTextCapture.value.value = ''
    }
    resetUrlDragState()
    openDroppedUrls(extractUrlsFromText(fallbackText))
  }, 0)
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
watch(currentSection, (section) => {
  if (section === 'schedules') {
    schedulesStore.markStartupChecksSeen()
  }
})

onMounted(async () => {
  await settingsStore.loadSettings()
  // Sync language from settings
  if (settingsStore.settings.language) {
    setLocale(settingsStore.settings.language as SupportedLocale)
  }
  applyTheme()
  await schedulesStore.setupScheduleListener()
  await downloadsStore.setupProgressListener(() => {
    libraryStore.loadItems()
  })
  await libraryStore.loadItems()
  document.addEventListener('keydown', handleKeydown)
  document.addEventListener('dragover', preventNativeDropNavigation, true)
  document.addEventListener('drop', preventNativeDropNavigation, true)
  window.addEventListener('dragenter', handleDragEnter)
  window.addEventListener('dragover', handleDragOver)
  window.addEventListener('dragleave', handleDragLeave)
  window.addEventListener('blur', resetUrlDragState)
  unlistenNativeDragDrop = await getCurrentWindow().onDragDropEvent(async (event) => {
    if (event.payload.type === 'enter' || event.payload.type === 'over') {
      isDraggingUrl.value = true
      focusDropCapture()
      return
    }
    if (event.payload.type === 'leave') {
      resetUrlDragState()
      return
    }
    resetUrlDragState()
    openDroppedUrls(await extractUrlsFromPaths(event.payload.paths))
  })
  darkModeQuery = window.matchMedia('(prefers-color-scheme: dark)')
  darkModeQuery.addEventListener('change', onDarkModeChange)
})

onUnmounted(() => {
  downloadsStore.cleanup()
  document.removeEventListener('keydown', handleKeydown)
  document.removeEventListener('dragover', preventNativeDropNavigation, true)
  document.removeEventListener('drop', preventNativeDropNavigation, true)
  window.removeEventListener('dragenter', handleDragEnter)
  window.removeEventListener('dragover', handleDragOver)
  window.removeEventListener('dragleave', handleDragLeave)
  window.removeEventListener('blur', resetUrlDragState)
  unlistenNativeDragDrop?.()
  unlistenNativeDragDrop = null
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
        :scheduleAttentionCount="schedulesStore.unseenStartupCheckIds.length"
        @update:currentSection="currentSection = $event"
        @open-channel-monitor="showChannelMonitorDialog = true"
      />

      <!-- Main Content -->
      <main class="flex-1 flex flex-col overflow-hidden bg-white dark:bg-neutral-900 relative" :style="backgroundStyle">
        <!-- Background overlay for readability -->
        <div v-if="hasBackground"
             class="absolute inset-0 bg-white dark:bg-neutral-900 pointer-events-none"
             :style="{ opacity: backgroundOverlayOpacity }" />
        <!-- Section header (for library views) -->
        <div v-if="isLibrarySection" class="flex items-center px-4 py-2 border-b border-[var(--color-separator)] relative z-10">
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

          <!-- Schedules -->
          <template v-else-if="currentSection === 'schedules'">
            <ScheduleView />
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
              <hr class="border-[var(--color-separator)]" />
              <PresetSettings />
            </div>
          </template>
        </div>
      </main>
    </div>

    <!-- Status Bar -->
    <AppStatusBar />

    <Teleport to="body">
      <Transition name="fade">
        <div
          v-if="isDraggingUrl"
          class="fixed inset-0 z-[9998] bg-black/10 backdrop-blur-[1px] flex items-center justify-center"
        >
          <textarea
            ref="dropTextCapture"
            class="absolute inset-0 z-0 opacity-0 pointer-events-auto resize-none"
            aria-hidden="true"
            tabindex="-1"
            @dragover.prevent
            @drop="handleTextCaptureDrop"
          />
          <div class="relative z-10 pointer-events-none px-6 py-4 rounded-2xl border border-[var(--color-accent)]/30 bg-white/90 dark:bg-neutral-900/90 shadow-2xl">
            <p class="text-base font-semibold text-neutral-900 dark:text-neutral-100">
              {{ t('toolbar.url_placeholder') }}
            </p>
          </div>
        </div>
      </Transition>
    </Teleport>

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
      :initial-urls="droppedBatchUrls"
      @close="closeBatchDialog"
      @start-batch="handleBatchDownload"
    />

    <!-- Channel Monitor Dialog -->
    <ChannelMonitorDialog
      :open="showChannelMonitorDialog"
      @close="showChannelMonitorDialog = false"
      @start="showChannelMonitorDialog = false; currentSection = 'schedules'"
    />
  </div>
</template>
