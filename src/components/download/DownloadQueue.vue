<script setup lang="ts">
import { ref } from 'vue'
import { useDownloadsStore } from '../../stores/downloads'
import DownloadProgressItem from './DownloadProgress.vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const store = useDownloadsStore()
const cancellingAll = ref(false)

async function handlePause(id: number) { await store.pauseDownload(id) }
async function handleResume(id: number) { await store.resumeDownload(id) }
async function handleCancel(id: number) { await store.cancelDownload(id) }

async function handleCancelAll() {
  cancellingAll.value = true
  store.cancelPlaylistFetch()
  for (const dl of store.activeDownloads) {
    try { await store.cancelDownload(dl.id) } catch { /* ignore */ }
  }
  cancellingAll.value = false
}
</script>

<template>
  <div>
    <div v-if="store.activeDownloads.length === 0" class="p-8 text-center text-neutral-400 text-sm">
      {{ t('download_queue.empty') }}
    </div>
    <template v-else>
      <!-- Cancel all button -->
      <div v-if="store.activeDownloads.length > 1"
           class="flex items-center justify-between px-4 py-2 bg-neutral-50 dark:bg-neutral-800/50 border-b border-[var(--color-separator)]">
        <span class="text-xs text-neutral-500">{{ t('statusbar.active_count', { count: store.activeDownloads.length }) }}</span>
        <button @click="handleCancelAll" :disabled="cancellingAll"
                class="px-3 py-1 text-xs rounded-md bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400 hover:bg-red-200 dark:hover:bg-red-900/50 disabled:opacity-50 transition-colors">
          {{ cancellingAll ? t('common.loading') : t('download_progress.cancel') }}
        </button>
      </div>
      <div class="divide-y divide-[var(--color-separator)]">
        <DownloadProgressItem
          v-for="dl in store.activeDownloads"
          :key="dl.id"
          :download="dl"
          :progress="store.progressMap.get(dl.id)"
          @pause="handlePause"
          @resume="handleResume"
          @cancel="handleCancel"
        />
      </div>
    </template>
  </div>
</template>
