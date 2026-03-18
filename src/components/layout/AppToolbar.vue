<script setup lang="ts">
import { ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'
import type { ViewMode } from '../../types'

defineProps<{
  currentView: ViewMode
  searchQuery: string
}>()

const emit = defineEmits<{
  'update:currentView': [mode: ViewMode]
  'update:searchQuery': [query: string]
  'submit-url': [url: string]
  'open-batch': []
}>()

const urlInput = ref('')
const showSearch = ref(false)
const fetchingBrowserUrl = ref(false)
const browserUrlError = ref('')

async function fetchBrowserUrl() {
  fetchingBrowserUrl.value = true
  browserUrlError.value = ''
  try {
    const url = await invoke<string>('get_browser_url')
    if (url) {
      urlInput.value = url
    }
  } catch (e) {
    browserUrlError.value = String(e)
    setTimeout(() => { browserUrlError.value = '' }, 3000)
  } finally {
    fetchingBrowserUrl.value = false
  }
}

function handleSubmitUrl() {
  const url = urlInput.value.trim()
  if (url) {
    emit('submit-url', url)
    urlInput.value = ''
  }
}

function handleUrlKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') handleSubmitUrl()
}

function toggleSearch() {
  showSearch.value = !showSearch.value
  if (!showSearch.value) {
    emit('update:searchQuery', '')
  }
}

function handleSearchInput(e: Event) {
  emit('update:searchQuery', (e.target as HTMLInputElement).value)
}

function handleToolbarMousedown(e: MouseEvent) {
  // Only drag from the toolbar background itself, not from child interactive elements
  const target = e.target as HTMLElement
  if (target.closest('input, button, select, a')) return
  if (e.buttons === 1) {
    if (e.detail === 2) {
      getCurrentWindow().toggleMaximize()
    } else {
      getCurrentWindow().startDragging()
    }
  }
}
</script>

<template>
  <header
    class="h-[var(--toolbar-height)] flex items-center px-4 gap-3 border-b border-[var(--color-separator)] bg-white/80 dark:bg-neutral-900/80 backdrop-blur-xl"
    @mousedown="handleToolbarMousedown"
  >
    <!-- Spacer for traffic lights -->
    <div class="w-16 flex-shrink-0" />

    <!-- URL input or Search -->
    <template v-if="showSearch">
      <input
        type="text"
        :value="searchQuery"
        @input="handleSearchInput"
        placeholder="検索..."
        class="flex-1 h-8 px-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm outline-none focus:ring-2 focus:ring-[var(--color-accent)]"
        autofocus
      />
    </template>
    <template v-else>
      <div class="flex-1 flex items-center gap-1.5">
        <input
          v-model="urlInput"
          type="url"
          placeholder="URLを入力..."
          class="flex-1 h-8 px-3 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm outline-none focus:ring-2 focus:ring-[var(--color-accent)]"
          @keydown="handleUrlKeydown"
        />
        <!-- ブラウザからURL取得ボタン -->
        <button
          @click="fetchBrowserUrl"
          :disabled="fetchingBrowserUrl"
          class="w-10 h-10 flex items-center justify-center rounded-md hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-colors disabled:opacity-40"
          :class="browserUrlError ? 'text-red-500' : 'text-neutral-500 hover:text-[var(--color-accent)]'"
          title="ブラウザから取得"
        >
          <svg class="w-5 h-5" :class="{ 'animate-spin': fetchingBrowserUrl }" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path v-if="!fetchingBrowserUrl" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 21a9 9 0 100-18 9 9 0 000 18zm0-18v18m-9-9h18M3.6 9h16.8M3.6 15h16.8" />
            <path v-else stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 12a8 8 0 018-8" />
          </svg>
        </button>
      </div>
    </template>

    <!-- Search toggle -->
    <button class="w-10 h-10 flex items-center justify-center rounded-md hover:bg-neutral-100 dark:hover:bg-neutral-800 text-neutral-500"
            :class="{ 'text-[var(--color-accent)]': showSearch }"
            @click="toggleSearch"
            title="検索">
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
      </svg>
    </button>

    <!-- Download button -->
    <button v-if="!showSearch"
            class="px-4 h-8 rounded-md bg-[var(--color-accent)] text-white text-sm font-medium flex-shrink-0"
            @click="handleSubmitUrl">
      ダウンロード
    </button>

    <!-- Batch URL button -->
    <button v-if="!showSearch"
            class="px-3 h-8 rounded-md text-sm font-medium flex-shrink-0 border border-[var(--color-accent)] text-[var(--color-accent)] hover:bg-[var(--color-accent)]/10 transition-colors"
            @click="emit('open-batch')">
      一括
    </button>

    <!-- View mode toggle -->
    <div class="flex gap-0.5 bg-neutral-100 dark:bg-neutral-800 rounded-md p-0.5 flex-shrink-0">
      <button
        v-for="mode in (['list', 'grid', 'column'] as ViewMode[])"
        :key="mode"
        class="px-2 py-1 text-xs rounded"
        :class="currentView === mode ? 'bg-white dark:bg-neutral-700 shadow-sm' : 'text-neutral-500'"
        @click="emit('update:currentView', mode)"
      >
        {{ mode === 'list' ? 'リスト' : mode === 'grid' ? 'グリッド' : 'カラム' }}
      </button>
    </div>
  </header>

  <!-- ブラウザURL取得エラーバナー -->
  <Teleport to="body">
    <Transition name="fade">
      <div v-if="browserUrlError"
           class="fixed top-[var(--toolbar-height)] left-1/2 -translate-x-1/2 mt-2 px-4 py-2 bg-red-500 text-white text-sm rounded-lg shadow-xl z-[9999]">
        {{ browserUrlError }}
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.2s; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
