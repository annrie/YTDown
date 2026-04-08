<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { homeDir } from '@tauri-apps/api/path'
import { usePresetsStore } from '../../stores/presets'
import { useAutoPresetStore } from '../../stores/autoPreset'
import { useI18n } from 'vue-i18n'
import type { Preset } from '../../types'

const { t } = useI18n()
const store = usePresetsStore()
const autoPresetStore = useAutoPresetStore()

const editingId = ref<number | null>(null)
const showCreateForm = ref(false)
const errorMsg = ref('')


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

// Auto-preset rule state
const newRuleDomain = ref('')
const newRulePresetId = ref<number | ''>('')
const ruleError = ref('')

onMounted(() => {
  store.fetchPresets()
  autoPresetStore.fetchRules()
})

async function addRule() {
  ruleError.value = ''
  const domain = newRuleDomain.value.trim()
  if (!domain || !newRulePresetId.value) return
  try {
    await autoPresetStore.createRule(domain, newRulePresetId.value as number, true)
    newRuleDomain.value = ''
    newRulePresetId.value = ''
  } catch (e) {
    ruleError.value = String(e)
  }
}

async function toggleRule(id: number, domain: string, presetId: number, enabled: boolean) {
  await autoPresetStore.updateRule(id, domain, presetId, !enabled)
}

async function deleteRule(id: number) {
  if (!confirm(t('auto_preset.delete_confirm'))) return
  await autoPresetStore.deleteRule(id)
}

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

async function selectDirectory() {
  try {
    const defaultPath = form.value.output_dir
      ? form.value.output_dir.replace(/^~/, await homeDir())
      : await homeDir()
    const selected = await openDialog({
      directory: true,
      multiple: false,
      defaultPath
    })
    if (selected && typeof selected === 'string') {
      form.value.output_dir = selected
    }
  } catch (e) {
    console.error('Failed to open dialog:', e)
  }
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
    errorMsg.value = t('presets.save_error', { error: e })
  }
}

async function onDelete(id: number) {
  if (!confirm(t('presets.delete_confirm'))) return
  try {
    await store.deletePreset(id)
  } catch (e) {
    errorMsg.value = t('presets.delete_error', { error: e })
  }
}
</script>

<template>
  <div>
    <div class="flex items-center justify-between mb-3">
      <h3 class="text-base font-semibold">{{ t('presets.title') }}</h3>
      <button
        class="text-sm px-3 py-1 rounded border border-[var(--color-separator)] hover:bg-white/10"
        @click="startCreate"
      >
        {{ t('presets.new_create') }}
      </button>
    </div>

    <!-- Create form (inline) -->
    <div v-if="showCreateForm" class="rounded border border-[var(--color-separator)] p-3 mb-4 space-y-2">
      <div>
        <label class="text-xs">{{ t('presets.name') }}</label>
        <input v-model="form.name" type="text" class="w-full rounded border border-[var(--color-separator)] bg-transparent px-2 py-1 text-sm mt-1" />
      </div>
      <div class="mb-3">
        <label class="text-xs">{{ t('presets.output_dir') }}</label>
        <div class="flex gap-2 items-center mt-1">
          <input v-model="form.output_dir" type="text" class="flex-1 rounded border border-[var(--color-separator)] bg-transparent px-2 py-1 text-sm" placeholder="~/Downloads/YTDown/" />
          <button @click="selectDirectory" class="px-3 rounded-md bg-[var(--color-accent)] text-white text-sm hover:opacity-90 transition-opacity">{{ t('common.browse') }}</button>
        </div>
      </div>
      <div class="flex flex-wrap gap-3 text-sm">
        <label><input type="checkbox" v-model="form.embed_thumbnail" class="mr-1">{{ t('presets.embed_thumbnail') }}</label>
        <label><input type="checkbox" v-model="form.embed_metadata" class="mr-1">{{ t('presets.embed_metadata') }}</label>
        <label><input type="checkbox" v-model="form.write_subs" class="mr-1">{{ t('presets.write_subs') }}</label>
        <label><input type="checkbox" v-model="form.embed_subs" class="mr-1">{{ t('presets.embed_subs') }}</label>
        <label><input type="checkbox" v-model="form.embed_chapters" class="mr-1">{{ t('presets.embed_chapters') }}</label>
        <label><input type="checkbox" v-model="form.sponsorblock" class="mr-1">{{ t('presets.sponsorblock') }}</label>
      </div>
      <div class="flex items-center gap-2 mt-2">
        <button class="text-sm px-3 py-1 rounded bg-blue-500 text-white hover:bg-blue-600 disabled:opacity-50"
                :disabled="!form.name.trim()" @click="saveForm">{{ t('presets.save') }}</button>
        <button class="text-sm px-3 py-1 rounded border border-[var(--color-separator)] hover:bg-white/10"
                @click="cancelForm">{{ t('presets.cancel') }}</button>
        <span v-if="errorMsg" class="text-xs text-red-400">{{ errorMsg }}</span>
      </div>
    </div>

    <!-- Preset list -->
    <div v-if="store.presets.length === 0 && !showCreateForm" class="text-sm text-neutral-500 py-4">
      {{ t('presets.empty') }}
    </div>

    <div class="space-y-2">
      <div v-for="preset in store.presets" :key="preset.id"
           class="rounded border border-[var(--color-separator)] p-3">
        <!-- Edit form (inline) -->
        <div v-if="editingId === preset.id" class="space-y-2">
          <div>
            <label class="text-xs">{{ t('presets.name') }}</label>
            <input v-model="form.name" type="text" class="w-full rounded border border-[var(--color-separator)] bg-transparent px-2 py-1 text-sm mt-1" />
          </div>
          <div class="mb-3">
            <label class="text-xs">{{ t('presets.output_dir') }}</label>
            <div class="flex gap-2 items-center mt-1">
              <input v-model="form.output_dir" type="text" class="flex-1 rounded border border-[var(--color-separator)] bg-transparent px-2 py-1 text-sm" />
              <button @click="selectDirectory" class="px-3 rounded-md bg-[var(--color-accent)] text-white text-sm hover:opacity-90 transition-opacity">{{ t('common.browse') }}</button>
            </div>
          </div>
          <div class="flex flex-wrap gap-3 text-sm">
            <label><input type="checkbox" v-model="form.embed_thumbnail" class="mr-1">{{ t('presets.embed_thumbnail') }}</label>
            <label><input type="checkbox" v-model="form.embed_metadata" class="mr-1">{{ t('presets.embed_metadata') }}</label>
            <label><input type="checkbox" v-model="form.write_subs" class="mr-1">{{ t('presets.write_subs') }}</label>
            <label><input type="checkbox" v-model="form.embed_subs" class="mr-1">{{ t('presets.embed_subs') }}</label>
            <label><input type="checkbox" v-model="form.embed_chapters" class="mr-1">{{ t('presets.embed_chapters') }}</label>
            <label><input type="checkbox" v-model="form.sponsorblock" class="mr-1">{{ t('presets.sponsorblock') }}</label>
          </div>
          <div class="flex items-center gap-2 mt-2">
            <button class="text-sm px-3 py-1 rounded bg-blue-500 text-white hover:bg-blue-600 disabled:opacity-50"
                    :disabled="!form.name.trim()" @click="saveForm">{{ t('presets.save') }}</button>
            <button class="text-sm px-3 py-1 rounded border border-[var(--color-separator)] hover:bg-white/10"
                    @click="cancelForm">{{ t('presets.cancel') }}</button>
            <span v-if="errorMsg" class="text-xs text-red-400">{{ errorMsg }}</span>
          </div>
        </div>
        <!-- Card display -->
        <div v-else class="flex items-center justify-between">
          <div>
            <span class="font-medium text-sm">{{ preset.name }}</span>
            <span class="ml-2 text-xs text-neutral-400 truncate max-w-[200px] inline-block align-bottom">
              {{ preset.output_dir }}
            </span>
          </div>
          <div class="flex gap-2">
            <button class="text-xs px-2 py-1 rounded border border-[var(--color-separator)] hover:bg-white/10"
                    @click="startEdit(preset)">{{ t('presets.edit') }}</button>
            <button class="text-xs px-2 py-1 rounded border border-red-400 text-red-400 hover:bg-red-400/10"
                    @click="onDelete(preset.id)">{{ t('common.delete') }}</button>
          </div>
        </div>
      </div>
    </div>
    <!-- Auto Preset Rules Section -->
    <div class="mt-6 pt-6 border-t border-[var(--color-separator)]">
      <h3 class="text-base font-semibold mb-3">{{ t('auto_preset.title') }}</h3>

      <!-- Add new rule -->
      <div class="flex items-center gap-2 mb-3">
        <input
          v-model="newRuleDomain"
          type="text"
          :placeholder="t('auto_preset.domain_placeholder')"
          class="flex-1 rounded border border-[var(--color-separator)] bg-transparent px-2 py-1 text-sm"
          @keyup.enter="addRule"
        />
        <select
          v-model="newRulePresetId"
          class="rounded border border-[var(--color-separator)] bg-transparent px-2 py-1 text-sm"
        >
          <option value="">{{ t('auto_preset.preset_label') }}...</option>
          <option v-for="p in store.presets" :key="p.id" :value="p.id">{{ p.name }}</option>
        </select>
        <button
          class="text-sm px-3 py-1 rounded bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-40"
          :disabled="!newRuleDomain.trim() || !newRulePresetId"
          @click="addRule"
        >{{ t('auto_preset.add') }}</button>
      </div>
      <span v-if="ruleError" class="text-xs text-red-400">{{ ruleError }}</span>

      <!-- Rule list -->
      <div v-if="autoPresetStore.rules.length === 0" class="text-sm text-neutral-500 py-2">
        {{ t('auto_preset.empty') }}
      </div>
      <div class="space-y-1">
        <div v-for="rule in autoPresetStore.rules" :key="rule.id"
             class="flex items-center gap-2 rounded border border-[var(--color-separator)] px-3 py-2">
          <span class="flex-1 text-sm font-mono">{{ rule.domain }}</span>
          <span class="text-xs text-neutral-400 truncate max-w-[120px]">
            {{ store.presets.find(p => p.id === rule.preset_id)?.name ?? '?' }}
          </span>
          <button
            class="text-xs px-2 py-0.5 rounded border"
            :class="rule.enabled
              ? 'border-green-500 text-green-600 hover:bg-green-50 dark:hover:bg-green-900/20'
              : 'border-neutral-400 text-neutral-400 hover:bg-neutral-50 dark:hover:bg-neutral-800'"
            @click="toggleRule(rule.id, rule.domain, rule.preset_id, rule.enabled)"
          >{{ rule.enabled ? t('auto_preset.enabled') : t('common.disabled') }}</button>
          <button
            class="text-xs px-2 py-0.5 rounded border border-red-400 text-red-400 hover:bg-red-400/10"
            @click="deleteRule(rule.id)"
          >{{ t('common.delete') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>
