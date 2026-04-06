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
  language: 'ja',
  auto_classify: false,
  background_image_light: '',
  background_image_dark: '',
  background_opacity: 30,
  restrict_filenames: false,
  no_overwrites: false,
  geo_bypass: false,
  rate_limit: '',
  sub_lang: '',
  convert_subs: '',
  merge_output_format: '',
  recode_video: '',
  retries: 10,
  proxy: '',
  extra_args: '',
}

const BOOLEAN_KEYS: (keyof AppSettings)[] = [
  'embed_thumbnail', 'embed_metadata', 'write_subs', 'embed_subs',
  'embed_chapters', 'sponsorblock', 'auto_classify',
  'restrict_filenames', 'no_overwrites', 'geo_bypass',
]

const INTEGER_KEYS: (keyof AppSettings)[] = [
  'concurrent_downloads', 'retries', 'background_opacity',
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
          const defaults = DEFAULTS as unknown as Record<string, unknown>
          if (BOOLEAN_KEYS.includes(k)) {
            Object.assign(settings.value, { [k]: value === 'true' })
          } else if (INTEGER_KEYS.includes(k)) {
            Object.assign(settings.value, { [k]: parseInt(value) || defaults[k] })
          } else {
            Object.assign(settings.value, { [k]: value })
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
