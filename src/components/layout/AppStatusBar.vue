<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useDownloadsStore } from '../../stores/downloads'
import { useLibraryStore } from '../../stores/library'
import { useYtdlp } from '../../composables/useYtdlp'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const downloadsStore = useDownloadsStore()
const libraryStore = useLibraryStore()
const { info: ytdlpInfo, loadInfo: loadYtdlpInfo, checkUpdate: checkYtdlpUpdate } = useYtdlp()

onMounted(() => {
  loadYtdlpInfo()
  // Background update check after startup — delay to avoid blocking initial render
  setTimeout(() => { void checkYtdlpUpdate() }, 5000)
})

const activeCount = computed(() => downloadsStore.activeDownloads.length)
const queuedCount = computed(() => downloadsStore.pendingQueue.length)

const totalProgress = computed(() => {
  if (activeCount.value === 0) return 0
  let total = 0
  for (const dl of downloadsStore.activeDownloads) {
    const prog = downloadsStore.progressMap.get(dl.id)
    total += prog?.percent ?? dl.progress * 100
  }
  return total / activeCount.value
})

const libraryCount = computed(() => libraryStore.filteredItems.length)

const statusText = computed(() => {
  if (activeCount.value > 0) {
    const queued = queuedCount.value > 0 ? ` (+${queuedCount.value})` : ''
    return `${t('statusbar.active_count', { count: activeCount.value })} (${totalProgress.value.toFixed(0)}%)${queued}`
  }
  if (queuedCount.value > 0) {
    return t('statusbar.active_count', { count: queuedCount.value })
  }
  return t('statusbar.idle')
})
</script>

<template>
  <footer class="h-[var(--statusbar-height)] flex items-center justify-between px-4 text-xs text-neutral-500 border-t border-[var(--color-separator)] bg-neutral-50 dark:bg-neutral-950 flex-shrink-0">
    <div class="flex items-center gap-3">
      <span>{{ statusText }}</span>
      <!-- Mini progress bar when downloading -->
      <div v-if="activeCount > 0" class="w-24 h-1 bg-neutral-200 dark:bg-neutral-700 rounded-full overflow-hidden">
        <div class="h-full bg-[var(--color-accent)] rounded-full transition-all"
             :style="{ width: `${totalProgress}%` }" />
      </div>
    </div>
    <div class="flex items-center gap-3">
      <span v-if="libraryCount > 0">{{ libraryCount }} アイテム</span>
      <span v-if="ytdlpInfo">
        yt-dlp {{ ytdlpInfo.version }}
        <span v-if="ytdlpInfo.update_available" class="text-orange-500 ml-1" :title="t('general.ytdlp_update_available')">●</span>
      </span>
      <span v-else class="text-red-400">{{ t('general.ytdlp_not_found') }}</span>
    </div>
  </footer>
</template>
