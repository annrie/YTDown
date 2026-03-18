<script setup lang="ts">
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

defineProps<{ open: boolean }>()
const emit = defineEmits<{
  close: []
  'start-batch': [urls: string[]]
}>()

const MAX_URLS = 10
const urls = ref<string[]>(Array(MAX_URLS).fill(''))
const fetchingBrowserUrl = ref(false)

async function addBrowserUrl() {
  fetchingBrowserUrl.value = true
  try {
    const url = await invoke<string>('get_browser_url')
    if (url) {
      // 空いている最初のスロットに挿入
      const emptyIndex = urls.value.findIndex(u => u.trim() === '')
      if (emptyIndex !== -1) {
        urls.value[emptyIndex] = url
      }
    }
  } catch (e) {
    console.error('ブラウザURL取得エラー:', e)
  } finally {
    fetchingBrowserUrl.value = false
  }
}

const validUrls = computed(() =>
  urls.value.filter(u => u.trim().length > 0)
)

function handlePaste(index: number, e: ClipboardEvent) {
  const text = e.clipboardData?.getData('text') ?? ''
  const lines = text.split(/\n/).map(l => l.trim()).filter(Boolean)
  if (lines.length > 1) {
    e.preventDefault()
    for (let i = 0; i < lines.length && index + i < MAX_URLS; i++) {
      urls.value[index + i] = lines[i]
    }
  }
}

function clearAll() {
  urls.value = Array(MAX_URLS).fill('')
}

function removeUrl(index: number) {
  urls.value[index] = ''
}

function handleStart() {
  if (validUrls.value.length === 0) return
  emit('start-batch', validUrls.value)
  clearAll()
  emit('close')
}
</script>

<template>
  <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
    <div class="bg-white dark:bg-neutral-800 rounded-xl shadow-2xl w-[620px] max-h-[85vh] flex flex-col">
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-b border-[var(--color-separator)]">
        <div>
          <h2 class="text-lg font-semibold">一括ダウンロード</h2>
          <p class="text-xs text-neutral-500 mt-0.5">最大{{ MAX_URLS }}件のURLを登録できます</p>
        </div>
        <button @click="emit('close')" class="text-neutral-400 hover:text-neutral-600 text-xl">&times;</button>
      </div>

      <!-- URL List -->
      <div class="flex-1 overflow-auto p-4 space-y-2">
        <div v-for="(_, index) in urls" :key="index" class="flex items-center gap-2">
          <span class="w-6 text-right text-xs text-neutral-400 flex-shrink-0">{{ index + 1 }}</span>
          <input
            v-model="urls[index]"
            type="url"
            :placeholder="`URL ${index + 1}`"
            class="flex-1 h-9 px-3 rounded-lg bg-neutral-100 dark:bg-neutral-700/60 text-sm outline-none focus:ring-2 focus:ring-[var(--color-accent)] transition-shadow"
            @paste="handlePaste(index, $event)"
          />
          <button v-if="urls[index].trim()"
                  @click="removeUrl(index)"
                  class="w-7 h-7 flex items-center justify-center rounded-md text-neutral-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 text-sm transition-colors">
            &times;
          </button>
          <div v-else class="w-7" />
        </div>
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-between p-4 border-t border-[var(--color-separator)]">
        <div class="flex items-center gap-2">
          <button @click="clearAll" class="px-3 py-1.5 rounded-md text-xs text-neutral-500 hover:bg-neutral-100 dark:hover:bg-neutral-700">
            すべてクリア
          </button>
          <button @click="addBrowserUrl" :disabled="fetchingBrowserUrl"
                  class="px-3 py-1.5 rounded-md text-xs text-neutral-500 hover:bg-neutral-100 dark:hover:bg-neutral-700 hover:text-[var(--color-accent)] disabled:opacity-40 flex items-center gap-1 transition-colors">
            <svg class="w-3.5 h-3.5" :class="{ 'animate-spin': fetchingBrowserUrl }" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path v-if="!fetchingBrowserUrl" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 21a9 9 0 100-18 9 9 0 000 18zm0-18v18m-9-9h18M3.6 9h16.8M3.6 15h16.8" />
              <path v-else stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 12a8 8 0 018-8" />
            </svg>
            ブラウザから追加
          </button>
        </div>
        <div class="flex items-center gap-3">
          <span class="text-xs text-neutral-500">{{ validUrls.length }} / {{ MAX_URLS }} 件</span>
          <button @click="emit('close')" class="px-4 py-1.5 rounded-md text-sm bg-neutral-100 dark:bg-neutral-700">
            キャンセル
          </button>
          <button @click="handleStart" :disabled="validUrls.length === 0"
                  class="px-5 py-1.5 rounded-md text-sm bg-[var(--color-accent)] text-white font-medium disabled:opacity-40 transition-opacity">
            {{ validUrls.length }}件 ダウンロード
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
