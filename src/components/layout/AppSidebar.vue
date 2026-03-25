<script setup lang="ts">
import type { SidebarSection } from '../../types'
import { useDownloadsStore } from '../../stores/downloads'
import PlaylistList from '../playlist/PlaylistList.vue'

const props = defineProps<{
  currentSection: SidebarSection
}>()

const emit = defineEmits<{
  'update:currentSection': [section: SidebarSection]
  'select-playlist': [id: number]
  'select-url-list': [id: number]
}>()

const downloadsStore = useDownloadsStore()

function isActive(section: SidebarSection) {
  return props.currentSection === section
}

function sidebarButtonClass(section: SidebarSection) {
  return [
    'w-full text-left px-2 py-1 rounded-md hover:bg-black/5 dark:hover:bg-white/5 text-neutral-800 dark:text-neutral-200',
    isActive(section) ? 'bg-[var(--color-accent)]/10 !text-[var(--color-accent)]' : '',
  ]
}
</script>

<template>
  <aside class="sidebar">
    <nav class="text-sm space-y-4">
      <!-- 動画 section -->
      <div>
        <h3 class="px-2 text-xs font-semibold text-neutral-500 uppercase tracking-wider mb-1">動画</h3>
        <ul class="space-y-0.5">
          <li>
            <button :class="sidebarButtonClass('downloads-active')"
                    @click="emit('update:currentSection', 'downloads-active')">
              進行中
              <span v-if="downloadsStore.activeDownloads.length > 0"
                    class="ml-1 text-xs text-neutral-400">
                ({{ downloadsStore.activeDownloads.length }})
              </span>
            </button>
          </li>
          <li>
            <button :class="sidebarButtonClass('downloads-completed')"
                    @click="emit('update:currentSection', 'downloads-completed')">
              完了
            </button>
          </li>
          <li>
            <button :class="sidebarButtonClass('library-all')"
                    @click="emit('update:currentSection', 'library-all')">
              ライブラリ
            </button>
            <ul v-if="isActive('library-all') || isActive('library-video') || isActive('library-audio')" class="ml-3 mt-0.5 space-y-0.5">
              <li>
                <button :class="sidebarButtonClass('library-video')"
                        @click="emit('update:currentSection', 'library-video')">
                  映像
                </button>
              </li>
              <li>
                <button :class="sidebarButtonClass('library-audio')"
                        @click="emit('update:currentSection', 'library-audio')">
                  音声
                </button>
              </li>
            </ul>
          </li>
        </ul>
      </div>

      <!-- 画像 section -->
      <div>
        <h3 class="px-2 text-xs font-semibold text-neutral-500 uppercase tracking-wider mb-1">画像</h3>
        <ul class="space-y-0.5">
          <li>
            <button :class="sidebarButtonClass('images-download')"
                    @click="emit('update:currentSection', 'images-download')">
              取得
            </button>
          </li>
          <li>
            <button :class="sidebarButtonClass('images-gallery')"
                    @click="emit('update:currentSection', 'images-gallery')">
              ギャラリー
            </button>
          </li>
        </ul>
      </div>

      <!-- プレイリスト section -->
      <div>
        <h3 class="px-2 text-xs font-semibold text-neutral-500 uppercase tracking-wider mb-1">プレイリスト</h3>
        <PlaylistList
          @select="(id) => { emit('select-playlist', id); emit('update:currentSection', 'playlist') }"
          @select-url-list="(id) => emit('select-url-list', id)"
        />
      </div>

      <!-- 設定 -->
      <div>
        <button :class="sidebarButtonClass('settings')"
                @click="emit('update:currentSection', 'settings')">
          設定
        </button>
      </div>
    </nav>
  </aside>
</template>

<style scoped>
.sidebar {
  width: var(--sidebar-width);
  background: var(--color-sidebar-bg);
  backdrop-filter: blur(20px);
  border-right: 1px solid var(--color-separator);
  overflow-y: auto;
  padding: 0.5rem;
  flex-shrink: 0;
}
</style>
