import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Download, ViewMode } from '../types'

export const useLibraryStore = defineStore('library', () => {
  const items = ref<Download[]>([])
  const viewMode = ref<ViewMode>('list')
  const searchQuery = ref('')
  const filterFormat = ref<string | null>(null)
  const filterSite = ref<string | null>(null)
  const sortBy = ref<'created_at' | 'title' | 'site'>('created_at')
  const sortOrder = ref<'asc' | 'desc'>('desc')
  const selectedIds = ref<Set<number>>(new Set())

  const filteredItems = computed(() => {
    let result = items.value.filter(d => d.status === 'completed')
    if (filterFormat.value) {
      result = result.filter(d => d.format === filterFormat.value)
    }
    if (filterSite.value) {
      result = result.filter(d => d.site === filterSite.value)
    }
    return result
  })

  const sites = computed(() =>
    [...new Set(items.value.map(d => d.site).filter(Boolean))] as string[]
  )

  const channels = computed(() => {
    const map = new Map<string, { channel: string; channel_id: string | null; site: string | null }>()
    for (const d of items.value) {
      if (d.channel) {
        const key = `${d.site}:${d.channel_id || d.channel}`
        if (!map.has(key)) {
          map.set(key, { channel: d.channel, channel_id: d.channel_id, site: d.site })
        }
      }
    }
    return [...map.values()]
  })

  const hasSelection = computed(() => selectedIds.value.size > 0)
  const selectionCount = computed(() => selectedIds.value.size)
  const isAllSelected = computed(() => {
    if (filteredItems.value.length === 0) return false
    return filteredItems.value.every(item => selectedIds.value.has(item.id))
  })

  function toggleSelect(id: number) {
    const next = new Set(selectedIds.value)
    if (next.has(id)) {
      next.delete(id)
    } else {
      next.add(id)
    }
    selectedIds.value = next
  }

  function toggleSelectAll() {
    if (isAllSelected.value) {
      selectedIds.value = new Set()
    } else {
      selectedIds.value = new Set(filteredItems.value.map(item => item.id))
    }
  }

  function clearSelection() {
    selectedIds.value = new Set()
  }

  async function deleteSelected(toTrash: boolean) {
    const ids = [...selectedIds.value]
    for (const id of ids) {
      const item = items.value.find(d => d.id === id)
      try {
        await invoke('delete_file', {
          path: item?.file_path ?? null,
          toTrash,
          downloadId: id,
        })
      } catch (e) {
        console.error(`Failed to delete item ${id}:`, e)
      }
    }
    clearSelection()
    await loadItems()
  }

  async function loadItems() {
    try {
      items.value = await invoke<Download[]>('list_library', { statusFilter: null })
    } catch (e) {
      console.error('Failed to load library:', e)
    }
  }

  return {
    items,
    viewMode,
    searchQuery,
    filterFormat,
    filterSite,
    sortBy,
    sortOrder,
    selectedIds,
    filteredItems,
    sites,
    channels,
    hasSelection,
    selectionCount,
    isAllSelected,
    toggleSelect,
    toggleSelectAll,
    clearSelection,
    deleteSelected,
    loadItems,
  }
})
