<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useLibraryStore } from '../../stores/library'
import { useFileManager } from '../../composables/useFileManager'
import { useI18n } from 'vue-i18n'
import type { Download } from '../../types'
import FileActions from './FileActions.vue'
import SelectionBar from './SelectionBar.vue'

const { t } = useI18n()

const props = defineProps<{ items: Download[] }>()

const libraryStore = useLibraryStore()
const { revealInFinder } = useFileManager()
const selectAllCheckbox = ref<HTMLInputElement | null>(null)

watch([() => libraryStore.hasSelection, () => libraryStore.isAllSelected], () => {
  if (selectAllCheckbox.value) {
    selectAllCheckbox.value.indeterminate = libraryStore.hasSelection && !libraryStore.isAllSelected
  }
})

const contextMenu = ref<{ show: boolean; x: number; y: number; item: Download | null }>({
  show: false, x: 0, y: 0, item: null
})

const sortedItems = computed(() => {
  const sorted = [...props.items]
  sorted.sort((a, b) => {
    const key = libraryStore.sortBy
    let aVal = a[key] ?? ''
    let bVal = b[key] ?? ''
    if (typeof aVal === 'string') aVal = aVal.toLowerCase()
    if (typeof bVal === 'string') bVal = bVal.toLowerCase()
    if (aVal < bVal) return libraryStore.sortOrder === 'asc' ? -1 : 1
    if (aVal > bVal) return libraryStore.sortOrder === 'asc' ? 1 : -1
    return 0
  })
  return sorted
})

function toggleSort(column: 'created_at' | 'title' | 'site') {
  if (libraryStore.sortBy === column) {
    libraryStore.sortOrder = libraryStore.sortOrder === 'asc' ? 'desc' : 'asc'
  } else {
    libraryStore.sortBy = column
    libraryStore.sortOrder = 'asc'
  }
}

function sortIndicator(column: string) {
  if (libraryStore.sortBy !== column) return ''
  return libraryStore.sortOrder === 'asc' ? ' ▲' : ' ▼'
}

function handleContextMenu(e: MouseEvent, item: Download) {
  e.preventDefault()
  contextMenu.value = { show: true, x: e.clientX, y: e.clientY, item }
}

function handleDoubleClick(item: Download) {
  if (item.file_path) {
    revealInFinder(item.file_path)
  }
}

function closeContextMenu() {
  contextMenu.value.show = false
}

function formatFileSize(bytes: number | null): string {
  if (!bytes) return '—'
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`
  return `${(bytes / 1073741824).toFixed(2)} GB`
}

function formatDate(dateStr: string): string {
  const d = new Date(dateStr)
  return d.toLocaleDateString(undefined, { year: 'numeric', month: '2-digit', day: '2-digit' })
}

function extractFileName(filePath: string | null): string {
  if (!filePath) return '—'
  const parts = filePath.split('/')
  return parts[parts.length - 1] ?? '—'
}
</script>

<template>
  <div>
    <SelectionBar />
    <table class="w-full text-sm">
      <thead>
        <tr class="text-left text-xs text-neutral-500 border-b border-[var(--color-separator)]">
          <th class="px-3 py-2 w-8">
            <input ref="selectAllCheckbox" type="checkbox"
                   :checked="libraryStore.isAllSelected"
                   @change="libraryStore.toggleSelectAll"
                   class="rounded border-neutral-300 dark:border-neutral-600 accent-[var(--color-accent)]" />
          </th>
          <th class="px-3 py-2 w-8"></th>
          <th class="px-3 py-2 cursor-pointer hover:text-neutral-700 dark:hover:text-neutral-300"
              @click="toggleSort('title')">
            {{ t('library.col_title') }}{{ sortIndicator('title') }}
          </th>
          <th class="px-3 py-2 cursor-pointer hover:text-neutral-700 dark:hover:text-neutral-300 w-32"
              @click="toggleSort('site')">
            {{ t('library.col_site') }}{{ sortIndicator('site') }}
          </th>
          <th class="px-3 py-2 w-24">{{ t('library.col_format') }}</th>
          <th class="px-3 py-2 w-48">{{ t('library.col_filename') }}</th>
          <th class="px-3 py-2 w-24">{{ t('library.col_size') }}</th>
          <th class="px-3 py-2 cursor-pointer hover:text-neutral-700 dark:hover:text-neutral-300 w-28"
              @click="toggleSort('created_at')">
            {{ t('library.col_date') }}{{ sortIndicator('created_at') }}
          </th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="item in sortedItems" :key="item.id"
            class="border-b border-[var(--color-separator)] hover:bg-neutral-50 dark:hover:bg-neutral-800/50 cursor-default"
            @contextmenu="handleContextMenu($event, item)"
            @dblclick="handleDoubleClick(item)">
          <td class="px-3 py-2 text-center">
            <input type="checkbox"
                   :checked="libraryStore.selectedIds.has(item.id)"
                   @change="libraryStore.toggleSelect(item.id)"
                   @click.stop
                   class="rounded border-neutral-300 dark:border-neutral-600 accent-[var(--color-accent)]" />
          </td>
          <td class="px-3 py-2 text-center">
            <button class="text-sm" :class="item.is_favorite ? 'text-yellow-500' : 'text-neutral-300 hover:text-yellow-400'"
                    @click.stop>
              ★
            </button>
          </td>
          <td class="px-3 py-2">
            <div class="flex items-center gap-2">
              <img v-if="item.thumbnail_url" :src="item.thumbnail_url"
                   class="w-10 h-7 object-cover rounded" />
              <div class="w-3 h-7 bg-neutral-200 dark:bg-neutral-700 rounded" v-else />
              <span class="truncate">{{ item.title || item.url }}</span>
            </div>
          </td>
          <td class="px-3 py-2 text-neutral-500">{{ item.site ?? '—' }}</td>
          <td class="px-3 py-2">
            <span class="px-1.5 py-0.5 text-xs rounded bg-neutral-100 dark:bg-neutral-700">
              {{ item.format?.toUpperCase() ?? '—' }}
            </span>
          </td>
          <td class="px-3 py-2 text-neutral-500 truncate max-w-[12rem]" :title="item.file_path ?? ''">{{ extractFileName(item.file_path) }}</td>
          <td class="px-3 py-2 text-neutral-500">{{ formatFileSize(item.file_size) }}</td>
          <td class="px-3 py-2 text-neutral-500">{{ formatDate(item.created_at) }}</td>
        </tr>
      </tbody>
    </table>

    <div v-if="sortedItems.length === 0" class="p-8 text-center text-neutral-400 text-sm">
      {{ t('library.empty_items') }}
    </div>

    <FileActions
      v-if="contextMenu.show && contextMenu.item"
      :item="contextMenu.item"
      :x="contextMenu.x"
      :y="contextMenu.y"
      @close="closeContextMenu"
    />
  </div>
</template>
