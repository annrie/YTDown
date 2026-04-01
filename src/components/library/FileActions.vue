<script setup lang="ts">
import { ref, nextTick, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { message } from '@tauri-apps/plugin-dialog'
import { useFileManager } from '../../composables/useFileManager'
import { useLibraryStore } from '../../stores/library'
import type { Download } from '../../types'

const props = defineProps<{
  item: Download
  x: number
  y: number
}>()

const emit = defineEmits<{ close: [] }>()

const { moveFile, deleteFile, revealInFinder } = useFileManager()
const libraryStore = useLibraryStore()

const showDeleteConfirm = ref(false)
const menuRef = ref<HTMLElement | null>(null)

function handleClickOutside(e: MouseEvent) {
  // メニュー内のクリックなら閉じない
  if (menuRef.value && menuRef.value.contains(e.target as Node)) {
    return
  }
  emit('close')
}

onMounted(() => {
  // nextTickで遅延させて、右クリックイベント自体で即閉じるのを防ぐ
  nextTick(() => {
    document.addEventListener('click', handleClickOutside, true)
    document.addEventListener('contextmenu', handleClickOutside, true)
  })
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside, true)
  document.removeEventListener('contextmenu', handleClickOutside, true)
})

async function handleReveal() {
  if (props.item.file_path) {
    await revealInFinder(props.item.file_path)
  }
  emit('close')
}

async function handleDelete(toTrash: boolean) {
  await deleteFile(props.item.file_path ?? null, toTrash, props.item.id)
  await libraryStore.loadItems()
  emit('close')
}

async function handleMove() {
  if (!props.item.file_path) {
    await message('ファイルパスが記録されていないため移動できません。', { title: 'エラー', kind: 'error' })
    emit('close')
    return
  }

  const selected = await open({
    directory: true,
    multiple: false,
    title: '移動先フォルダを選択',
  })

  if (selected) {
    const sourcePath = props.item.file_path
    const fileName = sourcePath.split('/').pop() ?? sourcePath
    const destPath = `${selected}/${fileName}`
    try {
      await moveFile(sourcePath, destPath, props.item.id)
      await libraryStore.loadItems()
    } catch (e) {
      await message(`ファイルの移動に失敗しました: ${e}`, { title: 'エラー', kind: 'error' })
    }
  }
  emit('close')
}

async function handleFavorite() {
  try {
    await invoke('toggle_favorite', { id: props.item.id })
    await libraryStore.loadItems()
  } catch (e) {
    console.error('Failed to toggle favorite:', e)
  }
  emit('close')
}

function handleAddToPlaylist() {
  // TODO: Show playlist picker
  emit('close')
}
</script>

<template>
  <div ref="menuRef"
       class="fixed z-50 bg-white dark:bg-neutral-800 rounded-lg shadow-xl border border-[var(--color-separator)] py-1 min-w-[180px] text-sm"
       :style="{ left: `${x}px`, top: `${y}px` }"
       @click.stop
       @contextmenu.stop.prevent>
    <button class="w-full text-left px-3 py-1.5 hover:bg-neutral-100 dark:hover:bg-neutral-700 dark:text-neutral-200"
            @click="handleReveal">
      Finderで表示
    </button>
    <button class="w-full text-left px-3 py-1.5 hover:bg-neutral-100 dark:hover:bg-neutral-700 dark:text-neutral-200"
            @click="handleMove">
      移動...
    </button>
    <div class="border-t border-[var(--color-separator)] my-1" />
    <button class="w-full text-left px-3 py-1.5 hover:bg-neutral-100 dark:hover:bg-neutral-700 dark:text-neutral-200"
            @click="handleFavorite">
      {{ item.is_favorite ? 'お気に入り解除' : 'お気に入りに追加' }}
    </button>
    <button class="w-full text-left px-3 py-1.5 hover:bg-neutral-100 dark:hover:bg-neutral-700 dark:text-neutral-200"
            @click="handleAddToPlaylist">
      プレイリストに追加...
    </button>
    <div class="border-t border-[var(--color-separator)] my-1" />
    <template v-if="!showDeleteConfirm">
      <button class="w-full text-left px-3 py-1.5 hover:bg-red-50 dark:hover:bg-red-900/20 text-red-500"
              @click="showDeleteConfirm = true">
        削除
      </button>
    </template>
    <template v-else>
      <button class="w-full text-left px-3 py-1.5 hover:bg-red-50 dark:hover:bg-red-900/20 text-red-500"
              @click="handleDelete(true)">
        ゴミ箱に移動
      </button>
      <button class="w-full text-left px-3 py-1.5 hover:bg-red-50 dark:hover:bg-red-900/20 text-red-600 font-medium"
              @click="handleDelete(false)">
        完全に削除
      </button>
    </template>
  </div>
</template>
