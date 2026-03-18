<script setup lang="ts">
import { computed } from 'vue'
import type { Download, DownloadProgress } from '../../types'

const props = defineProps<{
  download: Download
  progress?: DownloadProgress
}>()

const emit = defineEmits<{
  pause: [id: number]
  resume: [id: number]
  cancel: [id: number]
}>()

const percent = computed(() =>
  props.progress?.percent ?? props.download.progress * 100
)

const statusLabel = computed(() => {
  switch (props.download.status) {
    case 'downloading': return props.progress?.speed_str ?? 'ダウンロード中...'
    case 'paused': return '一時停止中'
    case 'pending': return 'キュー待ち'
    case 'completed': return '完了'
    case 'failed': return props.download.error_message ?? 'エラー'
    default: return props.download.status
  }
})

const isPausable = computed(() => props.download.status === 'downloading')
const isResumable = computed(() => props.download.status === 'paused')
const isCompleted = computed(() => props.download.status === 'completed')
const isError = computed(() => ['failed', 'error'].includes(props.download.status))

const isPlaylist = computed(() =>
  (props.progress?.playlist_count ?? 0) > 1
)

const playlistLabel = computed(() => {
  if (!isPlaylist.value || !props.progress) return ''
  return `${props.progress.playlist_index} / ${props.progress.playlist_count} 件`
})

const barColor = computed(() => {
  if (isCompleted.value) return 'bg-green-500'
  if (isError.value) return 'bg-red-500'
  if (props.download.status === 'paused') return 'bg-yellow-500'
  return 'bg-[var(--color-accent)]'
})

function formatBytes(bytes: number | null | undefined): string {
  if (!bytes) return ''
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(0)} KB`
  if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`
  return `${(bytes / 1073741824).toFixed(2)} GB`
}
</script>

<template>
  <div class="px-4 py-3 hover:bg-neutral-50 dark:hover:bg-neutral-800/50 transition-colors">
    <!-- Title row -->
    <div class="flex items-center justify-between mb-2">
      <div class="flex items-center gap-2 flex-1 mr-3 min-w-0">
        <p class="text-sm font-medium truncate">{{ download.title || download.url }}</p>
        <span v-if="isPlaylist"
              class="flex-shrink-0 px-1.5 py-0.5 text-[10px] font-bold rounded bg-blue-100 dark:bg-blue-900/40 text-blue-600 dark:text-blue-400">
          {{ playlistLabel }}
        </span>
      </div>
      <div class="flex items-center gap-2 flex-shrink-0">
        <button v-if="isPausable" @click="emit('pause', download.id)"
                class="w-10 h-10 flex items-center justify-center rounded-lg bg-neutral-100 dark:bg-neutral-700 hover:bg-yellow-100 dark:hover:bg-yellow-900/30 text-lg transition-colors" title="一時停止">
          ⏸
        </button>
        <button v-if="isResumable" @click="emit('resume', download.id)"
                class="w-10 h-10 flex items-center justify-center rounded-lg bg-neutral-100 dark:bg-neutral-700 hover:bg-green-100 dark:hover:bg-green-900/30 text-lg transition-colors" title="再開">
          ▶
        </button>
        <button @click="emit('cancel', download.id)"
                class="w-10 h-10 flex items-center justify-center rounded-lg bg-neutral-100 dark:bg-neutral-700 hover:bg-red-100 dark:hover:bg-red-900/30 text-red-500 text-lg font-bold transition-colors" title="キャンセル">
          ✕
        </button>
      </div>
    </div>

    <!-- Progress bar -->
    <div class="relative h-6 bg-neutral-200 dark:bg-neutral-700 rounded-lg overflow-hidden">
      <div
        class="absolute inset-y-0 left-0 rounded-lg transition-all duration-300 ease-out"
        :class="[barColor, { 'animate-pulse': download.status === 'downloading' && percent < 100 }]"
        :style="{ width: `${Math.max(percent, 1)}%` }"
      >
        <!-- Shimmer effect for active downloads -->
        <div v-if="download.status === 'downloading'" class="shimmer" />
      </div>
      <!-- Percent text overlay -->
      <div class="absolute inset-0 flex items-center justify-center">
        <span class="text-xs font-bold drop-shadow-sm"
              :class="percent > 50 ? 'text-white' : 'text-neutral-700 dark:text-neutral-200'">
          {{ percent.toFixed(0) }}%
        </span>
      </div>
    </div>

    <!-- Status row -->
    <div class="flex items-center justify-between mt-1.5 text-xs text-neutral-500">
      <span>{{ statusLabel }}</span>
      <div class="flex gap-3">
        <span v-if="progress?.downloaded_bytes">{{ formatBytes(progress.downloaded_bytes) }} / {{ formatBytes(progress.total_bytes) }}</span>
        <span v-if="progress?.eta_str && download.status === 'downloading'">残り {{ progress.eta_str }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.shimmer {
  position: absolute;
  inset: 0;
  background: linear-gradient(
    90deg,
    transparent 0%,
    rgba(255, 255, 255, 0.3) 50%,
    transparent 100%
  );
  animation: shimmer 1.5s infinite;
}

@keyframes shimmer {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(100%); }
}
</style>
