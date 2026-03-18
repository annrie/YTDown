use std::fs;
use std::process::Command;
use tauri::State;
use crate::state::AppState;

#[tauri::command]
pub async fn move_file(
    source: String,
    destination: String,
    download_id: Option<i64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Create destination directory if needed
    if let Some(parent) = std::path::Path::new(&destination).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create destination dir: {}", e))?;
    }

    // fs::rename fails across different volumes; fall back to copy + delete
    if let Err(_) = fs::rename(&source, &destination) {
        fs::copy(&source, &destination)
            .map_err(|e| format!("Failed to copy file: {}", e))?;
        fs::remove_file(&source)
            .map_err(|e| format!("Copied but failed to remove original: {}", e))?;
    }

    // Update DB file_path if download_id provided
    if let Some(id) = download_id {
        let db = state.db.lock().await;
        let _ = db.execute(
            "UPDATE downloads SET file_path = ?1 WHERE id = ?2",
            rusqlite::params![destination, id],
        );
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_file(
    path: Option<String>,
    to_trash: bool,
    download_id: Option<i64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Delete the physical file if path is provided
    if let Some(ref path) = path {
        if std::path::Path::new(path).exists() {
            if to_trash {
                Command::new("osascript")
                    .args([
                        "-e",
                        &format!(
                            "tell application \"Finder\" to delete POSIX file \"{}\"",
                            path
                        ),
                    ])
                    .output()
                    .map_err(|e| format!("Failed to trash file: {}", e))?;
            } else {
                fs::remove_file(path).map_err(|e| format!("Failed to delete file: {}", e))?;
            }
        }
    }

    // Delete the DB record
    if let Some(id) = download_id {
        let db = state.db.lock().await;
        db.execute(
            "DELETE FROM downloads WHERE id = ?1",
            rusqlite::params![id],
        ).map_err(|e| format!("Failed to delete DB record: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn reveal_in_finder(path: String) -> Result<(), String> {
    Command::new("open")
        .args(["-R", &path])
        .spawn()
        .map_err(|e| format!("Failed to reveal in Finder: {}", e))?;
    Ok(())
}
