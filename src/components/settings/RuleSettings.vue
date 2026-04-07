<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useSettingsStore } from '../../stores/settings'
import { useI18n } from 'vue-i18n'
import type { AutoClassifyRule } from '../../types'

const { t } = useI18n()
const settingsStore = useSettingsStore()

const rules = ref<AutoClassifyRule[]>([])
const editingRule = ref<Partial<AutoClassifyRule> | null>(null)
const showEditor = ref(false)
const saving = ref(false)
const errorMsg = ref('')

const ruleTypes = [
  { value: 'site', labelKey: 'rules.type_site_label' },
  { value: 'format', labelKey: 'rules.type_format_label' },
  { value: 'date', labelKey: 'rules.type_date_label' },
]

onMounted(() => {
  if (settingsStore.settings.auto_classify) {
    void loadRules()
  }
})

watch(
  () => settingsStore.settings.auto_classify,
  (enabled) => {
    if (enabled && rules.value.length === 0) {
      void loadRules()
    }
  }
)

async function loadRules() {
  try {
    rules.value = await invoke<AutoClassifyRule[]>('list_rules')
  } catch (e) {
    errorMsg.value = `${t('rules.load_error')}: ${e}`
  }
}

function addNewRule() {
  errorMsg.value = ''
  editingRule.value = {
    rule_type: 'site',
    pattern: '',
    target_dir: '',
    priority: 0,
    enabled: true,
  }
  showEditor.value = true
}

function editRule(rule: AutoClassifyRule) {
  errorMsg.value = ''
  editingRule.value = { ...rule }
  showEditor.value = true
}

async function browseFolder() {
  const selected = await open({ directory: true, multiple: false })
  if (selected && editingRule.value) {
    editingRule.value.target_dir = selected as string
  }
}

async function saveRule() {
  if (!editingRule.value) return
  const r = editingRule.value
  if (!r.pattern?.trim()) {
    errorMsg.value = t('rules.pattern_required')
    return
  }
  if (!r.target_dir?.trim()) {
    errorMsg.value = t('rules.target_required')
    return
  }

  saving.value = true
  errorMsg.value = ''
  try {
    if (r.id) {
      await invoke('update_rule', {
        id: r.id,
        ruleType: r.rule_type ?? 'site',
        pattern: r.pattern,
        targetDir: r.target_dir,
        priority: r.priority ?? 0,
        enabled: r.enabled ?? true,
      })
    } else {
      const newId = await invoke<number>('create_rule', {
        ruleType: r.rule_type ?? 'site',
        pattern: r.pattern,
        targetDir: r.target_dir,
        priority: r.priority ?? 0,
        enabled: r.enabled ?? true,
      })
      r.id = newId
    }
    await loadRules()
    showEditor.value = false
    editingRule.value = null
  } catch (e) {
    errorMsg.value = `${t('rules.save_error')}: ${e}`
  } finally {
    saving.value = false
  }
}

async function deleteRule(id: number) {
  try {
    await invoke('delete_rule', { id })
    rules.value = rules.value.filter(r => r.id !== id)
  } catch (e) {
    errorMsg.value = `${t('rules.delete_error')}: ${e}`
  }
}

function cancelEdit() {
  showEditor.value = false
  editingRule.value = null
  errorMsg.value = ''
}
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h3 class="text-base font-semibold">{{ t('rules.title') }}</h3>
        <p class="text-xs text-neutral-500 mt-0.5">{{ t('rules.description') }}</p>
      </div>
      <div class="flex items-center gap-3">
        <label class="flex items-center gap-2 text-sm">
          <input type="checkbox" :checked="settingsStore.settings.auto_classify"
                 @change="settingsStore.updateSetting('auto_classify', ($event.target as HTMLInputElement).checked)"
                 class="rounded" />
          {{ t('common.enabled') }}
        </label>
        <button class="px-3 py-1.5 rounded-md text-xs bg-[var(--color-accent)] text-white" @click="addNewRule">
          + {{ t('rules.add') }}
        </button>
      </div>
    </div>

    <!-- Error message -->
    <div v-if="errorMsg" class="px-3 py-2 rounded-md bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-xs">
      {{ errorMsg }}
    </div>

    <!-- Rules list -->
    <div v-if="rules.length === 0 && !showEditor" class="p-8 text-center text-neutral-400 text-sm">
      {{ t('rules.empty') }}
    </div>

    <div v-else class="space-y-2">
      <div v-for="rule in rules" :key="rule.id"
           class="flex items-center gap-3 px-3 py-2 rounded-md bg-neutral-50 dark:bg-neutral-800/50"
           :class="{ 'opacity-50': !rule.enabled }">
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2">
            <span class="px-1.5 py-0.5 text-[10px] rounded bg-neutral-200 dark:bg-neutral-700 uppercase">
              {{ rule.rule_type }}
            </span>
            <span class="text-sm font-mono truncate">{{ rule.pattern }}</span>
          </div>
          <p class="text-xs text-neutral-500 mt-0.5 truncate">
            → {{ rule.target_dir }}
            <span v-if="rule.priority" class="ml-2 text-neutral-400">{{ t('rules.priority') }}: {{ rule.priority }}</span>
          </p>
        </div>
        <div class="flex gap-1">
          <button class="px-2 py-1 rounded text-xs hover:bg-neutral-200 dark:hover:bg-neutral-700"
                  @click="editRule(rule)">
            {{ t('common.edit') }}
          </button>
          <button class="px-2 py-1 rounded text-xs text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20"
                  @click="deleteRule(rule.id)">
            {{ t('common.delete') }}
          </button>
        </div>
      </div>
    </div>

    <!-- Rule editor -->
    <div v-if="showEditor && editingRule"
         class="p-4 rounded-lg border border-[var(--color-separator)] bg-white dark:bg-neutral-800 space-y-4">
      <h4 class="text-sm font-medium">{{ editingRule.id ? t('rules.edit_rule') : t('rules.new_rule') }}</h4>

      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-xs text-neutral-500 mb-1">{{ t('rules.type_label') }}</label>
          <select v-model="editingRule.rule_type"
                  class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-700 text-sm">
            <option v-for="rt in ruleTypes" :key="rt.value" :value="rt.value">{{ t(rt.labelKey) }}</option>
          </select>
        </div>
        <div>
          <label class="block text-xs text-neutral-500 mb-1">{{ t('rules.priority') }}</label>
          <input type="number" v-model.number="editingRule.priority"
                 class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-700 text-sm outline-none" />
        </div>
      </div>

      <div>
        <label class="block text-xs text-neutral-500 mb-1">{{ t('rules.pattern') }}</label>
        <input v-model="editingRule.pattern"
               class="w-full h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-700 text-sm font-mono outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
               :placeholder="editingRule.rule_type === 'site' ? 'YouTube' : editingRule.rule_type === 'format' ? 'mp3' : '2026-*'" />
      </div>

      <div>
        <label class="block text-xs text-neutral-500 mb-1">{{ t('rules.target_dir') }}</label>
        <div class="flex gap-2">
          <input v-model="editingRule.target_dir"
                 class="flex-1 h-8 px-2 rounded-md bg-neutral-100 dark:bg-neutral-700 text-sm font-mono outline-none focus:ring-1 focus:ring-[var(--color-accent)]"
                 placeholder="~/Downloads/YTDown/Music/" />
          <button class="px-3 h-8 rounded-md text-sm bg-neutral-200 dark:bg-neutral-700 hover:bg-neutral-300 dark:hover:bg-neutral-600"
                  @click="browseFolder">
            {{ t('common.browse') }}
          </button>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <input type="checkbox" id="rule-enabled" v-model="editingRule.enabled" class="rounded" />
        <label for="rule-enabled" class="text-sm">{{ t('rules.enabled_label') }}</label>
      </div>

      <div class="flex justify-end gap-2">
        <button class="px-4 py-1.5 rounded-md text-sm bg-neutral-100 dark:bg-neutral-700" @click="cancelEdit">
          {{ t('common.cancel') }}
        </button>
        <button class="px-4 py-1.5 rounded-md text-sm bg-[var(--color-accent)] text-white disabled:opacity-50"
                :disabled="saving"
                @click="saveRule">
          {{ saving ? t('rules.saving') : t('common.save') }}
        </button>
      </div>
    </div>

    <!-- Hint -->
    <div class="p-3 rounded-md bg-neutral-50 dark:bg-neutral-800/50 text-xs text-neutral-500 space-y-1">
      <p class="font-medium">{{ t('rules.hint_title') }}</p>
      <ul class="list-disc list-inside space-y-0.5">
        <li><strong>{{ t('rules.type_site_label') }}</strong>: {{ t('rules.hint_site') }}</li>
        <li><strong>{{ t('rules.type_format_label') }}</strong>: {{ t('rules.hint_format') }}</li>
        <li><strong>{{ t('rules.type_date_label') }}</strong>: {{ t('rules.hint_date') }}</li>
        <li>{{ t('rules.hint_priority_note') }}</li>
      </ul>
    </div>
  </div>
</template>
