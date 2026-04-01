import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { VideoInfo, DownloadOptions } from '../types'

export function useDownload() {
  const videoInfo = ref<VideoInfo | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchChannelInfo(url: string) {
    loading.value = true
    error.value = null
    try {
      videoInfo.value = await invoke<VideoInfo>('fetch_channel_info', { url })
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function fetchFormats(url: string) {
    loading.value = true
    error.value = null
    try {
      videoInfo.value = await invoke<VideoInfo>('fetch_formats', { url })
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function startDownload(url: string, options: DownloadOptions): Promise<number> {
    return invoke<number>('start_download', { url, options })
  }

  return { videoInfo, loading, error, fetchFormats, fetchChannelInfo, startDownload }
}
