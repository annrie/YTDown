<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useSchedulesStore } from '../../stores/schedules'
import { usePresetsStore } from '../../stores/presets'
import { useSettingsStore } from '../../stores/settings'
import { useDownload } from '../../composables/useDownload'
import { useI18n } from 'vue-i18n'
import LoadingSpinner from '../common/LoadingSpinner.vue'
import ErrorAlert from '../common/ErrorAlert.vue'

const { t } = useI18n()

const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  close: []
  start: []
}>()

const schedulesStore = useSchedulesStore()
const presetsStore = usePresetsStore()
const settingsStore = useSettingsStore()

const url = ref('')
const name = ref('')
const frequency = ref<'1h' | '6h' | '12h' | 'daily'>('daily')
const dailyTime = ref('22:00')
const selectedPresetId = ref<number | ''>('')
const skipInitial = ref(true)

const { videoInfo, loading, error, fetchChannelInfo } = useDownload()

let fetchTimeout: number | undefined
watch(url, (val) => {
  if (fetchTimeout) clearTimeout(fetchTimeout)
  if (val.trim() && val.startsWith('http')) {
    fetchTimeout = window.setTimeout(() => {
      fetchChannelInfo(val.trim())
    }, 800)
  } else {
    videoInfo.value = null
  }
})

watch(videoInfo, (info) => {
  if (info && !name.value.trim()) {
    name.value = info.channel || info.title || ''
  }
})

watch(() => props.open, (isOpen) => {
  if (isOpen) {
    url.value = ''
    name.value = ''
    frequency.value = 'daily'
    dailyTime.value = '22:00'
    selectedPresetId.value = ''
    skipInitial.value = true
    videoInfo.value = null
    presetsStore.fetchPresets()
  }
})

function generateCron(): string {
  switch (frequency.value) {
    case '1h': return '0 * * * *'
    case '6h': return '0 */6 * * *'
    case '12h': return '0 */12 * * *'
    case 'daily': {
      const parts = dailyTime.value.split(':')
      const h = Number(parts[0]) || 0
      const m = Number(parts[1]) || 0
      return `${m} ${h} * * *`
    }
  }
}

const isValid = computed(() => {
  return url.value.trim().length > 0
})

const channelAvatarUrl = computed(() =>
  videoInfo.value?.channel_avatar_url || videoInfo.value?.thumbnail_url || ''
)

const channelDisplayName = computed(() =>
  videoInfo.value?.channel || videoInfo.value?.title || ''
)

const channelDisplayId = computed(() =>
  videoInfo.value?.channel_id ? `@${videoInfo.value.channel_id}` : ''
)

async function handleSubmit() {
  if (!isValid.value) return

  const preset = selectedPresetId.value
    ? presetsStore.presets.find(p => p.id === selectedPresetId.value)
    : null

  const defaultOptions = {
    format: settingsStore.settings?.default_video_format || 'mp4',
    quality: settingsStore.settings?.default_video_quality || 'best',
    output_dir: preset?.output_dir || settingsStore.settings?.download_dir || '~/Downloads/YTDown/',
    embed_thumbnail: preset?.embed_thumbnail ?? settingsStore.settings?.embed_thumbnail ?? true,
    embed_metadata: preset?.embed_metadata ?? settingsStore.settings?.embed_metadata ?? true,
    write_subs: preset?.write_subs ?? settingsStore.settings?.write_subs ?? false,
    embed_subs: preset?.embed_subs ?? settingsStore.settings?.embed_subs ?? false,
    embed_chapters: preset?.embed_chapters ?? settingsStore.settings?.embed_chapters ?? true,
    sponsorblock: preset?.sponsorblock ?? settingsStore.settings?.sponsorblock ?? false,
    playlist_mode: 'single',
    skip_initial: skipInitial.value,
    avatar_url: channelAvatarUrl.value,
    channel_id: videoInfo.value?.channel_id || '',
  } as any

  const finalName = name.value.trim() || t('channel_monitor.default_name')

  try {
    await schedulesStore.createSchedule({
      name: finalName,
      url: url.value.trim(),
      cron_expr: generateCron(),
      options_json: JSON.stringify(defaultOptions),
      is_channel: true,
    })
    emit('start')
  } catch (e) {
    console.error('Failed to create channel monitor:', e)
    alert(`${t('channel_monitor.error_start')}: ${e}`)
  }
}

function handleOverlayClick(e: MouseEvent) {
  if (e.target === e.currentTarget) {
    emit('close')
  }
}
</script>

<template>
  <div v-if="props.open" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" @click="handleOverlayClick">
    <div class="bg-white dark:bg-neutral-800 rounded-xl shadow-2xl w-[480px] max-w-[90vw] flex flex-col">
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-b border-[var(--color-separator)]">
        <h2 class="text-base font-semibold">{{ t('channel_monitor.title') }}</h2>
        <button class="w-8 h-8 flex items-center justify-center rounded-full hover:bg-neutral-100 dark:hover:bg-neutral-700 transition" @click="emit('close')">
          <svg class="w-5 h-5 text-neutral-500" viewBox="0 0 20 20" fill="currentColor">
            <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
          </svg>
        </button>
      </div>

      <!-- Body -->
      <div class="p-5 flex flex-col gap-4">
        <p class="text-xs text-neutral-500 leading-relaxed mb-1">
          {{ t('channel_monitor.description') }}
        </p>

        <!-- URL Input -->
        <div>
          <label class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 mb-1">{{ t('channel_monitor.url') }} <span class="text-red-500">*</span></label>
          <div class="relative">
            <input
              v-model="url"
              type="url"
              :placeholder="t('channel_monitor.url_placeholder')"
              class="w-full h-9 px-3 rounded-md bg-neutral-100 dark:bg-neutral-900 border border-transparent focus:border-[var(--color-accent)] focus:ring-1 focus:ring-[var(--color-accent)] outline-none text-sm transition-all"
              autofocus
            />
            <div v-if="loading" class="absolute right-3 top-2.5 flex items-center justify-center">
              <LoadingSpinner size="sm" />
            </div>
          </div>
          <ErrorAlert v-if="error" :message="error" class="mt-1" />
        </div>

        <!-- Frequency -->
        <div>
          <label class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 mb-1">{{ t('channel_monitor.interval') }}</label>
          <div class="flex items-center gap-3">
            <select v-model="frequency" class="h-9 px-2 rounded-md bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-700 text-sm outline-none focus:border-[var(--color-accent)]">
              <option value="1h">{{ t('channel_monitor.freq_1h') }}</option>
              <option value="6h">{{ t('channel_monitor.freq_6h') }}</option>
              <option value="12h">{{ t('channel_monitor.freq_12h') }}</option>
              <option value="daily">{{ t('channel_monitor.freq_daily') }}</option>
            </select>

            <input
              v-if="frequency === 'daily'"
              v-model="dailyTime"
              type="time"
              class="h-9 px-2 rounded-md bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-700 text-sm outline-none focus:border-[var(--color-accent)]"
            />
          </div>
        </div>

        <!-- Preset -->
        <div>
          <label class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 mb-1">{{ t('channel_monitor.preset') }}</label>
          <select
            v-model="selectedPresetId"
            class="w-full h-9 px-2 rounded-md bg-neutral-100 dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-700 text-sm outline-none focus:border-[var(--color-accent)]"
          >
            <option value="">{{ t('channel_monitor.preset_global') }}</option>
            <option v-for="preset in presetsStore.presets" :key="preset.id" :value="preset.id">
              {{ preset.name }}
            </option>
          </select>
        </div>

        <!-- Skip initial -->
        <label class="flex items-start gap-3 cursor-pointer select-none">
          <input type="checkbox" v-model="skipInitial" class="mt-0.5 w-4 h-4 rounded accent-[var(--color-accent)]" />
          <div>
            <span class="text-sm font-medium">{{ t('channel_monitor.skip_initial') }}</span>
            <p class="text-xs text-neutral-500 mt-0.5">{{ t('channel_monitor.skip_initial_hint') }}</p>
          </div>
        </label>

        <!-- Name (Optional) -->
        <div>
          <label class="block text-xs font-semibold text-neutral-600 dark:text-neutral-400 mb-1">{{ t('channel_monitor.name_optional') }}</label>
          <input
            v-model="name"
            type="text"
            :placeholder="t('channel_monitor.name_placeholder')"
            class="w-full h-9 px-3 rounded-md bg-neutral-100 dark:bg-neutral-900 border border-transparent focus:border-[var(--color-accent)] focus:ring-1 focus:ring-[var(--color-accent)] outline-none text-sm transition-all"
          />
          <!-- Avatar Preview -->
          <div v-if="channelAvatarUrl" class="mt-3 flex items-center gap-3 p-2 bg-neutral-100 dark:bg-neutral-900 rounded-md border border-[var(--color-separator)]">
            <img :src="channelAvatarUrl" class="w-10 h-10 rounded-full object-cover bg-neutral-200" />
            <div class="flex flex-col">
              <span class="text-sm font-semibold">{{ channelDisplayName }}</span>
              <span class="text-xs text-neutral-500">{{ channelDisplayId }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="p-4 border-t border-[var(--color-separator)] flex items-center justify-end gap-3 bg-neutral-50 dark:bg-neutral-800/50 rounded-b-xl">
        <button
          class="px-4 py-2 rounded-md text-sm font-medium text-neutral-600 dark:text-neutral-300 hover:bg-neutral-200 dark:hover:bg-neutral-700 transition"
          @click="emit('close')"
        >
          {{ t('common.cancel') }}
        </button>
        <button
          class="px-5 py-2 rounded-md text-sm font-medium bg-[var(--color-accent)] text-white hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition"
          :disabled="!isValid"
          @click="handleSubmit"
        >
          {{ t('channel_monitor.start') }}
        </button>
      </div>
    </div>
  </div>
</template>
