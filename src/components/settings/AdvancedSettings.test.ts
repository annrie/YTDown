import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { i18n } from '../../i18n'
import AdvancedSettings from './AdvancedSettings.vue'

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }))

function ytdlpInfo(path: string) {
  return { path, version: '2026.06.09', update_available: false, latest_version: null, managed_by: 'manual' }
}

function mountComponent() {
  return mount(AdvancedSettings, { global: { plugins: [i18n, createPinia()] } })
}

describe('AdvancedSettings — yt-dlp path', () => {
  beforeEach(() => {
    invokeMock.mockReset()
  })

  it('saves the path on change, then re-resolves yt-dlp info', async () => {
    const newPath = '/Users/me/.anyenv/envs/pyenv/versions/3.12.1/bin/yt-dlp'
    let currentPath = '/usr/local/bin/yt-dlp'
    invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'set_setting' && args?.key === 'ytdlp_path') {
        // Real IPC takes a tick; a missing `await` before re-resolving would read the old value
        await new Promise((resolve) => setTimeout(resolve, 0))
        currentPath = String(args.value)
      }
      if (cmd === 'get_ytdlp_info') return ytdlpInfo(currentPath)
      return undefined
    })

    const wrapper = mountComponent()
    await flushPromises()
    expect(wrapper.get('[data-testid="ytdlp-info-path"]').text()).toBe('/usr/local/bin/yt-dlp')

    const input = wrapper.get('input[data-testid="ytdlp-path"]')
    ;(input.element as HTMLInputElement).value = newPath
    await input.trigger('change')
    await flushPromises()

    expect(invokeMock).toHaveBeenCalledWith('set_setting', { key: 'ytdlp_path', value: newPath })
    const order = invokeMock.mock.calls.map((c) => c[0])
    expect(order.lastIndexOf('set_setting')).toBeLessThan(order.lastIndexOf('get_ytdlp_info'))
    expect(wrapper.get('[data-testid="ytdlp-info-path"]').text()).toBe(newPath)
  })

  it('shows the backend error when yt-dlp cannot be resolved', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_ytdlp_info') throw 'Manual yt-dlp path not found: /nope/yt-dlp'
      return undefined
    })

    const wrapper = mountComponent()
    await flushPromises()

    expect(wrapper.text()).toContain('Manual yt-dlp path not found: /nope/yt-dlp')
  })

  it('drops stale info and shows the error when a changed path cannot be resolved', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_ytdlp_info') return ytdlpInfo('/usr/local/bin/yt-dlp')
      return undefined
    })
    const wrapper = mountComponent()
    await flushPromises()
    expect(wrapper.get('[data-testid="ytdlp-info-path"]').text()).toBe('/usr/local/bin/yt-dlp')

    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_ytdlp_info') throw 'Manual yt-dlp path not found: /nope/yt-dlp'
      return undefined
    })
    const input = wrapper.get('input[data-testid="ytdlp-path"]')
    ;(input.element as HTMLInputElement).value = '/nope/yt-dlp'
    await input.trigger('change')
    await flushPromises()

    expect(wrapper.find('[data-testid="ytdlp-info-path"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('Manual yt-dlp path not found: /nope/yt-dlp')
  })

  it('does not re-resolve and shows a save error when persisting the path fails', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {})
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'set_setting') throw 'DB error: disk I/O error'
      if (cmd === 'get_ytdlp_info') return ytdlpInfo('/usr/local/bin/yt-dlp')
      return undefined
    })
    const wrapper = mountComponent()
    await flushPromises()

    const input = wrapper.get('input[data-testid="ytdlp-path"]')
    ;(input.element as HTMLInputElement).value = '/new/yt-dlp'
    await input.trigger('change')
    await flushPromises()

    const infoCalls = invokeMock.mock.calls.filter((c) => c[0] === 'get_ytdlp_info').length
    expect(infoCalls).toBe(1)
    expect(wrapper.text()).toContain(i18n.global.t('general.ytdlp_path_save_failed'))
    expect(wrapper.get('[data-testid="ytdlp-info-path"]').text()).toBe('/usr/local/bin/yt-dlp')
    consoleError.mockRestore()
  })
})
