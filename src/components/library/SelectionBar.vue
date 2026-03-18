<script setup lang="ts">
import { ref } from 'vue'
import { useLibraryStore } from '../../stores/library'

const libraryStore = useLibraryStore()
const showDeleteConfirm = ref(false)

function handleDelete(toTrash: boolean) {
  libraryStore.deleteSelected(toTrash)
  showDeleteConfirm.value = false
}
</script>

<template>
  <Transition name="slide">
    <div v-if="libraryStore.hasSelection"
         class="flex items-center gap-3 px-4 py-2 bg-[var(--color-accent)]/10 border-b border-[var(--color-separator)]">
      <span class="text-sm font-medium">
        {{ libraryStore.selectionCount }}件選択中
      </span>
      <button @click="libraryStore.toggleSelectAll"
              class="text-xs px-2 py-1 rounded hover:bg-neutral-200 dark:hover:bg-neutral-700">
        {{ libraryStore.isAllSelected ? '全選択解除' : '全選択' }}
      </button>
      <button @click="libraryStore.clearSelection"
              class="text-xs px-2 py-1 rounded hover:bg-neutral-200 dark:hover:bg-neutral-700">
        選択解除
      </button>
      <div class="flex-1" />
      <template v-if="!showDeleteConfirm">
        <button @click="showDeleteConfirm = true"
                class="text-xs px-3 py-1 rounded bg-red-500 text-white hover:bg-red-600 transition-colors">
          削除
        </button>
      </template>
      <template v-else>
        <span class="text-xs text-neutral-500">削除方法:</span>
        <button @click="handleDelete(true)"
                class="text-xs px-3 py-1 rounded bg-orange-500 text-white hover:bg-orange-600 transition-colors">
          ゴミ箱に移動
        </button>
        <button @click="handleDelete(false)"
                class="text-xs px-3 py-1 rounded bg-red-600 text-white hover:bg-red-700 transition-colors">
          完全に削除
        </button>
        <button @click="showDeleteConfirm = false"
                class="text-xs px-2 py-1 rounded hover:bg-neutral-200 dark:hover:bg-neutral-700">
          キャンセル
        </button>
      </template>
    </div>
  </Transition>
</template>

<style scoped>
.slide-enter-active,
.slide-leave-active {
  transition: all 0.2s ease;
}
.slide-enter-from,
.slide-leave-to {
  opacity: 0;
  transform: translateY(-100%);
}
</style>
