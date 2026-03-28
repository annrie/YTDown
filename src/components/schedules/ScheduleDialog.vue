<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { Cron } from 'croner'
import type { Schedule } from '../../types'
import { useSettingsStore } from '../../stores/settings'

const props = defineProps<{
  schedule?: Schedule | null
  initialUrl?: string
}>()

const emit = defineEmits<{
  save: [payload: {
    name: string; url: string; cron_expr: string
    options_json: string; is_channel: boolean
  }]
  cancel: []
}>()

const settingsStore = useSettingsStore()

const name = ref(props.schedule?.name ?? '')
const url = ref(props.schedule?.url ?? props.initialUrl ?? '')
const cronExpr = ref(props.schedule?.cron_expr ?? '0 9 * * *')
const isChannel = ref(props.schedule?.is_channel ?? false)
const optionsJson = ref(props.schedule?.options_json ?? JSON.stringify({
  format: 'mp4',
  quality: 'best',
  output_dir: settingsStore.settings?.download_dir ?? '~/Downloads/YTDown/',
  embed_thumbnail: true,
  embed_metadata: true,
  write_subs: false,
  embed_subs: false,
  embed_chapters: true,
  sponsorblock: false,
  playlist_mode: 'single',
}))

const cronError = ref('')
const nextRuns = ref<string[]>([])

watch(cronExpr, (expr) => {
  try {
    const job = new Cron(expr)
    cronError.value = ''
    const runs: string[] = []
    let next = job.nextRun()
    for (let i = 0; i < 5 && next; i++) {
      runs.push(next.toLocaleString('ja-JP'))
      next = job.nextRun(next)
    }
    nextRuns.value = runs
  } catch {
    cronError.value = '無効なcron式です'
    nextRuns.value = []
  }
}, { immediate: true })

const isValid = computed(() =>
  name.value.trim() !== '' &&
  url.value.trim() !== '' &&
  cronError.value === ''
)

function onSave() {
  if (!isValid.value) return
  emit('save', {
    name: name.value.trim(),
    url: url.value.trim(),
    cron_expr: cronExpr.value.trim(),
    options_json: optionsJson.value,
    is_channel: isChannel.value,
  })
}
</script>

<template>
  <div class="dialog-overlay" @click.self="emit('cancel')">
    <div class="dialog">
      <h2 class="dialog-title">{{ schedule ? 'スケジュールを編集' : 'スケジュールを追加' }}</h2>

      <div class="field">
        <label class="label">名前</label>
        <input v-model="name" class="input" placeholder="例: 深夜バックアップ" />
      </div>

      <div class="field">
        <label class="label">URL</label>
        <input v-model="url" class="input" placeholder="https://..." />
      </div>

      <div class="field">
        <label class="label toggle-label">
          <input type="checkbox" v-model="isChannel" />
          <span>チャンネル監視（新着のみ）</span>
        </label>
      </div>

      <div class="field">
        <label class="label">cron式</label>
        <input v-model="cronExpr" class="input font-mono" placeholder="0 9 * * *" />
        <p v-if="cronError" class="error-text">{{ cronError }}</p>
        <div v-if="nextRuns.length" class="next-runs">
          <p class="next-runs-label">次回5回の実行予定:</p>
          <ul>
            <li v-for="(run, i) in nextRuns" :key="i">{{ run }}</li>
          </ul>
        </div>
      </div>

      <div class="dialog-actions">
        <button class="btn btn-cancel" @click="emit('cancel')">キャンセル</button>
        <button class="btn btn-save" :disabled="!isValid" @click="onSave">
          {{ schedule ? '保存' : 'スケジュール登録' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dialog-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; z-index: 50; }
.dialog { background: var(--color-sidebar-bg, #1c1c1e); border: 1px solid var(--color-separator, rgba(120,120,128,0.2)); border-radius: 1rem; padding: 1.5rem; width: 480px; max-width: 90vw; }
.dialog-title { font-size: 1.125rem; font-weight: 700; margin-bottom: 1.25rem; }
.field { margin-bottom: 1rem; }
.label { display: block; font-size: 0.8125rem; font-weight: 600; margin-bottom: 0.375rem; color: rgba(100,100,108,0.9); }
.toggle-label { display: flex; align-items: center; gap: 0.5rem; cursor: pointer; }
.input { width: 100%; padding: 0.5rem 0.75rem; border: 1px solid var(--color-separator, rgba(120,120,128,0.2)); border-radius: 0.5rem; background: rgba(0,0,0,0.03); font-size: 0.875rem; box-sizing: border-box; color: inherit; }
.font-mono { font-family: 'SF Mono', 'Menlo', monospace; }
.error-text { color: #ff3b30; font-size: 0.75rem; margin-top: 0.25rem; }
.next-runs { margin-top: 0.5rem; font-size: 0.75rem; color: rgba(100,100,108,0.8); }
.next-runs-label { font-weight: 600; margin-bottom: 0.25rem; }
.next-runs ul { list-style: disc; margin-left: 1.25rem; }
.dialog-actions { display: flex; justify-content: flex-end; gap: 0.75rem; margin-top: 1.5rem; }
.btn { padding: 0.5rem 1.25rem; border-radius: 0.5rem; font-size: 0.875rem; font-weight: 600; cursor: pointer; border: none; }
.btn-cancel { background: rgba(120,120,128,0.12); color: inherit; }
.btn-save { background: var(--color-accent, #007aff); color: white; }
.btn-save:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
