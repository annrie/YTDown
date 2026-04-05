import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { YtdlpInfo } from '../types'

export function useYtdlp() {
  const info = ref<YtdlpInfo | null>(null)
  const loading = ref(false)
  const checking = ref(false)
  const updating = ref(false)
  const error = ref<string | null>(null)

  async function loadInfo() {
    loading.value = true
    error.value = null
    try {
      info.value = await invoke<YtdlpInfo>('get_ytdlp_info')
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function checkUpdate() {
    checking.value = true
    error.value = null
    try {
      const latestVersion = await invoke<string | null>('check_ytdlp_update')
      if (info.value) {
        info.value.update_available = latestVersion !== null
        info.value.latest_version = latestVersion
      }
      return latestVersion
    } catch (e) {
      // Silently ignore update check failures (network may be unavailable)
      return null
    } finally {
      checking.value = false
    }
  }

  async function performUpdate() {
    updating.value = true
    error.value = null
    try {
      const newVersion = await invoke<string>('update_ytdlp')
      if (info.value) {
        info.value.version = newVersion
        info.value.update_available = false
        info.value.latest_version = null
      }
      return newVersion
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      updating.value = false
    }
  }

  return { info, loading, checking, updating, error, loadInfo, checkUpdate, performUpdate }
}
