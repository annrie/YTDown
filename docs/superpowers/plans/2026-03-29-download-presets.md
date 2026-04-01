# Download Presets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow users to save and reuse download configurations (format, quality, output dir, embed options) via a dropdown in DownloadDialog and a management UI in Settings.

**Architecture:** New `download_presets` SQLite table with individual columns (not JSON). Rust command layer in `commands/presets.rs`. Pinia store in `stores/presets.ts`. Dropdown in DownloadDialog; CRUD UI in a new `PresetSettings.vue` settings section.

**Tech Stack:** Rust (rusqlite, Tauri v2), Vue 3 Composition API, TypeScript, Pinia, Tailwind CSS

---

## File Map

| Action | File |
|--------|------|
| Modify | `src-tauri/src/db/schema.sql` |
| Modify | `src-tauri/src/db/models.rs` |
| Modify | `src-tauri/src/db/queries.rs` |
| Create | `src-tauri/src/commands/presets.rs` |
| Modify | `src-tauri/src/commands/mod.rs` |
| Modify | `src-tauri/src/lib.rs` |
| Modify | `src-tauri/capabilities/default.json` |
| Modify | `src/types/index.ts` |
| Create | `src/stores/presets.ts` |
| Modify | `src/components/download/DownloadDialog.vue` |
| Create | `src/components/settings/PresetSettings.vue` |
| Modify | `src/App.vue` |

---

## Task 1: DB Schema + Rust Model

**Files:**
- Modify: `src-tauri/src/db/schema.sql`
- Modify: `src-tauri/src/db/models.rs`

- [ ] **Step 1: Add table to schema.sql**

Append after the `schedules` table (end of file):

```sql
CREATE TABLE IF NOT EXISTS download_presets (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  name            TEXT NOT NULL UNIQUE,
  format          TEXT NOT NULL,
  quality         TEXT NOT NULL,
  output_dir      TEXT NOT NULL,
  embed_thumbnail INTEGER NOT NULL DEFAULT 0,
  embed_metadata  INTEGER NOT NULL DEFAULT 0,
  write_subs      INTEGER NOT NULL DEFAULT 0,
  embed_subs      INTEGER NOT NULL DEFAULT 0,
  embed_chapters  INTEGER NOT NULL DEFAULT 0,
  sponsorblock    INTEGER NOT NULL DEFAULT 0,
  created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
```

- [ ] **Step 2: Add Preset struct to models.rs**

Append after the `Schedule` struct (end of file):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub id: i64,
    pub name: String,
    pub format: String,
    pub quality: String,
    pub output_dir: String,
    pub embed_thumbnail: bool,
    pub embed_metadata: bool,
    pub write_subs: bool,
    pub embed_subs: bool,
    pub embed_chapters: bool,
    pub sponsorblock: bool,
    pub created_at: String,
}
```

- [ ] **Step 3: Verify compile**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db/schema.sql src-tauri/src/db/models.rs
git commit -m "feat(presets): add download_presets table and Preset model"
```

---

## Task 2: DB Queries

**Files:**
- Modify: `src-tauri/src/db/queries.rs` (append to end of file)

- [ ] **Step 1: Add 4 query functions**

Append to the end of `src-tauri/src/db/queries.rs`:

```rust
// ── Presets ──────────────────────────────────────────────────────────────

pub fn list_presets(conn: &Connection) -> SqlResult<Vec<crate::db::models::Preset>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, format, quality, output_dir,
                embed_thumbnail, embed_metadata, write_subs, embed_subs,
                embed_chapters, sponsorblock, created_at
         FROM download_presets ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(crate::db::models::Preset {
            id: row.get(0)?,
            name: row.get(1)?,
            format: row.get(2)?,
            quality: row.get(3)?,
            output_dir: row.get(4)?,
            embed_thumbnail: row.get::<_, i64>(5)? != 0,
            embed_metadata: row.get::<_, i64>(6)? != 0,
            write_subs: row.get::<_, i64>(7)? != 0,
            embed_subs: row.get::<_, i64>(8)? != 0,
            embed_chapters: row.get::<_, i64>(9)? != 0,
            sponsorblock: row.get::<_, i64>(10)? != 0,
            created_at: row.get(11)?,
        })
    })?;
    rows.collect()
}

pub fn insert_preset(
    conn: &Connection,
    name: &str,
    format: &str,
    quality: &str,
    output_dir: &str,
    embed_thumbnail: bool,
    embed_metadata: bool,
    write_subs: bool,
    embed_subs: bool,
    embed_chapters: bool,
    sponsorblock: bool,
) -> SqlResult<i64> {
    conn.execute(
        "INSERT INTO download_presets
         (name, format, quality, output_dir, embed_thumbnail, embed_metadata,
          write_subs, embed_subs, embed_chapters, sponsorblock)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            name, format, quality, output_dir,
            embed_thumbnail as i64, embed_metadata as i64,
            write_subs as i64, embed_subs as i64,
            embed_chapters as i64, sponsorblock as i64,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_preset(
    conn: &Connection,
    id: i64,
    name: &str,
    format: &str,
    quality: &str,
    output_dir: &str,
    embed_thumbnail: bool,
    embed_metadata: bool,
    write_subs: bool,
    embed_subs: bool,
    embed_chapters: bool,
    sponsorblock: bool,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE download_presets SET
         name=?1, format=?2, quality=?3, output_dir=?4,
         embed_thumbnail=?5, embed_metadata=?6, write_subs=?7,
         embed_subs=?8, embed_chapters=?9, sponsorblock=?10
         WHERE id=?11",
        params![
            name, format, quality, output_dir,
            embed_thumbnail as i64, embed_metadata as i64,
            write_subs as i64, embed_subs as i64,
            embed_chapters as i64, sponsorblock as i64,
            id,
        ],
    )?;
    Ok(())
}

pub fn delete_preset(conn: &Connection, id: i64) -> SqlResult<()> {
    conn.execute("DELETE FROM download_presets WHERE id = ?1", params![id])?;
    Ok(())
}
```

- [ ] **Step 2: Verify compile**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/db/queries.rs
git commit -m "feat(presets): add preset CRUD queries"
```

---

## Task 3: Tauri Commands + Wiring

**Files:**
- Create: `src-tauri/src/commands/presets.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: Create commands/presets.rs**

```rust
use tauri::State;
use crate::state::AppState;
use crate::db::{queries, models::Preset};

#[tauri::command]
pub async fn list_presets(state: State<'_, AppState>) -> Result<Vec<Preset>, String> {
    let presets = {
        let db = state.db.lock().await;
        queries::list_presets(&db).map_err(|e| e.to_string())?
    };
    Ok(presets)
}

#[tauri::command]
pub async fn create_preset(
    name: String,
    format: String,
    quality: String,
    output_dir: String,
    embed_thumbnail: bool,
    embed_metadata: bool,
    write_subs: bool,
    embed_subs: bool,
    embed_chapters: bool,
    sponsorblock: bool,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let id = {
        let db = state.db.lock().await;
        queries::insert_preset(
            &db, &name, &format, &quality, &output_dir,
            embed_thumbnail, embed_metadata, write_subs,
            embed_subs, embed_chapters, sponsorblock,
        ).map_err(|e| e.to_string())?
    };
    Ok(id)
}

#[tauri::command]
pub async fn update_preset(
    id: i64,
    name: String,
    format: String,
    quality: String,
    output_dir: String,
    embed_thumbnail: bool,
    embed_metadata: bool,
    write_subs: bool,
    embed_subs: bool,
    embed_chapters: bool,
    sponsorblock: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let db = state.db.lock().await;
        queries::update_preset(
            &db, id, &name, &format, &quality, &output_dir,
            embed_thumbnail, embed_metadata, write_subs,
            embed_subs, embed_chapters, sponsorblock,
        ).map_err(|e| e.to_string())?
    };
    Ok(())
}

#[tauri::command]
pub async fn delete_preset(
    id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let db = state.db.lock().await;
        queries::delete_preset(&db, id).map_err(|e| e.to_string())?
    };
    Ok(())
}
```

- [ ] **Step 2: Register module in commands/mod.rs**

Add `pub mod presets;` after `pub mod schedules;`:

```rust
pub mod schedules;
pub mod presets;
```

- [ ] **Step 3: Register commands in lib.rs**

In `src-tauri/src/lib.rs`, add 4 commands to `generate_handler!`:

```rust
// after commands::schedules::run_schedule_now,
commands::presets::list_presets,
commands::presets::create_preset,
commands::presets::update_preset,
commands::presets::delete_preset,
```

- [ ] **Step 4: Add permissions to capabilities/default.json**

Add 4 entries after `"allow-run-schedule-now"`:

```json
"allow-list-presets",
"allow-create-preset",
"allow-update-preset",
"allow-delete-preset"
```

- [ ] **Step 5: Verify compile**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```
Expected: no errors. (Permission TOML files under `src-tauri/permissions/autogenerated/` are generated automatically by `cargo build` / `tauri dev` — do not create them manually.)

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/presets.rs src-tauri/src/commands/mod.rs \
        src-tauri/src/lib.rs src-tauri/capabilities/default.json
git commit -m "feat(presets): add Tauri commands for preset CRUD"
```

---

## Task 4: TypeScript Type + Pinia Store

**Files:**
- Modify: `src/types/index.ts`
- Create: `src/stores/presets.ts`

- [ ] **Step 1: Add Preset interface to src/types/index.ts**

Append after the `Schedule` interface (end of file):

```typescript
export interface Preset {
  id: number
  name: string
  format: string
  quality: string
  output_dir: string       // snake_case: matches Rust serde output
  embed_thumbnail: boolean
  embed_metadata: boolean
  write_subs: boolean
  embed_subs: boolean
  embed_chapters: boolean
  sponsorblock: boolean
  created_at: string
}
```

- [ ] **Step 2: Create src/stores/presets.ts**

```typescript
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Preset } from '../types'

export const usePresetsStore = defineStore('presets', () => {
  const presets = ref<Preset[]>([])

  async function fetchPresets() {
    presets.value = await invoke<Preset[]>('list_presets')
  }

  async function createPreset(payload: Omit<Preset, 'id' | 'created_at'>): Promise<number> {
    const id = await invoke<number>('create_preset', {
      name: payload.name,
      format: payload.format,
      quality: payload.quality,
      outputDir: payload.output_dir,
      embedThumbnail: payload.embed_thumbnail,
      embedMetadata: payload.embed_metadata,
      writeSubs: payload.write_subs,
      embedSubs: payload.embed_subs,
      embedChapters: payload.embed_chapters,
      sponsorblock: payload.sponsorblock,
    })
    await fetchPresets()
    return id
  }

  async function updatePreset(payload: Omit<Preset, 'created_at'>) {
    await invoke('update_preset', {
      id: payload.id,
      name: payload.name,
      format: payload.format,
      quality: payload.quality,
      outputDir: payload.output_dir,
      embedThumbnail: payload.embed_thumbnail,
      embedMetadata: payload.embed_metadata,
      writeSubs: payload.write_subs,
      embedSubs: payload.embed_subs,
      embedChapters: payload.embed_chapters,
      sponsorblock: payload.sponsorblock,
    })
    await fetchPresets()
  }

  async function deletePreset(id: number) {
    await invoke('delete_preset', { id })
    await fetchPresets()
  }

  return { presets, fetchPresets, createPreset, updatePreset, deletePreset }
})
```

- [ ] **Step 3: Type-check**

```bash
npx vue-tsc --noEmit
```
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add src/types/index.ts src/stores/presets.ts
git commit -m "feat(presets): add Preset type and Pinia store"
```

---

## Task 5: DownloadDialog Integration

**Files:**
- Modify: `src/components/download/DownloadDialog.vue`

The dialog currently has format/quality/embed option refs. We add a preset dropdown at the top and a "save as preset" button.

- [ ] **Step 1: Add store import and preset state**

In the `<script setup>` section, after the existing store imports (around line 7), add:

```typescript
import { usePresetsStore } from '../../stores/presets'
```

After `const schedulesStore = useSchedulesStore()`, add:

```typescript
const presetsStore = usePresetsStore()

const showSavePreset = ref(false)
const savePresetName = ref('')
const savePresetError = ref('')

onMounted(() => {
  presetsStore.fetchPresets()
})
```

Add `onMounted` to the Vue import: `import { ref, computed, watch, onMounted } from 'vue'`

- [ ] **Step 2: Add applyPreset function**

After the `handleScheduleRegister` function, add:

```typescript
function applyPreset(e: Event) {
  const id = Number((e.target as HTMLSelectElement).value)
  if (!id) return
  const preset = presetsStore.presets.find(p => p.id === id)
  if (!preset) return
  selectedFormat.value = preset.format
  selectedQuality.value = preset.quality
  embedThumbnail.value = preset.embed_thumbnail
  embedMetadata.value = preset.embed_metadata
  writeSubs.value = preset.write_subs
  embedSubs.value = preset.embed_subs
  embedChapters.value = preset.embed_chapters
  sponsorblock.value = preset.sponsorblock
  // Reset the select back to placeholder after applying
  ;(e.target as HTMLSelectElement).value = ''
}

async function handleSavePreset() {
  savePresetError.value = ''
  const name = savePresetName.value.trim()
  if (!name) return
  try {
    await presetsStore.createPreset({
      name,
      format: selectedFormat.value,
      quality: selectedQuality.value,
      output_dir: settingsStore.settings.download_dir,
      embed_thumbnail: embedThumbnail.value,
      embed_metadata: embedMetadata.value,
      write_subs: writeSubs.value,
      embed_subs: embedSubs.value,
      embed_chapters: embedChapters.value,
      sponsorblock: sponsorblock.value,
    })
    showSavePreset.value = false
    savePresetName.value = ''
  } catch (e) {
    savePresetError.value = `保存失敗: ${e}`
  }
}
```

- [ ] **Step 3: Add preset UI to template**

In the `<template>` section, find the first `<div>` containing the format/quality controls. Add the preset row **above** it. Look for the format select section and insert before it:

```html
<!-- Preset row -->
<div class="flex items-center gap-2 mb-3">
  <select
    class="flex-1 rounded border border-[var(--color-separator)] bg-transparent px-2 py-1 text-sm"
    @change="applyPreset"
  >
    <option value="">プリセットを選択…</option>
    <option v-for="p in presetsStore.presets" :key="p.id" :value="p.id">
      {{ p.name }}
    </option>
  </select>
  <button
    class="text-xs px-2 py-1 rounded border border-[var(--color-separator)] hover:bg-white/10"
    @click="showSavePreset = !showSavePreset"
  >
    保存
  </button>
</div>
<!-- Save preset inline form -->
<div v-if="showSavePreset" class="flex items-center gap-2 mb-3">
  <input
    v-model="savePresetName"
    type="text"
    placeholder="プリセット名"
    class="flex-1 rounded border border-[var(--color-separator)] bg-transparent px-2 py-1 text-sm"
    @keyup.enter="handleSavePreset"
  />
  <button
    class="text-xs px-2 py-1 rounded bg-blue-500 text-white hover:bg-blue-600 disabled:opacity-50"
    :disabled="!savePresetName.trim()"
    @click="handleSavePreset"
  >
    OK
  </button>
  <span v-if="savePresetError" class="text-xs text-red-400">{{ savePresetError }}</span>
</div>
```

- [ ] **Step 4: Type-check**

```bash
npx vue-tsc --noEmit
```
Expected: no errors

- [ ] **Step 5: Commit**

```bash
git add src/components/download/DownloadDialog.vue
git commit -m "feat(presets): add preset dropdown and save button to DownloadDialog"
```

---

## Task 6: PresetSettings Component

**Files:**
- Create: `src/components/settings/PresetSettings.vue`

- [ ] **Step 1: Create the component**

Do NOT define a separate `PresetForm` component. Use the inline form block directly in both the create and edit areas as shown below.

```vue
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
  await store.deletePreset(id)
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

    <div v-if="errorMsg" class="text-sm text-red-400 mb-2">{{ errorMsg }}</div>

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
      <div class="flex gap-2 mt-2">
        <button class="text-sm px-3 py-1 rounded bg-blue-500 text-white hover:bg-blue-600 disabled:opacity-50"
                :disabled="!form.name.trim()" @click="saveForm">保存</button>
        <button class="text-sm px-3 py-1 rounded border border-[var(--color-separator)] hover:bg-white/10"
                @click="cancelForm">キャンセル</button>
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
          <div class="flex gap-2 mt-2">
            <button class="text-sm px-3 py-1 rounded bg-blue-500 text-white hover:bg-blue-600 disabled:opacity-50"
                    :disabled="!form.name.trim()" @click="saveForm">保存</button>
            <button class="text-sm px-3 py-1 rounded border border-[var(--color-separator)] hover:bg-white/10"
                    @click="cancelForm">キャンセル</button>
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
```

- [ ] **Step 2: Type-check**

```bash
npx vue-tsc --noEmit
```
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add src/components/settings/PresetSettings.vue
git commit -m "feat(presets): add PresetSettings component for settings screen"
```

---

## Task 7: App.vue Wiring

**Files:**
- Modify: `src/App.vue`

- [ ] **Step 1: Import PresetSettings**

In `src/App.vue`, after `import RuleSettings from './components/settings/RuleSettings.vue'`, add:

```typescript
import PresetSettings from './components/settings/PresetSettings.vue'
```

- [ ] **Step 2: Mount in settings section**

In the settings template section (after `<RuleSettings />`), add:

```html
<hr class="border-[var(--color-separator)]" />
<PresetSettings />
```

- [ ] **Step 3: Type-check**

```bash
npx vue-tsc --noEmit
```
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add src/App.vue
git commit -m "feat(presets): wire PresetSettings into settings screen"
```

---

## Task 8: Integration Test

- [ ] **Step 1: Run the app**

```bash
pnpm tauri dev
```

- [ ] **Step 2: Test preset creation from Settings**

1. Open Settings → scroll to "ダウンロードプリセット"
2. Click "＋ 新規作成"
3. Enter name "1080p MP4", format "mp4", quality "1080", output_dir "~/Downloads/YTDown/"
4. Click 保存
5. Verify preset appears in the list

- [ ] **Step 3: Test preset application in DownloadDialog**

1. Paste a video URL → DownloadDialog opens
2. Verify "プリセットを選択…" dropdown is visible at the top
3. Select "1080p MP4" → verify format/quality fields update to mp4/1080

- [ ] **Step 4: Test save from DownloadDialog**

1. Change format to "mp3"
2. Click "保存" → enter name "Audio MP3"
3. Verify the new preset appears in the dropdown

- [ ] **Step 5: Test edit and delete from Settings**

1. Click "編集" on a preset → verify inline form appears with correct values
2. Change quality → click 保存 → verify card updates
3. Click "削除" → confirm → verify preset removed from list and dropdown

- [ ] **Step 6: Final commit**

```bash
git add -A
git commit -m "feat(presets): download presets feature complete (v0.6.0)"
```
