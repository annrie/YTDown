<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { usePresetsStore } from '../../stores/presets'
import type { Preset } from '../../types'

const store = usePresetsStore()

const editingId = ref<number | null>(null)
const showCreateForm = ref(false)
const errorMsg = ref('')

const allFormats = ['mp4', 'mkv', 'webm', 'mp3', 'm4a', 'flac', 'wav', 'opus']
const qualities = ['best', '2160', '1080', '720', '480']

function blankForm(): Omit<Preset, 'id' | 'created_at'> {
  return {
    name: '',
    format: 'mp4',
    quality: 'best',
    output_dir: '',
    embed_thumbnail: true,
    embed_metadata: true,
    write_subs: false,
    embed_subs: false,
    embed_chapters: true,
    sponsorblock: false,
  }
}

const form = ref(blankForm())

onMounted(() => store.fetchPresets())

function startCreate() {
  form.value = blankForm()
  editingId.value = null
  showCreateForm.value = true
  errorMsg.value = ''
}

function startEdit(preset: Preset) {
  form.value = {
    name: preset.name,
    format: preset.format,
    quality: preset.quality,
    output_dir: preset.output_dir,
    embed_thumbnail: preset.embed_thumbnail,
    embed_metadata: preset.embed_metadata,
    write_subs: preset.write_subs,
    embed_subs: preset.embed_subs,
    embed_chapters: preset.embed_chapters,
    sponsorblock: preset.sponsorblock,
  }
  editingId.value = preset.id
  showCreateForm.value = false
  errorMsg.value = ''
}

function cancelForm() {
  showCreateForm.value = false
  editingId.value = null
  errorMsg.value = ''
}

async function saveForm() {
  errorMsg.value = ''
  try {
    if (editingId.value !== null) {
      await store.updatePreset({ id: editingId.value, ...form.value })
      editingId.value = null
    } else {
      await store.createPreset(form.value)
      showCreateForm.value = false
    }
  } catch (e) {
    errorMsg.value = `保存失敗: ${e}`
  }
}

async function onDelete(id: number) {
  if (!confirm('このプリセットを削除しますか？')) return
  try {
    await store.deletePreset(id)
  } catch (e) {
    errorMsg.value = `削除失敗: ${e}`
  }
}
</script>

<template>
  <div>
    <div class="flex items-center justify-between mb-3">
      <h3 class="text-base font-semibold">ダウンロードプリセット</h3>
      <button
        class="text-sm px-3 py-1 rounded border border-[var(--color-separator)] hover:bg-white/10"
        @click="startCreate"
      >
        ＋ 新規作成
      </button>
    </div>

    <!-- Create form (inline) -->
    <div v-if="showCreateForm" class="rounded border border-[var(--color-separator)] p-3 mb-4 space-y-2">
      <div>
        <label class="text-xs">名前</label>
        <input v-model="form.name" type="text" class="w-full rounded border border-[var(--color-separator)] bg-transparent px-2 py-1 text-sm mt-1" />
      </div>
      <div class="flex gap-2">
        <div class="flex-1">
          <label class="text-xs">フォーマット</label>
          <select v-model="form.format" class="w-full rounded border border-[var(--color-separator)] bg-transparent px-2 py-1 text-sm mt-1">
            <option v-for="f in allFormats" :key="f" :value="f">{{ f }}</option>
          </select>
        </div>
        <div class="flex-1">
          <label class="text-xs">品質</label>
          <select v-model="form.quality" class="w-full rounded border border-[var(--color-separator)] bg-transparent px-2 py-1 text-sm mt-1">
            <option v-for="q in qualities" :key="q" :value="q">{{ q }}</option>
          </select>
        </div>
      </div>
      <div>
        <label class="text-xs">出力先ディレクトリ</label>
        <input v-model="form.output_dir" type="text" class="w-full rounded border border-[var(--color-separator)] bg-transparent px-2 py-1 text-sm mt-1" placeholder="~/Downloads/YTDown/" />
      </div>
      <div class="flex flex-wrap gap-3 text-sm">
        <label><input type="checkbox" v-model="form.embed_thumbnail" class="mr-1">サムネイル埋め込み</label>
        <label><input type="checkbox" v-model="form.embed_metadata" class="mr-1">メタデータ埋め込み</label>
        <label><input type="checkbox" v-model="form.write_subs" class="mr-1">字幕保存</label>
        <label><input type="checkbox" v-model="form.embed_subs" class="mr-1">字幕埋め込み</label>
        <label><input type="checkbox" v-model="form.embed_chapters" class="mr-1">チャプター埋め込み</label>
        <label><input type="checkbox" v-model="form.sponsorblock" class="mr-1">SponsorBlock</label>
      </div>
      <div class="flex items-center gap-2 mt-2">
        <button class="text-sm px-3 py-1 rounded bg-blue-500 text-white hover:bg-blue-600 disabled:opacity-50"
                :disabled="!form.name.trim()" @click="saveForm">保存</button>
        <button class="text-sm px-3 py-1 rounded border border-[var(--color-separator)] hover:bg-white/10"
                @click="cancelForm">キャンセル</button>
        <span v-if="errorMsg" class="text-xs text-red-400">{{ errorMsg }}</span>
      </div>
    </div>

    <!-- Preset list -->
    <div v-if="store.presets.length === 0 && !showCreateForm" class="text-sm text-neutral-500 py-4">
      プリセットはありません
    </div>

    <div class="space-y-2">
      <div v-for="preset in store.presets" :key="preset.id"
           class="rounded border border-[var(--color-separator)] p-3">
        <!-- Edit form (inline) -->
        <div v-if="editingId === preset.id" class="space-y-2">
          <div>
            <label class="text-xs">名前</label>
            <input v-model="form.name" type="text" class="w-full rounded border border-[var(--color-separator)] bg-transparent px-2 py-1 text-sm mt-1" />
          </div>
          <div class="flex gap-2">
            <div class="flex-1">
              <label class="text-xs">フォーマット</label>
              <select v-model="form.format" class="w-full rounded border border-[var(--color-separator)] bg-transparent px-2 py-1 text-sm mt-1">
                <option v-for="f in allFormats" :key="f" :value="f">{{ f }}</option>
              </select>
            </div>
            <div class="flex-1">
              <label class="text-xs">品質</label>
              <select v-model="form.quality" class="w-full rounded border border-[var(--color-separator)] bg-transparent px-2 py-1 text-sm mt-1">
                <option v-for="q in qualities" :key="q" :value="q">{{ q }}</option>
              </select>
            </div>
          </div>
          <div>
            <label class="text-xs">出力先ディレクトリ</label>
            <input v-model="form.output_dir" type="text" class="w-full rounded border border-[var(--color-separator)] bg-transparent px-2 py-1 text-sm mt-1" />
          </div>
          <div class="flex flex-wrap gap-3 text-sm">
            <label><input type="checkbox" v-model="form.embed_thumbnail" class="mr-1">サムネイル埋め込み</label>
            <label><input type="checkbox" v-model="form.embed_metadata" class="mr-1">メタデータ埋め込み</label>
            <label><input type="checkbox" v-model="form.write_subs" class="mr-1">字幕保存</label>
            <label><input type="checkbox" v-model="form.embed_subs" class="mr-1">字幕埋め込み</label>
            <label><input type="checkbox" v-model="form.embed_chapters" class="mr-1">チャプター埋め込み</label>
            <label><input type="checkbox" v-model="form.sponsorblock" class="mr-1">SponsorBlock</label>
          </div>
          <div class="flex items-center gap-2 mt-2">
            <button class="text-sm px-3 py-1 rounded bg-blue-500 text-white hover:bg-blue-600 disabled:opacity-50"
                    :disabled="!form.name.trim()" @click="saveForm">保存</button>
            <button class="text-sm px-3 py-1 rounded border border-[var(--color-separator)] hover:bg-white/10"
                    @click="cancelForm">キャンセル</button>
            <span v-if="errorMsg" class="text-xs text-red-400">{{ errorMsg }}</span>
          </div>
        </div>
        <!-- Card display -->
        <div v-else class="flex items-center justify-between">
          <div>
            <span class="font-medium text-sm">{{ preset.name }}</span>
            <span class="ml-2 text-xs text-neutral-500">{{ preset.format }} / {{ preset.quality }}</span>
            <span class="ml-2 text-xs text-neutral-400 truncate max-w-[200px] inline-block align-bottom">
              {{ preset.output_dir }}
            </span>
          </div>
          <div class="flex gap-2">
            <button class="text-xs px-2 py-1 rounded border border-[var(--color-separator)] hover:bg-white/10"
                    @click="startEdit(preset)">編集</button>
            <button class="text-xs px-2 py-1 rounded border border-red-400 text-red-400 hover:bg-red-400/10"
                    @click="onDelete(preset.id)">削除</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
