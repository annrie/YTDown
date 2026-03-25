# Image Download Feature Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add image downloading from web pages to YTDown, with preview/selection, gallery view with size slider, and fullscreen slideshow.

**Architecture:** Rust backend handles HTML scraping (scraper crate), image downloading/conversion (image crate with WebP), and SQLite storage. Vue 3 frontend adds 5 components under `components/images/`, a Pinia store, and extends the sidebar navigation. Communication via Tauri IPC invoke/listen pattern.

**Tech Stack:** Tauri v2, Vue 3 (Composition API), Pinia v3, Tailwind CSS v4, Rust (scraper, image, reqwest async), SQLite

**Spec:** `docs/superpowers/specs/2026-03-24-image-download-feature-design.md`

---

## File Map

### New Files (Rust)

| File | Responsibility |
|---|---|
| `src-tauri/src/images/mod.rs` | Module exports |
| `src-tauri/src/images/scraper.rs` | HTML fetching, `<img>` extraction, URL resolution |
| `src-tauri/src/images/downloader.rs` | Image download, format conversion, file saving |
| `src-tauri/src/commands/images.rs` | 5 Tauri commands (scrape, download, list, delete) |

### New Files (Vue)

| File | Responsibility |
|---|---|
| `src/components/images/ImageDownloadView.vue` | Main view: URL input, preview grid, download trigger |
| `src/components/images/ImageGalleryView.vue` | Gallery: session groups, size slider, slideshow entry |
| `src/components/images/ImageSlideshow.vue` | Fullscreen overlay with keyboard nav, auto-play |
| `src/components/images/ImagePreviewGrid.vue` | Selectable thumbnail grid for scraping results |
| `src/components/images/ImageThumbnail.vue` | Single thumbnail with checkbox and dimension label |
| `src/stores/images.ts` | Pinia store for image state management |

### Modified Files

| File | Change |
|---|---|
| `src-tauri/Cargo.toml` | Add `scraper`, `image`, `url` crates |
| `src-tauri/src/db/schema.sql` | Add `image_sessions`, `images` tables |
| `src-tauri/src/lib.rs` | Register 5 image commands, add `mod images` |
| `src-tauri/src/commands/mod.rs` | Add `pub mod images;` |
| `src/types/index.ts` | Add image types, extend `SidebarSection` |
| `src/App.vue` | Add routing for `images-download`, `images-gallery` |
| `src/components/layout/AppSidebar.vue` | Add 2 image nav buttons |

---

## Task 1: Rust Dependencies & DB Schema

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/db/schema.sql`

- [ ] **Step 1: Add Rust crate dependencies**

Add to `src-tauri/Cargo.toml` under `[dependencies]`:

```toml
scraper = "0.20"
image = { version = "0.25", features = ["webp"] }
url = "2"
```

Note: `url` is needed for `url::Url::parse()` used in scraper.rs and commands/images.rs.

- [ ] **Step 2: Add DB schema for image tables**

Append to `src-tauri/src/db/schema.sql`:

```sql
-- Image download feature
CREATE TABLE IF NOT EXISTS image_sessions (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  source_url    TEXT NOT NULL,
  site_name     TEXT,
  image_count   INTEGER DEFAULT 0,
  output_dir    TEXT NOT NULL,
  created_at    DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS images (
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

- [ ] **Step 3: Verify Rust compiles**

Run: `cd /Volumes/Logitec2/work/YTDown/src-tauri && cargo check`
Expected: Compiles without errors (new crates downloaded)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/db/schema.sql
git commit -m "feat(images): add scraper/image crates and DB schema"
```

---

## Task 2: Rust Image Scraper Module

**Files:**
- Create: `src-tauri/src/images/mod.rs`
- Create: `src-tauri/src/images/scraper.rs`

- [ ] **Step 1: Create images module root**

Create `src-tauri/src/images/mod.rs`:

```rust
pub mod scraper;
pub mod downloader;
```

- [ ] **Step 2: Implement scraper**

Create `src-tauri/src/images/scraper.rs`:

```rust
use reqwest::Client;
use scraper::{Html, Selector};
use serde::Serialize;
use std::collections::HashSet;
use url::Url;

#[derive(Debug, Clone, Serialize)]
pub struct ScrapedImage {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub alt: Option<String>,
}

/// Fetch a web page and extract all <img> src URLs.
/// Returns deduplicated absolute URLs filtered by minimum dimensions.
pub async fn scrape_images_from_url(
    page_url: &str,
    min_width: u32,
    min_height: u32,
) -> Result<Vec<ScrapedImage>, String> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (compatible; YTDown/0.2)")
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let response = client
        .get(page_url)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "タイムアウト: 30秒以内にページを取得できませんでした".to_string()
            } else if e.is_connect() {
                "ネットワークエラー: ページに接続できませんでした".to_string()
            } else {
                format!("ページを取得できませんでした: {e}")
            }
        })?;

    let status = response.status();
    if status == reqwest::StatusCode::FORBIDDEN {
        return Err("アクセスがブロックされました。別のURLを試してください".to_string());
    }
    if !status.is_success() {
        return Err(format!("ページを取得できませんでした（ステータス: {status}）"));
    }

    let html = response.text().await.map_err(|e| format!("HTML read error: {e}"))?;
    let base_url = Url::parse(page_url).map_err(|e| format!("Invalid URL: {e}"))?;

    let document = Html::parse_document(&html);
    let img_selector = Selector::parse("img").unwrap();

    let mut seen_urls = HashSet::new();
    let mut images = Vec::new();

    for element in document.select(&img_selector) {
        let src = match element.value().attr("src") {
            Some(s) if !s.is_empty() => s,
            _ => continue,
        };

        let absolute_url = match base_url.join(src) {
            Ok(u) => u.to_string(),
            Err(_) => continue,
        };

        if !seen_urls.insert(absolute_url.clone()) {
            continue; // duplicate
        }

        // Parse width/height from attributes
        let width = element.value().attr("width").and_then(|w| w.parse::<u32>().ok());
        let height = element.value().attr("height").and_then(|h| h.parse::<u32>().ok());
        let alt = element.value().attr("alt").map(|s| s.to_string());

        // Filter: skip if both dimensions are known and both are below threshold
        // Also skip tiny images (< 32px either dimension — favicons, tracking pixels)
        if let (Some(w), Some(h)) = (width, height) {
            if w < 32 || h < 32 {
                continue;
            }
            if w < min_width && h < min_height {
                continue;
            }
        }

        images.push(ScrapedImage {
            url: absolute_url,
            width,
            height,
            alt,
        });
    }

    Ok(images)
}
```

- [ ] **Step 3: Register module in lib.rs**

Add `mod images;` to `src-tauri/src/lib.rs` near the other module declarations (after `mod ytdlp;`):

```rust
mod images;  // Add this line after existing mod declarations
```

- [ ] **Step 4: Verify compilation**

First, create stub `src-tauri/src/images/downloader.rs` (will be implemented in Task 3):

```rust
// Image downloader — implemented in Task 3
```

Run: `cd /Volumes/Logitec2/work/YTDown/src-tauri && cargo check`
Expected: Compiles.

- [ ] **Step 5: Write Rust test for scraper**

Add to bottom of `src-tauri/src/images/scraper.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_small_images() {
        // Images with both dimensions below min should be filtered
        // Images with one dimension above min should pass
        // Images with unknown dimensions should pass
    }

    #[tokio::test]
    async fn test_scrape_invalid_url() {
        let result = scrape_images_from_url("not-a-url", 100, 100).await;
        assert!(result.is_err());
    }
}
```

Run: `cd /Volumes/Logitec2/work/YTDown/src-tauri && cargo test`
Expected: Tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/images/
git commit -m "feat(images): add HTML scraper with img tag extraction"
```

---

## Task 3: Rust Image Downloader Module

**Files:**
- Create: `src-tauri/src/images/downloader.rs`

- [ ] **Step 1: Implement downloader**

Replace stub `src-tauri/src/images/downloader.rs`:

```rust
use image::ImageFormat;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct ImageToDownload {
    pub url: String,
    pub filename_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadedImage {
    pub original_url: String,
    pub file_path: String,
    pub filename: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub file_size: u64,
    pub format: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub session_id: i64,
    pub image_index: usize,
    pub total_images: usize,
    pub current_url: String,
    pub percent: f64,
    pub status: String,       // "downloading" | "completed" | "failed"
    pub error_message: Option<String>,
}

/// Download a single image, optionally convert to target format, and save to disk.
pub async fn download_and_save(
    client: &Client,
    img: &ImageToDownload,
    output_dir: &Path,
    target_format: Option<&str>,
    index: usize,
) -> Result<DownloadedImage, String> {
    // Fetch image bytes
    let response = client
        .get(&img.url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let bytes = response.bytes().await.map_err(|e| format!("Read error: {e}"))?;

    // Determine filename
    let original_filename = img
        .filename_hint
        .as_deref()
        .unwrap_or("")
        .to_string();

    let filename = if original_filename.is_empty() {
        format!("image_{index:04}")
    } else {
        // Strip extension for re-adding later
        Path::new(&original_filename)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("image_{index:04}"))
    };

    // Try to decode image for dimensions and conversion
    let (final_bytes, format_str, width, height) = match image::load_from_memory(&bytes) {
        Ok(img_data) => {
            let w = img_data.width();
            let h = img_data.height();

            if let Some("webp") = target_format {
                // Convert to WebP
                let mut buf = std::io::Cursor::new(Vec::new());
                match img_data.write_to(&mut buf, ImageFormat::WebP) {
                    Ok(_) => (buf.into_inner(), "webp".to_string(), Some(w), Some(h)),
                    Err(_) => {
                        // Fallback: save original
                        let ext = guess_extension(&bytes);
                        (bytes.to_vec(), ext, Some(w), Some(h))
                    }
                }
            } else {
                let ext = guess_extension(&bytes);
                (bytes.to_vec(), ext, Some(w), Some(h))
            }
        }
        Err(_) => {
            // Can't decode — save original bytes
            let ext = guess_extension(&bytes);
            (bytes.to_vec(), ext, None, None)
        }
    };

    let full_filename = format!("{filename}.{format_str}");
    let file_path = output_dir.join(&full_filename);

    // Handle filename collision
    let file_path = ensure_unique_path(file_path);
    let final_filename = file_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    // Create output directory if needed
    fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Cannot create directory: {e}"))?;

    // Write file
    let file_size = final_bytes.len() as u64;
    fs::write(&file_path, &final_bytes)
        .await
        .map_err(|e| format!("Write error: {e}"))?;

    Ok(DownloadedImage {
        original_url: img.url.clone(),
        file_path: file_path.to_string_lossy().to_string(),
        filename: final_filename,
        width,
        height,
        file_size,
        format: format_str,
    })
}

/// Guess file extension from magic bytes
fn guess_extension(bytes: &[u8]) -> String {
    if bytes.starts_with(b"\x89PNG") {
        "png".to_string()
    } else if bytes.starts_with(b"\xFF\xD8\xFF") {
        "jpg".to_string()
    } else if bytes.starts_with(b"GIF8") {
        "gif".to_string()
    } else if bytes.starts_with(b"RIFF") && bytes.len() > 12 && &bytes[8..12] == b"WEBP" {
        "webp".to_string()
    } else {
        "bin".to_string()
    }
}

/// If file exists, append _1, _2, etc.
fn ensure_unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }
    let stem = path.file_stem().unwrap().to_string_lossy().to_string();
    let ext = path.extension().unwrap_or_default().to_string_lossy().to_string();
    let parent = path.parent().unwrap();

    for i in 1..1000 {
        let new_name = format!("{stem}_{i}.{ext}");
        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return new_path;
        }
    }
    path
}

/// Create a shared HTTP client for image downloads.
pub fn create_download_client() -> Result<Client, String> {
    Client::builder()
        .user_agent("Mozilla/5.0 (compatible; YTDown/0.2)")
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Client error: {e}"))
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd /Volumes/Logitec2/work/YTDown/src-tauri && cargo check`
Expected: Compiles without errors.

- [ ] **Step 3: Write test for downloader utilities**

Add to bottom of `downloader.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_extension_png() {
        let png_header = b"\x89PNG\r\n\x1a\n";
        assert_eq!(guess_extension(png_header), "png");
    }

    #[test]
    fn test_guess_extension_jpg() {
        let jpg_header = b"\xFF\xD8\xFF\xE0";
        assert_eq!(guess_extension(jpg_header), "jpg");
    }

    #[test]
    fn test_guess_extension_unknown() {
        assert_eq!(guess_extension(b"unknown"), "bin");
    }

    #[test]
    fn test_webp_conversion_works() {
        // Create a minimal valid PNG image and verify WebP conversion
        let img = image::RgbImage::new(2, 2);
        let dynamic_img = image::DynamicImage::ImageRgb8(img);
        let mut buf = std::io::Cursor::new(Vec::new());
        let result = dynamic_img.write_to(&mut buf, image::ImageFormat::WebP);
        assert!(result.is_ok(), "WebP encoding must work with the image crate features we have");
        assert!(!buf.into_inner().is_empty());
    }
}
```

Run: `cd /Volumes/Logitec2/work/YTDown/src-tauri && cargo test`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/images/downloader.rs
git commit -m "feat(images): add image downloader with WebP conversion"
```

---

## Task 4: Tauri Commands

**Files:**
- Create: `src-tauri/src/commands/images.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 0: Verify AppState structure**

Read `src-tauri/src/state.rs` (or wherever `AppState` is defined) to confirm the DB field name and type. The commands below assume `state.db.lock()` returns a `MutexGuard<Connection>`. If the field is named differently or uses `Arc<Mutex<>>`, adjust the lock calls accordingly.

- [ ] **Step 1: Create Tauri image commands**

Create `src-tauri/src/commands/images.rs`:

```rust
use crate::images::downloader::{self, DownloadProgress, ImageToDownload};
use crate::images::scraper::{self, ScrapedImage};
use rusqlite::params;
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct ImageSession {
    pub id: i64,
    pub source_url: String,
    pub site_name: Option<String>,
    pub image_count: i64,
    pub output_dir: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageRecord {
    pub id: i64,
    pub session_id: i64,
    pub original_url: String,
    pub file_path: Option<String>,
    pub filename: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub file_size: Option<i64>,
    pub format: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[tauri::command]
pub async fn scrape_images(
    url: String,
    min_width: u32,
    min_height: u32,
) -> Result<Vec<ScrapedImage>, String> {
    scraper::scrape_images_from_url(&url, min_width, min_height).await
}

#[tauri::command]
pub async fn download_images(
    images: Vec<ImageToDownload>,
    output_dir: String,
    format: Option<String>,
    session_url: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let output_path = Path::new(&output_dir);

    // Extract site name from URL
    let site_name = url::Url::parse(&session_url)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()));

    // Create session in DB
    let db = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
    db.execute(
        "INSERT INTO image_sessions (source_url, site_name, image_count, output_dir) VALUES (?1, ?2, ?3, ?4)",
        params![session_url, site_name, images.len() as i64, output_dir],
    ).map_err(|e| format!("DB insert error: {e}"))?;
    let session_id = db.last_insert_rowid();
    drop(db); // Release lock before async work

    let client = downloader::create_download_client()?;
    let total = images.len();
    let format_ref = format.as_deref();

    for (i, img) in images.iter().enumerate() {
        // Emit progress
        let _ = app.emit("image-download-progress", DownloadProgress {
            session_id,
            image_index: i,
            total_images: total,
            current_url: img.url.clone(),
            percent: (i as f64 / total as f64) * 100.0,
            status: "downloading".to_string(),
            error_message: None,
        });

        match downloader::download_and_save(&client, img, output_path, format_ref, i).await {
            Ok(result) => {
                let db = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
                db.execute(
                    "INSERT INTO images (session_id, original_url, file_path, filename, width, height, file_size, format, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'completed')",
                    params![
                        session_id,
                        result.original_url,
                        result.file_path,
                        result.filename,
                        result.width.map(|v| v as i64),
                        result.height.map(|v| v as i64),
                        result.file_size as i64,
                        result.format,
                    ],
                ).map_err(|e| format!("DB insert error: {e}"))?;
            }
            Err(err) => {
                let db = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
                db.execute(
                    "INSERT INTO images (session_id, original_url, status) VALUES (?1, ?2, 'failed')",
                    params![session_id, img.url],
                ).map_err(|e| format!("DB insert error: {e}"))?;

                let _ = app.emit("image-download-progress", DownloadProgress {
                    session_id,
                    image_index: i,
                    total_images: total,
                    current_url: img.url.clone(),
                    percent: ((i + 1) as f64 / total as f64) * 100.0,
                    status: "failed".to_string(),
                    error_message: Some(err),
                });
            }
        }
    }

    // Update session count
    let db = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
    let completed_count: i64 = db.query_row(
        "SELECT COUNT(*) FROM images WHERE session_id = ?1 AND status = 'completed'",
        params![session_id],
        |row| row.get(0),
    ).unwrap_or(0);
    db.execute(
        "UPDATE image_sessions SET image_count = ?1 WHERE id = ?2",
        params![completed_count, session_id],
    ).map_err(|e| format!("DB update error: {e}"))?;

    // Final progress
    let _ = app.emit("image-download-progress", DownloadProgress {
        session_id,
        image_index: total,
        total_images: total,
        current_url: String::new(),
        percent: 100.0,
        status: "completed".to_string(),
        error_message: None,
    });

    Ok(session_id)
}

#[tauri::command]
pub async fn list_image_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<ImageSession>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
    let mut stmt = db
        .prepare("SELECT id, source_url, site_name, image_count, output_dir, created_at FROM image_sessions ORDER BY created_at DESC")
        .map_err(|e| format!("Query error: {e}"))?;

    let sessions = stmt
        .query_map([], |row| {
            Ok(ImageSession {
                id: row.get(0)?,
                source_url: row.get(1)?,
                site_name: row.get(2)?,
                image_count: row.get(3)?,
                output_dir: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| format!("Query error: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(sessions)
}

#[tauri::command]
pub async fn list_session_images(
    session_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<ImageRecord>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
    let mut stmt = db
        .prepare("SELECT id, session_id, original_url, file_path, filename, width, height, file_size, format, status, created_at FROM images WHERE session_id = ?1 ORDER BY id")
        .map_err(|e| format!("Query error: {e}"))?;

    let images = stmt
        .query_map(params![session_id], |row| {
            Ok(ImageRecord {
                id: row.get(0)?,
                session_id: row.get(1)?,
                original_url: row.get(2)?,
                file_path: row.get(3)?,
                filename: row.get(4)?,
                width: row.get(5)?,
                height: row.get(6)?,
                file_size: row.get(7)?,
                format: row.get(8)?,
                status: row.get(9)?,
                created_at: row.get(10)?,
            })
        })
        .map_err(|e| format!("Query error: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(images)
}

#[tauri::command]
pub async fn delete_image_session(
    session_id: i64,
    delete_files: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if delete_files {
        let db = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
        let mut stmt = db
            .prepare("SELECT file_path FROM images WHERE session_id = ?1 AND file_path IS NOT NULL")
            .map_err(|e| format!("Query error: {e}"))?;

        let paths: Vec<String> = stmt
            .query_map(params![session_id], |row| row.get(0))
            .map_err(|e| format!("Query error: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);  // Must drop stmt before db (stmt borrows db)
        drop(db);

        for path in paths {
            let _ = tokio::fs::remove_file(&path).await;
        }
    }

    let db = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
    db.execute(
        "DELETE FROM image_sessions WHERE id = ?1",
        params![session_id],
    )
    .map_err(|e| format!("DB delete error: {e}"))?;

    Ok(())
}
```

- [ ] **Step 2: Register in commands/mod.rs**

Add to `src-tauri/src/commands/mod.rs`:

```rust
pub mod images;
```

- [ ] **Step 3: Register commands in lib.rs**

Add to the `tauri::generate_handler![]` macro in `src-tauri/src/lib.rs`:

```rust
commands::images::scrape_images,
commands::images::download_images,
commands::images::list_image_sessions,
commands::images::list_session_images,
commands::images::delete_image_session,
```

- [ ] **Step 4: Verify compilation**

Run: `cd /Volumes/Logitec2/work/YTDown/src-tauri && cargo check`
Expected: Compiles. Fix any type mismatches with existing `AppState` structure.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/images.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(images): add Tauri commands for image scrape/download/list/delete"
```

---

## Task 5: TypeScript Types & Pinia Store

**Files:**
- Modify: `src/types/index.ts`
- Create: `src/stores/images.ts`

- [ ] **Step 1: Add TypeScript types**

Add to `src/types/index.ts`:

```typescript
// Image Download Feature
export interface ScrapedImage {
  url: string
  width: number | null
  height: number | null
  alt: string | null
}

export interface ImageToDownload {
  url: string
  filename_hint: string | null
}

export interface ImageSession {
  id: number
  source_url: string
  site_name: string | null
  image_count: number
  output_dir: string
  created_at: string
}

export interface ImageRecord {
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

export interface ImageDownloadProgress {
  session_id: number
  image_index: number
  total_images: number
  current_url: string
  percent: number
  status: 'downloading' | 'completed' | 'failed'
  error_message: string | null
}
```

Also update the `SidebarSection` type to add `'images-download' | 'images-gallery'`.

- [ ] **Step 2: Create Pinia store**

Create `src/stores/images.ts`:

```typescript
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
  ScrapedImage,
  ImageToDownload,
  ImageSession,
  ImageRecord,
  ImageDownloadProgress,
} from '@/types'

export const useImagesStore = defineStore('images', () => {
  // State
  const scrapedImages = ref<ScrapedImage[]>([])
  const selectedIds = ref<Set<number>>(new Set())
  const sessions = ref<ImageSession[]>([])
  const sessionImages = ref<Map<number, ImageRecord[]>>(new Map())
  const downloadProgress = ref<ImageDownloadProgress | null>(null)
  const scraping = ref(false)
  const downloading = ref(false)
  const error = ref<string | null>(null)
  const currentPageUrl = ref('')  // The page URL that was scraped

  // Computed
  const selectedCount = computed(() => selectedIds.value.size)
  const hasSelection = computed(() => selectedIds.value.size > 0)

  // Actions
  async function scrapeUrl(url: string, minWidth = 100, minHeight = 100) {
    scraping.value = true
    error.value = null
    scrapedImages.value = []
    selectedIds.value = new Set()

    try {
      currentPageUrl.value = url  // Store the page URL for later use
      const images = await invoke<ScrapedImage[]>('scrape_images', {
        url,
        minWidth,
        minHeight,
      })
      scrapedImages.value = images
      // Auto-select all by default
      selectedIds.value = new Set(images.map((_, i) => i))
    } catch (e) {
      error.value = String(e)
    } finally {
      scraping.value = false
    }
  }

  async function startDownload(outputDir: string, format?: string) {
    downloading.value = true
    error.value = null

    const imagesToDownload: ImageToDownload[] = Array.from(selectedIds.value)
      .sort((a, b) => a - b)
      .map((index) => {
        const img = scrapedImages.value[index]
        // Extract filename hint from URL
        const urlPath = new URL(img.url).pathname
        const filename = urlPath.split('/').pop() || null
        return { url: img.url, filename_hint: filename }
      })

    try {
      await invoke<number>('download_images', {
        images: imagesToDownload,
        outputDir,
        format: format || null,
        sessionUrl: currentPageUrl.value,  // Use the scraped page URL, not an image URL
      })
      await loadSessions()
    } catch (e) {
      error.value = String(e)
    } finally {
      downloading.value = false
      downloadProgress.value = null
    }
  }

  async function loadSessions() {
    try {
      sessions.value = await invoke<ImageSession[]>('list_image_sessions')
    } catch (e) {
      error.value = String(e)
    }
  }

  async function loadSessionImages(sessionId: number): Promise<ImageRecord[]> {
    try {
      const images = await invoke<ImageRecord[]>('list_session_images', { sessionId })
      sessionImages.value.set(sessionId, images)
      return images
    } catch (e) {
      error.value = String(e)
      return []
    }
  }

  async function deleteSession(sessionId: number, deleteFiles = false) {
    try {
      await invoke('delete_image_session', { sessionId, deleteFiles })
      sessions.value = sessions.value.filter((s) => s.id !== sessionId)
      sessionImages.value.delete(sessionId)
    } catch (e) {
      error.value = String(e)
    }
  }

  function toggleSelect(index: number) {
    const newSet = new Set(selectedIds.value)
    if (newSet.has(index)) {
      newSet.delete(index)
    } else {
      newSet.add(index)
    }
    selectedIds.value = newSet
  }

  function selectAll() {
    selectedIds.value = new Set(scrapedImages.value.map((_, i) => i))
  }

  function deselectAll() {
    selectedIds.value = new Set()
  }

  // Progress listener setup
  function setupProgressListener() {
    return listen<ImageDownloadProgress>('image-download-progress', (event) => {
      downloadProgress.value = event.payload
    })
  }

  return {
    scrapedImages,
    selectedIds,
    sessions,
    sessionImages,
    downloadProgress,
    scraping,
    downloading,
    error,
    selectedCount,
    hasSelection,
    currentPageUrl,
    scrapeUrl,
    startDownload,
    loadSessions,
    loadSessionImages,
    deleteSession,
    toggleSelect,
    selectAll,
    deselectAll,
    setupProgressListener,
  }
})
```

- [ ] **Step 3: Verify TypeScript compilation**

Run: `cd /Volumes/Logitec2/work/YTDown && npx tsc --noEmit`
Expected: No type errors (or only pre-existing ones).

- [ ] **Step 4: Commit**

```bash
git add src/types/index.ts src/stores/images.ts
git commit -m "feat(images): add TypeScript types and Pinia store"
```

---

## Task 6: ImageThumbnail & ImagePreviewGrid Components

**Files:**
- Create: `src/components/images/ImageThumbnail.vue`
- Create: `src/components/images/ImagePreviewGrid.vue`

- [ ] **Step 1: Create ImageThumbnail component**

Create `src/components/images/ImageThumbnail.vue`:

```vue
<script setup lang="ts">
import type { ScrapedImage } from '@/types'

defineProps<{
  image: ScrapedImage
  index: number
  selected: boolean
}>()

defineEmits<{
  'toggle-select': [index: number]
}>()
</script>

<template>
  <div
    class="relative group cursor-pointer rounded-lg overflow-hidden border-2 transition-all"
    :class="selected ? 'border-blue-500 ring-2 ring-blue-500/30' : 'border-transparent hover:border-neutral-400 dark:hover:border-neutral-600'"
    @click="$emit('toggle-select', index)"
  >
    <!-- Thumbnail image -->
    <div class="aspect-square bg-neutral-200 dark:bg-neutral-800">
      <img
        :src="image.url"
        :alt="image.alt || ''"
        class="w-full h-full object-cover"
        loading="lazy"
        @error="($event.target as HTMLImageElement).style.display = 'none'"
      />
    </div>

    <!-- Checkbox overlay -->
    <div class="absolute top-1.5 left-1.5">
      <div
        class="w-5 h-5 rounded border-2 flex items-center justify-center text-xs font-bold transition-colors"
        :class="selected
          ? 'bg-blue-500 border-blue-500 text-white'
          : 'bg-black/30 border-white/60 text-transparent group-hover:border-white'"
      >
        ✓
      </div>
    </div>

    <!-- Dimension label -->
    <div
      v-if="image.width && image.height"
      class="absolute bottom-0 inset-x-0 bg-black/60 text-white text-[10px] text-center py-0.5 opacity-0 group-hover:opacity-100 transition-opacity"
    >
      {{ image.width }}×{{ image.height }}
    </div>
  </div>
</template>
```

- [ ] **Step 2: Create ImagePreviewGrid component**

Create `src/components/images/ImagePreviewGrid.vue`:

```vue
<script setup lang="ts">
import type { ScrapedImage } from '@/types'
import ImageThumbnail from './ImageThumbnail.vue'

defineProps<{
  images: ScrapedImage[]
  selectedIds: Set<number>
}>()

defineEmits<{
  'toggle-select': [index: number]
  'select-all': []
  'deselect-all': []
}>()
</script>

<template>
  <div>
    <!-- Selection toolbar -->
    <div class="flex items-center justify-between mb-3 text-sm">
      <span class="text-neutral-500 dark:text-neutral-400">
        {{ images.length }} 枚の画像が見つかりました
        <span v-if="selectedIds.size > 0" class="text-blue-500 font-medium">
          （{{ selectedIds.size }} 枚選択中）
        </span>
      </span>
      <div class="flex gap-2">
        <button
          class="px-2 py-1 text-xs rounded bg-neutral-200 dark:bg-neutral-700 hover:bg-neutral-300 dark:hover:bg-neutral-600"
          @click="$emit('select-all')"
        >
          全選択
        </button>
        <button
          class="px-2 py-1 text-xs rounded bg-neutral-200 dark:bg-neutral-700 hover:bg-neutral-300 dark:hover:bg-neutral-600"
          @click="$emit('deselect-all')"
        >
          全解除
        </button>
      </div>
    </div>

    <!-- Grid -->
    <div class="grid grid-cols-4 sm:grid-cols-5 md:grid-cols-6 lg:grid-cols-8 gap-2">
      <ImageThumbnail
        v-for="(image, index) in images"
        :key="image.url"
        :image="image"
        :index="index"
        :selected="selectedIds.has(index)"
        @toggle-select="$emit('toggle-select', $event)"
      />
    </div>
  </div>
</template>
```

- [ ] **Step 3: Commit**

```bash
git add src/components/images/ImageThumbnail.vue src/components/images/ImagePreviewGrid.vue
git commit -m "feat(images): add ImageThumbnail and ImagePreviewGrid components"
```

---

## Task 7: ImageDownloadView — Main Download Screen

**Files:**
- Create: `src/components/images/ImageDownloadView.vue`

- [ ] **Step 1: Create the main download view**

Create `src/components/images/ImageDownloadView.vue`:

```vue
<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useImagesStore } from '@/stores/images'
import { useSettingsStore } from '@/stores/settings'
import ImagePreviewGrid from './ImagePreviewGrid.vue'

const imagesStore = useImagesStore()
const settingsStore = useSettingsStore()

const url = ref('')
const minWidth = ref(100)
const minHeight = ref(100)
const format = ref<string | undefined>(undefined)

let unlistenProgress: (() => void) | null = null

onMounted(async () => {
  const unlisten = await imagesStore.setupProgressListener()
  unlistenProgress = unlisten as unknown as () => void
})

onUnmounted(() => {
  unlistenProgress?.()
})

async function handleScrape() {
  if (!url.value.trim()) return
  await imagesStore.scrapeUrl(url.value.trim(), minWidth.value, minHeight.value)
}

async function handleDownload() {
  if (!imagesStore.hasSelection) return
  const outputDir = settingsStore.settings.download_dir
    ? `${settingsStore.settings.download_dir}/images`
    : `~/Downloads/YTDown/images`
  await imagesStore.startDownload(outputDir, format.value)
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !imagesStore.scraping) {
    handleScrape()
  }
}
</script>

<template>
  <div class="flex flex-col h-full p-4 overflow-y-auto">
    <!-- URL input bar -->
    <div class="flex gap-2 mb-3">
      <input
        v-model="url"
        type="url"
        placeholder="画像を取得するURLを入力..."
        class="flex-1 px-3 py-2 rounded-lg bg-neutral-100 dark:bg-neutral-800 border border-neutral-300 dark:border-neutral-600 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
        :disabled="imagesStore.scraping"
        @keydown="handleKeydown"
      />
      <button
        class="px-4 py-2 rounded-lg bg-blue-500 text-white text-sm font-medium hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed"
        :disabled="imagesStore.scraping || !url.trim()"
        @click="handleScrape"
      >
        {{ imagesStore.scraping ? '取得中...' : '取得' }}
      </button>
    </div>

    <!-- Filter settings -->
    <div class="flex gap-4 mb-4 text-sm">
      <label class="flex items-center gap-1.5">
        <span class="text-neutral-500 dark:text-neutral-400">最小幅:</span>
        <input
          v-model.number="minWidth"
          type="number"
          min="0"
          class="w-16 px-2 py-1 rounded bg-neutral-100 dark:bg-neutral-800 border border-neutral-300 dark:border-neutral-600"
        />
        <span class="text-neutral-400">px</span>
      </label>
      <label class="flex items-center gap-1.5">
        <span class="text-neutral-500 dark:text-neutral-400">最小高さ:</span>
        <input
          v-model.number="minHeight"
          type="number"
          min="0"
          class="w-16 px-2 py-1 rounded bg-neutral-100 dark:bg-neutral-800 border border-neutral-300 dark:border-neutral-600"
        />
        <span class="text-neutral-400">px</span>
      </label>
      <label class="flex items-center gap-1.5">
        <span class="text-neutral-500 dark:text-neutral-400">変換形式:</span>
        <select
          v-model="format"
          class="px-2 py-1 rounded bg-neutral-100 dark:bg-neutral-800 border border-neutral-300 dark:border-neutral-600"
        >
          <option :value="undefined">オリジナル</option>
          <option value="webp">WebP</option>
          <option value="avif" disabled title="将来対応予定">AVIF (準備中)</option>
        </select>
      </label>
    </div>

    <!-- Error message -->
    <div v-if="imagesStore.error" class="mb-3 px-3 py-2 rounded-lg bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400 text-sm">
      {{ imagesStore.error }}
    </div>

    <!-- Preview grid -->
    <div v-if="imagesStore.scrapedImages.length > 0" class="flex-1 min-h-0">
      <ImagePreviewGrid
        :images="imagesStore.scrapedImages"
        :selected-ids="imagesStore.selectedIds"
        @toggle-select="imagesStore.toggleSelect"
        @select-all="imagesStore.selectAll"
        @deselect-all="imagesStore.deselectAll"
      />
    </div>

    <!-- Empty state -->
    <div
      v-else-if="!imagesStore.scraping && !imagesStore.error"
      class="flex-1 flex items-center justify-center text-neutral-400 dark:text-neutral-500"
    >
      <div class="text-center">
        <div class="text-4xl mb-2">🖼</div>
        <p>URLを入力して画像を取得</p>
      </div>
    </div>

    <!-- Loading state -->
    <div
      v-if="imagesStore.scraping"
      class="flex-1 flex items-center justify-center text-neutral-400"
    >
      <div class="text-center">
        <div class="animate-spin text-2xl mb-2">⏳</div>
        <p>ページを解析中...</p>
      </div>
    </div>

    <!-- Download bar -->
    <div v-if="imagesStore.scrapedImages.length > 0" class="mt-3 pt-3 border-t border-neutral-200 dark:border-neutral-700">
      <!-- Progress bar (during download) -->
      <div v-if="imagesStore.downloading && imagesStore.downloadProgress" class="mb-2">
        <div class="flex justify-between text-xs text-neutral-500 mb-1">
          <span>{{ imagesStore.downloadProgress.image_index + 1 }} / {{ imagesStore.downloadProgress.total_images }}</span>
          <span>{{ Math.round(imagesStore.downloadProgress.percent) }}%</span>
        </div>
        <div class="h-1.5 bg-neutral-200 dark:bg-neutral-700 rounded-full overflow-hidden">
          <div
            class="h-full bg-blue-500 rounded-full transition-all duration-300"
            :style="{ width: `${imagesStore.downloadProgress.percent}%` }"
          />
        </div>
      </div>

      <button
        class="w-full py-2 rounded-lg bg-blue-500 text-white text-sm font-medium hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed"
        :disabled="!imagesStore.hasSelection || imagesStore.downloading"
        @click="handleDownload"
      >
        {{ imagesStore.downloading ? 'ダウンロード中...' : `${imagesStore.selectedCount} 枚をダウンロード` }}
      </button>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/images/ImageDownloadView.vue
git commit -m "feat(images): add ImageDownloadView main screen"
```

---

## Task 8: ImageGalleryView & ImageSlideshow

**Files:**
- Create: `src/components/images/ImageGalleryView.vue`
- Create: `src/components/images/ImageSlideshow.vue`

- [ ] **Step 1: Create ImageSlideshow component**

Create `src/components/images/ImageSlideshow.vue`:

```vue
<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, computed } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import type { ImageRecord } from '@/types'

const props = defineProps<{
  images: ImageRecord[]
  startIndex: number
  open: boolean
}>()

const emit = defineEmits<{
  close: []
  'reveal-in-finder': [filePath: string]
}>()

const currentIndex = ref(0)
const playing = ref(false)
const interval = ref(3)
const showControls = ref(true)
let playTimer: ReturnType<typeof setInterval> | null = null
let hideTimer: ReturnType<typeof setTimeout> | null = null

const currentImage = computed(() => props.images[currentIndex.value])
const imageSrc = computed(() => {
  const path = currentImage.value?.file_path
  return path ? convertFileSrc(path) : ''
})

watch(() => props.open, (open) => {
  if (open) {
    currentIndex.value = props.startIndex
    showControls.value = true
    resetHideTimer()
  } else {
    stopPlaying()
  }
})

function next() {
  if (currentIndex.value < props.images.length - 1) {
    currentIndex.value++
  } else {
    stopPlaying()
  }
}

function prev() {
  if (currentIndex.value > 0) {
    currentIndex.value--
  }
}

function togglePlay() {
  if (playing.value) {
    stopPlaying()
  } else {
    startPlaying()
  }
}

function startPlaying() {
  playing.value = true
  playTimer = setInterval(() => {
    if (currentIndex.value < props.images.length - 1) {
      currentIndex.value++
    } else {
      stopPlaying()
    }
  }, interval.value * 1000)
}

function stopPlaying() {
  playing.value = false
  if (playTimer) {
    clearInterval(playTimer)
    playTimer = null
  }
}

function setInterval_(sec: number) {
  interval.value = sec
  if (playing.value) {
    stopPlaying()
    startPlaying()
  }
}

function resetHideTimer() {
  showControls.value = true
  if (hideTimer) clearTimeout(hideTimer)
  hideTimer = setTimeout(() => {
    showControls.value = false
  }, 3000)
}

function handleMouseMove() {
  resetHideTimer()
}

function handleKeydown(e: KeyboardEvent) {
  switch (e.key) {
    case 'ArrowLeft':
      prev()
      break
    case 'ArrowRight':
      next()
      break
    case ' ':
      e.preventDefault()
      togglePlay()
      break
    case 'Escape':
      emit('close')
      break
  }
  resetHideTimer()
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
  stopPlaying()
  if (hideTimer) clearTimeout(hideTimer)
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-[9999] bg-black flex items-center justify-center select-none"
      @mousemove="handleMouseMove"
    >
      <!-- Image -->
      <img
        v-if="imageSrc"
        :src="imageSrc"
        :alt="currentImage?.filename || ''"
        class="max-w-full max-h-full object-contain"
      />

      <!-- Controls overlay -->
      <div
        class="absolute inset-0 transition-opacity duration-300"
        :class="showControls ? 'opacity-100' : 'opacity-0 pointer-events-none'"
      >
        <!-- Top bar -->
        <div class="absolute top-0 inset-x-0 flex justify-between items-center p-4">
          <div class="bg-black/50 px-3 py-1.5 rounded-lg text-white text-sm">
            {{ currentIndex + 1 }} / {{ images.length }}
          </div>
          <button
            class="bg-black/50 px-3 py-1.5 rounded-lg text-white text-sm hover:bg-black/70"
            @click="emit('close')"
          >
            ✕
          </button>
        </div>

        <!-- Prev / Next buttons -->
        <button
          v-if="currentIndex > 0"
          class="absolute left-4 top-1/2 -translate-y-1/2 bg-black/50 w-10 h-10 rounded-full text-white text-lg flex items-center justify-center hover:bg-black/70"
          @click="prev"
        >
          ◀
        </button>
        <button
          v-if="currentIndex < images.length - 1"
          class="absolute right-4 top-1/2 -translate-y-1/2 bg-black/50 w-10 h-10 rounded-full text-white text-lg flex items-center justify-center hover:bg-black/70"
          @click="next"
        >
          ▶
        </button>

        <!-- Bottom bar -->
        <div class="absolute bottom-0 inset-x-0 flex justify-center items-center gap-3 p-4">
          <button
            class="bg-black/50 px-3 py-1.5 rounded-lg text-white text-sm hover:bg-black/70"
            @click="togglePlay"
          >
            {{ playing ? '⏸' : '▶' }} {{ interval }}秒
          </button>
          <select
            :value="interval"
            class="bg-black/50 px-2 py-1.5 rounded-lg text-white text-sm border-none"
            @change="setInterval_(Number(($event.target as HTMLSelectElement).value))"
          >
            <option value="1">1秒</option>
            <option value="3">3秒</option>
            <option value="5">5秒</option>
            <option value="10">10秒</option>
          </select>
          <button
            v-if="currentImage?.file_path"
            class="bg-black/50 px-3 py-1.5 rounded-lg text-white text-sm hover:bg-black/70"
            @click="emit('reveal-in-finder', currentImage.file_path!)"
          >
            📂 Finder
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
```

- [ ] **Step 2: Create ImageGalleryView component**

Create `src/components/images/ImageGalleryView.vue`:

```vue
<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { convertFileSrc } from '@tauri-apps/api/core'
import { useImagesStore } from '@/stores/images'
import type { ImageRecord } from '@/types'
import ImageSlideshow from './ImageSlideshow.vue'

const imagesStore = useImagesStore()

const thumbnailSize = ref(150)
const slideshowOpen = ref(false)
const slideshowImages = ref<ImageRecord[]>([])
const slideshowStartIndex = ref(0)
const expandedSessionId = ref<number | null>(null)
const sessionImagesMap = ref<Map<number, ImageRecord[]>>(new Map())

onMounted(async () => {
  await imagesStore.loadSessions()
})

async function toggleSession(sessionId: number) {
  if (expandedSessionId.value === sessionId) {
    expandedSessionId.value = null
    return
  }
  expandedSessionId.value = sessionId
  if (!sessionImagesMap.value.has(sessionId)) {
    const images = await imagesStore.loadSessionImages(sessionId)
    sessionImagesMap.value.set(sessionId, images)
  }
}

function getSessionImages(sessionId: number): ImageRecord[] {
  return sessionImagesMap.value.get(sessionId) || []
}

function completedImages(sessionId: number): ImageRecord[] {
  return getSessionImages(sessionId).filter((img) => img.status === 'completed' && img.file_path)
}

function openSlideshow(sessionId: number, startIndex = 0) {
  slideshowImages.value = completedImages(sessionId)
  slideshowStartIndex.value = startIndex
  slideshowOpen.value = true
}

async function handleDeleteSession(sessionId: number) {
  if (confirm('このセッションを削除しますか？\n\n「OK」でファイルも削除します。')) {
    await imagesStore.deleteSession(sessionId, true)
    sessionImagesMap.value.delete(sessionId)
    if (expandedSessionId.value === sessionId) {
      expandedSessionId.value = null
    }
  }
}

async function revealInFinder(filePath: string) {
  await invoke('reveal_in_finder', { path: filePath })
}

function getImageSrc(record: ImageRecord): string {
  return record.file_path ? convertFileSrc(record.file_path) : ''
}
</script>

<template>
  <div class="flex flex-col h-full p-4 overflow-y-auto">
    <!-- Header with size slider -->
    <div class="flex items-center justify-between mb-4">
      <h2 class="text-lg font-medium text-neutral-800 dark:text-neutral-200">画像ギャラリー</h2>
      <div class="flex items-center gap-3">
        <label class="flex items-center gap-2 text-sm text-neutral-500">
          <span class="text-xs">小</span>
          <input
            v-model.number="thumbnailSize"
            type="range"
            min="80"
            max="300"
            step="10"
            class="w-24 accent-blue-500"
          />
          <span class="text-xs">大</span>
        </label>
      </div>
    </div>

    <!-- Empty state -->
    <div
      v-if="imagesStore.sessions.length === 0"
      class="flex-1 flex items-center justify-center text-neutral-400"
    >
      <div class="text-center">
        <div class="text-4xl mb-2">📂</div>
        <p>ダウンロードした画像はありません</p>
      </div>
    </div>

    <!-- Session groups -->
    <div v-for="session in imagesStore.sessions" :key="session.id" class="mb-4">
      <!-- Session header -->
      <div
        class="flex items-center justify-between px-3 py-2 rounded-lg bg-neutral-100 dark:bg-neutral-800 cursor-pointer hover:bg-neutral-200 dark:hover:bg-neutral-700"
        @click="toggleSession(session.id)"
      >
        <div class="flex items-center gap-2">
          <span class="text-sm">{{ expandedSessionId === session.id ? '▼' : '▶' }}</span>
          <span class="text-sm font-medium">{{ session.site_name || session.source_url }}</span>
          <span class="text-xs text-neutral-400">{{ session.image_count }}枚</span>
        </div>
        <div class="flex items-center gap-2">
          <button
            v-if="expandedSessionId === session.id && completedImages(session.id).length > 0"
            class="px-2 py-1 text-xs rounded bg-blue-500 text-white hover:bg-blue-600"
            @click.stop="openSlideshow(session.id)"
          >
            ▶ スライドショー
          </button>
          <button
            class="px-2 py-1 text-xs rounded bg-neutral-300 dark:bg-neutral-600 hover:bg-red-500 hover:text-white"
            @click.stop="handleDeleteSession(session.id)"
          >
            削除
          </button>
        </div>
      </div>

      <!-- Session images grid -->
      <div
        v-if="expandedSessionId === session.id"
        class="mt-2 grid gap-2"
        :style="{ gridTemplateColumns: `repeat(auto-fill, minmax(${thumbnailSize}px, 1fr))` }"
      >
        <div
          v-for="(img, idx) in completedImages(session.id)"
          :key="img.id"
          class="relative group cursor-pointer rounded-lg overflow-hidden bg-neutral-200 dark:bg-neutral-800"
          :style="{ height: `${thumbnailSize}px` }"
          @click="openSlideshow(session.id, idx)"
        >
          <img
            :src="getImageSrc(img)"
            :alt="img.filename || ''"
            class="w-full h-full object-cover"
            loading="lazy"
          />
          <div class="absolute bottom-0 inset-x-0 bg-black/60 text-white text-[10px] text-center py-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
            {{ img.filename }}
          </div>
        </div>
      </div>
    </div>

    <!-- Slideshow -->
    <ImageSlideshow
      :images="slideshowImages"
      :start-index="slideshowStartIndex"
      :open="slideshowOpen"
      @close="slideshowOpen = false"
      @reveal-in-finder="revealInFinder"
    />
  </div>
</template>
```

- [ ] **Step 3: Commit**

```bash
git add src/components/images/ImageGalleryView.vue src/components/images/ImageSlideshow.vue
git commit -m "feat(images): add gallery view with size slider and slideshow"
```

---

## Task 9: Sidebar & App.vue Integration

**Files:**
- Modify: `src/types/index.ts` (already done in Task 5 — verify)
- Modify: `src/components/layout/AppSidebar.vue`
- Modify: `src/App.vue`

- [ ] **Step 1: Add image section to AppSidebar.vue**

In `src/components/layout/AppSidebar.vue`, add a new "画像" section between the "ライブラリ" `</div>` (line 79) and "プレイリスト" `<div>` (line 82):

```vue
      <!-- Images section -->
      <div>
        <h3 class="px-2 text-xs font-semibold text-neutral-500 uppercase tracking-wider mb-1">画像</h3>
        <ul class="space-y-0.5">
          <li>
            <button :class="sidebarButtonClass('images-download')"
                    @click="emit('update:currentSection', 'images-download')">
              画像取得
            </button>
          </li>
          <li>
            <button :class="sidebarButtonClass('images-gallery')"
                    @click="emit('update:currentSection', 'images-gallery')">
              ギャラリー
            </button>
          </li>
        </ul>
      </div>
```

- [ ] **Step 2: Add image section routing and labels to App.vue**

In `src/App.vue`:

a) Add imports at the top of `<script setup>`:

```typescript
import ImageDownloadView from '@/components/images/ImageDownloadView.vue'
import ImageGalleryView from '@/components/images/ImageGalleryView.vue'
```

b) Add entries to the `sectionLabel` computed's `labels` Record (this is required because `Record<SidebarSection, string>` demands all keys):

```typescript
'images-download': '画像取得',
'images-gallery': '画像ギャラリー',
```

c) Add routing in the template's content area (after existing section templates):

```vue
<!-- Image download -->
<template v-else-if="currentSection === 'images-download'">
  <ImageDownloadView />
</template>

<!-- Image gallery -->
<template v-else-if="currentSection === 'images-gallery'">
  <ImageGalleryView />
</template>
```

- [ ] **Step 5: Build and verify**

Run: `cd /Volumes/Logitec2/work/YTDown && pnpm build`
Expected: Build succeeds. If type errors occur, fix imports.

- [ ] **Step 6: Commit**

```bash
git add src/components/layout/AppSidebar.vue src/App.vue
git commit -m "feat(images): integrate image views into sidebar navigation"
```

---

## Task 10: Full Build & Manual Testing

**Files:** None (verification only)

- [ ] **Step 1: Full Rust build**

Run: `cd /Volumes/Logitec2/work/YTDown/src-tauri && cargo build`
Expected: Compiles without errors.

- [ ] **Step 2: Full frontend build**

Run: `cd /Volumes/Logitec2/work/YTDown && pnpm build`
Expected: No TypeScript or build errors.

- [ ] **Step 3: Run Rust tests**

Run: `cd /Volumes/Logitec2/work/YTDown/src-tauri && cargo test`
Expected: All tests pass.

- [ ] **Step 4: Launch dev mode for manual testing**

Run: `cd /Volumes/Logitec2/work/YTDown && pnpm tauri dev`

Manual test checklist:
- サイドバーに「画像取得」「ギャラリー」ボタンが表示される
- 「画像取得」画面でURLを入力 → 取得 → サムネイルグリッド表示
- サムネイルのチェック/解除、全選択/全解除が動作する
- フィルタ設定（最小幅/高さ、変換形式）が反映される
- ダウンロード実行 → 進捗バー表示 → 完了
- 「ギャラリー」画面にセッションが表示される
- サイズスライダーでサムネイルサイズが変わる
- 画像クリック → スライドショー表示
- スライドショーのキーボード操作（←→、Space、Escape）
- 自動再生の開始/停止、間隔変更
- セッション削除（ファイルも削除）

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "feat(images): complete image download feature with gallery and slideshow"
```
