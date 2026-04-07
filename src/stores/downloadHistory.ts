import { ref } from 'vue'
import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { HistoryEntry } from '../types'

export const useDownloadHistoryStore = defineStore('downloadHistory', () => {
  const entries = ref<HistoryEntry[]>([])

  async function fetchHistory(limit?: number) {
    try {
      entries.value = await invoke<HistoryEntry[]>('list_history', { limit })
    } catch (e) {
      console.error('Failed to fetch history:', e)
    }
  }

  async function deleteEntry(id: number) {
    await invoke('delete_history_entry', { id })
    entries.value = entries.value.filter(e => e.id !== id)
  }

  async function clearAll() {
    await invoke('clear_history')
    entries.value = []
  }

  return { entries, fetchHistory, deleteEntry, clearAll }
})
