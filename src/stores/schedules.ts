import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { Schedule, VideoInfo } from '../types'
import { useLibraryStore } from './library'

export const useSchedulesStore = defineStore('schedules', () => {
  const schedules = ref<Schedule[]>([])
  const startupCheckingScheduleIds = ref<number[]>([])
  const unseenStartupCheckIds = ref<number[]>([])
  const checkingScheduleIds = ref<number[]>([])
  const listenerReady = ref(false)

  function addStartupCheckId(id: number) {
    if (!unseenStartupCheckIds.value.includes(id)) {
      unseenStartupCheckIds.value = [...unseenStartupCheckIds.value, id]
    }
  }

  function markStartupCheckingStarted(id: number) {
    if (!startupCheckingScheduleIds.value.includes(id)) {
      startupCheckingScheduleIds.value = [...startupCheckingScheduleIds.value, id]
    }
  }

  function markStartupCheckingFinished(id: number) {
    if (!startupCheckingScheduleIds.value.includes(id)) return
    startupCheckingScheduleIds.value = startupCheckingScheduleIds.value.filter(scheduleId => scheduleId !== id)
  }

  function markCheckingStarted(id: number) {
    if (!checkingScheduleIds.value.includes(id)) {
      checkingScheduleIds.value = [...checkingScheduleIds.value, id]
    }
  }

  function markCheckingFinished(id: number) {
    if (!checkingScheduleIds.value.includes(id)) return
    checkingScheduleIds.value = checkingScheduleIds.value.filter(scheduleId => scheduleId !== id)
  }

  function parseOptionsJson(schedule: Schedule): Record<string, unknown> | null {
    try {
      const parsed = JSON.parse(schedule.options_json)
      return parsed && typeof parsed === 'object' ? parsed : null
    } catch {
      return null
    }
  }

  function isLegacyYoutubeVideoThumbnail(url: string): boolean {
    return /https?:\/\/(?:i|img)\.ytimg\.com\/vi(?:_webp)?\//.test(url)
  }

  function shouldRefreshAvatar(schedule: Schedule): boolean {
    if (!schedule.is_channel) return false

    const options = parseOptionsJson(schedule)
    const avatarUrl = typeof options?.avatar_url === 'string' ? options.avatar_url : ''

    return !avatarUrl || isLegacyYoutubeVideoThumbnail(avatarUrl)
  }

  async function refreshChannelAvatars() {
    const targets = schedules.value.filter(shouldRefreshAvatar)

    await Promise.allSettled(targets.map(async (schedule) => {
      const options = parseOptionsJson(schedule)
      if (!options) return

      const currentAvatar = typeof options.avatar_url === 'string' ? options.avatar_url : ''
      const info = await invoke<VideoInfo>('fetch_channel_info', { url: schedule.url })
      const nextAvatar = info.channel_avatar_url || info.thumbnail_url

      if (!nextAvatar || nextAvatar === currentAvatar) return

      const nextOptions = {
        ...options,
        avatar_url: nextAvatar,
        channel_id: info.channel_id || options.channel_id || '',
      }

      await invoke('update_schedule', {
        id: schedule.id,
        name: schedule.name,
        url: schedule.url,
        cronExpr: schedule.cron_expr,
        optionsJson: JSON.stringify(nextOptions),
        isChannel: schedule.is_channel,
      })

      schedule.options_json = JSON.stringify(nextOptions)
    }))
  }

  async function fetchSchedules() {
    schedules.value = await invoke<Schedule[]>('list_schedules')
    void refreshChannelAvatars()
  }

  async function createSchedule(payload: {
    name: string
    url: string
    cron_expr: string
    options_json: string
    is_channel: boolean
  }): Promise<number> {
    const id = await invoke<number>('create_schedule', {
      name: payload.name,
      url: payload.url,
      cronExpr: payload.cron_expr,
      optionsJson: payload.options_json,
      isChannel: payload.is_channel,
    })
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
    await invoke('update_schedule', {
      id: payload.id,
      name: payload.name,
      url: payload.url,
      cronExpr: payload.cron_expr,
      optionsJson: payload.options_json,
      isChannel: payload.is_channel,
    })
    await fetchSchedules()
  }

  async function deleteSchedule(id: number) {
    await invoke('delete_schedule', { id })
    await fetchSchedules()
  }

  async function toggleSchedule(id: number, is_active: boolean) {
    await invoke('toggle_schedule', { id, isActive: is_active })
    await fetchSchedules()
  }

  async function runNow(id: number) {
    await invoke('run_schedule_now', { id })
  }

  async function stopSchedule(id: number) {
    await invoke('stop_schedule_run', { id })
    await fetchSchedules()
  }

  async function setupScheduleListener() {
    if (listenerReady.value) return
    listenerReady.value = true

    await listen<number>('startup-schedule-started', (event) => {
      markStartupCheckingStarted(event.payload)
    })
    await listen<number>('schedule-checking-started', (event) => {
      markCheckingStarted(event.payload)
    })
    await listen<number>('schedule-updated', (event) => {
      markCheckingFinished(event.payload)
      fetchSchedules()
    })
    await listen<number>('schedule-fired', (event) => {
      markCheckingFinished(event.payload)
      fetchSchedules()
      useLibraryStore().loadItems()
    })
    await listen<number>('startup-schedule-result', (event) => {
      addStartupCheckId(event.payload)
      markStartupCheckingFinished(event.payload)
      markCheckingFinished(event.payload)
      fetchSchedules()
      useLibraryStore().loadItems()
    })
  }

  function markStartupChecksSeen() {
    unseenStartupCheckIds.value = []
  }

  return {
    schedules,
    startupCheckingScheduleIds,
    unseenStartupCheckIds,
    checkingScheduleIds,
    fetchSchedules,
    createSchedule,
    updateSchedule,
    deleteSchedule,
    toggleSchedule,
    runNow,
    stopSchedule,
    setupScheduleListener,
    markStartupChecksSeen,
  }
})
