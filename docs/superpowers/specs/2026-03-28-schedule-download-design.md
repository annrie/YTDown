# Schedule Download — Design Spec
**Date**: 2026-03-28
**Status**: Approved (rev 2)
**Project**: YTDown (Tauri v2 + Vue 3 + Rust + SQLite)

---

## Overview

Add scheduled download functionality to YTDown, allowing users to trigger downloads at specified times using cron expressions. Supports one-time and recurring schedules, as well as channel monitoring (new content only). When the app is closed at execution time, the schedule is skipped and the user is notified on next launch.

---

## Requirements

- One-time and recurring schedules (cron expression based, full cron support)
- Schedule creation from both the existing download dialog and a dedicated management screen
- Skip + notify behavior when app is not running at scheduled time
- Channel monitoring: given a channel URL, download only videos published after last successful run
- Download options stored inline per schedule — no dependency on a separate preset feature

---

## Data Model

New table added to `src-tauri/src/db/schema.sql`:

```sql
CREATE TABLE IF NOT EXISTS schedules (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  name         TEXT NOT NULL,
  url          TEXT NOT NULL,
  cron_expr    TEXT NOT NULL,
  options_json TEXT NOT NULL,       -- JSON-serialized DownloadOptions
  is_active    INTEGER NOT NULL DEFAULT 1,
  is_channel   INTEGER NOT NULL DEFAULT 0,
  last_error   TEXT,
  fail_count   INTEGER NOT NULL DEFAULT 0,
  is_running   INTEGER NOT NULL DEFAULT 0,  -- duplicate execution guard
  last_run_at  TEXT,                -- ISO8601; used as --dateafter for channel mode
  next_run_at  TEXT,                -- ISO8601; used for skip detection on launch
  created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Field notes**:
- `options_json`: `DownloadOptions` struct serialized as JSON string. Stored as TEXT in DB; deserialized in Rust with `serde_json::from_str()`. `DownloadOptions` uses `#[serde(default)]` on all fields (existing pattern) for forward/backward compatibility. If deserialization fails for a record, that schedule is skipped and the error is logged — `list_schedules` does not fail entirely.
- `is_channel`: when true, uses `--dateafter` (derived from `last_run_at`) to download only new videos. No per-video ID tracking needed.
- `last_run_at`: doubles as the `--dateafter` value for channel monitoring (YYYYMMDD format for yt-dlp).
- `last_error` + `fail_count`: auto-disable schedule after 3 consecutive failures.
- `next_run_at`: computed after each run; used on app launch and sleep-wake to detect and skip missed executions.

> **`last_seen_id` removed**: Channel monitoring uses yt-dlp's `--dateafter <YYYYMMDD>` flag (derived from `last_run_at`) rather than per-video ID comparison. This is simpler and more reliable across platforms.

---

## Backend Architecture

### New files
- `src-tauri/src/commands/schedules.rs` — Tauri IPC command handlers
- `src-tauri/src/scheduler.rs` — `tokio-cron-scheduler` initialization and job management

### AppState update (`src-tauri/src/state.rs`)

```rust
use tokio_cron_scheduler::JobScheduler;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub active_downloads: Mutex<HashMap<i64, ActiveDownload>>,
    pub ytdlp_path: Mutex<Option<String>>,
    pub scheduler: Arc<Mutex<JobScheduler>>,  // NEW
}
```

`JobScheduler` is initialized asynchronously at app startup in `lib.rs` before the Tauri builder runs. The `Arc<Mutex<JobScheduler>>` is passed into `AppState` and into each scheduled job closure via `Arc::clone`.

### Tauri Commands

Full signatures following existing patterns (`app: AppHandle`, `state: State<'_, AppState>`):

```rust
#[tauri::command]
pub async fn create_schedule(
    app: AppHandle,
    name: String,
    url: String,
    cron_expr: String,
    options_json: String,      // JSON string; Rust calls serde_json::from_str()
    is_channel: bool,
    state: State<'_, AppState>,
) -> Result<i64, String>

#[tauri::command]
pub async fn update_schedule(
    app: AppHandle,
    id: i64,
    name: String,
    url: String,
    cron_expr: String,
    options_json: String,
    is_channel: bool,
    state: State<'_, AppState>,
) -> Result<(), String>

#[tauri::command]
pub async fn delete_schedule(
    id: i64,
    state: State<'_, AppState>,  // accesses scheduler via state.scheduler (Arc<Mutex<JobScheduler>>)
) -> Result<(), String>

#[tauri::command]
pub async fn toggle_schedule(
    id: i64,
    is_active: bool,
    state: State<'_, AppState>,  // accesses scheduler via state.scheduler
) -> Result<(), String>

#[tauri::command]
pub async fn list_schedules(
    state: State<'_, AppState>,
) -> Result<Vec<Schedule>, String>

#[tauri::command]
pub async fn get_schedule(
    id: i64,
    state: State<'_, AppState>,
) -> Result<Schedule, String>

#[tauri::command]
pub async fn run_schedule_now(
    id: i64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String>
```

> `run_schedule_now` added: allows manual "今すぐ実行" from `ScheduleCard`. Reuses same execution logic as cron-fired jobs.

All commands registered in `src-tauri/src/lib.rs` and declared in `src-tauri/capabilities/default.json`.

### Scheduler Lifecycle

1. **App startup**: `JobScheduler::new().await` in `lib.rs`; load all active schedules from DB; register each as a Tokio cron job; call `scheduler.start().await`
2. **Skip detection on launch**: after registering jobs, query schedules where `next_run_at < now()`; emit skip notification for each; update `next_run_at` to next future occurrence
3. **Sleep/wake handling**: listen for Tauri `window:focus` event (fires on macOS wake + app foreground); re-run the same skip detection logic
4. **On cron execution**: call internal download logic (same as `start_download`); for channel mode, append `--dateafter <last_run_at as YYYYMMDD>` to yt-dlp args; update `last_run_at` and `next_run_at` in DB; emit `schedule-fired` Tauri event
5. **Frontend notification via event**: emit `app.emit("schedule-fired", schedule_id)` so the downloads store can refresh the queue in real-time even when `ScheduleView` is not visible
6. **Duplicate execution guard**: add `is_running INTEGER NOT NULL DEFAULT 0` column to `schedules` table; set to 1 at execution start, 0 at end (including on error). If `is_running = 1` when a job fires, skip the run.
7. **Schedule mutation**: on create/update/delete/toggle, cancel the affected Tokio job by UUID and re-register (or not, if toggled off)
8. **Failure handling**: increment `fail_count`, store `last_error`; auto-disable and notify after 3 consecutive failures

### New Cargo dependency

```toml
# src-tauri/Cargo.toml
tokio-cron-scheduler = "0.13"
```

---

## Frontend Architecture

### Type definitions (`src/types/index.ts`)

Add `'schedules'` to `SidebarSection`:
```typescript
export type SidebarSection =
  | 'downloads-active' | 'downloads-completed'
  | 'library-all' | 'library-video' | 'library-audio'
  | 'images-download' | 'images-gallery'
  | 'schedules'   // NEW
  | 'settings'
```

> Affected files to update: `AppSidebar.vue`, `App.vue` (section switch/if logic). No other files reference `SidebarSection` directly.

Add `Schedule` interface:
```typescript
export interface Schedule {
  id: number
  name: string
  url: string
  cron_expr: string
  options: DownloadOptions     // deserialized from options_json on Rust side
  is_active: boolean
  is_channel: boolean
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
- `fetchSchedules()`, `createSchedule()`, `updateSchedule()`, `deleteSchedule()`, `toggleSchedule()`, `runNow(id)`
- `setupScheduleListener()`: listen for `schedule-fired` Tauri event; call `fetchSchedules()` and trigger downloads store refresh

### New components

```
src/components/schedules/
  ScheduleView.vue     — list and management screen
  ScheduleDialog.vue   — create/edit dialog with cron input, next-5-runs preview, channel mode toggle
  ScheduleCard.vue     — name, cron expression, next_run_at, active toggle, 今すぐ実行, edit, delete
```

### Sidebar integration

Add `'schedules'` section in `AppSidebar.vue` between 画像 and 設定, with a clock SVG icon.

### DownloadDialog integration

Add "スケジュール実行" toggle to existing `DownloadDialog.vue`:
- **OFF** (default): immediate download, no behavior change
- **ON**: reveal name input + cron expression field + next-5-runs preview; submit → "スケジュール登録"

### cron expression UX (`croner` npm package)

```
pnpm add croner
```

- Validate cron expression in real-time as user types
- Display next 5 execution times below the input
- Disable submit if expression is invalid

---

## Notifications (`tauri-plugin-notification`)

**This plugin is not yet installed.** Add it:

```toml
# src-tauri/Cargo.toml
tauri-plugin-notification = "2"
```

```rust
// src-tauri/src/lib.rs — in builder chain
.plugin(tauri_plugin_notification::init())
```

```json
// src-tauri/capabilities/default.json — add to permissions array
"notification:default",
"notification:allow-send-notification"
```

```json
// src-tauri/tauri.conf.json — bundle.macOS.infoPlist section
// (macOS does not use NSUserNotificationsUsageDescription; notification permission
//  is handled by the OS dialog on first use. No Info.plist key required for
//  tauri-plugin-notification on macOS.)
```
> macOS notification permission is requested automatically by the OS at runtime (first notification sent). No `Info.plist` entry is needed. Verify against `tauri-plugin-notification` v2 docs before implementation.

| Event | Message |
|---|---|
| Skipped schedule | 「{name}」のスケジュールが実行されませんでした（アプリが起動していませんでした） |
| Channel new content | 「{name}」: {n}件の新着動画をダウンロードしました |
| Auto-disabled | 「{name}」が3回連続で失敗したため無効化されました |

---

## Error Handling

| Scenario | Behavior |
|---|---|
| yt-dlp not found at execution time | Skip, record `last_error`, notify |
| Channel fetch fails | Skip this run, increment `fail_count` |
| 3 consecutive failures | Auto-disable (`is_active = 0`), notify user |
| Invalid cron expression | Blocked at UI level before save |
| App closed at scheduled time | Skip, notify on next launch |
| App asleep at scheduled time | Skip, notify on window focus (wake) |
| Previous run still in progress | Skip current execution (guard via `last_run_at`) |

---

## DB Migration

`CREATE TABLE IF NOT EXISTS` ensures backward compatibility. No migration script required.

---

## Out of Scope

- LaunchAgent / background daemon execution (app must be running)
- Preset/template feature (schedules store options inline)
- Schedule execution history / log UI
- Per-video-ID deduplication for channel mode (handled via `--dateafter`)
