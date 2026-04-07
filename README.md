<p align="center">
  <img src="src-tauri/icons/icon.png" width="128" alt="YTDown icon" />
</p>

<h1 align="center">YTDown</h1>

<p align="center">
  A native desktop app for downloading videos, audio, and images from the web.<br/>
  Built with <strong>Tauri v2</strong> + <strong>Vue 3</strong> + <strong>yt-dlp</strong>.
</p>

<p align="center">
  <a href="#features">Features</a> · <a href="#installation">Installation</a> · <a href="#build-from-source">Build</a> · <a href="#日本語">日本語</a>
</p>

---

## Features

### 🌐 One-Click Browser Capture <sup>macOS only</sup>

> **Grab the URL directly from your browser — no copy-paste needed.**

Click the **globe icon** ( <img src="https://api.iconify.design/mdi/web.svg?color=%23007aff" width="18" align="center" /> ) next to the URL bar to instantly capture the URL from the frontmost tab of your browser.

**Supported browsers:** Safari, Chrome, Brave, Arc, Edge, Vivaldi, Opera, Firefox, Biscuit

> [!NOTE]
> This feature uses macOS-native APIs (AppleScript / CoreGraphics) and is **available on macOS only**. On Windows and Linux, please enter URLs manually or paste from the clipboard.

### Video & Audio

- **Multi-site support** — YouTube, Vimeo, Twitter/X, and [1000+ sites](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md) via yt-dlp
- **Video & Audio** — Download as MP4, MKV, WebM, MP3, FLAC, M4A, WAV, and more
- **Quality selection** — Choose from Best, 4K, 1080p, 720p, 480p
- **Playlist support** — Download entire playlists with parallel processing
- **Batch download** — Paste multiple URLs at once
- **URL drag & drop** — Drop URLs from browsers, selected text, `.txt`, `.webloc`, or `.url` files
- **Cookie authentication** — Access private/age-restricted content via browser cookies
- **Post-processing** — Embed thumbnails, metadata, subtitles, chapters; SponsorBlock integration
- **Library management** — Browse, search, filter, and organize downloaded media
- **YouTube channel folders** — Automatically organizes downloads into channel-name subfolders
- **Channel monitoring** — Watch YouTube channels and automatically check for new uploads on schedule or app startup
- **Download notifications** — Show a desktop notification when downloads finish
- **URL history** — Recent URLs are saved automatically and accessible via the clock icon
- **Auto yt-dlp install** — If yt-dlp is not found, install it directly from the app

### 🖼 Image Download

> **Scrape and download images from any web page.**

- **Page scraping** — Enter a URL to scan for images with size filtering (min width/height)
- **Browser URL capture** — Grab the current page URL from your browser directly in the image download view
- **Preview & select** — Browse scraped images in a thumbnail grid, select which ones to download
- **WebP conversion** — Optionally convert images to WebP format on download
- **Gallery view** — Browse downloaded images organized by session with adjustable thumbnail sizes
- **Slideshow** — Fullscreen slideshow with keyboard navigation, auto-play, and speed controls

### General

- **Custom background** — Set separate wallpapers for light and dark mode
- **Multilingual UI** — Interface available in 8 languages: Japanese, English, German, Spanish, French, Korean, Portuguese (Brazil), Traditional Chinese

### Platform Support

| Feature | macOS | Windows | Linux |
|---------|:-----:|:-------:|:-----:|
| Video/audio download | ✅ | ✅ | ✅ |
| Image scraping & download | ✅ | ✅ | ✅ |
| Browser URL capture | ✅ | — | — |
| Pause / Resume download | ✅ | — | ✅ |
| Move to Trash | ✅ | ✅ | ✅ |
| Reveal in file manager | ✅ | ✅ | ✅ |

> [!IMPORTANT]
> Current real-device testing has only been performed on macOS Intel. Windows, Linux, and macOS Apple Silicon (arm64) builds are provided on a best-effort basis and have not been validated on physical machines yet.

## Installation

### Requirements

- **yt-dlp** — If not installed, YTDown will offer to download and install it automatically. Or install manually:
  - macOS: `brew install yt-dlp`
  - Windows: `winget install yt-dlp` or `scoop install yt-dlp`
  - Linux: `sudo apt install yt-dlp` or download from [yt-dlp releases](https://github.com/yt-dlp/yt-dlp/releases)

### Download

Download the latest release from the [Releases](../../releases) page.

| Platform | File |
|----------|------|
| macOS (Universal) | `YTDown_x.x.x_universal.dmg` |
| Windows (64-bit) | `YTDown_x.x.x_x64-setup.exe` |
| Linux (Debian/Ubuntu) | `YTDown_x.x.x_amd64.deb` |
| Linux (AppImage) | `YTDown_x.x.x_amd64.AppImage` |

## Build from Source

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [pnpm](https://pnpm.io/)
- [Rust](https://rustup.rs/) toolchain
- [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/)

### Steps

```bash
git clone https://github.com/annrie/YTDown.git
cd YTDown
pnpm install

# Development
pnpm tauri dev

# Production build
pnpm tauri build
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust, Tauri v2, SQLite (rusqlite) |
| Frontend | Vue 3 (Composition API), TypeScript, Pinia |
| Styling | Tailwind CSS v4 |
| Internationalization | vue-i18n v10 (8 locales) |
| Video engine | yt-dlp |
| Image processing | Rust image crate (scraping, WebP conversion) |

## License

MIT

---

<a id="日本語"></a>

<p align="center">
  <img src="src-tauri/icons/icon.png" width="128" alt="YTDown アイコン" />
</p>

<h2 align="center">YTDown — 日本語ドキュメント</h2>

<p align="center">
  動画・音声・画像をウェブからダウンロードするネイティブデスクトップアプリ<br/>
  <strong>Tauri v2</strong> + <strong>Vue 3</strong> + <strong>yt-dlp</strong> で構築
</p>

---

### 主な機能

#### 🌐 ブラウザから URL をワンクリック取得 <sup>macOS 限定</sup>

> **コピー＆ペースト不要。ブラウザの URL をそのまま取得します。**

URL 入力欄の横にある **地球アイコン**（ <img src="https://api.iconify.design/mdi/web.svg?color=%23007aff" width="18" align="center" /> ）をクリックすると、最前面のブラウザタブから URL を自動的に取得してセットします。

**対応ブラウザ:** Safari, Chrome, Brave, Arc, Edge, Vivaldi, Opera, Firefox, Biscuit

> [!NOTE]
> この機能は macOS 固有の API（AppleScript / CoreGraphics）を使用しているため、**macOS でのみ利用可能**です。Windows・Linux では URL を手動で入力するか、クリップボードから貼り付けてください。

#### 動画・音声

- **マルチサイト対応** — YouTube, Vimeo, Twitter/X ほか yt-dlp が対応する [1000 以上のサイト](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md)
- **映像 & 音声** — MP4, MKV, WebM, MP3, FLAC, M4A, WAV など多数のフォーマットに対応
- **画質選択** — 最高画質, 4K, 1080p, 720p, 480p から選択
- **プレイリスト対応** — プレイリスト全体を並列ダウンロード
- **一括ダウンロード** — 複数の URL をまとめて入力可能
- **URL ドラッグ&ドロップ** — ブラウザ、テキスト選択、`.txt`、`.webloc`、`.url` から URL をドロップ可能
- **Cookie 認証** — ブラウザの Cookie を利用して限定公開・年齢制限コンテンツにアクセス
- **ポストプロセス** — サムネイル・メタデータ・字幕・チャプターの埋め込み、SponsorBlock 対応
- **ライブラリ管理** — ダウンロード済みメディアの一覧・検索・フィルタリング
- **YouTube チャンネルフォルダ** — チャンネル名のサブフォルダに自動整理
- **チャンネル監視** — YouTube チャンネルを登録し、定期実行や起動時チェックで新着を自動確認
- **完了通知** — ダウンロード完了時にデスクトップ通知を表示
- **URL 履歴** — 直近の URL を自動保存、時計アイコンからすぐに呼び出し可能
- **yt-dlp 自動インストール** — yt-dlp 未検出時、アプリ内からワンクリックでインストール

#### 🖼 画像ダウンロード

> **ウェブページから画像をスクレイピングしてダウンロード**

- **ページスクレイピング** — URL を入力してページ内の画像を取得（最小幅・最小高さでフィルタリング可能）
- **ブラウザ URL 取得** — 画像ダウンロード画面からもブラウザの URL をワンクリックで取得可能
- **プレビュー & 選択** — サムネイルグリッドで画像を確認し、ダウンロードする画像を選択
- **WebP 変換** — ダウンロード時に WebP 形式への変換が可能
- **ギャラリー表示** — セッション別にダウンロード済み画像を一覧表示（サムネイルサイズ調整可能）
- **スライドショー** — フルスクリーン表示、キーボード操作、自動再生（速度調整付き）

#### 全般

- **背景画像のカスタマイズ** — ライトモード・ダークモードそれぞれに壁紙を設定可能
- **多言語対応** — 日本語・英語・ドイツ語・スペイン語・フランス語・韓国語・ポルトガル語（ブラジル）・繁体字中国語の 8 言語に対応

### プラットフォーム対応状況

| 機能 | macOS | Windows | Linux |
|------|:-----:|:-------:|:-----:|
| 動画・音声ダウンロード | ✅ | ✅ | ✅ |
| 画像スクレイピング & ダウンロード | ✅ | ✅ | ✅ |
| ブラウザ URL 取得 | ✅ | — | — |
| 一時停止 / 再開 | ✅ | — | ✅ |
| ゴミ箱へ移動 | ✅ | ✅ | ✅ |
| ファイルマネージャで表示 | ✅ | ✅ | ✅ |

> [!IMPORTANT]
> 現時点で実機確認を行っているのは macOS Intel 版のみです。Windows、Linux、macOS Apple Silicon（arm64）向けビルドは提供できますが、まだ実機での検証はしていません。

### インストール

#### 必要なもの

- **yt-dlp** — 未インストールの場合、アプリ内から自動でダウンロード・インストールできます。手動の場合:
  - macOS: `brew install yt-dlp`
  - Windows: `winget install yt-dlp` または `scoop install yt-dlp`
  - Linux: `sudo apt install yt-dlp` または [yt-dlp リリースページ](https://github.com/yt-dlp/yt-dlp/releases)からダウンロード

[Releases](../../releases) ページから最新版をダウンロードしてください。

### ソースからビルド

```bash
git clone https://github.com/annrie/YTDown.git
cd YTDown
pnpm install
pnpm tauri dev      # 開発モード
pnpm tauri build    # プロダクションビルド
```
