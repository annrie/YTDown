<script setup lang="ts">
import { onMounted } from 'vue'
import { confirm } from '@tauri-apps/plugin-dialog'
import { useI18n } from 'vue-i18n'
import { useDownloadHistoryStore } from '../../stores/downloadHistory'

const { t } = useI18n()
const historyStore = useDownloadHistoryStore()

onMounted(() => historyStore.fetchHistory())

async function handleDelete(id: number) {
  await historyStore.deleteEntry(id)
}

async function handleClearAll() {
  const ok = await confirm(t('history.clear_confirm'), {
    title: t('history.title'),
    kind: 'warning',
  })
  if (ok) await historyStore.clearAll()
}

function handleRedownload(url: string) {
  // Dispatch a custom event to open DownloadDialog with the URL pre-filled
  window.dispatchEvent(new CustomEvent('open-download-dialog', { detail: { url } }))
}

function formatDate(dt: string) {
  return new Date(dt).toLocaleDateString(undefined, {
    year: 'numeric', month: 'short', day: 'numeric',
    hour: '2-digit', minute: '2-digit',
  })
}
</script>

<template>
  <div class="flex flex-col h-full">
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-3 border-b border-[var(--color-separator)]">
      <h2 class="text-sm font-semibold">{{ t('history.title') }}</h2>
      <button v-if="historyStore.entries.length > 0"
              @click="handleClearAll"
              class="text-xs text-red-500 hover:text-red-600 px-2 py-1 rounded hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors">
        {{ t('history.clear_all') }}
      </button>
    </div>

    <!-- Empty -->
    <div v-if="historyStore.entries.length === 0"
         class="flex-1 flex items-center justify-center text-sm text-neutral-400">
      {{ t('history.empty') }}
    </div>

    <!-- List -->
    <div v-else class="flex-1 overflow-auto divide-y divide-[var(--color-separator)]">
      <div v-for="entry in historyStore.entries" :key="entry.id"
           class="flex items-start gap-3 px-4 py-3 hover:bg-neutral-50 dark:hover:bg-neutral-800/50 group">
        <!-- Info -->
        <div class="flex-1 min-w-0">
          <p class="text-sm font-medium truncate">
            {{ entry.title ?? entry.url }}
          </p>
          <p class="text-xs text-neutral-400 truncate mt-0.5">{{ entry.url }}</p>
          <div class="flex items-center gap-2 mt-1">
            <span v-if="entry.site" class="text-xs bg-neutral-100 dark:bg-neutral-700 px-1.5 py-0.5 rounded">
              {{ entry.site }}
            </span>
            <span class="text-xs text-neutral-400">{{ formatDate(entry.completed_at) }}</span>
          </div>
        </div>

        <!-- Actions -->
        <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0">
          <button @click="handleRedownload(entry.url)"
                  :title="t('history.redownload')"
                  class="w-7 h-7 flex items-center justify-center rounded text-neutral-400 hover:text-[var(--color-accent)] hover:bg-neutral-100 dark:hover:bg-neutral-700 transition-colors">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                    d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
            </svg>
          </button>
          <button @click="handleDelete(entry.id)"
                  :title="t('history.delete')"
                  class="w-7 h-7 flex items-center justify-center rounded text-neutral-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                    d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
