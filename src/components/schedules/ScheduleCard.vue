<script setup lang="ts">
import type { Schedule } from '../../types'
import { computed } from 'vue'

const props = defineProps<{
  schedule: Schedule
  isStartupChecking?: boolean
  isChecking?: boolean
}>()
const emit = defineEmits<{
  toggle: [id: number, is_active: boolean]
  edit: [schedule: Schedule]
  delete: [id: number]
  runNow: [id: number]
  stop: [id: number]
}>()

function formatDate(iso: string | null): string {
  if (!iso) return 'なし'
  return new Date(iso).toLocaleString('ja-JP')
}

const parsedOptions = computed(() => {
  try {
    return JSON.parse(props.schedule.options_json)
  } catch {
    return {}
  }
})

const isBusy = computed(() => !!props.isChecking || !!props.isStartupChecking || props.schedule.is_running)
</script>

<template>
  <div class="schedule-card" :class="{ inactive: !schedule.is_active }">
    <div class="card-header">
      <div class="flex items-center gap-2.5">
        <img v-if="parsedOptions.avatar_url" :src="parsedOptions.avatar_url" class="w-8 h-8 rounded-full object-cover shadow-sm bg-neutral-200 dark:bg-neutral-800" />
        <div class="flex items-center flex-wrap gap-1.5 leading-tight">
          <span class="card-name">{{ schedule.name }}</span>
          <span v-if="parsedOptions.channel_id" class="text-xs font-mono text-neutral-500">(@{{ parsedOptions.channel_id }})</span>
        </div>
      </div>
      <label class="toggle-switch">
        <input type="checkbox" :checked="schedule.is_active"
               @change="emit('toggle', schedule.id, !schedule.is_active)" />
        <span class="slider" />
      </label>
    </div>

    <div class="card-meta">
      <span class="meta-item">
        <svg class="meta-icon" viewBox="0 0 16 16" fill="currentColor">
          <path d="M8 3.5a.5.5 0 00-1 0V9a.5.5 0 00.252.434l3.5 2a.5.5 0 00.496-.868L8 8.71V3.5z"/>
          <path d="M8 16A8 8 0 108 0a8 8 0 000 16zm7-8A7 7 0 111 8a7 7 0 0114 0z"/>
        </svg>
        {{ schedule.cron_expr }}
      </span>
      <span class="meta-item">次回: {{ formatDate(schedule.next_run_at) }}</span>
      <span v-if="schedule.last_run_at" class="meta-item">前回: {{ formatDate(schedule.last_run_at) }}</span>
      <span v-if="schedule.is_channel" class="badge-channel">チャンネル監視</span>
      <span v-if="props.isStartupChecking" class="badge-startup-check">起動時確認</span>
    </div>

    <div v-if="props.isChecking || schedule.is_running || schedule.last_run_status" class="last-run-status">
      <span v-if="props.isChecking && !schedule.is_running" class="status-badge status-checking">確認中</span>
      <span v-else-if="schedule.is_running" class="status-badge status-running">ダウンロード中</span>
      <span v-else-if="schedule.last_run_status === 'completed'" class="status-badge status-completed">ダウンロード済み</span>
      <span v-else-if="schedule.last_run_status === 'no_new'" class="status-badge status-no-new">新着なし</span>
      <span v-else-if="schedule.last_run_status === 'stopped'" class="status-badge status-stopped">停止しました</span>
    </div>

    <div v-if="schedule.last_error" class="card-error">
      <span class="error-icon">⚠</span> {{ schedule.last_error }}
      <span class="fail-count">({{ schedule.fail_count }}/3)</span>
    </div>

    <div class="card-actions">
      <!-- 実行中: 停止ボタン / 停止中: 実行ボタン -->
      <button v-if="schedule.is_running" class="btn-action btn-stop" @click="emit('stop', schedule.id)" title="停止">
        <svg viewBox="0 0 16 16" fill="currentColor" class="action-icon">
          <path d="M5 3.5h6A1.5 1.5 0 0112.5 5v6a1.5 1.5 0 01-1.5 1.5H5A1.5 1.5 0 013.5 11V5A1.5 1.5 0 015 3.5z"/>
        </svg>
      </button>
      <button
        v-else
        class="btn-action"
        :class="{ 'btn-disabled': isBusy }"
        :disabled="isBusy"
        @click="emit('runNow', schedule.id)"
        :title="isBusy ? '確認中' : '今すぐ実行'"
      >
        <svg viewBox="0 0 16 16" fill="currentColor" class="action-icon">
          <path d="M11.596 8.697l-6.363 3.692c-.54.313-1.233-.066-1.233-.697V4.308c0-.63.692-1.01 1.233-.696l6.363 3.692a.802.802 0 010 1.393z"/>
        </svg>
      </button>
      <button class="btn-action" @click="emit('edit', schedule)" title="編集">
        <svg viewBox="0 0 16 16" fill="currentColor" class="action-icon">
          <path d="M12.146.146a.5.5 0 01.708 0l3 3a.5.5 0 010 .708l-10 10a.5.5 0 01-.168.11l-5 2a.5.5 0 01-.65-.65l2-5a.5.5 0 01.11-.168l10-10zM11.207 2.5L13.5 4.793 14.793 3.5 12.5 1.207 11.207 2.5zm1.586 3L10.5 3.207 4 9.707V10h.5a.5.5 0 01.5.5v.5h.5a.5.5 0 01.5.5v.5h.293l6.5-6.5zm-9.761 5.175l-.106.106-1.528 3.821 3.821-1.528.106-.106A.5.5 0 015 12.5V12h-.5a.5.5 0 01-.5-.5V11h-.5a.5.5 0 01-.468-.325z"/>
        </svg>
      </button>
      <button class="btn-action btn-danger" @click="emit('delete', schedule.id)" title="削除">
        <svg viewBox="0 0 16 16" fill="currentColor" class="action-icon">
          <path d="M5.5 5.5A.5.5 0 016 6v6a.5.5 0 01-1 0V6a.5.5 0 01.5-.5zm2.5 0a.5.5 0 01.5.5v6a.5.5 0 01-1 0V6a.5.5 0 01.5-.5zm3 .5a.5.5 0 00-1 0v6a.5.5 0 001 0V6z"/>
          <path fill-rule="evenodd" d="M14.5 3a1 1 0 01-1 1H13v9a2 2 0 01-2 2H5a2 2 0 01-2-2V4h-.5a1 1 0 01-1-1V2a1 1 0 011-1H6a1 1 0 011-1h2a1 1 0 011 1h3.5a1 1 0 011 1v1zM4.118 4L4 4.059V13a1 1 0 001 1h6a1 1 0 001-1V4.059L11.882 4H4.118zM2.5 3V2h11v1h-11z" clip-rule="evenodd"/>
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.schedule-card {
  background: var(--color-sidebar-bg, rgba(255,255,255,0.05));
  border: 1px solid var(--color-separator, rgba(120,120,128,0.2));
  border-radius: 0.75rem;
  padding: 1rem;
  transition: opacity 0.2s;
}
.schedule-card.inactive { opacity: 0.5; }
.card-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem; }
.card-name { font-weight: 600; font-size: 0.9375rem; }
.card-meta { display: flex; gap: 0.75rem; font-size: 0.8125rem; color: rgba(120,120,128,0.8); flex-wrap: wrap; margin-bottom: 0.5rem; }
.meta-item { display: flex; align-items: center; gap: 0.25rem; }
.meta-icon { width: 0.875rem; height: 0.875rem; }
.badge-channel { background: var(--color-accent, #007aff); color: white; font-size: 0.625rem; padding: 0.1rem 0.4rem; border-radius: 999px; }
.badge-startup-check { background: rgba(255, 149, 0, 0.15); color: #ff9500; font-size: 0.625rem; padding: 0.1rem 0.4rem; border-radius: 999px; }
.last-run-status { margin-bottom: 0.5rem; }
.status-badge { font-size: 0.6875rem; padding: 0.15rem 0.5rem; border-radius: 999px; font-weight: 600; }
.status-completed { background: rgba(52, 199, 89, 0.15); color: #34c759; }
.status-no-new { background: rgba(120, 120, 128, 0.15); color: rgba(120, 120, 128, 0.9); }
.status-stopped { background: rgba(255, 149, 0, 0.15); color: #ff9500; }
.status-checking { background: rgba(90, 145, 255, 0.12); color: #2f6fed; }
.status-running { background: rgba(0, 122, 255, 0.15); color: #007aff; }
.card-error { font-size: 0.75rem; color: #ff3b30; margin-bottom: 0.5rem; }
.fail-count { opacity: 0.7; }
.card-actions { display: flex; gap: 0.5rem; justify-content: flex-end; }
.btn-action { background: transparent; border: 1px solid var(--color-separator, rgba(120,120,128,0.2)); border-radius: 0.375rem; padding: 0.25rem; cursor: pointer; color: inherit; }
.btn-disabled { opacity: 0.5; cursor: not-allowed; }
.btn-danger { color: #ff3b30; }
.btn-stop { color: #ff9500; border-color: #ff9500; }
.action-icon { width: 1rem; height: 1rem; }
.toggle-switch { position: relative; display: inline-block; width: 36px; height: 20px; }
.toggle-switch input { opacity: 0; width: 0; height: 0; }
.slider { position: absolute; inset: 0; background: #ccc; border-radius: 20px; cursor: pointer; transition: 0.2s; }
.slider::before { content: ''; position: absolute; height: 14px; width: 14px; left: 3px; bottom: 3px; background: white; border-radius: 50%; transition: 0.2s; }
input:checked + .slider { background: var(--color-accent, #007aff); }
input:checked + .slider::before { transform: translateX(16px); }
</style>
