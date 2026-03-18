import { invoke } from '@tauri-apps/api/core'

export function useFileManager() {
  async function moveFile(source: string, destination: string, downloadId?: number) {
    return invoke('move_file', { source, destination, downloadId: downloadId ?? null })
  }

  async function deleteFile(path: string | null, toTrash: boolean = true, downloadId?: number) {
    return invoke('delete_file', { path, toTrash, downloadId: downloadId ?? null })
  }

  async function revealInFinder(path: string) {
    return invoke('reveal_in_finder', { path })
  }

  return { moveFile, deleteFile, revealInFinder }
}
