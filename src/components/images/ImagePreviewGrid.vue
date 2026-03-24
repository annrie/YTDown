<script setup lang="ts">
import type { ScrapedImage } from '@/types'
import ImageThumbnail from './ImageThumbnail.vue'

defineProps<{
  images: ScrapedImage[]
  selectedIds: Set<number>
}>()

defineEmits<{
  'toggle-select': [index: number]
  'select-all': []
  'deselect-all': []
}>()
</script>

<template>
  <div>
    <!-- Selection toolbar -->
    <div class="flex items-center justify-between mb-3 text-sm">
      <span class="text-neutral-500 dark:text-neutral-400">
        {{ images.length }} 枚の画像が見つかりました
        <span v-if="selectedIds.size > 0" class="text-blue-500 font-medium">
          （{{ selectedIds.size }} 枚選択中）
        </span>
      </span>
      <div class="flex gap-2">
        <button
          class="px-2 py-1 text-xs rounded bg-neutral-200 dark:bg-neutral-700 hover:bg-neutral-300 dark:hover:bg-neutral-600"
          @click="$emit('select-all')"
        >
          全選択
        </button>
        <button
          class="px-2 py-1 text-xs rounded bg-neutral-200 dark:bg-neutral-700 hover:bg-neutral-300 dark:hover:bg-neutral-600"
          @click="$emit('deselect-all')"
        >
          全解除
        </button>
      </div>
    </div>

    <!-- Grid -->
    <div class="grid grid-cols-4 sm:grid-cols-5 md:grid-cols-6 lg:grid-cols-8 gap-2">
      <ImageThumbnail
        v-for="(image, index) in images"
        :key="image.url"
        :image="image"
        :index="index"
        :selected="selectedIds.has(index)"
        @toggle-select="$emit('toggle-select', $event)"
      />
    </div>
  </div>
</template>
