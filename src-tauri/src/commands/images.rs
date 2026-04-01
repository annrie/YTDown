use crate::images::downloader::{self, DownloadProgress, ImageToDownload};
use crate::images::scraper::{self, ScrapedImage};
use rusqlite::params;
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

use crate::state::AppState;

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
    // Expand ~ in output_dir
    let output_dir = if output_dir.starts_with("~/") {
        let home = dirs::home_dir().unwrap_or_default();
        home.join(&output_dir[2..]).to_string_lossy().to_string()
    } else {
        output_dir
    };
    let output_path = Path::new(&output_dir);

    // Extract site name from URL
    let site_name = url::Url::parse(&session_url)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()));

    // Create session in DB
    let session_id = {
        let db = state.db.lock().await;
        db.execute(
            "INSERT INTO image_sessions (source_url, site_name, image_count, output_dir) VALUES (?1, ?2, ?3, ?4)",
            params![session_url, site_name, 0_i64, output_dir],
        ).map_err(|e| format!("DB insert error: {e}"))?;
        db.last_insert_rowid()
    }; // Lock released here

    let client = downloader::create_download_client()?;
    let total = images.len();
    let format_ref = format.as_deref();

    for (i, img) in images.iter().enumerate() {
        // Emit progress
        let _ = app.emit(
            "image-download-progress",
            DownloadProgress {
                session_id,
                image_index: i,
                total_images: total,
                current_url: img.url.clone(),
                percent: (i as f64 / total as f64) * 100.0,
                status: "downloading".to_string(),
                error_message: None,
            },
        );

        match downloader::download_and_save(&client, img, output_path, format_ref, i).await {
            Ok(result) => {
                let db = state.db.lock().await;
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
                let db = state.db.lock().await;
                db.execute(
                    "INSERT INTO images (session_id, original_url, status) VALUES (?1, ?2, 'failed')",
                    params![session_id, img.url],
                ).map_err(|e| format!("DB insert error: {e}"))?;
                drop(db);

                let _ = app.emit(
                    "image-download-progress",
                    DownloadProgress {
                        session_id,
                        image_index: i,
                        total_images: total,
                        current_url: img.url.clone(),
                        percent: ((i + 1) as f64 / total as f64) * 100.0,
                        status: "failed".to_string(),
                        error_message: Some(err),
                    },
                );
            }
        }
    }

    // Update session count
    {
        let db = state.db.lock().await;
        let completed_count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM images WHERE session_id = ?1 AND status = 'completed'",
                params![session_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        db.execute(
            "UPDATE image_sessions SET image_count = ?1 WHERE id = ?2",
            params![completed_count, session_id],
        )
        .map_err(|e| format!("DB update error: {e}"))?;
    }

    // Final progress
    let _ = app.emit(
        "image-download-progress",
        DownloadProgress {
            session_id,
            image_index: total,
            total_images: total,
            current_url: String::new(),
            percent: 100.0,
            status: "completed".to_string(),
            error_message: None,
        },
    );

    Ok(session_id)
}

#[tauri::command]
pub async fn list_image_sessions(state: State<'_, AppState>) -> Result<Vec<ImageSession>, String> {
    let db = state.db.lock().await;
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
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Row mapping error: {e}"))?;

    Ok(sessions)
}

#[tauri::command]
pub async fn list_session_images(
    session_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<ImageRecord>, String> {
    let db = state.db.lock().await;
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
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Row mapping error: {e}"))?;

    Ok(images)
}

#[tauri::command]
pub async fn delete_image_session(
    session_id: i64,
    delete_files: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if delete_files {
        let paths: Vec<String> = {
            let db = state.db.lock().await;
            let mut stmt = db
                .prepare(
                    "SELECT file_path FROM images WHERE session_id = ?1 AND file_path IS NOT NULL",
                )
                .map_err(|e| format!("Query error: {e}"))?;

            let result: Vec<String> = stmt
                .query_map(params![session_id], |row| row.get(0))
                .map_err(|e| format!("Query error: {e}"))?
                .filter_map(|r| r.ok())
                .collect();
            result
        }; // Lock released, stmt dropped

        for path in paths {
            let _ = tokio::fs::remove_file(&path).await;
        }
    }

    let db = state.db.lock().await;
    db.execute(
        "DELETE FROM images WHERE session_id = ?1",
        params![session_id],
    )
    .map_err(|e| format!("DB delete error: {e}"))?;
    db.execute(
        "DELETE FROM image_sessions WHERE id = ?1",
        params![session_id],
    )
    .map_err(|e| format!("DB delete error: {e}"))?;

    Ok(())
}
