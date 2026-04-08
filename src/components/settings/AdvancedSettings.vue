<script setup lang="ts">
import { useSettingsStore } from '../../stores/settings'
import { useYtdlp } from '../../composables/useYtdlp'
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const settingsStore = useSettingsStore()

const subFormats = ['', 'srt', 'ass', 'vtt', 'lrc']
const videoFormats = ['', 'mp4', 'mkv', 'webm', 'flv', 'avi']

const { info: ytdlpInfo, loading, checking, updating, error: ytdlpError, loadInfo, checkUpdate, performUpdate } = useYtdlp()

onMounted(() => { void loadInfo() })
</script>

<template>
  <div class="space-y-6">
    <h3 class="text-base font-semibold">{{ t('advanced.title') }}</h3>

    <!-- yt-dlp version management -->
    <div class="p-3 rounded-lg border border-[var(--color-separator)] space-y-3">
      <h4 class="text-sm font-medium">yt-dlp</h4>

      <div v-if="loading" class="text-xs text-neutral-400">{{ t('common.loading') }}</div>

      <template v-else-if="ytdlpInfo">
        <div class="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
          <span class="text-neutral-500">{{ t('general.ytdlp_version') }}</span>
          <span class="font-mono">{{ ytdlpInfo.version }}</span>
          <span class="text-neutral-500">{{ t('general.ytdlp_path_label') }}</span>
          <span class="font-mono truncate" :title="ytdlpInfo.path">{{ ytdlpInfo.path }}</span>
          <span class="text-neutral-500">{{ t('general.ytdlp_managed') }}</span>
          <span>{{ ytdlpInfo.managed_by === 'homebrew' ? t('general.ytdlp_homebrew') : ytdlpInfo.managed_by === 'bundled' ? t('general.ytdlp_bundled') : t('general.ytdlp_manual') }}</span>
          <template v-if="ytdlpInfo.latest_version">
            <span class="text-neutral-500">{{ t('general.ytdlp_update_available') }}</span>
            <span class="font-mono text-orange-500">{{ ytdlpInfo.latest_version }}</span>
          </template>
        </div>

        <!-- Update available banner -->
        <div v-if="ytdlpInfo.update_available"
             class="px-3 py-2 rounded-md bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-800 text-xs text-orange-700 dark:text-orange-300">
          <template v-if="ytdlpInfo.managed_by === 'homebrew'">
            {{ ytdlpInfo.latest_version }} —
            <code class="font-mono bg-orange-100 dark:bg-orange-900/40 px-1 rounded">brew upgrade yt-dlp</code>
          </template>
          <template v-else>
            {{ ytdlpInfo.latest_version }}
          </template>
        </div>

        <p v-if="ytdlpError" class="text-xs text-red-500">{{ ytdlpError }}</p>

        <div class="flex gap-2">
          <button @click="checkUpdate"
                  :disabled="checking"
                  class="px-3 py-1.5 text-xs rounded-md bg-neutral-100 dark:bg-neutral-700 text-neutral-700 dark:text-neutral-100 hover:bg-neutral-200 dark:hover:bg-neutral-600 disabled:opacity-50 transition-colors">
            {{ checking ? t('common.loading') : t('general.ytdlp_check_update') }}
          </button>
          <button v-if="ytdlpInfo.update_available && ytdlpInfo.managed_by === 'bundled'"
                  @click="performUpdate"
                  :disabled="updating"
                  class="px-3 py-1.5 text-xs rounded-md bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50 transition-colors">
            {{ updating ? t('common.loading') : t('common.ok') }}
          </button>
        </div>

        <p v-if="!ytdlpInfo.update_available && ytdlpInfo.latest_version !== null"
           class="text-xs text-green-600 dark:text-green-400">
          ✓ {{ t('general.ytdlp_version') }}
        </p>
      </template>

      <div v-else class="text-xs text-red-400">{{ t('general.ytdlp_not_found') }}</div>
    </div>

    <!-- yt-dlp path override -->
    <div>
      <label class="block text-sm font-medium mb-1">{{ t('general.ytdlp_path_override') }}</label>
      <input :value="settingsStore.settings.ytdlp_path"
             @input="settingsStore.updateSetting('ytdlp_path', ($event.target as HTMLInputElement).value)"
             class="w-full h-8 px-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm font-mono outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
             :placeholder="t('general.ytdlp_path_placeholder')" />
      <p class="text-xs text-neutral-400 mt-1">{{ t('general.ytdlp_path_hint') }}</p>
    </div>

    <!-- Boolean options -->
    <div class="space-y-3">
      <label class="flex items-center gap-3 text-sm">
        <input type="checkbox" :checked="settingsStore.settings.restrict_filenames"
               @change="settingsStore.updateSetting('restrict_filenames', ($event.target as HTMLInputElement).checked)"
               class="rounded" />
        {{ t('advanced.restrict_filenames') }}
        <span class="text-xs text-neutral-400">--restrict-filenames</span>
      </label>

      <label class="flex items-center gap-3 text-sm">
        <input type="checkbox" :checked="settingsStore.settings.no_overwrites"
               @change="settingsStore.updateSetting('no_overwrites', ($event.target as HTMLInputElement).checked)"
               class="rounded" />
        {{ t('advanced.no_overwrites') }}
        <span class="text-xs text-neutral-400">--no-overwrites</span>
      </label>

      <label class="flex items-center gap-3 text-sm">
        <input type="checkbox" :checked="settingsStore.settings.geo_bypass"
               @change="settingsStore.updateSetting('geo_bypass', ($event.target as HTMLInputElement).checked)"
               class="rounded" />
        {{ t('advanced.geo_bypass') }}
        <span class="text-xs text-neutral-400">--geo-bypass</span>
      </label>
    </div>

    <!-- Video/Subtitle options -->
    <div class="space-y-4">
      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-xs text-neutral-500 mb-1">{{ t('advanced.merge_output_format') }} <span class="text-neutral-400">--merge-output-format</span></label>
          <select :value="settingsStore.settings.merge_output_format"
                  @change="settingsStore.updateSetting('merge_output_format', ($event.target as HTMLSelectElement).value)"
                  class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm">
            <option value="">{{ t('common.none') }}</option>
            <option v-for="f in videoFormats.slice(1)" :key="f" :value="f">{{ f.toUpperCase() }}</option>
          </select>
        </div>
        <div>
          <label class="block text-xs text-neutral-500 mb-1">{{ t('advanced.recode_video') }} <span class="text-neutral-400">--recode-video</span></label>
          <select :value="settingsStore.settings.recode_video"
                  @change="settingsStore.updateSetting('recode_video', ($event.target as HTMLSelectElement).value)"
                  class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm">
            <option value="">{{ t('common.none') }}</option>
            <option v-for="f in videoFormats.slice(1)" :key="f" :value="f">{{ f.toUpperCase() }}</option>
          </select>
        </div>
      </div>

      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-xs text-neutral-500 mb-1">{{ t('advanced.sub_lang') }} <span class="text-neutral-400">--sub-lang</span></label>
          <input :value="settingsStore.settings.sub_lang"
                 @input="settingsStore.updateSetting('sub_lang', ($event.target as HTMLInputElement).value)"
                 class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm font-mono"
                 :placeholder="t('advanced.sub_lang_placeholder')" />
        </div>
        <div>
          <label class="block text-xs text-neutral-500 mb-1">{{ t('advanced.convert_subs') }} <span class="text-neutral-400">--convert-subs</span></label>
          <select :value="settingsStore.settings.convert_subs"
                  @change="settingsStore.updateSetting('convert_subs', ($event.target as HTMLSelectElement).value)"
                  class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm">
            <option v-for="f in subFormats" :key="f" :value="f">{{ f || t('common.none') }}</option>
          </select>
        </div>
      </div>
    </div>

    <!-- Network options -->
    <div class="space-y-4">
      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-xs text-neutral-500 mb-1">{{ t('advanced.rate_limit') }} <span class="text-neutral-400">-r</span></label>
          <input :value="settingsStore.settings.rate_limit"
                 @input="settingsStore.updateSetting('rate_limit', ($event.target as HTMLInputElement).value)"
                 class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm font-mono"
                 :placeholder="t('advanced.rate_limit_placeholder')" />
        </div>
        <div>
          <label class="block text-xs text-neutral-500 mb-1">{{ t('advanced.retries') }} <span class="text-neutral-400">--retries</span></label>
          <input type="number" :value="settingsStore.settings.retries"
                 @input="settingsStore.updateSetting('retries', parseInt(($event.target as HTMLInputElement).value) || 10)"
                 class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm"
                 min="0" max="100" />
        </div>
      </div>

      <div>
        <label class="block text-xs text-neutral-500 mb-1">{{ t('advanced.proxy') }} <span class="text-neutral-400">--proxy</span></label>
        <input :value="settingsStore.settings.proxy"
               @input="settingsStore.updateSetting('proxy', ($event.target as HTMLInputElement).value)"
               class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm font-mono"
               :placeholder="t('advanced.proxy_placeholder')" />
      </div>
    </div>

    <!-- Extra args -->
    <div>
      <label class="block text-xs text-neutral-500 mb-1">{{ t('advanced.extra_args') }}</label>
      <input :value="settingsStore.settings.extra_args"
             @input="settingsStore.updateSetting('extra_args', ($event.target as HTMLInputElement).value)"
             class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm font-mono"
             :placeholder="t('advanced.extra_args_placeholder')" />
    </div>
  </div>
</template>
