import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useSettingsStore } from './settings'
import type { Download, DownloadProgress, DownloadOptions, VideoInfo } from '../types'

export interface PlaylistItemInfo {
  url: string
  title: string | null
  channel: string | null
  channel_id: string | null
  channel_url: string | null
  site: string | null
  thumbnail_url: string | null
  duration: number | null
}

interface PendingItem {
  url: string
  options: DownloadOptions
  meta: PlaylistItemInfo | null
}

export const useDownloadsStore = defineStore('downloads', () => {
  const settingsStore = useSettingsStore()

  const queue = ref<Download[]>([])
  const progressMap = ref<Map<number, DownloadProgress>>(new Map())
  // Buffer for progress events that arrive before queue entry exists
  const pendingEvents = new Map<number, Array<DownloadProgress & { status?: string; title?: string }>>()

  // Items waiting to be dispatched (not yet sent to Rust / DB)
  const pendingQueue = ref<PendingItem[]>([])

  // Playlist fetch state
  const playlistFetching = ref(false)
  const playlistCancelled = ref(false)

  const activeDownloads = computed(() =>
    queue.value.filter(d => ['downloading', 'paused', 'pending'].includes(d.status))
  )
  const completedDownloads = computed(() =>
    queue.value.filter(d => d.status === 'completed')
  )

  // Count of items actively running (not paused) — used for concurrency limit
  const runningCount = computed(() =>
    queue.value.filter(d => d.status === 'downloading').length
  )

  async function fetchFormats(url: string): Promise<VideoInfo> {
    return invoke<VideoInfo>('fetch_formats', { url })
  }

  async function fetchPlaylistItems(url: string): Promise<PlaylistItemInfo[]> {
    playlistFetching.value = true
    playlistCancelled.value = false
    try {
      const items = await invoke<PlaylistItemInfo[]>('fetch_playlist_items', { url })
      if (playlistCancelled.value) return []
      return items
    } finally {
      playlistFetching.value = false
    }
  }

  function cancelPlaylistFetch() {
    playlistCancelled.value = true
    playlistFetching.value = false
  }

  async function startDownload(url: string, options: DownloadOptions): Promise<number> {
    if (options.playlist_mode === 'all') {
      return startPlaylistDownload(url, options)
    }
    return enqueue(url, options, null)
  }

  async function startPlaylistDownload(url: string, options: DownloadOptions): Promise<number> {
    const items = await fetchPlaylistItems(url)
    if (items.length === 0) return 0

    const singleOptions = { ...options, playlist_mode: 'single' as const }

    // Push all playlist items to pendingQueue, then dispatch available slots
    for (const item of items) {
      if (playlistCancelled.value) break
      pendingQueue.value.push({ url: item.url, options: singleOptions, meta: item })
    }

    dispatchPending()
    return 0
  }

  // Add to pendingQueue or dispatch immediately if slot available
  function enqueue(url: string, options: DownloadOptions, meta: PlaylistItemInfo | null): number {
    const limit = settingsStore.settings.concurrent_downloads
    if (runningCount.value < limit) {
      void dispatchOne({ url, options, meta })
    } else {
      pendingQueue.value.push({ url, options, meta })
    }
    return 0
  }

  // Dispatch a single item immediately (invoke Rust, add to queue)
  async function dispatchOne(item: PendingItem) {
    const { url, options, meta } = item
    try {
      const id = await invoke<number>('start_download', { url, options })
      addToQueue(
        id, url,
        meta?.title ?? null,
        meta?.channel ?? null,
        meta?.site ?? null,
        meta?.thumbnail_url ?? null,
        meta?.channel_id ?? null,
        options.format, options.quality,
        meta?.duration ?? null,
      )
      applyPendingEvents(id)
    } catch (e) {
      console.error(`[downloads] Failed to start download for ${meta?.title ?? url}:`, e)
    }
  }

  // Start pending items up to the concurrent limit
  function dispatchPending() {
    const limit = settingsStore.settings.concurrent_downloads
    const slots = limit - runningCount.value
    const toDispatch = pendingQueue.value.splice(0, Math.max(0, slots))
    for (const item of toDispatch) {
      void dispatchOne(item)
    }
  }

  function addToQueue(
    id: number, url: string, title: string | null,
    channel: string | null, site: string | null,
    thumbnail_url: string | null, channel_id: string | null,
    format: string, quality: string, duration: number | null,
  ) {
    if (queue.value.some(d => d.id === id)) return
    queue.value.push({
      id,
      url,
      title,
      channel,
      channel_id,
      channel_url: null,
      site,
      thumbnail_url,
      format,
      quality,
      file_path: null,
      file_size: null,
      bytes_downloaded: 0,
      duration,
      status: 'downloading',
      progress: 0,
      pid: null,
      error_message: null,
      metadata_json: null,
      created_at: new Date().toISOString(),
      completed_at: null,
      is_favorite: false,
    })
  }

  function applyPendingEvents(downloadId: number) {
    const events = pendingEvents.get(downloadId)
    if (!events) return
    pendingEvents.delete(downloadId)
    const item = queue.value.find(d => d.id === downloadId)
    if (!item) return
    for (const p of events) {
      applyProgressToItem(item, p)
    }
  }

  function applyProgressToItem(item: Download, p: DownloadProgress & { status?: string; title?: string; error_message?: string }) {
    item.progress = (p.percent ?? 0) / 100
    if (p.title) item.title = p.title
    if (p.status === 'completed') {
      item.status = 'completed'
      item.completed_at = new Date().toISOString()
      onCompletedCallback?.()
      dispatchPending()
    } else if (p.status === 'error') {
      item.status = 'error'
      if (p.error_message) item.error_message = p.error_message
      dispatchPending()
    } else if (p.status === 'paused') {
      item.status = 'paused'
    } else if (p.status === 'downloading') {
      item.status = 'downloading'
    }
  }

  async function cancelDownload(downloadId: number) {
    try {
      await invoke('cancel_download', { downloadId })
      // Free the slot for pending items
      dispatchPending()
    } catch (e) {
      console.error('[downloads] cancel_download failed:', e)
      throw e
    }
  }

  // Cancel a queued (not-yet-started) item by index
  function cancelQueued(index: number) {
    pendingQueue.value.splice(index, 1)
  }

  async function pauseDownload(downloadId: number) {
    try {
      await invoke('pause_download', { downloadId })
      const item = queue.value.find(d => d.id === downloadId)
      if (item) item.status = 'paused'
    } catch (e) {
      console.error('[downloads] pause_download failed:', e)
      throw e
    }
  }

  async function resumeDownload(downloadId: number) {
    try {
      await invoke('resume_download', { downloadId })
      const item = queue.value.find(d => d.id === downloadId)
      if (item) item.status = 'downloading'
    } catch (e) {
      console.error('[downloads] resume_download failed:', e)
      throw e
    }
  }

  let unlistenFn: (() => void) | null = null
  let onCompletedCallback: (() => void) | null = null

  async function setupProgressListener(onCompleted?: () => void) {
    onCompletedCallback = onCompleted ?? null
    if (unlistenFn) unlistenFn()
    unlistenFn = await listen<DownloadProgress & { status?: string; title?: string }>(
      'download-progress',
      (event) => {
        const p = event.payload
        progressMap.value.set(p.download_id, p)
        const item = queue.value.find(d => d.id === p.download_id)
        if (item) {
          applyProgressToItem(item, p)
        } else {
          if (!pendingEvents.has(p.download_id)) {
            pendingEvents.set(p.download_id, [])
          }
          pendingEvents.get(p.download_id)!.push(p)
        }
      },
    )
  }

  function cleanup() {
    if (unlistenFn) { unlistenFn(); unlistenFn = null }
  }

  return {
    queue,
    progressMap,
    pendingQueue,
    activeDownloads,
    completedDownloads,
    runningCount,
    playlistFetching,
    fetchFormats,
    fetchPlaylistItems,
    cancelPlaylistFetch,
    startDownload,
    cancelDownload,
    cancelQueued,
    pauseDownload,
    resumeDownload,
    setupProgressListener,
    cleanup,
  }
})
