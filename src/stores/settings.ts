import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { AppSettings, Setting } from '../types'

const DEFAULTS: AppSettings = {
  download_dir: '~/Downloads/YTDown/',
  filename_template: '%(title)s.%(ext)s',
  concurrent_downloads: 3,
  default_video_format: 'mp4',
  default_video_quality: 'best',
  default_audio_format: 'mp3',
  embed_thumbnail: true,
  embed_metadata: true,
  write_subs: false,
  embed_subs: false,
  embed_chapters: true,
  sponsorblock: false,
  cookie_browser: 'none',
  cookie_file: '',
  ytdlp_path: 'auto',
  theme: 'system',
  auto_classify: false,
}

const BOOLEAN_KEYS: (keyof AppSettings)[] = [
  'embed_thumbnail', 'embed_metadata', 'write_subs', 'embed_subs',
  'embed_chapters', 'sponsorblock', 'auto_classify',
]

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AppSettings>({ ...DEFAULTS })
  const loaded = ref(false)

  async function loadSettings() {
    try {
      const all = await invoke<Setting[]>('get_all_settings')
      for (const { key, value } of all) {
        if (key in settings.value) {
          const k = key as keyof AppSettings
          if (BOOLEAN_KEYS.includes(k)) {
            (settings.value as Record<string, unknown>)[k] = value === 'true'
          } else if (k === 'concurrent_downloads') {
            settings.value.concurrent_downloads = parseInt(value) || 3
          } else {
            (settings.value as Record<string, unknown>)[k] = value
          }
        }
      }
      loaded.value = true
    } catch (e) {
      console.error('Failed to load settings:', e)
    }
  }

  async function updateSetting<K extends keyof AppSettings>(key: K, value: AppSettings[K]) {
    settings.value[key] = value
    try {
      await invoke('set_setting', { key, value: String(value) })
    } catch (e) {
      console.error('Failed to save setting:', e)
    }
  }

  return { settings, loaded, loadSettings, updateSetting }
})
