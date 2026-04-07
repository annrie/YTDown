<script setup lang="ts">
import { ref, nextTick, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { message } from '@tauri-apps/plugin-dialog'
import { useFileManager } from '../../composables/useFileManager'
import { useLibraryStore } from '../../stores/library'
import { useI18n } from 'vue-i18n'
import type { Download } from '../../types'

const { t } = useI18n()

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
  if (menuRef.value && menuRef.value.contains(e.target as Node)) {
    return
  }
  emit('close')
}

onMounted(() => {
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
    await message(t('file_actions.no_path_error'), { title: t('common.error'), kind: 'error' })
    emit('close')
    return
  }

  const selected = await open({
    directory: true,
    multiple: false,
    title: t('file_actions.move_title'),
  })

  if (selected) {
    const sourcePath = props.item.file_path
    const fileName = sourcePath.split('/').pop() ?? sourcePath
    const destPath = `${selected}/${fileName}`
    try {
      await moveFile(sourcePath, destPath, props.item.id)
      await libraryStore.loadItems()
    } catch (e) {
      await message(t('file_actions.move_error', { error: e }), { title: t('common.error'), kind: 'error' })
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
      {{ t('file_actions.reveal') }}
    </button>
    <button class="w-full text-left px-3 py-1.5 hover:bg-neutral-100 dark:hover:bg-neutral-700 dark:text-neutral-200"
            @click="handleMove">
      {{ t('file_actions.move') }}
    </button>
    <div class="border-t border-[var(--color-separator)] my-1" />
    <button class="w-full text-left px-3 py-1.5 hover:bg-neutral-100 dark:hover:bg-neutral-700 dark:text-neutral-200"
            @click="handleFavorite">
      {{ item.is_favorite ? t('file_actions.unfavorite') : t('file_actions.favorite') }}
    </button>
    <button class="w-full text-left px-3 py-1.5 hover:bg-neutral-100 dark:hover:bg-neutral-700 dark:text-neutral-200"
            @click="handleAddToPlaylist">
      {{ t('file_actions.add_to_playlist') }}
    </button>
    <div class="border-t border-[var(--color-separator)] my-1" />
    <template v-if="!showDeleteConfirm">
      <button class="w-full text-left px-3 py-1.5 hover:bg-red-50 dark:hover:bg-red-900/20 text-red-500"
              @click="showDeleteConfirm = true">
        {{ t('file_actions.delete') }}
      </button>
    </template>
    <template v-else>
      <button class="w-full text-left px-3 py-1.5 hover:bg-red-50 dark:hover:bg-red-900/20 text-red-500"
              @click="handleDelete(true)">
        {{ t('file_actions.trash') }}
      </button>
      <button class="w-full text-left px-3 py-1.5 hover:bg-red-50 dark:hover:bg-red-900/20 text-red-600 font-medium"
              @click="handleDelete(false)">
        {{ t('file_actions.delete_permanently') }}
      </button>
    </template>
  </div>
</template>
