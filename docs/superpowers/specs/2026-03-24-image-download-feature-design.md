# Image Download Feature Design — YTDown

## Overview

YTDown に画像ダウンロード機能を追加する。任意の Web ページから画像をスクレイピングし、プレビュー＆選択後にダウンロード。ダウンロード済み画像はギャラリーで確認でき、スライドショーで閲覧できる。

## Design Decisions

| 判断項目 | 決定 | 理由 |
|---|---|---|
| 画像ソース | Web ページスクレイピングのみ | サムネイルは既存機能で対応済み。将来拡張可能な設計とする |
| 画像処理 | Rust 側（`image` クレート） | Tauri との親和性が最高。sharp のネイティブバイナリ問題を回避 |
| フォーマット変換 | WebP / AVIF 対応 | HEIC は将来対応 |
| UI 構成 | 統合型（1 画面完結） | YTDown の既存 UI パターンに近く、操作が速い |
| ギャラリー | サイドバー統合、サイズスライダー付きグリッド | 動画/音声ライブラリと統一的な操作感 |
| スライドショー | フルスクリーンオーバーレイ | キーボード操作、自動再生対応 |
| 管理レベル | ダウンロード特化（ビューア的位置づけ） | YTDown はダウンローダーであり、画像管理アプリにはしない |
| img-dl との関係 | Rust で再実装。img-dl は CLI として独立維持 | ネイティブバイナリ互換性問題の回避 |

## Architecture

### Layer Structure

```
┌─────────────────────────────────────────────┐
│  Vue 3 Frontend                              │
│  ├── components/images/                      │
│  │   ├── ImageDownloadView.vue  (URL入力&選択) │
│  │   ├── ImageGalleryView.vue   (ギャラリー)   │
│  │   ├── ImageSlideshow.vue     (スライドショー) │
│  │   ├── ImagePreviewGrid.vue   (プレビュー)   │
│  │   └── ImageThumbnail.vue     (サムネイル)   │
│  ├── stores/images.ts           (Pinia)      │
│  └── types/index.ts             (型定義追加)   │
├─────────────────────────────────────────────┤
│  Tauri IPC (invoke / listen)                 │
├─────────────────────────────────────────────┤
│  Rust Backend                                │
│  ├── commands/images.rs   (Tauri コマンド)    │
│  ├── images/                                 │
│  │   ├── mod.rs           (モジュール定義)     │
│  │   ├── scraper.rs       (HTML 解析)        │
│  │   └── downloader.rs    (DL & 変換処理)     │
│  └── db/schema.sql        (テーブル追記)      │
└─────────────────────────────────────────────┘
```

### Files to Modify (既存ファイル変更リスト)

| ファイル | 変更内容 |
|---|---|
| `src/types/index.ts` | `SidebarSection` に `'images-download' \| 'images-gallery'` 追加、画像関連型の追加 |
| `src/App.vue` | 画像セクションの条件分岐ルーティング追加 |
| `src/components/layout/AppSidebar.vue` | 画像セクションのナビボタン 2 つ追加 |
| `src-tauri/src/lib.rs` | `tauri::generate_handler![]` に画像コマンド 5 つを登録 |
| `src-tauri/src/commands/mod.rs` | `pub mod images;` 追加 |
| `src-tauri/src/db/schema.sql` | `image_sessions` / `images` テーブルを追記（`CREATE TABLE IF NOT EXISTS`） |
| `src-tauri/Cargo.toml` | `scraper`, `image` クレートを追加 |

### Data Flow

```
URL入力 → [Rust] HTML スクレイピング → 画像 URL 一覧を返す
  → [Vue] サムネイルグリッドでプレビュー表示
  → ユーザーが選択/解除
  → [Rust] 選択画像を DL → フォーマット変換(任意) → ファイル保存 → DB 登録
  → [Vue] 進捗イベントをリッスン → 完了後ギャラリーで表示
```

## Database Schema

```sql
CREATE TABLE image_sessions (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  source_url    TEXT NOT NULL,
  site_name     TEXT,
  image_count   INTEGER DEFAULT 0,
  output_dir    TEXT NOT NULL,
  created_at    DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE images (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id    INTEGER NOT NULL REFERENCES image_sessions(id) ON DELETE CASCADE,
  original_url  TEXT NOT NULL,
  file_path     TEXT,
  filename      TEXT,
  width         INTEGER,
  height        INTEGER,
  file_size     INTEGER,
  format        TEXT,
  status        TEXT DEFAULT 'pending',
  created_at    DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

- `image_sessions` でスクレイピング単位をグループ化（ギャラリーでサイト別表示に活用）
- 既存の `downloads` テーブルとは独立（動画と画像は別管理）
- 既存の `schema.sql` に `CREATE TABLE IF NOT EXISTS` 形式で追記する

## TypeScript Types

`src/types/index.ts` に追加する型定義:

```typescript
// スクレイピング結果（未ダウンロード画像）
interface ScrapedImage {
  url: string
  width: number | null
  height: number | null
  alt: string | null
}

// ダウンロード指示
interface ImageToDownload {
  url: string
  filename_hint: string | null  // 元URLから推測したファイル名
}

// DB: image_sessions テーブル
interface ImageSession {
  id: number
  source_url: string
  site_name: string | null
  image_count: number
  output_dir: string
  created_at: string
}

// DB: images テーブル
interface ImageRecord {
  id: number
  session_id: number
  original_url: string
  file_path: string | null
  filename: string | null
  width: number | null
  height: number | null
  file_size: number | null
  format: string | null
  status: 'pending' | 'downloading' | 'completed' | 'failed'
  created_at: string
}

// 画像ダウンロード進捗（Tauri イベントペイロード）
interface ImageDownloadProgress {
  session_id: number
  image_index: number       // 現在DL中の画像インデックス
  total_images: number      // 合計枚数
  current_url: string
  percent: number           // 全体進捗 (0-100)
  status: 'downloading' | 'completed' | 'failed'
  error_message: string | null
}

// SidebarSection 拡張
type SidebarSection =
  | 'downloads-active' | 'downloads-completed'
  | 'library-all' | 'library-video' | 'library-audio'
  | 'images-download'
  | 'images-gallery'
  | 'playlist' | 'settings'
```

## Vue Components

### ImageDownloadView.vue — メイン画面

統合型 1 画面で URL 入力・フィルタ設定・プレビュー選択・ダウンロードが完結する。

構成要素:
- URL 入力バー + 「取得」ボタン
- フィルタ設定: 最小幅/高さ、変換形式（WebP / AVIF / なし）
- `ImagePreviewGrid` でスクレイピング結果をサムネイル表示
- 選択カウンター + 「全選択」/「全解除」ボタン
- 「ダウンロード開始」ボタン + 進捗バー

### ImageGalleryView.vue — ギャラリー

ダウンロード済み画像をグリッド表示する。

構成要素:
- セッション（スクレイピング元サイト）別グループ表示
- サムネイルサイズ調整スライダー（小 80px 〜 大 300px）
- 「▶ スライドショー」ボタン
- 「Finder で表示」ボタン
- セッション削除機能（確認ダイアログ付き）
- 画像クリック → スライドショー開始

### ImageSlideshow.vue — スライドショー

```typescript
defineProps<{
  images: ImageRecord[]
  startIndex: number
  open: boolean
}>()

defineEmits<{
  close: []
  'reveal-in-finder': [filePath: string]
}>()
```

仕様:
- 黒背景フルスクリーンオーバーレイ（z-index 最上位）
- 画像は `object-fit: contain` でアスペクト比維持
- ナビゲーション: ← → キー / 画面端クリック（前後移動）、Space（再生/一時停止）、Escape（閉じる）
- 自動再生: デフォルト 3 秒間隔、変更可能（1 / 3 / 5 / 10 秒）、最後の画像で停止
- カウンター表示: "3 / 12"
- 「Finder で表示」ボタン
- マウス非操作 3 秒後にコントロール自動非表示

### ImagePreviewGrid.vue — プレビューグリッド

スクレイピング結果のサムネイルを選択可能なグリッドで表示。

```typescript
defineProps<{
  images: ScrapedImage[]
  selectedIds: Set<number>
}>()

defineEmits<{
  'toggle-select': [index: number]
  'select-all': []
  'deselect-all': []
}>()
```

### ImageThumbnail.vue — 個別サムネイル

チェックボックス + 寸法表示付きのサムネイルカード。

```typescript
defineProps<{
  image: ScrapedImage
  index: number
  selected: boolean
}>()

defineEmits<{
  'toggle-select': [index: number]
}>()
```

## Pinia Store

```typescript
// src/stores/images.ts
useImagesStore {
  // State
  scrapedImages: ScrapedImage[]
  selectedIds: Set<number>
  sessions: ImageSession[]
  downloadProgress: ImageDownloadProgress | null
  scraping: boolean
  downloading: boolean

  // Actions
  scrapeUrl(url, options)              // → invoke('scrape_images')
  startDownload(indices, options)      // → invoke('download_images')
  loadSessions()                       // → invoke('list_image_sessions')
  loadSessionImages(sessionId)         // → invoke('list_session_images')
  deleteSession(sessionId)             // → invoke('delete_image_session')
  toggleSelect(index)
  selectAll()
  deselectAll()
}
```

## Rust Backend

### Tauri Commands

```rust
// 注意: すべてのコマンドで reqwest の async API を使用すること。
// 既存の reqwest blocking フィーチャーは Tokio async コンテキスト内でパニックするため使用禁止。

#[tauri::command]
async fn scrape_images(
  url: String,
  min_width: u32,
  min_height: u32,
) -> Result<Vec<ScrapedImage>, String>
// HTTP クライアント設定:
//   User-Agent: "Mozilla/5.0 (compatible; YTDown/0.2)" (サイトブロック回避)
//   タイムアウト: 30 秒
//   リダイレクト: 最大 5 回まで自動追従

#[tauri::command]
async fn download_images(
  images: Vec<ImageToDownload>,
  output_dir: String,
  format: Option<String>,  // "webp" | "avif" | None
  session_url: String,
  app: AppHandle,
  state: State<'_, AppState>,  // DB アクセス用
) -> Result<i64, String>
// セッション作成 → 画像 DL → 変換 → 保存 → DB 登録
// 進捗: app.emit("image-download-progress", ImageDownloadProgress)

#[tauri::command]
async fn list_image_sessions(
  state: State<'_, AppState>,
) -> Result<Vec<ImageSession>, String>

#[tauri::command]
async fn list_session_images(
  session_id: i64,
  state: State<'_, AppState>,
) -> Result<Vec<ImageRecord>, String>

#[tauri::command]
async fn delete_image_session(
  session_id: i64,
  delete_files: bool,
  state: State<'_, AppState>,
) -> Result<(), String>
```

### lib.rs Registration

`tauri::generate_handler![]` マクロに以下を追加:

```rust
commands::images::scrape_images,
commands::images::download_images,
commands::images::list_image_sessions,
commands::images::list_session_images,
commands::images::delete_image_session,
```

### Dependencies (additions to Cargo.toml)

```toml
scraper = "0.20"                                    # HTML parsing
image = { version = "0.25", features = ["webp"] }   # Image processing & conversion
```

**AVIF サポートについて**: `image` クレートの `avif` フィーチャーはネイティブライブラリ（`rav1e` / `aom`）に依存し、macOS クロスコンパイル時に問題が起きやすい。初期リリースでは **WebP のみ対応**とし、AVIF は動作検証後に追加する。変換形式の選択肢 UI は AVIF を含めて表示するが、未対応の場合は disabled + ツールチップで説明する。

**reqwest の使い方**: 既存の `Cargo.toml` に `reqwest = { version = "0.12", features = ["blocking"] }` があるが、画像スクレイピング/DL では **async API (`reqwest::Client`) のみ使用する**こと。`blocking` クライアントを Tokio async コンテキスト内で呼ぶとランタイムパニックが発生する。

## Error Handling

### Scraping

| エラー | 対応 |
|---|---|
| URL 不正 | 入力欄下にエラーメッセージ |
| ネットワークエラー / HTTP エラー | 「ページを取得できませんでした（ステータスコード表示）」 |
| サイトブロック (403) | 「アクセスがブロックされました。別のURLを試してください」 |
| 画像なし | 「画像が見つかりませんでした」+ フィルタ緩和の提案 |
| タイムアウト | 30 秒で打ち切り、取得済み結果を表示 |

### Download

| エラー | 対応 |
|---|---|
| 個別画像の失敗 | スキップして続行、status='failed' で DB 記録 |
| ディスク容量不足 | DL 中断、エラーメッセージ表示 |
| 変換失敗 | オリジナル形式で保存（フォールバック） |

### Gallery

| エラー | 対応 |
|---|---|
| ファイル不在（手動削除） | プレースホルダー表示 + ツールチップ通知 |
| セッション削除 | 確認ダイアログ「DB レコードのみ / ファイルも削除」 |
