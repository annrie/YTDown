<script setup lang="ts">
import { ref } from 'vue'
import { useLibraryStore } from '../../stores/library'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
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
        {{ t('selection_bar.selected', { count: libraryStore.selectionCount }) }}
      </span>
      <button @click="libraryStore.toggleSelectAll"
              class="text-xs px-2 py-1 rounded hover:bg-neutral-200 dark:hover:bg-neutral-700">
        {{ libraryStore.isAllSelected ? t('selection_bar.cancel') : t('library.filter_all') }}
      </button>
      <button @click="libraryStore.clearSelection"
              class="text-xs px-2 py-1 rounded hover:bg-neutral-200 dark:hover:bg-neutral-700">
        {{ t('selection_bar.cancel') }}
      </button>
      <div class="flex-1" />
      <template v-if="!showDeleteConfirm">
        <button @click="showDeleteConfirm = true"
                class="text-xs px-3 py-1 rounded bg-red-500 text-white hover:bg-red-600 transition-colors">
          {{ t('common.delete') }}
        </button>
      </template>
      <template v-else>
        <button @click="handleDelete(true)"
                class="text-xs px-3 py-1 rounded bg-orange-500 text-white hover:bg-orange-600 transition-colors">
          Trash
        </button>
        <button @click="handleDelete(false)"
                class="text-xs px-3 py-1 rounded bg-red-600 text-white hover:bg-red-700 transition-colors">
          {{ t('common.delete') }}
        </button>
        <button @click="showDeleteConfirm = false"
                class="text-xs px-2 py-1 rounded hover:bg-neutral-200 dark:hover:bg-neutral-700">
          {{ t('common.cancel') }}
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
