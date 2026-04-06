<script setup lang="ts">
import { useSettingsStore } from '../../stores/settings'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const settingsStore = useSettingsStore()

const videoFormats = ['mp4', 'mkv', 'webm']
const audioFormats = ['mp3', 'm4a', 'flac', 'wav', 'opus']
const qualities = ['best', '2160', '1080', '720', '480']
</script>

<template>
  <div class="space-y-6">
    <h3 class="text-base font-semibold">{{ t('format.title') }}</h3>

    <!-- Default video format -->
    <div class="grid grid-cols-2 gap-4">
      <div>
        <label class="block text-sm font-medium mb-1">{{ t('format.default_video_format') }}</label>
        <select :value="settingsStore.settings.default_video_format"
                @change="settingsStore.updateSetting('default_video_format', ($event.target as HTMLSelectElement).value)"
                class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm">
          <option v-for="f in videoFormats" :key="f" :value="f">{{ f.toUpperCase() }}</option>
        </select>
      </div>
      <div>
        <label class="block text-sm font-medium mb-1">{{ t('format.default_video_quality') }}</label>
        <select :value="settingsStore.settings.default_video_quality"
                @change="settingsStore.updateSetting('default_video_quality', ($event.target as HTMLSelectElement).value)"
                class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm">
          <option v-for="q in qualities" :key="q" :value="q">
            {{ q === 'best' ? t('download_dialog.best') : q + 'p' }}
          </option>
        </select>
      </div>
    </div>

    <!-- Default audio format -->
    <div>
      <label class="block text-sm font-medium mb-1">{{ t('format.default_audio_format') }}</label>
      <select :value="settingsStore.settings.default_audio_format"
              @change="settingsStore.updateSetting('default_audio_format', ($event.target as HTMLSelectElement).value)"
              class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm">
        <option v-for="f in audioFormats" :key="f" :value="f">{{ f.toUpperCase() }}</option>
      </select>
    </div>

    <!-- Post-process defaults -->
    <div class="space-y-3">
      <label class="flex items-center gap-3 text-sm">
        <input type="checkbox" :checked="settingsStore.settings.embed_thumbnail"
               @change="settingsStore.updateSetting('embed_thumbnail', ($event.target as HTMLInputElement).checked)"
               class="rounded" />
        {{ t('format.embed_thumbnail') }}
      </label>

      <label class="flex items-center gap-3 text-sm">
        <input type="checkbox" :checked="settingsStore.settings.embed_metadata"
               @change="settingsStore.updateSetting('embed_metadata', ($event.target as HTMLInputElement).checked)"
               class="rounded" />
        {{ t('format.embed_metadata') }}
      </label>

      <label class="flex items-center gap-3 text-sm">
        <input type="checkbox" :checked="settingsStore.settings.write_subs"
               @change="settingsStore.updateSetting('write_subs', ($event.target as HTMLInputElement).checked)"
               class="rounded" />
        {{ t('format.write_subs') }}
      </label>

      <label class="flex items-center gap-3 text-sm">
        <input type="checkbox" :checked="settingsStore.settings.embed_subs"
               @change="settingsStore.updateSetting('embed_subs', ($event.target as HTMLInputElement).checked)"
               class="rounded" />
        {{ t('format.embed_subs') }}
      </label>

      <label class="flex items-center gap-3 text-sm">
        <input type="checkbox" :checked="settingsStore.settings.embed_chapters"
               @change="settingsStore.updateSetting('embed_chapters', ($event.target as HTMLInputElement).checked)"
               class="rounded" />
        {{ t('format.embed_chapters') }}
      </label>

      <label class="flex items-center gap-3 text-sm">
        <input type="checkbox" :checked="settingsStore.settings.sponsorblock"
               @change="settingsStore.updateSetting('sponsorblock', ($event.target as HTMLInputElement).checked)"
               class="rounded" />
        {{ t('format.sponsorblock') }}
      </label>
    </div>
  </div>
</template>
