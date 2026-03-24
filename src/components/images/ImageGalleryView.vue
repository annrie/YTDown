<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { convertFileSrc } from '@tauri-apps/api/core'
import { ask } from '@tauri-apps/plugin-dialog'
import { useImagesStore } from '../../stores/images'
import type { ImageRecord } from '../../types'
import ImageSlideshow from './ImageSlideshow.vue'

const imagesStore = useImagesStore()

const thumbnailSize = ref(150)
const slideshowOpen = ref(false)
const slideshowImages = ref<ImageRecord[]>([])
const slideshowStartIndex = ref(0)
const expandedSessionId = ref<number | null>(null)
const sessionImagesMap = ref<Map<number, ImageRecord[]>>(new Map())

onMounted(async () => {
  await imagesStore.loadSessions()
})

async function toggleSession(sessionId: number) {
  if (expandedSessionId.value === sessionId) {
    expandedSessionId.value = null
    return
  }
  expandedSessionId.value = sessionId
  if (!sessionImagesMap.value.has(sessionId)) {
    const images = await imagesStore.loadSessionImages(sessionId)
    sessionImagesMap.value.set(sessionId, images)
  }
}

function getSessionImages(sessionId: number): ImageRecord[] {
  return sessionImagesMap.value.get(sessionId) || []
}

function completedImages(sessionId: number): ImageRecord[] {
  return getSessionImages(sessionId).filter((img) => img.status === 'completed' && img.file_path)
}

function openSlideshow(sessionId: number, startIndex = 0) {
  slideshowImages.value = completedImages(sessionId)
  slideshowStartIndex.value = startIndex
  slideshowOpen.value = true
}

async function handleDeleteSession(sessionId: number) {
  const confirmed = await ask('このセッションを削除しますか？\n\nファイルも削除されます。', {
    title: 'セッション削除',
    kind: 'warning',
  })
  if (confirmed) {
    await imagesStore.deleteSession(sessionId, true)
    sessionImagesMap.value.delete(sessionId)
    if (expandedSessionId.value === sessionId) {
      expandedSessionId.value = null
    }
  }
}

async function revealInFinder(filePath: string) {
  await invoke('reveal_in_finder', { path: filePath })
}

function getImageSrc(record: ImageRecord): string {
  return record.file_path ? convertFileSrc(record.file_path) : ''
}
</script>

<template>
  <div class="flex flex-col h-full p-4 overflow-y-auto">
    <!-- Header with size slider -->
    <div class="flex items-center justify-between mb-4">
      <h2 class="text-lg font-medium text-neutral-800 dark:text-neutral-200">画像ギャラリー</h2>
      <div class="flex items-center gap-3">
        <label class="flex items-center gap-2 text-sm text-neutral-500">
          <span class="text-xs">小</span>
          <input
            v-model.number="thumbnailSize"
            type="range"
            min="80"
            max="300"
            step="10"
            class="w-24 accent-blue-500"
          />
          <span class="text-xs">大</span>
        </label>
      </div>
    </div>

    <!-- Empty state -->
    <div
      v-if="imagesStore.sessions.length === 0"
      class="flex-1 flex items-center justify-center text-neutral-400"
    >
      <div class="text-center">
        <div class="text-4xl mb-2">📂</div>
        <p>ダウンロードした画像はありません</p>
      </div>
    </div>

    <!-- Session groups -->
    <div v-for="session in imagesStore.sessions" :key="session.id" class="mb-4">
      <!-- Session header -->
      <div
        class="flex items-center justify-between px-3 py-2 rounded-lg bg-neutral-100 dark:bg-neutral-800 cursor-pointer hover:bg-neutral-200 dark:hover:bg-neutral-700"
        @click="toggleSession(session.id)"
      >
        <div class="flex items-center gap-2">
          <span class="text-sm">{{ expandedSessionId === session.id ? '▼' : '▶' }}</span>
          <span class="text-sm font-medium">{{ session.site_name || session.source_url }}</span>
          <span class="text-xs text-neutral-400">{{ session.image_count }}枚</span>
        </div>
        <div class="flex items-center gap-2">
          <button
            v-if="expandedSessionId === session.id && completedImages(session.id).length > 0"
            class="px-2 py-1 text-xs rounded bg-blue-500 text-white hover:bg-blue-600"
            @click.stop="openSlideshow(session.id)"
          >
            ▶ スライドショー
          </button>
          <button
            class="px-2 py-1 text-xs rounded bg-neutral-300 dark:bg-neutral-600 hover:bg-red-500 hover:text-white"
            @click.stop="handleDeleteSession(session.id)"
          >
            削除
          </button>
        </div>
      </div>

      <!-- Session images grid -->
      <div
        v-if="expandedSessionId === session.id"
        class="mt-2 grid gap-2"
        :style="{ gridTemplateColumns: `repeat(auto-fill, minmax(${thumbnailSize}px, 1fr))` }"
      >
        <div
          v-for="(img, idx) in completedImages(session.id)"
          :key="img.id"
          class="relative group cursor-pointer rounded-lg overflow-hidden bg-neutral-200 dark:bg-neutral-800"
          :style="{ height: `${thumbnailSize}px` }"
          @click="openSlideshow(session.id, idx)"
        >
          <img
            :src="getImageSrc(img)"
            :alt="img.filename || ''"
            class="w-full h-full object-cover"
            loading="lazy"
          />
          <div class="absolute bottom-0 inset-x-0 bg-black/60 text-white text-[10px] text-center py-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
            {{ img.filename }}
          </div>
        </div>
      </div>
    </div>

    <!-- Slideshow -->
    <ImageSlideshow
      :images="slideshowImages"
      :start-index="slideshowStartIndex"
      :open="slideshowOpen"
      @close="slideshowOpen = false"
      @reveal-in-finder="revealInFinder"
    />
  </div>
</template>
