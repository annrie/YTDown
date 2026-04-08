use crate::state::AppState;
use std::fs;
use std::process::Command;
use tauri::State;

fn normalize_url_candidate(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches(|c: char| {
        matches!(c, '"' | '\'' | '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';')
    });
    let trimmed = trimmed.trim_end_matches(['.', ',', ';', ':', ')', ']', '}']);
    let parsed = url::Url::parse(trimmed).ok()?;
    match parsed.scheme() {
        "http" | "https" => Some(parsed.to_string()),
        _ => None,
    }
}

fn extract_urls_from_text(text: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for token in text.split_whitespace() {
        for prefix in ["https://", "http://"] {
            let Some(index) = token.find(prefix) else {
                continue;
            };
            if let Some(url) = normalize_url_candidate(&token[index..]) {
                if seen.insert(url.clone()) {
                    urls.push(url);
                }
            }
        }
    }
    urls
}

fn read_text_via_command(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn extract_urls_from_path(path: &str) -> Vec<String> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() && metadata.len() <= 2 * 1024 * 1024 => metadata,
        _ => return Vec::new(),
    };

    let mut candidates = Vec::new();
    if let Ok(bytes) = fs::read(path) {
        candidates.push(String::from_utf8_lossy(&bytes).into_owned());
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(text) = read_text_via_command("textutil", &["-convert", "txt", "-stdout", path]) {
            candidates.push(text);
        }
        if let Some(text) =
            read_text_via_command("plutil", &["-convert", "xml1", "-o", "-", "--", path])
        {
            candidates.push(text);
        }
    }

    let mut urls = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for text in candidates {
        for url in extract_urls_from_text(&text) {
            if seen.insert(url.clone()) {
                urls.push(url);
            }
        }
    }

    let _ = metadata;
    urls
}

#[tauri::command]
pub async fn read_text_file(path: String) -> Result<String, String> {
    let metadata = fs::metadata(&path).map_err(|e| format!("Failed to stat file: {}", e))?;
    if !metadata.is_file() {
        return Err("Not a file".to_string());
    }
    if metadata.len() > 512 * 1024 {
        return Err("File too large".to_string());
    }

    let bytes = fs::read(&path).map_err(|e| format!("Failed to read file: {}", e))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[tauri::command]
pub async fn write_text_file(path: String, contents: String) -> Result<(), String> {
    if let Some(parent) = std::path::Path::new(&path).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    fs::write(&path, contents.as_bytes())
        .map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn extract_urls_from_paths(paths: Vec<String>) -> Result<Vec<String>, String> {
    let mut urls = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for path in paths {
        for url in extract_urls_from_path(&path) {
            if seen.insert(url.clone()) {
                urls.push(url);
            }
        }
    }
    Ok(urls)
}

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
        fs::copy(&source, &destination).map_err(|e| format!("Failed to copy file: {}", e))?;
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

/// Move file to trash (platform-specific)
fn move_to_trash(path: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
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
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        // Use PowerShell to move to recycle bin
        let ps_script = format!(
            "Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile('{}', 'OnlyErrorDialogs', 'SendToRecycleBin')",
            path.replace("'", "''")
        );
        Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .output()
            .map_err(|e| format!("Failed to trash file: {}", e))?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        // Try gio trash first, fall back to gvfs-trash
        let result = Command::new("gio").args(["trash", path]).output();
        match result {
            Ok(output) if output.status.success() => Ok(()),
            _ => {
                Command::new("gvfs-trash")
                    .arg(path)
                    .output()
                    .map_err(|e| format!("Failed to trash file: {}", e))?;
                Ok(())
            }
        }
    }
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
                move_to_trash(path)?;
            } else {
                fs::remove_file(path).map_err(|e| format!("Failed to delete file: {}", e))?;
            }
        }
    }

    // Delete the DB record
    if let Some(id) = download_id {
        let db = state.db.lock().await;
        db.execute("DELETE FROM downloads WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| format!("Failed to delete DB record: {}", e))?;
    }

    Ok(())
}

/// Reveal file in the native file manager
#[tauri::command]
pub async fn reveal_in_finder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| format!("Failed to reveal in Finder: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args([&format!("/select,{}", path)])
            .spawn()
            .map_err(|e| format!("Failed to reveal in Explorer: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        // Open the parent directory
        let parent = std::path::Path::new(&path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        Command::new("xdg-open")
            .arg(&parent)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
    }
    Ok(())
}
