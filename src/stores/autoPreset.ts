import { ref } from 'vue'
import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { AutoPresetRule, Preset } from '../types'

export const useAutoPresetStore = defineStore('autoPreset', () => {
  const rules = ref<AutoPresetRule[]>([])

  async function fetchRules() {
    try {
      rules.value = await invoke<AutoPresetRule[]>('list_auto_preset_rules')
    } catch (e) {
      console.error('Failed to fetch auto preset rules:', e)
    }
  }

  async function createRule(domain: string, presetId: number, enabled: boolean) {
    const id = await invoke<number>('create_auto_preset_rule', { domain, presetId, enabled })
    await fetchRules()
    return id
  }

  async function updateRule(id: number, domain: string, presetId: number, enabled: boolean) {
    await invoke('update_auto_preset_rule', { id, domain, presetId, enabled })
    await fetchRules()
  }

  async function deleteRule(id: number) {
    await invoke('delete_auto_preset_rule', { id })
    rules.value = rules.value.filter(r => r.id !== id)
  }

  async function resolvePresetForUrl(url: string): Promise<Preset | null> {
    try {
      return await invoke<Preset | null>('resolve_preset_for_url', { url })
    } catch {
      return null
    }
  }

  return { rules, fetchRules, createRule, updateRule, deleteRule, resolvePresetForUrl }
})
