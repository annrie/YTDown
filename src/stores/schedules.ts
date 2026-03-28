import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { Schedule } from '../types'

export const useSchedulesStore = defineStore('schedules', () => {
  const schedules = ref<Schedule[]>([])

  async function fetchSchedules() {
    schedules.value = await invoke<Schedule[]>('list_schedules')
  }

  async function createSchedule(payload: {
    name: string
    url: string
    cron_expr: string
    options_json: string
    is_channel: boolean
  }): Promise<number> {
    const id = await invoke<number>('create_schedule', payload)
    await fetchSchedules()
    return id
  }

  async function updateSchedule(payload: {
    id: number
    name: string
    url: string
    cron_expr: string
    options_json: string
    is_channel: boolean
  }) {
    await invoke('update_schedule', payload)
    await fetchSchedules()
  }

  async function deleteSchedule(id: number) {
    await invoke('delete_schedule', { id })
    await fetchSchedules()
  }

  async function toggleSchedule(id: number, is_active: boolean) {
    await invoke('toggle_schedule', { id, is_active })
    await fetchSchedules()
  }

  async function runNow(id: number) {
    await invoke('run_schedule_now', { id })
  }

  async function setupScheduleListener() {
    await listen('schedule-fired', () => {
      fetchSchedules()
    })
  }

  return {
    schedules,
    fetchSchedules,
    createSchedule,
    updateSchedule,
    deleteSchedule,
    toggleSchedule,
    runNow,
    setupScheduleListener,
  }
})
