<script setup lang="ts">
import type { SidebarSection } from '../../types'
import { useDownloadsStore } from '../../stores/downloads'

const props = defineProps<{
  currentSection: SidebarSection
}>()

const emit = defineEmits<{
  'update:currentSection': [section: SidebarSection]
}>()

const downloadsStore = useDownloadsStore()

function isActive(section: SidebarSection) {
  return props.currentSection === section
}

function isLibraryExpanded() {
  return isActive('library-all') || isActive('library-video') || isActive('library-audio')
}
</script>

<template>
  <aside class="sidebar">
    <nav class="sidebar-nav">
      <!-- 動画 section -->
      <div class="sidebar-section">
        <h3 class="section-header">動画</h3>
        <ul class="section-list">
          <li>
            <button class="sidebar-item" :class="{ active: isActive('downloads-active') }"
                    @click="emit('update:currentSection', 'downloads-active')">
              <span class="item-indicator" />
              <svg class="item-icon" viewBox="0 0 20 20" fill="currentColor">
                <path d="M10 3a1 1 0 011 1v5.586l2.293-2.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L9 9.586V4a1 1 0 011-1z"/>
                <path d="M3 15a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1z"/>
              </svg>
              <span class="item-label">進行中</span>
              <span v-if="downloadsStore.activeDownloads.length > 0" class="item-badge">
                {{ downloadsStore.activeDownloads.length }}
              </span>
            </button>
          </li>
          <li>
            <button class="sidebar-item" :class="{ active: isActive('downloads-completed') }"
                    @click="emit('update:currentSection', 'downloads-completed')">
              <span class="item-indicator" />
              <svg class="item-icon" viewBox="0 0 20 20" fill="currentColor">
                <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd"/>
              </svg>
              <span class="item-label">完了</span>
            </button>
          </li>
          <li>
            <button class="sidebar-item" :class="{ active: isActive('library-all') }"
                    @click="emit('update:currentSection', 'library-all')">
              <span class="item-indicator" />
              <svg class="item-icon" viewBox="0 0 20 20" fill="currentColor">
                <path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z"/>
              </svg>
              <span class="item-label">ライブラリ</span>
            </button>
            <!-- Subtree with connector lines -->
            <ul v-if="isLibraryExpanded()" class="subtree">
              <li>
                <button class="sidebar-item sub-item" :class="{ active: isActive('library-video') }"
                        @click="emit('update:currentSection', 'library-video')">
                  <span class="item-indicator" />
                  <span class="tree-connector" />
                  <svg class="item-icon" viewBox="0 0 20 20" fill="currentColor">
                    <path d="M2 6a2 2 0 012-2h6a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6zm12.553-1.106A1 1 0 0014 5.882v8.236a1 1 0 001.447.894l4-2a1 1 0 000-1.788l-4-2.118z"/>
                  </svg>
                  <span class="item-label">映像</span>
                </button>
              </li>
              <li>
                <button class="sidebar-item sub-item" :class="{ active: isActive('library-audio') }"
                        @click="emit('update:currentSection', 'library-audio')">
                  <span class="item-indicator" />
                  <span class="tree-connector last" />
                  <svg class="item-icon" viewBox="0 0 20 20" fill="currentColor">
                    <path fill-rule="evenodd" d="M9.383 3.076A1 1 0 0110 4v12a1 1 0 01-1.707.707L4.586 13H2a1 1 0 01-1-1V8a1 1 0 011-1h2.586l3.707-3.707a1 1 0 011.09-.217zM14.657 2.929a1 1 0 011.414 0A9.972 9.972 0 0119 10a9.972 9.972 0 01-2.929 7.071 1 1 0 01-1.414-1.414A7.971 7.971 0 0017 10c0-2.21-.894-4.208-2.343-5.657a1 1 0 010-1.414zm-2.829 2.828a1 1 0 011.415 0A5.983 5.983 0 0115 10a5.984 5.984 0 01-1.757 4.243 1 1 0 01-1.415-1.415A3.984 3.984 0 0013 10a3.983 3.983 0 00-1.172-2.828 1 1 0 010-1.415z" clip-rule="evenodd"/>
                  </svg>
                  <span class="item-label">音声</span>
                </button>
              </li>
            </ul>
          </li>
        </ul>
      </div>

      <!-- Gradient divider -->
      <div class="section-divider" />

      <!-- 画像 section -->
      <div class="sidebar-section">
        <h3 class="section-header">画像</h3>
        <ul class="section-list">
          <li>
            <button class="sidebar-item" :class="{ active: isActive('images-download') }"
                    @click="emit('update:currentSection', 'images-download')">
              <span class="item-indicator" />
              <svg class="item-icon" viewBox="0 0 20 20" fill="currentColor">
                <path fill-rule="evenodd" d="M4 3a2 2 0 00-2 2v10a2 2 0 002 2h12a2 2 0 002-2V5a2 2 0 00-2-2H4zm12 12H4l4-8 3 6 2-4 3 6z" clip-rule="evenodd"/>
              </svg>
              <span class="item-label">取得</span>
            </button>
          </li>
          <li>
            <button class="sidebar-item" :class="{ active: isActive('images-gallery') }"
                    @click="emit('update:currentSection', 'images-gallery')">
              <span class="item-indicator" />
              <svg class="item-icon" viewBox="0 0 20 20" fill="currentColor">
                <path d="M5 3a2 2 0 00-2 2v2a2 2 0 002 2h2a2 2 0 002-2V5a2 2 0 00-2-2H5zM5 11a2 2 0 00-2 2v2a2 2 0 002 2h2a2 2 0 002-2v-2a2 2 0 00-2-2H5zM11 5a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V5zM11 13a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"/>
              </svg>
              <span class="item-label">ギャラリー</span>
            </button>
          </li>
        </ul>
      </div>
    </nav>

    <!-- Settings pinned to bottom -->
    <div class="sidebar-footer">
      <div class="section-divider" />
      <button class="sidebar-item settings-item" :class="{ active: isActive('settings') }"
              @click="emit('update:currentSection', 'settings')">
        <span class="item-indicator" />
        <svg class="item-icon" viewBox="0 0 20 20" fill="currentColor">
          <path fill-rule="evenodd" d="M11.49 3.17c-.38-1.56-2.6-1.56-2.98 0a1.532 1.532 0 01-2.286.948c-1.372-.836-2.942.734-2.106 2.106.54.886.061 2.042-.947 2.287-1.561.379-1.561 2.6 0 2.978a1.532 1.532 0 01.947 2.287c-.836 1.372.734 2.942 2.106 2.106a1.532 1.532 0 012.287.947c.379 1.561 2.6 1.561 2.978 0a1.533 1.533 0 012.287-.947c1.372.836 2.942-.734 2.106-2.106a1.533 1.533 0 01.947-2.287c1.561-.379 1.561-2.6 0-2.978a1.532 1.532 0 01-.947-2.287c.836-1.372-.734-2.942-2.106-2.106a1.532 1.532 0 01-2.287-.947zM10 13a3 3 0 100-6 3 3 0 000 6z" clip-rule="evenodd"/>
        </svg>
        <span class="item-label">設定</span>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: var(--sidebar-width);
  border-right: 1px solid var(--color-separator);
  overflow-y: auto;
  padding: 0;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  position: relative;

  /* Marble base (light mode) — dense veining, deeper tone */
  background:
    /* Primary veins — fine, frequent */
    linear-gradient(125deg, transparent 42%, rgba(150, 140, 130, 0.14) 42.8%, rgba(150, 140, 130, 0.18) 43.2%, transparent 43.8%),
    linear-gradient(128deg, transparent 58%, rgba(140, 132, 122, 0.10) 58.5%, rgba(140, 132, 122, 0.14) 59%, transparent 59.5%),
    linear-gradient(235deg, transparent 35%, rgba(135, 128, 118, 0.12) 35.4%, rgba(135, 128, 118, 0.16) 35.8%, transparent 36.2%),
    linear-gradient(232deg, transparent 68%, rgba(145, 138, 128, 0.09) 68.3%, rgba(145, 138, 128, 0.13) 68.7%, transparent 69%),
    linear-gradient(158deg, transparent 25%, rgba(155, 148, 138, 0.11) 25.3%, rgba(155, 148, 138, 0.15) 25.7%, transparent 26%),
    linear-gradient(162deg, transparent 50%, rgba(148, 140, 130, 0.08) 50.4%, rgba(148, 140, 130, 0.12) 50.8%, transparent 51.2%),
    /* Secondary veins — crossing at different angles */
    linear-gradient(85deg, transparent 22%, rgba(130, 125, 115, 0.07) 22.5%, rgba(130, 125, 115, 0.10) 23.5%, transparent 24%),
    linear-gradient(195deg, transparent 45%, rgba(140, 135, 125, 0.06) 45.5%, rgba(140, 135, 125, 0.09) 46.5%, transparent 47%),
    linear-gradient(310deg, transparent 72%, rgba(135, 128, 118, 0.05) 72.5%, rgba(135, 128, 118, 0.08) 73.5%, transparent 74%),
    /* Wider vein bands */
    linear-gradient(110deg, transparent 18%, rgba(140, 132, 120, 0.08) 19%, rgba(140, 132, 120, 0.12) 22%, transparent 23%),
    linear-gradient(200deg, transparent 55%, rgba(150, 142, 130, 0.06) 56%, rgba(150, 142, 130, 0.10) 59%, transparent 60%),
    /* Color variation patches — stronger */
    radial-gradient(ellipse at 15% 15%, rgba(180, 170, 158, 0.22) 0%, transparent 45%),
    radial-gradient(ellipse at 75% 25%, rgba(175, 168, 155, 0.18) 0%, transparent 40%),
    radial-gradient(ellipse at 85% 65%, rgba(170, 162, 150, 0.20) 0%, transparent 38%),
    radial-gradient(ellipse at 30% 75%, rgba(185, 175, 162, 0.16) 0%, transparent 42%),
    radial-gradient(ellipse at 55% 45%, rgba(178, 170, 158, 0.12) 0%, transparent 35%),
    /* Deeper base tone */
    linear-gradient(135deg, #e8e4de 0%, #ddd8d0 25%, #e5e0d8 50%, #d8d3cb 75%, #e2ddd5 100%);
  backdrop-filter: blur(20px);
}

/* Obsidian (dark mode) */
:where(.dark) .sidebar {
  background:
    /* Obsidian glassy reflections */
    linear-gradient(135deg, transparent 25%, rgba(100, 120, 160, 0.06) 26%, rgba(100, 120, 160, 0.03) 30%, transparent 31%),
    linear-gradient(225deg, transparent 45%, rgba(80, 100, 140, 0.05) 46%, rgba(80, 100, 140, 0.02) 50%, transparent 51%),
    linear-gradient(170deg, transparent 60%, rgba(90, 80, 120, 0.04) 61%, rgba(90, 80, 120, 0.02) 65%, transparent 66%),
    /* Subtle iridescent sheen */
    radial-gradient(ellipse at 30% 10%, rgba(80, 100, 180, 0.07) 0%, transparent 40%),
    radial-gradient(ellipse at 70% 50%, rgba(100, 60, 140, 0.05) 0%, transparent 35%),
    radial-gradient(ellipse at 20% 80%, rgba(60, 100, 120, 0.04) 0%, transparent 30%),
    /* Conchoidal fracture highlights */
    linear-gradient(115deg, transparent 15%, rgba(255, 255, 255, 0.02) 16%, rgba(255, 255, 255, 0.01) 20%, transparent 21%),
    linear-gradient(250deg, transparent 70%, rgba(255, 255, 255, 0.015) 71%, transparent 74%),
    /* Deep dark base */
    linear-gradient(180deg, #1a1a1e 0%, #141416 40%, #111113 70%, #0e0e10 100%);
  border-right-color: rgba(255, 255, 255, 0.06);
}

.sidebar-nav {
  flex: 1;
  padding: 0.75rem 0.5rem;
}

/* Section header */
.section-header {
  padding: 0.25rem 0.75rem;
  font-size: 0.8125rem;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 0.14em;
  color: rgba(60, 55, 48, 0.85);
  margin-bottom: 0.375rem;
  text-shadow: 0 0.5px 0 rgba(255, 255, 255, 0.4);
  background: rgba(0, 0, 0, 0.05);
  border-radius: 0.375rem;
  padding: 0.3rem 0.75rem;
  backdrop-filter: blur(4px);
  border: 1px solid rgba(0, 0, 0, 0.04);
}

.section-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.section-list li {
  margin-bottom: 1px;
}

/* Gradient divider */
.section-divider {
  height: 1px;
  margin: 0.625rem 0.75rem;
  background: linear-gradient(
    90deg,
    transparent 0%,
    var(--color-accent) 30%,
    var(--color-accent) 70%,
    transparent 100%
  );
  opacity: 0.2;
}

/* Sidebar item base */
.sidebar-item {
  position: relative;
  display: flex;
  align-items: center;
  width: 100%;
  padding: 0.4375rem 0.75rem;
  border-radius: 0.5rem;
  border: none;
  background: transparent;
  color: rgba(220, 220, 226, 0.95);
  font-size: 0.9375rem;
  font-weight: 500;
  cursor: pointer;
  transition: none;
  overflow: hidden;
  text-align: left;
  gap: 0.5rem;
}

/* Hover: sliding background animation */
.sidebar-item::before {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: 0.5rem;
  background: rgba(120, 120, 128, 0.08);
  transform: translateX(-100%);
  transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.sidebar-item:hover::before {
  transform: translateX(0);
}

.sidebar-item:hover {
  color: rgba(245, 245, 250, 1);
  filter: none;
  transform: none;
}

/* Active state */
.sidebar-item.active {
  color: var(--color-accent);
}

.sidebar-item.active::before {
  background: rgba(0, 122, 255, 0.08);
  transform: translateX(0);
}

/* Glow indicator dot */
.item-indicator {
  position: absolute;
  left: 0.125rem;
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: transparent;
  transition: background 0.2s ease, box-shadow 0.3s ease;
}

.sidebar-item.active .item-indicator {
  background: var(--color-accent);
  box-shadow: 0 0 6px 2px color-mix(in srgb, var(--color-accent) 50%, transparent);
}

/* Icon */
.item-icon {
  width: 1rem;
  height: 1rem;
  flex-shrink: 0;
  opacity: 0.6;
  transition: opacity 0.2s ease;
  position: relative;
}

.sidebar-item:hover .item-icon,
.sidebar-item.active .item-icon {
  opacity: 1;
}

/* Label */
.item-label {
  flex: 1;
  position: relative;
}

/* Badge */
.item-badge {
  font-size: 0.625rem;
  font-weight: 700;
  min-width: 1.125rem;
  height: 1.125rem;
  line-height: 1.125rem;
  text-align: center;
  border-radius: 999px;
  background: var(--color-accent);
  color: white;
  padding: 0 0.3rem;
  position: relative;
}

/* Subtree / tree connectors */
.subtree {
  list-style: none;
  padding: 0;
  margin: 0 0 0 0.5rem;
}

.subtree li {
  margin-bottom: 1px;
}

.sub-item {
  padding-left: 1.25rem;
}

.tree-connector {
  position: absolute;
  left: 1rem;
  top: -0.25rem;
  width: 0.5rem;
  height: calc(50% + 0.25rem);
  border-left: 1px solid rgba(120, 120, 128, 0.2);
  border-bottom: 1px solid rgba(120, 120, 128, 0.2);
  border-bottom-left-radius: 0.25rem;
}

.tree-connector::after {
  content: '';
  position: absolute;
  left: -1px;
  top: -100%;
  width: 1px;
  height: 100%;
  background: rgba(120, 120, 128, 0.2);
}

.tree-connector.last::after {
  display: none;
}

/* Footer (settings pinned to bottom) */
.sidebar-footer {
  padding: 0 0.5rem 0.75rem;
}

.settings-item .item-icon {
  transition: transform 0.6s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.2s ease;
}

.settings-item:hover .item-icon {
  transform: rotate(60deg);
}

/* Light mode adjustments */
:root:not(.dark) .sidebar-item {
  color: rgba(40, 38, 35, 0.8);
}

:root:not(.dark) .sidebar-item:hover {
  color: rgba(30, 28, 25, 1);
}

:root:not(.dark) .sidebar-item.active {
  color: var(--color-accent);
}

:root:not(.dark) .sidebar-item.active::before {
  background: rgba(0, 122, 255, 0.06);
}

:root:not(.dark) .sidebar-item::before {
  background: rgba(60, 60, 67, 0.04);
}

/* Dark mode section header */
:where(.dark) .section-header {
  color: rgba(200, 198, 194, 0.7);
  text-shadow: 0 0.5px 0 rgba(0, 0, 0, 0.5);
  background: rgba(255, 255, 255, 0.05);
  border-color: rgba(255, 255, 255, 0.06);
}

/* Sidebar section spacing */
.sidebar-section {
  margin-bottom: 0.25rem;
}
</style>
