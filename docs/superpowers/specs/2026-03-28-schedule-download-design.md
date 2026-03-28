# スケジュールダウンロード — 設計仕様書
**日付**: 2026-03-28
**ステータス**: 承認済み (rev 3)
**プロジェクト**: YTDown (Tauri v2 + Vue 3 + Rust + SQLite)

---

## 概要

YTDownにスケジュールダウンロード機能を追加する。cron式で指定した日時にダウンロードを自動実行できるようにする。一回限りの実行と繰り返しの両方に対応し、チャンネル監視（新着動画のみダウンロード）もサポートする。スケジュール実行時にアプリが起動していない場合は実行をスキップし、次回起動時に通知する。

---

## 要件

- 一回限り・繰り返し両対応のスケジュール（cron式ベース、フル構文サポート）
- 既存のダウンロードダイアログと専用管理画面の両方からスケジュールを作成可能
- アプリ未起動時はスキップ＋通知
- チャンネル監視: チャンネルURLを指定し、前回実行以降に公開された動画のみダウンロード
- ダウンロードオプションをスケジュール単位でインライン保存（プリセット機能への依存なし）

---

## データモデル

`src-tauri/src/db/schema.sql` に新テーブルを追加:

```sql
CREATE TABLE IF NOT EXISTS schedules (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  name         TEXT NOT NULL,               -- ユーザーが付ける名前
  url          TEXT NOT NULL,               -- ダウンロード対象URL
  cron_expr    TEXT NOT NULL,               -- cron式 (例: "0 23 * * *")
  options_json TEXT NOT NULL,               -- DownloadOptions をJSON文字列化
  is_active    INTEGER NOT NULL DEFAULT 1,  -- 有効/無効フラグ
  is_channel   INTEGER NOT NULL DEFAULT 0,  -- チャンネル監視モードか
  last_error   TEXT,                        -- 直近のエラーメッセージ
  fail_count   INTEGER NOT NULL DEFAULT 0,  -- 連続失敗回数
  is_running   INTEGER NOT NULL DEFAULT 0,  -- 重複実行防止フラグ
  last_run_at  TEXT,                        -- 前回実行日時 (ISO8601); チャンネルモードの --dateafter にも使用
  next_run_at  TEXT,                        -- 次回実行予定日時 (ISO8601); 起動時スキップ判定に使用
  created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**フィールド補足**:
- `options_json`: `DownloadOptions` 構造体をJSON文字列化して保存。Rust側では `serde_json::from_str()` でデシリアライズ。`DownloadOptions` は既存パターン通り全フィールドに `#[serde(default)]` を付与し、新旧バージョン間の互換性を確保する。デシリアライズ失敗時はそのレコードをスキップしてログに記録（`list_schedules` 全体はエラーにしない）
- `is_channel`: trueの場合、`last_run_at` から導出した `--dateafter <YYYYMMDD>` フラグをyt-dlpに渡し新着のみ取得。動画IDの個別追跡は不要
- `is_running`: ジョブ実行開始時に1、終了時（エラー含む）に0。1のままジョブが発火した場合はスキップ
- `last_error` + `fail_count`: 連続3回失敗でスケジュールを自動無効化
- `next_run_at`: 実行後に次回発火時刻を計算して更新。起動時・スリープ復帰時のスキップ判定に使用

> **設計メモ**: チャンネル監視は動画ID追跡ではなく `--dateafter` を使用する。実装がシンプルになり、yt-dlpの信頼性の高いフィルタリングに委ねられる。

---

## バックエンド構成

### 新規ファイル
- `src-tauri/src/commands/schedules.rs` — Tauri IPCコマンドハンドラ群
- `src-tauri/src/scheduler.rs` — `tokio-cron-scheduler` の初期化・ジョブ管理

### AppState の変更 (`src-tauri/src/state.rs`)

```rust
use tokio_cron_scheduler::JobScheduler;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub active_downloads: Mutex<HashMap<i64, ActiveDownload>>,
    pub ytdlp_path: Mutex<Option<String>>,
    pub scheduler: Arc<Mutex<JobScheduler>>,  // 追加
}
```

`JobScheduler` はアプリ起動時に `lib.rs` 内で非同期初期化し、`AppState` に格納する。スケジュールジョブのクロージャ内からは `Arc::clone` で参照する。`delete_schedule` / `toggle_schedule` コマンドも `state.scheduler` 経由でジョブのキャンセル・再登録を行う（`AppHandle` は不要）。

### Tauriコマンド

既存パターン（`app: AppHandle`, `state: State<'_, AppState>`）に準拠:

```rust
#[tauri::command]
pub async fn create_schedule(
    app: AppHandle,
    name: String,
    url: String,
    cron_expr: String,
    options_json: String,   // JSON文字列; Rust側で serde_json::from_str() する
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
    state: State<'_, AppState>,  // state.scheduler 経由でジョブキャンセル
) -> Result<(), String>

#[tauri::command]
pub async fn toggle_schedule(
    id: i64,
    is_active: bool,
    state: State<'_, AppState>,  // state.scheduler 経由でジョブ登録/キャンセル
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

> `run_schedule_now`: `ScheduleCard` の「今すぐ実行」ボタン用。cron発火時と同じ実行ロジックを再利用する。

全コマンドを `src-tauri/src/lib.rs` に登録し、`src-tauri/capabilities/default.json` にパーミッションを宣言する。

### スケジューラのライフサイクル

1. **アプリ起動時**: `lib.rs` 内で `JobScheduler::new().await`; DBから有効なスケジュールを全件ロードしTokioジョブとして登録; `scheduler.start().await`
2. **起動時スキップ判定**: ジョブ登録後、`next_run_at < now()` のスケジュールを検索; 各スケジュールにスキップ通知を発行; `next_run_at` を次回将来時刻に更新
3. **スリープ/復帰対応**: Tauriの `window:focus` イベントをリッスン（macOSのスリープ復帰・アプリ前面化時に発火）; 上記と同じスキップ判定ロジックを再実行
4. **cron実行時**: `is_running` を1にセット; 内部で `start_download` と同じダウンロードロジックを呼び出す; チャンネルモードの場合は `--dateafter <last_run_at のYYYYMMDD>` を追加; 完了後 `last_run_at`・`next_run_at` を更新、`is_running` を0に戻す; `app.emit("schedule-fired", schedule_id)` を発行
5. **フロントエンドへの通知**: `schedule-fired` イベントをフロントエンドがリッスンし、ダウンロードストアのキューをリアルタイム更新（`ScheduleView` が非表示でも機能する）
6. **重複実行防止**: ジョブ発火時に `is_running = 1` であればスキップ
7. **スケジュール変更時**: 対象ジョブをUUIDで特定してキャンセルし、必要に応じて再登録（`toggle_schedule` でOFFにした場合は再登録しない）
8. **失敗処理**: `fail_count` をインクリメントし `last_error` に記録; 3回連続失敗で `is_active = 0` に更新し、ユーザーに通知

### 追加するCargo依存

```toml
# src-tauri/Cargo.toml
tokio-cron-scheduler = "0.13"
```

---

## フロントエンド構成

### 型定義の追加 (`src/types/index.ts`)

`SidebarSection` に `'schedules'` を追加:
```typescript
export type SidebarSection =
  | 'downloads-active' | 'downloads-completed'
  | 'library-all' | 'library-video' | 'library-audio'
  | 'images-download' | 'images-gallery'
  | 'schedules'   // 追加
  | 'settings'
```

> 影響ファイル: `AppSidebar.vue`（セクション追加）、`App.vue`（セクション切り替えロジック）

`Schedule` インターフェースを追加:
```typescript
export interface Schedule {
  id: number
  name: string
  url: string
  cron_expr: string
  options: DownloadOptions     // Rust側でoptions_jsonをデシリアライズして返す
  is_active: boolean
  is_channel: boolean
  last_error: string | null
  fail_count: number
  is_running: boolean
  last_run_at: string | null
  next_run_at: string | null
  created_at: string
}
```

### Piniaストア (`src/stores/schedules.ts`)

**State**:
- `schedules: Schedule[]`

**Actions**:
- `fetchSchedules()`, `createSchedule()`, `updateSchedule()`, `deleteSchedule()`, `toggleSchedule()`, `runNow(id)`
- `setupScheduleListener()`: `schedule-fired` Tauriイベントをリッスンし、`fetchSchedules()` を呼んでダウンロードストアを更新

### 新規コンポーネント

```
src/components/schedules/
  ScheduleView.vue     — スケジュール一覧・管理画面（サイドバーから表示）
  ScheduleDialog.vue   — 作成/編集ダイアログ（cron入力・次回5回プレビュー・チャンネルモードトグル）
  ScheduleCard.vue     — 1件分のカード表示（名前・cron式・次回実行日時・有効トグル・今すぐ実行・編集・削除）
```

### サイドバーへの統合

`AppSidebar.vue` の 画像 セクションと 設定 の間に `'schedules'` セクションを追加。時計のSVGアイコンを使用。

### DownloadDialogへの統合

既存の `DownloadDialog.vue` に「スケジュール実行」トグルを追加:
- **OFF**（デフォルト）: 即時ダウンロード、既存の動作に変更なし
- **ON**: 名前入力フィールド・cron式入力フィールド・次回5回プレビューが展開; 送信ボタンが「スケジュール登録」に変化

### cron式のUX (`croner` npmパッケージ)

```
pnpm add croner
```

- ユーザーが入力中にリアルタイムでcron式を検証
- 次回5回の実行予定日時を入力欄の下に表示
- 不正な式の場合は送信ボタンを無効化

---

## 通知設定 (`tauri-plugin-notification`)

**現在未導入のため、以下の手順で追加する**:

```toml
# src-tauri/Cargo.toml
tauri-plugin-notification = "2"
```

```rust
// src-tauri/src/lib.rs — builderチェインに追加
.plugin(tauri_plugin_notification::init())
```

```json
// src-tauri/capabilities/default.json — permissionsに追加
"notification:default",
"notification:allow-send-notification"
```

> macOSではInfo.plistへのエントリは不要。通知権限は初回通知送信時にOSが自動的にダイアログを表示して取得する。実装前に `tauri-plugin-notification` v2の公式ドキュメントで確認すること。

| イベント | 通知メッセージ |
|---|---|
| スキップ | 「{name}」のスケジュールが実行されませんでした（アプリが起動していませんでした） |
| チャンネル新着 | 「{name}」: {n}件の新着動画をダウンロードしました |
| 自動無効化 | 「{name}」が3回連続で失敗したため無効化されました |

---

## エラーハンドリング

| シナリオ | 対応 |
|---|---|
| 実行時にyt-dlpが見つからない | スキップ、`last_error` に記録、通知 |
| チャンネル取得失敗 | 今回の実行をスキップ、`fail_count` インクリメント |
| 3回連続失敗 | `is_active = 0` に更新、ユーザーに通知 |
| 不正なcron式 | UI側で保存前にブロック |
| アプリ未起動時 | スキップ、次回起動時に通知 |
| スリープ中に時刻通過 | スキップ、`window:focus` 時に通知 |
| 前回実行がまだ進行中 | `is_running = 1` を検出してスキップ |
| `options_json` のデシリアライズ失敗 | 該当レコードをスキップしてログに記録、一覧表示は継続 |

---

## DBマイグレーション

`CREATE TABLE IF NOT EXISTS` により既存インストールとの後方互換性を維持。マイグレーションスクリプトは不要。

---

## スコープ外

- LaunchAgentによるバックグラウンドデーモン実行（アプリ起動中のみ動作する設計）
- プリセット/テンプレート機能（スケジュールはオプションをインラインで保持）
- スケジュール実行履歴・ログ画面（将来の拡張として検討）
- 動画ID単位の重複排除（`--dateafter` で代替）
