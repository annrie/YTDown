<script setup lang="ts">
import { useSettingsStore } from '../../stores/settings'

const settingsStore = useSettingsStore()

const subFormats = ['', 'srt', 'ass', 'vtt', 'lrc']
const videoFormats = ['', 'mp4', 'mkv', 'webm', 'flv', 'avi']
</script>

<template>
  <div class="space-y-6">
    <h3 class="text-base font-semibold">詳細オプション</h3>
    <p class="text-xs text-neutral-500">yt-dlpの追加オプションを設定します。これらはすべてのダウンロードに適用されます。</p>

    <!-- Boolean options -->
    <div class="space-y-3">
      <h4 class="text-sm font-medium">ファイル・ネットワーク</h4>

      <label class="flex items-center gap-3 text-sm">
        <input type="checkbox" :checked="settingsStore.settings.restrict_filenames"
               @change="settingsStore.updateSetting('restrict_filenames', ($event.target as HTMLInputElement).checked)"
               class="rounded" />
        ファイル名をASCIIに制限
        <span class="text-xs text-neutral-400">--restrict-filenames</span>
      </label>

      <label class="flex items-center gap-3 text-sm">
        <input type="checkbox" :checked="settingsStore.settings.no_overwrites"
               @change="settingsStore.updateSetting('no_overwrites', ($event.target as HTMLInputElement).checked)"
               class="rounded" />
        既存ファイルを上書きしない
        <span class="text-xs text-neutral-400">--no-overwrites</span>
      </label>

      <label class="flex items-center gap-3 text-sm">
        <input type="checkbox" :checked="settingsStore.settings.geo_bypass"
               @change="settingsStore.updateSetting('geo_bypass', ($event.target as HTMLInputElement).checked)"
               class="rounded" />
        地域制限を回避
        <span class="text-xs text-neutral-400">--geo-bypass</span>
      </label>
    </div>

    <!-- String/Select options -->
    <div class="space-y-4">
      <h4 class="text-sm font-medium">映像・字幕</h4>

      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-xs text-neutral-500 mb-1">マージ出力形式 <span class="text-neutral-400">--merge-output-format</span></label>
          <select :value="settingsStore.settings.merge_output_format"
                  @change="settingsStore.updateSetting('merge_output_format', ($event.target as HTMLSelectElement).value)"
                  class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm">
            <option value="">指定なし</option>
            <option v-for="f in videoFormats.slice(1)" :key="f" :value="f">{{ f.toUpperCase() }}</option>
          </select>
        </div>
        <div>
          <label class="block text-xs text-neutral-500 mb-1">再エンコード形式 <span class="text-neutral-400">--recode-video</span></label>
          <select :value="settingsStore.settings.recode_video"
                  @change="settingsStore.updateSetting('recode_video', ($event.target as HTMLSelectElement).value)"
                  class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm">
            <option value="">指定なし</option>
            <option v-for="f in videoFormats.slice(1)" :key="f" :value="f">{{ f.toUpperCase() }}</option>
          </select>
        </div>
      </div>

      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-xs text-neutral-500 mb-1">字幕言語 <span class="text-neutral-400">--sub-lang</span></label>
          <input :value="settingsStore.settings.sub_lang"
                 @input="settingsStore.updateSetting('sub_lang', ($event.target as HTMLInputElement).value)"
                 class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm font-mono"
                 placeholder="ja,en（カンマ区切り）" />
        </div>
        <div>
          <label class="block text-xs text-neutral-500 mb-1">字幕変換形式 <span class="text-neutral-400">--convert-subs</span></label>
          <select :value="settingsStore.settings.convert_subs"
                  @change="settingsStore.updateSetting('convert_subs', ($event.target as HTMLSelectElement).value)"
                  class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm">
            <option v-for="f in subFormats" :key="f" :value="f">{{ f || '指定なし' }}</option>
          </select>
        </div>
      </div>
    </div>

    <div class="space-y-4">
      <h4 class="text-sm font-medium">ネットワーク</h4>

      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-xs text-neutral-500 mb-1">速度制限 <span class="text-neutral-400">-r</span></label>
          <input :value="settingsStore.settings.rate_limit"
                 @input="settingsStore.updateSetting('rate_limit', ($event.target as HTMLInputElement).value)"
                 class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm font-mono"
                 placeholder="例: 1M, 500K" />
        </div>
        <div>
          <label class="block text-xs text-neutral-500 mb-1">リトライ回数 <span class="text-neutral-400">--retries</span></label>
          <input type="number" :value="settingsStore.settings.retries"
                 @input="settingsStore.updateSetting('retries', parseInt(($event.target as HTMLInputElement).value) || 10)"
                 class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm"
                 min="0" max="100" />
        </div>
      </div>

      <div>
        <label class="block text-xs text-neutral-500 mb-1">プロキシ <span class="text-neutral-400">--proxy</span></label>
        <input :value="settingsStore.settings.proxy"
               @input="settingsStore.updateSetting('proxy', ($event.target as HTMLInputElement).value)"
               class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm font-mono"
               placeholder="例: socks5://127.0.0.1:1080" />
      </div>
    </div>

    <!-- Extra args (free input) -->
    <div>
      <h4 class="text-sm font-medium mb-1">カスタム引数</h4>
      <p class="text-xs text-neutral-500 mb-2">上記にないyt-dlpオプションをスペース区切りで入力できます。</p>
      <input :value="settingsStore.settings.extra_args"
             @input="settingsStore.updateSetting('extra_args', ($event.target as HTMLInputElement).value)"
             class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-800 text-sm font-mono"
             placeholder="例: --write-description --write-info-json" />
    </div>
  </div>
</template>
