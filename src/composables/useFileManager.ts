import { invoke } from '@tauri-apps/api/core'

export function useFileManager() {
  async function moveFile(source: string, destination: string, downloadId?: number): Promise<void> {
    try {
      await invoke('move_file', { source, destination, downloadId: downloadId ?? null })
    } catch (e) {
      console.error('[useFileManager] move_file failed:', e)
      throw e
    }
  }

  async function deleteFile(path: string | null, toTrash: boolean = true, downloadId?: number): Promise<void> {
    try {
      await invoke('delete_file', { path, toTrash, downloadId: downloadId ?? null })
    } catch (e) {
      console.error('[useFileManager] delete_file failed:', e)
      throw e
    }
  }

  async function revealInFinder(path: string): Promise<void> {
    try {
      await invoke('reveal_in_finder', { path })
    } catch (e) {
      console.error('[useFileManager] reveal_in_finder failed:', e)
      throw e
    }
  }

  return { moveFile, deleteFile, revealInFinder }
}
