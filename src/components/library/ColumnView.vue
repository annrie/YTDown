<script setup lang="ts">
import { ref, computed } from 'vue'
import { useLibraryStore } from '../../stores/library'
import { useFileManager } from '../../composables/useFileManager'
import type { Download } from '../../types'
import FileActions from './FileActions.vue'
import SelectionBar from './SelectionBar.vue'

const props = defineProps<{ items: Download[] }>()

const libraryStore = useLibraryStore()
const { revealInFinder } = useFileManager()

const selectedSite = ref<string | null>(null)
const selectedChannel = ref<string | null>(null)

const contextMenu = ref<{ show: boolean; x: number; y: number; item: Download | null }>({
  show: false, x: 0, y: 0, item: null
})

// Column 1: Sites
const siteList = computed(() => {
  const sites = new Map<string, number>()
  for (const item of props.items) {
    const site = item.site ?? '不明'
    sites.set(site, (sites.get(site) ?? 0) + 1)
  }
  return [...sites.entries()].map(([name, count]) => ({ name, count }))
})

// Column 2: Channels for selected site
const channelList = computed(() => {
  if (!selectedSite.value) return []
  const channels = new Map<string, { name: string; id: string | null; count: number }>()
  for (const item of props.items) {
    const site = item.site ?? '不明'
    if (site !== selectedSite.value) continue
    const channelName = item.channel ?? '不明'
    const key = item.channel_id ?? channelName
    const existing = channels.get(key)
    if (existing) {
      existing.count++
    } else {
      channels.set(key, { name: channelName, id: item.channel_id, count: 1 })
    }
  }
  return [...channels.values()]
})

// Column 3: Videos for selected channel
const videoList = computed(() => {
  if (!selectedSite.value || !selectedChannel.value) return []
  return props.items.filter(item => {
    const site = item.site ?? '不明'
    const channelKey = item.channel_id ?? item.channel ?? '不明'
    return site === selectedSite.value && channelKey === selectedChannel.value
  })
})

function selectSite(site: string) {
  selectedSite.value = site
  selectedChannel.value = null
}

function selectChannel(channelKey: string) {
  selectedChannel.value = channelKey
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
</script>

<template>
  <div class="flex flex-col h-full">
    <SelectionBar />
    <div class="flex flex-1 min-h-0">
    <!-- Column 1: Sites -->
    <div class="w-1/3 border-r border-[var(--color-separator)] overflow-y-auto">
      <div class="px-2 py-1.5 text-xs font-semibold text-neutral-500 uppercase tracking-wider border-b border-[var(--color-separator)]">
        サイト
      </div>
      <div v-if="siteList.length === 0" class="p-4 text-xs text-neutral-400 text-center">
        データなし
      </div>
      <button v-for="site in siteList" :key="site.name"
              class="w-full text-left px-3 py-1.5 text-sm flex items-center justify-between hover:bg-neutral-50 dark:hover:bg-neutral-800/50"
              :class="{ 'bg-[var(--color-accent)]/10 text-[var(--color-accent)]': selectedSite === site.name }"
              @click="selectSite(site.name)">
        <span class="truncate">{{ site.name }}</span>
        <span class="text-xs text-neutral-400 ml-2">{{ site.count }}</span>
      </button>
    </div>

    <!-- Column 2: Channels -->
    <div class="w-1/3 border-r border-[var(--color-separator)] overflow-y-auto">
      <div class="px-2 py-1.5 text-xs font-semibold text-neutral-500 uppercase tracking-wider border-b border-[var(--color-separator)]">
        チャンネル
      </div>
      <div v-if="!selectedSite" class="p-4 text-xs text-neutral-400 text-center">
        サイトを選択してください
      </div>
      <div v-else-if="channelList.length === 0" class="p-4 text-xs text-neutral-400 text-center">
        チャンネルなし
      </div>
      <button v-for="ch in channelList" :key="ch.id ?? ch.name"
              class="w-full text-left px-3 py-1.5 text-sm flex items-center justify-between hover:bg-neutral-50 dark:hover:bg-neutral-800/50"
              :class="{ 'bg-[var(--color-accent)]/10 text-[var(--color-accent)]': selectedChannel === (ch.id ?? ch.name) }"
              @click="selectChannel(ch.id ?? ch.name)">
        <span class="truncate">{{ ch.name }}</span>
        <span class="text-xs text-neutral-400 ml-2">{{ ch.count }}</span>
      </button>
    </div>

    <!-- Column 3: Videos -->
    <div class="w-1/3 overflow-y-auto">
      <div class="px-2 py-1.5 text-xs font-semibold text-neutral-500 uppercase tracking-wider border-b border-[var(--color-separator)]">
        動画
      </div>
      <div v-if="!selectedChannel" class="p-4 text-xs text-neutral-400 text-center">
        チャンネルを選択してください
      </div>
      <div v-else-if="videoList.length === 0" class="p-4 text-xs text-neutral-400 text-center">
        動画なし
      </div>
      <div v-for="item in videoList" :key="item.id"
           class="px-3 py-2 hover:bg-neutral-50 dark:hover:bg-neutral-800/50 cursor-default border-b border-[var(--color-separator)]"
           @contextmenu="handleContextMenu($event, item)"
           @dblclick="handleDoubleClick(item)">
        <div class="flex gap-2 items-start">
          <input type="checkbox"
                 :checked="libraryStore.selectedIds.has(item.id)"
                 @change="libraryStore.toggleSelect(item.id)"
                 @click.stop
                 class="mt-1 rounded border-neutral-300 dark:border-neutral-600 accent-[var(--color-accent)] flex-shrink-0" />
          <img v-if="item.thumbnail_url" :src="item.thumbnail_url"
               class="w-16 h-10 object-cover rounded flex-shrink-0" />
          <div class="min-w-0">
            <p class="text-xs font-medium line-clamp-2">{{ item.title || item.url }}</p>
            <p class="text-[10px] text-neutral-500 mt-0.5">
              {{ item.format?.toUpperCase() ?? '' }}
              <span v-if="item.is_favorite" class="text-yellow-500 ml-1">★</span>
            </p>
            <p v-if="item.file_path" class="text-[10px] text-neutral-400 truncate" :title="item.file_path">
              {{ item.file_path.split('/').pop() }}
            </p>
          </div>
        </div>
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
  </div>
</template>
