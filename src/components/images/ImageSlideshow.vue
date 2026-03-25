<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, computed } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import type { ImageRecord } from '../../types'

const props = defineProps<{
  images: ImageRecord[]
  startIndex: number
  open: boolean
}>()

const emit = defineEmits<{
  close: []
  'reveal-in-finder': [filePath: string]
}>()

const currentIndex = ref(0)
const playing = ref(false)
const interval = ref(3)
const showControls = ref(true)
let playTimer: ReturnType<typeof setInterval> | null = null
let hideTimer: ReturnType<typeof setTimeout> | null = null

const currentImage = computed(() => props.images[currentIndex.value])
const imageSrc = computed(() => {
  const path = currentImage.value?.file_path
  return path ? convertFileSrc(path) : ''
})

watch(() => props.open, (open) => {
  if (open) {
    currentIndex.value = props.startIndex
    showControls.value = true
    resetHideTimer()
  } else {
    stopPlaying()
  }
})

function next() {
  if (currentIndex.value < props.images.length - 1) {
    currentIndex.value++
  } else {
    stopPlaying()
  }
}

function prev() {
  if (currentIndex.value > 0) {
    currentIndex.value--
  }
}

function togglePlay() {
  if (playing.value) {
    stopPlaying()
  } else {
    startPlaying()
  }
}

function startPlaying() {
  playing.value = true
  playTimer = setInterval(() => {
    if (currentIndex.value < props.images.length - 1) {
      currentIndex.value++
    } else {
      stopPlaying()
    }
  }, interval.value * 1000)
}

function stopPlaying() {
  playing.value = false
  if (playTimer) {
    clearInterval(playTimer)
    playTimer = null
  }
}

function setInterval_(sec: number) {
  interval.value = sec
  if (playing.value) {
    stopPlaying()
    startPlaying()
  }
}

function resetHideTimer() {
  showControls.value = true
  if (hideTimer) clearTimeout(hideTimer)
  hideTimer = setTimeout(() => {
    showControls.value = false
  }, 3000)
}

function handleMouseMove() {
  resetHideTimer()
}

function handleKeydown(e: KeyboardEvent) {
  switch (e.key) {
    case 'ArrowLeft':
      prev()
      break
    case 'ArrowRight':
      next()
      break
    case ' ':
      e.preventDefault()
      togglePlay()
      break
    case 'Escape':
      emit('close')
      break
  }
  resetHideTimer()
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
  stopPlaying()
  if (hideTimer) clearTimeout(hideTimer)
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-[9999] bg-black flex items-center justify-center select-none"
      @mousemove="handleMouseMove"
    >
      <!-- Image -->
      <img
        v-if="imageSrc"
        :src="imageSrc"
        :alt="currentImage?.filename || ''"
        class="max-w-full max-h-full object-contain"
      />

      <!-- Controls overlay -->
      <div
        class="absolute inset-0 transition-opacity duration-300"
        :class="showControls ? 'opacity-100' : 'opacity-0 pointer-events-none'"
      >
        <!-- Top bar -->
        <div class="absolute top-0 inset-x-0 flex justify-between items-center p-4">
          <div class="bg-black/50 px-3 py-1.5 rounded-lg text-white text-sm">
            {{ currentIndex + 1 }} / {{ images.length }}
          </div>
          <button
            class="bg-black/50 px-3 py-1.5 rounded-lg text-white text-sm hover:bg-black/70"
            @click="emit('close')"
          >
            ✕
          </button>
        </div>

        <!-- Prev / Next buttons -->
        <button
          v-if="currentIndex > 0"
          class="absolute left-4 top-1/2 -translate-y-1/2 bg-black/50 w-10 h-10 rounded-full text-white text-lg flex items-center justify-center hover:bg-black/70"
          @click="prev"
        >
          ◀
        </button>
        <button
          v-if="currentIndex < images.length - 1"
          class="absolute right-4 top-1/2 -translate-y-1/2 bg-black/50 w-10 h-10 rounded-full text-white text-lg flex items-center justify-center hover:bg-black/70"
          @click="next"
        >
          ▶
        </button>

        <!-- Bottom bar -->
        <div class="absolute bottom-0 inset-x-0 flex justify-center items-center gap-3 p-4">
          <button
            class="bg-black/50 px-3 py-1.5 rounded-lg text-white text-sm hover:bg-black/70"
            @click="togglePlay"
          >
            {{ playing ? '⏸' : '▶' }} {{ interval }}秒
          </button>
          <select
            :value="interval"
            class="bg-black/50 px-2 py-1.5 rounded-lg text-white text-sm border-none"
            @change="setInterval_(Number(($event.target as HTMLSelectElement).value))"
          >
            <option value="1">1秒</option>
            <option value="3">3秒</option>
            <option value="5">5秒</option>
            <option value="10">10秒</option>
          </select>
          <button
            v-if="currentImage?.file_path"
            class="bg-black/50 px-3 py-1.5 rounded-lg text-white text-sm hover:bg-black/70"
            @click="emit('reveal-in-finder', currentImage.file_path!)"
          >
            📂 Finder
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
