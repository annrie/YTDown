import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Preset } from '../types'

export const usePresetsStore = defineStore('presets', () => {
  const presets = ref<Preset[]>([])

  async function fetchPresets() {
    presets.value = await invoke<Preset[]>('list_presets')
  }

  async function createPreset(payload: Omit<Preset, 'id' | 'created_at'>): Promise<number> {
    const id = await invoke<number>('create_preset', {
      name: payload.name,
      format: payload.format,
      quality: payload.quality,
      outputDir: payload.output_dir,
      embedThumbnail: payload.embed_thumbnail,
      embedMetadata: payload.embed_metadata,
      writeSubs: payload.write_subs,
      embedSubs: payload.embed_subs,
      embedChapters: payload.embed_chapters,
      sponsorblock: payload.sponsorblock,
    })
    await fetchPresets()
    return id
  }

  async function updatePreset(payload: Omit<Preset, 'created_at'>) {
    await invoke('update_preset', {
      id: payload.id,
      name: payload.name,
      format: payload.format,
      quality: payload.quality,
      outputDir: payload.output_dir,
      embedThumbnail: payload.embed_thumbnail,
      embedMetadata: payload.embed_metadata,
      writeSubs: payload.write_subs,
      embedSubs: payload.embed_subs,
      embedChapters: payload.embed_chapters,
      sponsorblock: payload.sponsorblock,
    })
    await fetchPresets()
  }

  async function deletePreset(id: number) {
    await invoke('delete_preset', { id })
    await fetchPresets()
  }

  return { presets, fetchPresets, createPreset, updatePreset, deletePreset }
})
