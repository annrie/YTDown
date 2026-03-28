<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useDownload } from '../../composables/useDownload'
import { useDownloadsStore, type PlaylistItemInfo } from '../../stores/downloads'
import { useSettingsStore } from '../../stores/settings'
import { useSchedulesStore } from '../../stores/schedules'
import { Cron } from 'croner'
import type { DownloadOptions, PlaylistMode } from '../../types'

const props = defineProps<{ url: string; open: boolean }>()
const emit = defineEmits<{
  close: []
  start: [url: string, options: DownloadOptions]
}>()

const { videoInfo, loading, error, fetchFormats } = useDownload()
const downloadsStore = useDownloadsStore()
const settingsStore = useSettingsStore()
const schedulesStore = useSchedulesStore()

const showScheduleMode = ref(false)

// Inline schedule form state
const scheduleName = ref('')
const scheduleCronExpr = ref('0 9 * * *')
const scheduleIsChannel = ref(false)

const scheduleCronError = computed(() => {
  try { new Cron(scheduleCronExpr.value); return '' } catch { return '無効なcron式です' }
})

const scheduleNextRun = computed(() => {
  try {
    const job = new Cron(scheduleCronExpr.value)
    return job.nextRun()?.toLocaleString('ja-JP') ?? ''
  } catch { return '' }
})

const isScheduleValid = computed(() =>
  scheduleName.value.trim() !== '' && scheduleCronError.value === ''
)

watch(showScheduleMode, (enabled) => {
  if (enabled && !scheduleName.value) {
    try { scheduleName.value = new URL(props.url).hostname } catch { scheduleName.value = props.url }
  }
})

function handleScheduleRegister() {
  const s = settingsStore.settings
  const options: DownloadOptions = {
    format: selectedFormat.value,
    quality: selectedQuality.value,
    output_dir: s.download_dir,
    embed_thumbnail: embedThumbnail.value,
    embed_metadata: embedMetadata.value,
    write_subs: writeSubs.value,
    embed_subs: embedSubs.value,
    embed_chapters: embedChapters.value,
    sponsorblock: sponsorblock.value,
    custom_format: useCustomFormat.value ? customFormat.value : null,
    playlist_mode: isPlaylistUrl.value ? playlistMode.value : 'single',
    restrict_filenames: s.restrict_filenames,
    no_overwrites: s.no_overwrites,
    geo_bypass: s.geo_bypass,
    rate_limit: s.rate_limit,
    sub_lang: s.sub_lang,
    convert_subs: s.convert_subs,
    merge_output_format: s.merge_output_format,
    recode_video: s.recode_video,
    retries: s.retries,
    proxy: s.proxy,
    extra_args: s.extra_args,
  }
  emit('close')
  schedulesStore.createSchedule({
    name: scheduleName.value.trim(),
    url: props.url,
    cron_expr: scheduleCronExpr.value.trim(),
    options_json: JSON.stringify(options),
    is_channel: scheduleIsChannel.value,
  }).catch(e => console.error('スケジュール登録失敗:', e))
}

const installing = ref(false)

async function handleInstallYtdlp() {
  installing.value = true
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('install_ytdlp')
    // Retry fetching formats after install
    fetchFormats(props.url)
  } catch (e) {
    error.value = `インストール失敗: ${e}`
  } finally {
    installing.value = false
  }
}

const isYtdlpMissing = computed(() =>
  !!error.value && (error.value.includes('not found') || error.value.includes('見つかりません'))
)

const mediaType = ref<'video' | 'audio'>('video')
const selectedFormat = ref('mp4')
const selectedQuality = ref('best')
const embedThumbnail = ref(true)
const embedMetadata = ref(true)
const writeSubs = ref(false)
const embedSubs = ref(false)
const embedChapters = ref(true)
const sponsorblock = ref(false)
const customFormat = ref('')
const useCustomFormat = ref(false)
const playlistMode = ref<PlaylistMode>('single')

// Playlist preview state
const playlistItems = ref<PlaylistItemInfo[]>([])
const playlistFetchError = ref<string | null>(null)
const playlistPreviewLoaded = ref(false)

const isPlaylistUrl = computed(() => {
  const u = props.url.toLowerCase()
  return u.includes('list=') || u.includes('/playlist') || u.includes('/sets/') || u.includes('/album/')
})

/** Detect channel upload lists (list=UU...) which contain ALL channel videos */
const isChannelUploadList = computed(() => {
  const match = props.url.match(/[?&]list=(UU[A-Za-z0-9_-]+)/)
  return !!match
})

const videoFormats = ['mp4', 'mkv', 'webm']
const audioFormats = ['mp3', 'm4a', 'flac', 'wav', 'opus']
const qualities = ['best', '2160', '1080', '720', '480']

const availableFormats = computed(() =>
  mediaType.value === 'video' ? videoFormats : audioFormats
)

const LARGE_PLAYLIST_THRESHOLD = 50

watch(() => props.open, (isOpen) => {
  if (isOpen && props.url) {
    fetchFormats(props.url)
    // Reset playlist state
    playlistItems.value = []
    playlistFetchError.value = null
    playlistPreviewLoaded.value = false
    // Default to 'single' for channel upload lists
    playlistMode.value = isChannelUploadList.value ? 'single' : 'single'
    // Load defaults from settings
    embedThumbnail.value = settingsStore.settings.embed_thumbnail
    embedMetadata.value = settingsStore.settings.embed_metadata
    writeSubs.value = settingsStore.settings.write_subs
    embedSubs.value = settingsStore.settings.embed_subs
    embedChapters.value = settingsStore.settings.embed_chapters
    sponsorblock.value = settingsStore.settings.sponsorblock
  }
})

// When user selects "all", fetch playlist preview to show item count
watch(playlistMode, async (mode) => {
  if (mode === 'all' && !playlistPreviewLoaded.value) {
    playlistFetchError.value = null
    try {
      const items = await downloadsStore.fetchPlaylistItems(props.url)
      if (items.length > 0) {
        playlistItems.value = items
        playlistPreviewLoaded.value = true
      }
    } catch (e) {
      playlistFetchError.value = `プレイリスト取得に失敗しました: ${e}`
    }
  }
})

function handleCancelPlaylistFetch() {
  downloadsStore.cancelPlaylistFetch()
  playlistMode.value = 'single'
}

const playlistConfirmed = ref(false)

const needsPlaylistConfirmation = computed(() =>
  playlistMode.value === 'all'
  && playlistPreviewLoaded.value
  && playlistItems.value.length >= LARGE_PLAYLIST_THRESHOLD
  && !playlistConfirmed.value
)

function handleStart() {
  const s = settingsStore.settings
  const options: DownloadOptions = {
    format: selectedFormat.value,
    quality: selectedQuality.value,
    output_dir: s.download_dir,
    embed_thumbnail: embedThumbnail.value,
    embed_metadata: embedMetadata.value,
    write_subs: writeSubs.value,
    embed_subs: embedSubs.value,
    embed_chapters: embedChapters.value,
    sponsorblock: sponsorblock.value,
    custom_format: useCustomFormat.value ? customFormat.value : null,
    playlist_mode: isPlaylistUrl.value ? playlistMode.value : 'single',
    restrict_filenames: s.restrict_filenames,
    no_overwrites: s.no_overwrites,
    geo_bypass: s.geo_bypass,
    rate_limit: s.rate_limit,
    sub_lang: s.sub_lang,
    convert_subs: s.convert_subs,
    merge_output_format: s.merge_output_format,
    recode_video: s.recode_video,
    retries: s.retries,
    proxy: s.proxy,
    extra_args: s.extra_args,
  }
  emit('start', props.url, options)
  emit('close')
}
</script>

<template>
  <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
    <div class="bg-white dark:bg-neutral-800 rounded-xl shadow-2xl w-[560px] max-h-[80vh] flex flex-col">
      <!-- Header (fixed) -->
      <div class="flex items-center justify-between p-4 border-b border-[var(--color-separator)] flex-shrink-0">
        <h2 class="text-lg font-semibold">ダウンロード</h2>
        <button @click="emit('close')" class="text-neutral-400 hover:text-neutral-600">&times;</button>
      </div>

      <!-- Scrollable content -->
      <div class="flex-1 overflow-auto">
        <!-- Loading state -->
        <div v-if="loading" class="p-8 text-center text-neutral-500">
          情報を取得中...
        </div>

        <!-- Error state -->
        <div v-else-if="error" class="p-8 text-center">
          <p class="text-red-500">{{ error }}</p>
          <button v-if="isYtdlpMissing" @click="handleInstallYtdlp" :disabled="installing"
                  class="mt-4 px-4 py-2 rounded-md text-sm bg-[var(--color-accent)] text-white disabled:opacity-50">
            {{ installing ? 'インストール中...' : 'yt-dlp をインストール' }}
          </button>
        </div>

        <!-- Video info -->
        <div v-else-if="videoInfo" class="p-4 space-y-4">
          <!-- Thumbnail + Title -->
          <div class="flex gap-4">
            <img v-if="videoInfo.thumbnail_url" :src="videoInfo.thumbnail_url"
                 class="w-40 h-24 object-cover rounded-lg" />
            <div>
              <p class="font-medium line-clamp-2">{{ videoInfo.title }}</p>
              <p class="text-sm text-neutral-500">{{ videoInfo.channel }}</p>
              <p class="text-xs text-neutral-400">{{ videoInfo.site }}</p>
            </div>
          </div>

          <!-- Playlist mode selector -->
          <div v-if="isPlaylistUrl" class="p-3 rounded-lg bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800">
            <p class="text-xs font-semibold text-blue-600 dark:text-blue-400 mb-2">プレイリストが検出されました</p>

            <!-- Channel upload list warning -->
            <p v-if="isChannelUploadList" class="text-xs text-amber-600 dark:text-amber-400 mb-2">
              ⚠ このリストはチャンネルの全動画一覧です。「すべて」を選択すると大量の動画がダウンロードされます。
            </p>

            <div class="flex gap-2">
              <button
                class="flex-1 px-3 py-2 rounded-md text-sm transition-colors"
                :class="playlistMode === 'single'
                  ? 'bg-blue-500 text-white'
                  : 'bg-white dark:bg-neutral-700 text-neutral-700 dark:text-neutral-300 border border-neutral-200 dark:border-neutral-600'"
                @click="playlistMode = 'single'"
              >
                1件のみ
                <span class="block text-xs opacity-75 mt-0.5">この動画だけ</span>
              </button>
              <button
                class="flex-1 px-3 py-2 rounded-md text-sm transition-colors"
                :class="playlistMode === 'all'
                  ? 'bg-blue-500 text-white'
                  : 'bg-white dark:bg-neutral-700 text-neutral-700 dark:text-neutral-300 border border-neutral-200 dark:border-neutral-600'"
                @click="playlistMode = 'all'; playlistConfirmed = false"
              >
                すべて
                <span class="block text-xs opacity-75 mt-0.5">リスト全件ダウンロード</span>
              </button>
            </div>

            <!-- Playlist fetching indicator -->
            <div v-if="playlistMode === 'all' && downloadsStore.playlistFetching" class="mt-3 flex items-center gap-2">
              <div class="animate-spin w-4 h-4 border-2 border-blue-500 border-t-transparent rounded-full" />
              <span class="text-xs text-neutral-500">プレイリスト情報を取得中...</span>
              <button @click="handleCancelPlaylistFetch"
                      class="ml-auto px-2 py-1 text-xs rounded bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400 hover:bg-red-200 dark:hover:bg-red-900/50 transition-colors">
                中止
              </button>
            </div>

            <!-- Playlist fetch error -->
            <p v-if="playlistFetchError" class="mt-2 text-xs text-red-500">{{ playlistFetchError }}</p>

            <!-- Playlist item count preview -->
            <div v-if="playlistMode === 'all' && playlistPreviewLoaded && playlistItems.length > 0" class="mt-3">
              <p class="text-xs text-neutral-600 dark:text-neutral-400">
                <span class="font-semibold">{{ playlistItems.length }}件</span> の動画が見つかりました
              </p>

              <!-- Large playlist confirmation -->
              <div v-if="playlistItems.length >= LARGE_PLAYLIST_THRESHOLD && !playlistConfirmed"
                   class="mt-2 p-2 rounded bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800">
                <p class="text-xs text-amber-700 dark:text-amber-300 mb-2">
                  ⚠ {{ playlistItems.length }}件は大量です。本当にすべてダウンロードしますか？
                </p>
                <div class="flex gap-2">
                  <button @click="playlistConfirmed = true"
                          class="px-3 py-1 text-xs rounded bg-amber-500 text-white hover:bg-amber-600 transition-colors">
                    すべてダウンロード
                  </button>
                  <button @click="playlistMode = 'single'"
                          class="px-3 py-1 text-xs rounded bg-neutral-200 dark:bg-neutral-700 hover:bg-neutral-300 dark:hover:bg-neutral-600 transition-colors">
                    この動画のみ
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- Media type toggle -->
          <div class="flex gap-2">
            <button v-for="type_ in (['video', 'audio'] as const)" :key="type_"
                    class="px-4 py-1.5 rounded-md text-sm"
                    :class="mediaType === type_ ? 'bg-[var(--color-accent)] text-white' : 'bg-neutral-100 dark:bg-neutral-700'"
                    @click="mediaType = type_; selectedFormat = type_ === 'video' ? 'mp4' : 'mp3'">
              {{ type_ === 'video' ? '映像' : '音声' }}
            </button>
          </div>

          <!-- Format & Quality -->
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="block text-xs text-neutral-500 mb-1">フォーマット</label>
              <select v-model="selectedFormat" class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-700 text-sm">
                <option v-for="f in availableFormats" :key="f" :value="f">{{ f.toUpperCase() }}</option>
              </select>
            </div>
            <div v-if="mediaType === 'video'">
              <label class="block text-xs text-neutral-500 mb-1">画質</label>
              <select v-model="selectedQuality" class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-700 text-sm">
                <option v-for="q in qualities" :key="q" :value="q">
                  {{ q === 'best' ? '最高画質' : q + 'p' }}
                </option>
              </select>
            </div>
          </div>

          <!-- Custom format -->
          <div>
            <label class="flex items-center gap-2 text-sm">
              <input type="checkbox" v-model="useCustomFormat" />
              カスタムフォーマット指定
            </label>
            <input v-if="useCustomFormat" v-model="customFormat" placeholder="bestvideo+bestaudio/best"
                   class="mt-1 w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-700 text-sm font-mono" />
          </div>

          <!-- Post-process options -->
          <div class="space-y-2">
            <p class="text-xs text-neutral-500 font-semibold">ポストプロセス</p>
            <label class="flex items-center gap-2 text-sm">
              <input type="checkbox" v-model="embedThumbnail" /> サムネイル埋め込み
            </label>
            <label class="flex items-center gap-2 text-sm">
              <input type="checkbox" v-model="embedMetadata" /> メタデータ埋め込み
            </label>
            <label class="flex items-center gap-2 text-sm">
              <input type="checkbox" v-model="writeSubs" /> 字幕ダウンロード
            </label>
            <label class="flex items-center gap-2 text-sm">
              <input type="checkbox" v-model="embedSubs" /> 字幕埋め込み
            </label>
            <label class="flex items-center gap-2 text-sm">
              <input type="checkbox" v-model="embedChapters" /> チャプター埋め込み
            </label>
            <label class="flex items-center gap-2 text-sm">
              <input type="checkbox" v-model="sponsorblock" /> SponsorBlock
            </label>
          </div>
        </div>
      </div>

      <!-- Footer (fixed) -->
      <div class="flex flex-col gap-2 p-4 border-t border-[var(--color-separator)] flex-shrink-0">
        <!-- スケジュール実行トグル -->
        <div>
          <label class="flex items-center gap-2 text-sm cursor-pointer">
            <input type="checkbox" v-model="showScheduleMode" />
            <span>スケジュール実行</span>
          </label>
        </div>

        <!-- インラインスケジュールフォーム -->
        <div v-if="showScheduleMode" class="flex flex-col gap-2 pt-2 border-t border-[var(--color-separator)]">
          <div>
            <label class="block text-xs text-neutral-500 mb-1">スケジュール名</label>
            <input v-model="scheduleName" class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-700 text-sm" placeholder="例: 毎朝ダウンロード" />
          </div>
          <div>
            <label class="block text-xs text-neutral-500 mb-1">cron式</label>
            <input v-model="scheduleCronExpr" class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-700 text-sm font-mono" placeholder="0 9 * * *" />
            <p v-if="scheduleCronError" class="text-xs text-red-500 mt-0.5">{{ scheduleCronError }}</p>
            <p v-else-if="scheduleNextRun" class="text-xs text-neutral-400 mt-0.5">次回: {{ scheduleNextRun }}</p>
          </div>
          <label class="flex items-center gap-2 text-sm cursor-pointer">
            <input type="checkbox" v-model="scheduleIsChannel" />
            <span>チャンネル監視（新着のみ）</span>
          </label>
        </div>

        <div class="flex justify-end gap-2">
          <button @click="emit('close')" class="px-4 py-1.5 rounded-md text-sm bg-neutral-100 dark:bg-neutral-700">
            キャンセル
          </button>
          <button v-if="!showScheduleMode" @click="handleStart"
                  :disabled="loading || !!error || needsPlaylistConfirmation || (playlistMode === 'all' && downloadsStore.playlistFetching)"
                  class="px-4 py-1.5 rounded-md text-sm bg-[var(--color-accent)] text-white disabled:opacity-50">
            ダウンロード開始
          </button>
          <button v-else @click="handleScheduleRegister"
                  :disabled="!isScheduleValid"
                  class="px-4 py-1.5 rounded-md text-sm bg-[var(--color-accent)] text-white disabled:opacity-50">
            スケジュール登録
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
