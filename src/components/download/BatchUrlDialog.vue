<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  open: boolean
  initialUrls?: string[]
}>()
const emit = defineEmits<{
  close: []
  'start-batch': [urls: string[]]
}>()

const { t } = useI18n()
const MIN_SLOTS = 10

interface UrlItem { url: string; selected: boolean }
const items = ref<UrlItem[]>(makeSlots(MIN_SLOTS))
const fetchingBrowserUrl = ref(false)

function makeSlots(count: number): UrlItem[] {
  return Array.from({ length: count }, () => ({ url: '', selected: false }))
}

function ensureSlots(urls: string[]) {
  const needed = Math.max(MIN_SLOTS, urls.length + 2)
  items.value = Array.from({ length: needed }, (_, i) => ({
    url: urls[i] ?? '',
    selected: false,
  }))
}

function applyInitialUrls(nextUrls: string[]) {
  ensureSlots(nextUrls)
}

watch(() => props.open, (isOpen) => {
  if (isOpen && props.initialUrls?.length) {
    applyInitialUrls(props.initialUrls)
  }
})

watch(() => props.initialUrls, (nextUrls) => {
  if (props.open && nextUrls?.length) {
    applyInitialUrls(nextUrls)
  }
}, { deep: true })

async function addBrowserUrl() {
  fetchingBrowserUrl.value = true
  try {
    const url = await invoke<string>('get_browser_url')
    if (url) {
      const emptyIndex = items.value.findIndex(i => i.url.trim() === '')
      if (emptyIndex !== -1) {
        items.value[emptyIndex].url = url
      }
    }
  } catch (e) {
    console.error('ブラウザURL取得エラー:', e)
  } finally {
    fetchingBrowserUrl.value = false
  }
}

async function loadFromFile() {
  const path = await openDialog({
    title: t('batch_url_dialog.load_file'),
    multiple: false,
    filters: [
      { name: 'JSON / Text', extensions: ['json', 'txt'] },
    ],
  })
  if (!path || typeof path !== 'string') return
  try {
    const text = await invoke<string>('read_text_file', { path })
    let urls: string[] = []
    try {
      const parsed = JSON.parse(text)
      if (Array.isArray(parsed)) {
        urls = parsed.map((item: unknown) => {
          if (typeof item === 'string') return item
          if (item && typeof item === 'object' && 'url' in item) return (item as { url: string }).url
          return ''
        }).filter(Boolean)
      }
    } catch {
      urls = text.split(/\n/).map(l => l.trim()).filter(l => l.startsWith('http'))
    }
    if (urls.length === 0) return
    const existing = items.value.filter(i => i.url.trim()).map(i => i.url)
    ensureSlots([...existing, ...urls])
    let slot = items.value.findIndex(i => !i.url.trim())
    for (const url of urls) {
      if (slot === -1) break
      items.value[slot].url = url
      slot = items.value.findIndex((i, idx) => idx > slot && !i.url.trim())
    }
  } catch (e) {
    console.error('Failed to load file:', e)
  }
}

const validUrls = computed(() =>
  items.value.filter(i => i.url.trim().length > 0).map(i => i.url)
)

const filledItems = computed(() => items.value.filter(i => i.url.trim().length > 0))
const selectedCount = computed(() => items.value.filter(i => i.selected && i.url.trim().length > 0).length)
const isAllSelected = computed(() =>
  filledItems.value.length > 0 && filledItems.value.every(i => i.selected)
)

function toggleSelectAll() {
  const next = !isAllSelected.value
  items.value.forEach(i => { if (i.url.trim().length > 0) i.selected = next })
}

function deleteSelected() {
  const remaining = items.value.filter(i => !i.selected || i.url.trim() === '')
    .map(i => i.url)
    .filter(Boolean)
  ensureSlots(remaining)
}

function handlePaste(index: number, e: ClipboardEvent) {
  const text = e.clipboardData?.getData('text') ?? ''
  const lines = text.split(/\n/).map(l => l.trim()).filter(Boolean)
  if (lines.length > 1) {
    e.preventDefault()
    const needed = index + lines.length
    while (items.value.length < needed) {
      items.value.push({ url: '', selected: false })
    }
    for (let i = 0; i < lines.length; i++) {
      items.value[index + i].url = lines[i]
    }
  }
}

function clearAll() {
  items.value = makeSlots(MIN_SLOTS)
}

function removeUrl(index: number) {
  items.value[index].url = ''
  items.value[index].selected = false
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
          <h2 class="text-lg font-semibold">{{ t('batch_url_dialog.title') }}</h2>
          <p class="text-xs text-neutral-500 mt-0.5">{{ t('batch_url_dialog.count', { count: items.length }) }}</p>
        </div>
        <button @click="emit('close')" class="text-neutral-400 hover:text-neutral-600 text-xl">&times;</button>
      </div>

      <!-- Select All bar (visible when any URL is filled) -->
      <div v-if="filledItems.length > 0"
           class="flex items-center gap-2 px-4 py-2 border-b border-[var(--color-separator)] bg-neutral-50 dark:bg-neutral-900/40">
        <input type="checkbox" :checked="isAllSelected" @change="toggleSelectAll"
               class="rounded" />
        <span class="text-xs text-neutral-500">
          {{ isAllSelected ? t('batch_url_dialog.deselect_all') : t('batch_url_dialog.select_all') }}
        </span>
        <span v-if="selectedCount > 0" class="text-xs text-neutral-400 ml-1">
          {{ t('batch_url_dialog.selected_count', { count: selectedCount }) }}
        </span>
        <button v-if="selectedCount > 0"
                @click="deleteSelected"
                class="ml-auto px-2 py-1 rounded text-xs text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors">
          {{ t('batch_url_dialog.delete_selected', { count: selectedCount }) }}
        </button>
      </div>

      <!-- URL List -->
      <div class="flex-1 overflow-auto p-4 space-y-2">
        <div v-for="(item, index) in items" :key="index" class="flex items-center gap-2">
          <input v-if="item.url.trim()"
                 type="checkbox"
                 v-model="item.selected"
                 class="rounded flex-shrink-0" />
          <div v-else class="w-4 flex-shrink-0" />
          <span class="w-6 text-right text-xs text-neutral-400 flex-shrink-0">{{ index + 1 }}</span>
          <input
            v-model="item.url"
            type="url"
            :placeholder="`URL ${index + 1}`"
            class="flex-1 h-9 px-3 rounded-lg bg-neutral-100 dark:bg-neutral-700/60 text-sm outline-none focus:ring-2 focus:ring-[var(--color-accent)] transition-shadow"
            :class="{ 'opacity-50': item.selected }"
            @paste="handlePaste(index, $event)"
          />
          <button v-if="item.url.trim()"
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
            {{ t('download_queue.clear_completed') }}
          </button>
          <button @click="loadFromFile"
                  class="px-3 py-1.5 rounded-md text-xs text-neutral-500 hover:bg-neutral-100 dark:hover:bg-neutral-700 hover:text-[var(--color-accent)] flex items-center gap-1 transition-colors">
            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
            </svg>
            {{ t('batch_url_dialog.load_file') }}
          </button>
          <button @click="addBrowserUrl" :disabled="fetchingBrowserUrl"
                  class="px-3 py-1.5 rounded-md text-xs text-neutral-500 hover:bg-neutral-100 dark:hover:bg-neutral-700 hover:text-[var(--color-accent)] disabled:opacity-50 flex items-center gap-1 transition-colors">
            <svg class="w-3.5 h-3.5" :class="{ 'animate-spin': fetchingBrowserUrl }" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path v-if="!fetchingBrowserUrl" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 21a9 9 0 100-18 9 9 0 000 18zm0-18v18m-9-9h18M3.6 9h16.8M3.6 15h16.8" />
              <path v-else stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 12a8 8 0 018-8" />
            </svg>
            {{ t('toolbar.fetch_browser_url') }}
          </button>
        </div>
        <div class="flex items-center gap-3">
          <span class="text-xs text-neutral-500">{{ validUrls.length }} / {{ items.length }}</span>
          <button @click="emit('close')" class="px-4 py-1.5 rounded-md text-sm bg-neutral-100 dark:bg-neutral-700">
            {{ t('common.cancel') }}
          </button>
          <button @click="handleStart" :disabled="validUrls.length === 0"
                  class="px-5 py-1.5 rounded-md text-sm bg-[var(--color-accent)] text-white font-medium disabled:opacity-50 transition-opacity">
            {{ t('batch_url_dialog.start') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
