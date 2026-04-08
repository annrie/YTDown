use crate::db::{models::Download, queries};
use crate::state::AppState;
use tauri::State;

fn expand_home_dir(path: &str) -> std::path::PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        return dirs::home_dir().unwrap_or_default().join(stripped);
    }
    std::path::PathBuf::from(path)
}

fn output_dir_from_options_json(options_json: &str) -> Option<std::path::PathBuf> {
    let value: serde_json::Value = serde_json::from_str(options_json).ok()?;
    let output_dir = value.get("output_dir")?.as_str()?;
    Some(expand_home_dir(output_dir))
}

fn channel_id_from_options_json(options_json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(options_json).ok()?;
    value
        .get("channel_id")
        .and_then(|channel_id| channel_id.as_str())
        .filter(|channel_id| !channel_id.is_empty())
        .map(|channel_id| channel_id.to_string())
}

fn known_media_extension(path: &std::path::Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let known = [
        "mp4", "mkv", "webm", "mov", "avi", "mp3", "m4a", "flac", "wav", "opus",
    ];
    known.contains(&ext.as_str()).then_some(ext)
}

fn collect_media_files(root: &std::path::Path, max_depth: usize) -> Vec<std::path::PathBuf> {
    fn visit(
        current: &std::path::Path,
        depth: usize,
        max_depth: usize,
        files: &mut Vec<std::path::PathBuf>,
    ) {
        let Ok(entries) = std::fs::read_dir(current) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            // Skip hidden files/directories (e.g. .DS_Store)
            if path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.')) {
                continue;
            }
            if path.is_dir() {
                if depth < max_depth {
                    visit(&path, depth + 1, max_depth, files);
                }
                continue;
            }

            if known_media_extension(&path).is_some() {
                files.push(path);
            }
        }
    }

    let mut files = Vec::new();
    if root.exists() {
        visit(root, 0, max_depth, &mut files);
    }
    files
}

fn infer_site(url: &str) -> Option<&'static str> {
    if url.contains("youtube.com/") || url.contains("youtu.be/") {
        Some("YouTube")
    } else {
        None
    }
}

fn file_timestamp_iso(path: &std::path::Path) -> String {
    let ts = std::fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .map(chrono::DateTime::<chrono::Utc>::from)
        .unwrap_or_else(chrono::Utc::now);
    ts.to_rfc3339()
}

fn reconcile_missing_schedule_downloads(conn: &rusqlite::Connection) -> Result<(), String> {
    let schedules = queries::list_schedules(conn).map_err(|e| format!("DB error: {}", e))?;

    for schedule in schedules {
        let Some(output_dir) = output_dir_from_options_json(&schedule.options_json) else {
            continue;
        };

        let channel_id = channel_id_from_options_json(&schedule.options_json);
        let quality = serde_json::from_str::<serde_json::Value>(&schedule.options_json)
            .ok()
            .and_then(|value| {
                value
                    .get("quality")
                    .and_then(|quality| quality.as_str())
                    .map(str::to_string)
            });

        for file_path in collect_media_files(&output_dir, 3) {
            let file_path_str = file_path.to_string_lossy().to_string();
            if queries::has_download_for_file_path(conn, &file_path_str)
                .map_err(|e| format!("DB error: {}", e))?
            {
                continue;
            }

            let title = file_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_string);
            let file_size = std::fs::metadata(&file_path)
                .ok()
                .map(|meta| meta.len() as i64);
            let timestamp = file_timestamp_iso(&file_path);
            let format = known_media_extension(&file_path);

            queries::insert_completed_download(
                conn,
                &schedule.url,
                title.as_deref(),
                Some(schedule.name.as_str()),
                channel_id.as_deref(),
                Some(schedule.url.as_str()),
                infer_site(&schedule.url),
                None,
                format.as_deref(),
                quality.as_deref(),
                None,
                &file_path_str,
                file_size,
                &timestamp,
                &timestamp,
            )
            .map_err(|e| format!("DB error: {}", e))?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn list_library(
    status_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Download>, String> {
    let db = state.db.lock().await;
    reconcile_missing_schedule_downloads(&db)?;
    queries::list_downloads(&db, status_filter.as_deref()).map_err(|e| format!("DB error: {}", e))
}

#[tauri::command]
pub async fn search_library(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<Download>, String> {
    let db = state.db.lock().await;
    queries::search_downloads(&db, &query).map_err(|e| format!("Search error: {}", e))
}

#[tauri::command]
pub async fn toggle_favorite(id: i64, state: State<'_, AppState>) -> Result<bool, String> {
    let db = state.db.lock().await;
    queries::toggle_favorite(&db, id).map_err(|e| format!("DB error: {}", e))
}

#[tauri::command]
pub async fn get_download(id: i64, state: State<'_, AppState>) -> Result<Download, String> {
    let db = state.db.lock().await;
    queries::get_download(&db, id).map_err(|e| format!("DB error: {}", e))
}
