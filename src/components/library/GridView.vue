<script setup lang="ts">
import { ref } from 'vue'
import { useLibraryStore } from '../../stores/library'
import { useFileManager } from '../../composables/useFileManager'
import type { Download } from '../../types'
import FileActions from './FileActions.vue'
import SelectionBar from './SelectionBar.vue'

defineProps<{ items: Download[] }>()

const libraryStore = useLibraryStore()
const { revealInFinder } = useFileManager()

const contextMenu = ref<{ show: boolean; x: number; y: number; item: Download | null }>({
  show: false, x: 0, y: 0, item: null
})

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

function formatDuration(secs: number | null): string {
  if (!secs) return ''
  const h = Math.floor(secs / 3600)
  const m = Math.floor((secs % 3600) / 60)
  const s = secs % 60
  if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
  return `${m}:${String(s).padStart(2, '0')}`
}
</script>

<template>
  <div>
    <SelectionBar />
    <div v-if="items.length === 0" class="p-8 text-center text-neutral-400 text-sm">
      ライブラリにアイテムがありません
    </div>

    <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4 p-4">
      <div v-for="item in items" :key="item.id"
           class="group relative rounded-lg overflow-hidden bg-neutral-100 dark:bg-neutral-800 cursor-default"
           @contextmenu="handleContextMenu($event, item)"
           @dblclick="handleDoubleClick(item)">
        <!-- Thumbnail -->
        <div class="relative aspect-video bg-neutral-200 dark:bg-neutral-700">
          <img v-if="item.thumbnail_url" :src="item.thumbnail_url"
               class="w-full h-full object-cover" />
          <div v-else class="w-full h-full flex items-center justify-center text-neutral-400 text-2xl">
            ▶
          </div>
          <!-- Duration badge -->
          <span v-if="item.duration" class="absolute bottom-1 right-1 px-1 py-0.5 text-[10px] bg-black/70 text-white rounded">
            {{ formatDuration(item.duration) }}
          </span>
          <!-- Format badge -->
          <span class="absolute top-1 left-7 px-1 py-0.5 text-[10px] bg-black/70 text-white rounded uppercase">
            {{ item.format ?? '—' }}
          </span>
          <!-- Checkbox -->
          <div class="absolute top-1 left-1 z-20"
               :class="libraryStore.selectedIds.has(item.id) || libraryStore.hasSelection ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'"
               style="transition: opacity 0.15s">
            <input type="checkbox"
                   :checked="libraryStore.selectedIds.has(item.id)"
                   @change="libraryStore.toggleSelect(item.id)"
                   @click.stop
                   class="rounded border-white/70 accent-[var(--color-accent)] w-4 h-4" />
          </div>
          <!-- Hover overlay -->
          <div class="absolute inset-0 bg-black/0 group-hover:bg-black/20 transition-colors flex items-center justify-center z-10">
            <button class="opacity-0 group-hover:opacity-100 transition-opacity px-3 py-1 bg-white/90 dark:bg-neutral-800/90 rounded-md text-xs font-medium"
                    @click.stop="handleDoubleClick(item)">
              開く
            </button>
          </div>
        </div>
        <!-- Info -->
        <div class="p-2">
          <p class="text-xs font-medium line-clamp-2 leading-tight">{{ item.title || item.url }}</p>
          <p class="text-[10px] text-neutral-500 mt-1 truncate">{{ item.channel ?? item.site ?? '' }}</p>
          <p v-if="item.file_path" class="text-[10px] text-neutral-400 mt-0.5 truncate" :title="item.file_path">
            {{ item.file_path.split('/').pop() }}
          </p>
        </div>
        <!-- Favorite -->
        <button class="absolute top-1 right-1 text-sm opacity-0 group-hover:opacity-100 transition-opacity"
                :class="item.is_favorite ? 'text-yellow-500 opacity-100' : 'text-white/70'"
                @click.stop>
          ★
        </button>
      </div>
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
