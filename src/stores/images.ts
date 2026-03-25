import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
  ScrapedImage,
  ImageToDownload,
  ImageSession,
  ImageRecord,
  ImageDownloadProgress,
} from '../types'

export const useImagesStore = defineStore('images', () => {
  // State
  const scrapedImages = ref<ScrapedImage[]>([])
  const selectedIds = ref<Set<number>>(new Set())
  const sessions = ref<ImageSession[]>([])
  const sessionImages = ref<Map<number, ImageRecord[]>>(new Map())
  const downloadProgress = ref<ImageDownloadProgress | null>(null)
  const scraping = ref(false)
  const downloading = ref(false)
  const downloadCompleted = ref(false)
  const error = ref<string | null>(null)
  const currentPageUrl = ref('')

  // Computed
  const selectedCount = computed(() => selectedIds.value.size)
  const hasSelection = computed(() => selectedIds.value.size > 0)

  // Actions
  async function scrapeUrl(url: string, minWidth = 100, minHeight = 100) {
    scraping.value = true
    error.value = null
    downloadCompleted.value = false
    scrapedImages.value = []
    selectedIds.value = new Set()

    try {
      currentPageUrl.value = url
      const images = await invoke<ScrapedImage[]>('scrape_images', {
        url,
        minWidth,
        minHeight,
      })
      scrapedImages.value = images
      selectedIds.value = new Set(images.map((_: ScrapedImage, i: number) => i))
    } catch (e) {
      error.value = String(e)
    } finally {
      scraping.value = false
    }
  }

  async function startDownload(outputDir: string, format?: string) {
    downloading.value = true
    error.value = null

    const imagesToDownload: ImageToDownload[] = Array.from(selectedIds.value)
      .sort((a, b) => a - b)
      .map((index) => {
        const img = scrapedImages.value[index]
        const urlPath = new URL(img.url).pathname
        const filename = urlPath.split('/').pop() || null
        return { url: img.url, filename_hint: filename }
      })

    try {
      await invoke<number>('download_images', {
        images: imagesToDownload,
        outputDir,
        format: format || null,
        sessionUrl: currentPageUrl.value,
      })
      await loadSessions()
      downloadCompleted.value = true
    } catch (e) {
      error.value = String(e)
    } finally {
      downloading.value = false
      downloadProgress.value = null
    }
  }

  async function loadSessions() {
    try {
      sessions.value = await invoke<ImageSession[]>('list_image_sessions')
    } catch (e) {
      error.value = String(e)
    }
  }

  async function loadSessionImages(sessionId: number): Promise<ImageRecord[]> {
    try {
      const images = await invoke<ImageRecord[]>('list_session_images', { sessionId })
      sessionImages.value.set(sessionId, images)
      return images
    } catch (e) {
      error.value = String(e)
      return []
    }
  }

  async function deleteSession(sessionId: number, deleteFiles = false) {
    try {
      await invoke('delete_image_session', { sessionId, deleteFiles })
      sessions.value = sessions.value.filter((s: ImageSession) => s.id !== sessionId)
      sessionImages.value.delete(sessionId)
    } catch (e) {
      error.value = String(e)
    }
  }

  function toggleSelect(index: number) {
    const newSet = new Set(selectedIds.value)
    if (newSet.has(index)) {
      newSet.delete(index)
    } else {
      newSet.add(index)
    }
    selectedIds.value = newSet
  }

  function selectAll() {
    selectedIds.value = new Set(scrapedImages.value.map((_: ScrapedImage, i: number) => i))
  }

  function deselectAll() {
    selectedIds.value = new Set()
  }

  function resetScrape() {
    scrapedImages.value = []
    selectedIds.value = new Set()
    downloadCompleted.value = false
    downloadProgress.value = null
    downloading.value = false
    error.value = null
    currentPageUrl.value = ''
  }

  function setupProgressListener() {
    return listen<ImageDownloadProgress>('image-download-progress', (event) => {
      downloadProgress.value = event.payload
    })
  }

  return {
    scrapedImages,
    selectedIds,
    sessions,
    sessionImages,
    downloadProgress,
    scraping,
    downloading,
    downloadCompleted,
    error,
    selectedCount,
    hasSelection,
    currentPageUrl,
    scrapeUrl,
    startDownload,
    loadSessions,
    loadSessionImages,
    deleteSession,
    toggleSelect,
    selectAll,
    deselectAll,
    resetScrape,
    setupProgressListener,
  }
})
