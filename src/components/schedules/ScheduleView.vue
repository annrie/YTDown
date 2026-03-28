<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useSchedulesStore } from '../../stores/schedules'
import type { Schedule } from '../../types'
import ScheduleCard from './ScheduleCard.vue'
import ScheduleDialog from './ScheduleDialog.vue'

const store = useSchedulesStore()
const showDialog = ref(false)
const editTarget = ref<Schedule | null>(null)
const errorMsg = ref('')

onMounted(async () => {
  try {
    await store.fetchSchedules()
  } catch (e) {
    errorMsg.value = `リスト取得失敗: ${e}`
  }
  await store.setupScheduleListener()
})

function openCreate() {
  editTarget.value = null
  showDialog.value = true
}

function openEdit(schedule: Schedule) {
  editTarget.value = schedule
  showDialog.value = true
}

async function onSave(payload: {
  name: string; url: string; cron_expr: string
  options_json: string; is_channel: boolean
}) {
  showDialog.value = false
  try {
    if (editTarget.value) {
      await store.updateSchedule({ id: editTarget.value.id, ...payload })
    } else {
      await store.createSchedule(payload)
    }
  } catch (e) {
    errorMsg.value = `保存失敗: ${e}`
  }
}

async function onDelete(id: number) {
  if (confirm('このスケジュールを削除しますか？')) {
    await store.deleteSchedule(id)
  }
}

async function onRunNow(id: number) {
  try {
    await store.runNow(id)
  } catch (e) {
    errorMsg.value = `実行失敗: ${e}`
  }
}
</script>

<template>
  <div class="schedule-view">
    <div class="view-header">
      <h2 class="view-title">スケジュール（動画のみ）</h2>
      <button class="btn-add" @click="openCreate">
        <svg viewBox="0 0 20 20" fill="currentColor" class="add-icon">
          <path fill-rule="evenodd" d="M10 3a1 1 0 011 1v5h5a1 1 0 110 2h-5v5a1 1 0 11-2 0v-5H4a1 1 0 110-2h5V4a1 1 0 011-1z" clip-rule="evenodd"/>
        </svg>
        新規スケジュール
      </button>
    </div>

    <div v-if="errorMsg" class="error-banner">{{ errorMsg }}</div>

    <div v-if="store.schedules.length === 0" class="empty-state">
      <p>スケジュールはありません</p>
      <button class="btn-add-empty" @click="openCreate">最初のスケジュールを追加</button>
    </div>

    <div v-else class="cards-grid">
      <ScheduleCard
        v-for="s in store.schedules"
        :key="s.id"
        :schedule="s"
        @toggle="(id, isActive) => store.toggleSchedule(id, isActive)"
        @edit="openEdit"
        @delete="onDelete"
        @run-now="onRunNow"
      />
    </div>

    <ScheduleDialog
      v-if="showDialog"
      :schedule="editTarget"
      @save="onSave"
      @cancel="showDialog = false"
    />
  </div>
</template>

<style scoped>
.schedule-view { padding: 1.5rem; height: 100%; overflow-y: auto; }
.view-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 1.5rem; }
.view-title { font-size: 1.25rem; font-weight: 700; }
.btn-add { display: flex; align-items: center; gap: 0.375rem; background: var(--color-accent, #007aff); color: white; border: none; padding: 0.5rem 1rem; border-radius: 0.5rem; font-size: 0.875rem; font-weight: 600; cursor: pointer; }
.add-icon { width: 1rem; height: 1rem; }
.empty-state { text-align: center; padding: 3rem; color: rgba(120,120,128,0.7); }
.btn-add-empty { margin-top: 1rem; background: rgba(0,122,255,0.1); color: var(--color-accent, #007aff); border: 1px solid var(--color-accent, #007aff); padding: 0.5rem 1rem; border-radius: 0.5rem; cursor: pointer; }
.cards-grid { display: grid; gap: 0.75rem; }
.error-banner { background: rgba(255,59,48,0.1); color: #ff3b30; border: 1px solid #ff3b30; border-radius: 0.5rem; padding: 0.75rem 1rem; margin-bottom: 1rem; font-size: 0.875rem; word-break: break-all; }
</style>
