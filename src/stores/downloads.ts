import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { Download, DownloadProgress, DownloadOptions, VideoInfo } from '../types'

export const useDownloadsStore = defineStore('downloads', () => {
  const queue = ref<Download[]>([])
  const progressMap = ref<Map<number, DownloadProgress>>(new Map())
  // Buffer for progress events that arrive before queue entry exists
  const pendingEvents = new Map<number, Array<DownloadProgress & { status?: string; title?: string }>>()

  const activeDownloads = computed(() =>
    queue.value.filter(d => ['downloading', 'paused', 'pending'].includes(d.status))
  )
  const completedDownloads = computed(() =>
    queue.value.filter(d => d.status === 'completed')
  )

  async function fetchFormats(url: string): Promise<VideoInfo> {
    return invoke<VideoInfo>('fetch_formats', { url })
  }

  async function startDownload(url: string, options: DownloadOptions): Promise<number> {
    if (options.playlist_mode === 'all') {
      return startPlaylistDownload(url, options)
    }
    const id = await invoke<number>('start_download', { url, options })
    addToQueue(id, url, null, null, null, null, null, options.format, options.quality, null)
    return id
  }

  interface PlaylistItemInfo {
    url: string
    title: string | null
    channel: string | null
    channel_id: string | null
    channel_url: string | null
    site: string | null
    thumbnail_url: string | null
    duration: number | null
  }

  async function startPlaylistDownload(url: string, options: DownloadOptions): Promise<number> {
    // Step 1: Fetch playlist items (metadata only)
    const items = await invoke<PlaylistItemInfo[]>('fetch_playlist_items', { url })

    const singleOptions = { ...options, playlist_mode: 'single' as const }
    let firstId = 0

    // Step 2: Start each item sequentially
    for (const item of items) {
      try {
        const id = await invoke<number>('start_download', { url: item.url, options: singleOptions })
        addToQueue(
          id, item.url, item.title, item.channel, item.site,
          item.thumbnail_url, item.channel_id,
          options.format, options.quality, item.duration,
        )
        applyPendingEvents(id)
        if (!firstId) firstId = id
      } catch (e) {
        console.error(`Failed to start download for ${item.title ?? item.url}:`, e)
      }
    }
    return firstId
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

  function applyProgressToItem(item: Download, p: DownloadProgress & { status?: string; title?: string }) {
    item.progress = (p.percent ?? 0) / 100
    if (p.title) item.title = p.title
    if (p.status === 'completed') {
      item.status = 'completed'
      item.completed_at = new Date().toISOString()
      onCompletedCallback?.()
    } else if (p.status === 'error') {
      item.status = 'error'
    } else if (p.status === 'paused') {
      item.status = 'paused'
    } else if (p.status === 'downloading') {
      item.status = 'downloading'
    }
  }

  async function cancelDownload(downloadId: number) {
    await invoke('cancel_download', { downloadId })
  }

  async function pauseDownload(downloadId: number) {
    await invoke('pause_download', { downloadId })
    const item = queue.value.find(d => d.id === downloadId)
    if (item) item.status = 'paused'
  }

  async function resumeDownload(downloadId: number) {
    await invoke('resume_download', { downloadId })
    const item = queue.value.find(d => d.id === downloadId)
    if (item) item.status = 'downloading'
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
    activeDownloads,
    completedDownloads,
    fetchFormats,
    startDownload,
    cancelDownload,
    pauseDownload,
    resumeDownload,
    setupProgressListener,
    cleanup,
  }
})
