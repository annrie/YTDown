<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { UrlHistoryEntry } from '../../types'

const props = defineProps<{
  type: 'video' | 'image'
}>()

const emit = defineEmits<{
  select: [url: string]
}>()

const open = ref(false)
const items = ref<UrlHistoryEntry[]>([])
const dropdownRef = ref<HTMLElement | null>(null)
const buttonRef = ref<HTMLElement | null>(null)

async function toggle() {
  if (open.value) {
    open.value = false
    return
  }
  try {
    items.value = await invoke<UrlHistoryEntry[]>('get_url_history', { historyType: props.type })
  } catch (e) {
    console.error('Failed to fetch URL history:', e)
    items.value = []
  }
  open.value = true
}

function selectItem(url: string) {
  emit('select', url)
  open.value = false
}

function handleClickOutside(e: MouseEvent) {
  if (!open.value) return
  const target = e.target as Node
  if (dropdownRef.value?.contains(target) || buttonRef.value?.contains(target)) return
  open.value = false
}

onMounted(() => document.addEventListener('click', handleClickOutside))
onUnmounted(() => document.removeEventListener('click', handleClickOutside))
</script>

<template>
  <div class="relative">
    <button
      ref="buttonRef"
      @click="toggle"
      class="w-8 h-8 flex items-center justify-center rounded-md hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-colors text-neutral-500 hover:text-[var(--color-accent)]"
      title="URL履歴"
    >
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
    </button>

    <div
      v-if="open"
      ref="dropdownRef"
      class="absolute top-full mt-1 right-0 w-80 max-h-64 overflow-y-auto rounded-lg bg-white dark:bg-neutral-800 shadow-lg border border-[var(--color-separator)] z-50"
    >
      <div v-if="items.length === 0" class="px-3 py-4 text-center text-xs text-neutral-400">
        履歴がありません
      </div>
      <button
        v-for="item in items"
        :key="item.id"
        class="w-full text-left px-3 py-2 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-700 transition-colors border-b border-[var(--color-separator)] last:border-b-0"
        @click="selectItem(item.url)"
      >
        <span class="block truncate font-mono">{{ item.url }}</span>
        <span class="block text-[10px] text-neutral-400 mt-0.5">{{ item.created_at }}</span>
      </button>
    </div>
  </div>
</template>
