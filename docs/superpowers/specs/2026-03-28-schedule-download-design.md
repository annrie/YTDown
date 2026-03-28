# Schedule Download — Design Spec
**Date**: 2026-03-28
**Status**: Approved
**Project**: YTDown (Tauri v2 + Vue 3 + Rust + SQLite)

---

## Overview

Add scheduled download functionality to YTDown, allowing users to trigger downloads at specified times using cron expressions. Supports one-time and recurring schedules, as well as channel monitoring (new content only). When the app is closed at execution time, the schedule is skipped and the user is notified on next launch.

---

## Requirements

- One-time and recurring schedules (cron expression based)
- Schedule creation from both the existing download dialog and a dedicated management screen
- Skip + notify behavior when app is not running at scheduled time
- Channel monitoring: given a channel URL, download only newly published videos since last run
- Full cron expression support (including seconds-level granularity via `tokio-cron-scheduler`)
- Download options (format, quality, etc.) stored inline per schedule — no dependency on a separate preset feature

---

## Data Model

New table added to `src-tauri/src/db/schema.sql`:

```sql
CREATE TABLE IF NOT EXISTS schedules (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  name         TEXT NOT NULL,
  url          TEXT NOT NULL,
  cron_expr    TEXT NOT NULL,
  options_json TEXT NOT NULL,
  is_active    INTEGER NOT NULL DEFAULT 1,
  is_channel   INTEGER NOT NULL DEFAULT 0,
  last_seen_id TEXT,
  last_error   TEXT,
  fail_count   INTEGER NOT NULL DEFAULT 0,
  last_run_at  TEXT,
  next_run_at  TEXT,
  created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Field notes**:
- `options_json`: serialized `DownloadOptions` struct; self-contained, no preset table reference
- `is_channel` + `last_seen_id`: enables channel monitoring (skip already-downloaded videos)
- `last_error` + `fail_count`: used for auto-disable after 3 consecutive failures
- `next_run_at`: used on app launch to detect and skip missed executions

---

## Backend Architecture

### New files
- `src-tauri/src/commands/schedules.rs` — Tauri IPC command handlers
- `src-tauri/src/scheduler.rs` — `tokio-cron-scheduler` initialization and job management

### Tauri Commands

```rust
#[tauri::command] create_schedule(name, url, cron_expr, options_json, is_channel) -> Result<i64, String>
#[tauri::command] update_schedule(id, name, url, cron_expr, options_json, is_channel) -> Result<(), String>
#[tauri::command] delete_schedule(id) -> Result<(), String>
#[tauri::command] toggle_schedule(id, is_active) -> Result<(), String>
#[tauri::command] list_schedules() -> Result<Vec<Schedule>, String>
#[tauri::command] get_schedule(id) -> Result<Schedule, String>
```

All commands registered in `src-tauri/src/lib.rs` and declared in `src-tauri/capabilities/default.json`.

### Scheduler Lifecycle

1. **App startup**: Initialize `tokio-cron-scheduler` in `AppState`; load all active schedules from DB and register each as a Tokio job
2. **On execution**: Call existing `start_download()` logic internally (code reuse, no duplication)
3. **Channel mode**: Call `fetch_playlist_items()` → compare against `last_seen_id` → download only newer items → update `last_seen_id`
4. **Skip detection**: At startup, any schedule whose `next_run_at` is in the past is skipped; emit macOS notification and advance `next_run_at`
5. **Schedule mutation**: On create/update/delete/toggle, cancel and re-register the affected Tokio job
6. **Duplicate execution guard**: If `last_run_at` is within the current execution window, skip to prevent overlapping runs
7. **Failure handling**: On error, increment `fail_count` and store `last_error`; auto-disable schedule and notify user after 3 consecutive failures

### New Cargo dependency

```toml
# src-tauri/Cargo.toml
tokio-cron-scheduler = "0.13"
```

---

## Frontend Architecture

### Type definitions (`src/types/index.ts`)

```typescript
export interface Schedule {
  id: number
  name: string
  url: string
  cron_expr: string
  options: DownloadOptions
  is_active: boolean
  is_channel: boolean
  last_seen_id: string | null
  last_error: string | null
  fail_count: number
  last_run_at: string | null
  next_run_at: string | null
  created_at: string
}
```

### Pinia store (`src/stores/schedules.ts`)

State:
- `schedules: Schedule[]`

Actions:
- `fetchSchedules()`, `createSchedule()`, `updateSchedule()`, `deleteSchedule()`, `toggleSchedule()`

### New components

```
src/components/schedules/
  ScheduleView.vue     — list and management screen (activated from sidebar)
  ScheduleDialog.vue   — create/edit dialog with cron input and preview
  ScheduleCard.vue     — card per schedule: name, cron, next_run_at, active toggle, edit/delete
```

### Sidebar integration

Add `'schedules'` to `SidebarSection` type and a new section in `AppSidebar.vue` between 画像 and 設定, with a clock icon.

### DownloadDialog integration

Add a "スケジュール実行" toggle to the existing `DownloadDialog.vue`:
- **OFF** (default): immediate download, existing behavior unchanged
- **ON**: reveals name input + cron expression input + next-5-runs preview; submit button becomes "スケジュール登録"

### cron expression UX

- Use `croner` npm package for client-side cron validation and next-run preview
- Real-time display of next 5 scheduled execution times as the user types
- Invalid expression disables the submit button

---

## Notifications

Use `tauri-plugin-notification` (add if not already present):

| Event | Message |
|---|---|
| Skipped schedule | 「{name}」のスケジュールが実行されませんでした（アプリが起動していませんでした） |
| Channel new content | 「{name}」: {n}件の新着動画をダウンロードしました |
| Auto-disabled | 「{name}」のスケジュールが3回連続で失敗したため無効化されました |

---

## Error Handling

| Scenario | Behavior |
|---|---|
| yt-dlp not found at execution time | Skip, record error, notify |
| Channel fetch fails | Skip this run, increment fail_count |
| 3 consecutive failures | Auto-disable schedule, notify user |
| Invalid cron expression | Blocked at UI level before save |
| App closed at scheduled time | Skip, notify on next launch |
| Previous run still in progress | Skip current execution |

---

## DB Migration

`CREATE TABLE IF NOT EXISTS` ensures backward compatibility with existing installs. No migration script needed.

---

## Out of Scope

- LaunchAgent / background daemon (app must be running for execution)
- Preset feature (schedules store options inline)
- Schedule history / execution log UI (future enhancement)
