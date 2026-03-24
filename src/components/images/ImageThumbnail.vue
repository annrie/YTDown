<script setup lang="ts">
import type { ScrapedImage } from '@/types'

defineProps<{
  image: ScrapedImage
  index: number
  selected: boolean
}>()

defineEmits<{
  'toggle-select': [index: number]
}>()
</script>

<template>
  <div
    class="relative group cursor-pointer rounded-lg overflow-hidden border-2 transition-all"
    :class="selected ? 'border-blue-500 ring-2 ring-blue-500/30' : 'border-transparent hover:border-neutral-400 dark:hover:border-neutral-600'"
    @click="$emit('toggle-select', index)"
  >
    <!-- Thumbnail image -->
    <div class="aspect-square bg-neutral-200 dark:bg-neutral-800">
      <img
        :src="image.url"
        :alt="image.alt || ''"
        class="w-full h-full object-cover"
        loading="lazy"
        @error="($event.target as HTMLImageElement).style.display = 'none'"
      />
    </div>

    <!-- Checkbox overlay -->
    <div class="absolute top-1.5 left-1.5">
      <div
        class="w-5 h-5 rounded border-2 flex items-center justify-center text-xs font-bold transition-colors"
        :class="selected
          ? 'bg-blue-500 border-blue-500 text-white'
          : 'bg-black/30 border-white/60 text-transparent group-hover:border-white'"
      >
        ✓
      </div>
    </div>

    <!-- Dimension label -->
    <div
      v-if="image.width && image.height"
      class="absolute bottom-0 inset-x-0 bg-black/60 text-white text-[10px] text-center py-0.5 opacity-0 group-hover:opacity-100 transition-opacity"
    >
      {{ image.width }}×{{ image.height }}
    </div>
  </div>
</template>
