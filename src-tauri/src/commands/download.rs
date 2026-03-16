use tauri::{AppHandle, State};
use serde::Deserialize;
use crate::state::{ActiveDownload, AppState};
use crate::ytdlp::{binary, process::DownloadConfig};

#[derive(Deserialize)]
pub struct DownloadOptions {
    pub format: String,
    pub quality: String,
    pub output_dir: String,
    pub embed_thumbnail: bool,
    pub embed_metadata: bool,
    pub write_subs: bool,
    pub embed_subs: bool,
    pub embed_chapters: bool,
    pub sponsorblock: bool,
    pub custom_format: Option<String>,
}

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    url: String,
    options: DownloadOptions,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let ytdlp_path_lock = state.ytdlp_path.lock().await;
    let bin = binary::detect_binary(ytdlp_path_lock.as_deref())?;

    // Insert download record to DB
    let db = state.db.lock().await;
    let download_id = crate::db::queries::insert_download(
        &db, &url, None, None, None, None, None, None,
        Some(&options.format), Some(&options.quality), None,
    ).map_err(|e| format!("DB insert failed: {}", e))?;
    drop(db); // Release lock before spawning async work

    // Expand ~ in output_dir
    let output_dir = if options.output_dir.starts_with("~/") {
        let home = dirs::home_dir().unwrap_or_default();
        home.join(&options.output_dir[2..]).to_string_lossy().to_string()
    } else {
        options.output_dir
    };
    // Ensure output directory exists
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    let config = DownloadConfig {
        ytdlp_path: bin.path.to_string_lossy().to_string(),
        url,
        format: options.format,
        quality: options.quality,
        output_dir,
        output_template: "%(title)s.%(ext)s".to_string(),
        embed_thumbnail: options.embed_thumbnail,
        embed_metadata: options.embed_metadata,
        write_subs: options.write_subs,
        embed_subs: options.embed_subs,
        embed_chapters: options.embed_chapters,
        sponsorblock: options.sponsorblock,
        custom_format: options.custom_format,
        cookie_browser: None,
        cookie_file: None,
    };

    let pid = crate::ytdlp::process::start_download(app, download_id, config).await?;

    // Track active download
    let mut downloads = state.active_downloads.lock().await;
    downloads.insert(download_id, ActiveDownload {
        download_id,
        pid,
        paused: false,
    });

    Ok(download_id)
}

#[tauri::command]
pub async fn cancel_download(
    download_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut downloads = state.active_downloads.lock().await;
    if let Some(dl) = downloads.remove(&download_id) {
        let result = unsafe { libc::kill(dl.pid as i32, libc::SIGTERM) };
        if result != 0 {
            // Process may already be dead, that's OK for cancel
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() != Some(libc::ESRCH) {
                return Err(format!("SIGTERM failed: {}", err));
            }
        }
        let db = state.db.lock().await;
        let _ = crate::db::queries::update_download_status(&db, download_id, "cancelled");
        Ok(())
    } else {
        Err("Download not found".to_string())
    }
}

#[tauri::command]
pub async fn pause_download(
    download_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut downloads = state.active_downloads.lock().await;
    if let Some(dl) = downloads.get_mut(&download_id) {
        let result = unsafe { libc::kill(dl.pid as i32, libc::SIGSTOP) };
        if result != 0 {
            return Err(format!("SIGSTOP failed: {}", std::io::Error::last_os_error()));
        }
        dl.paused = true;
        let db = state.db.lock().await;
        let _ = crate::db::queries::update_download_status(&db, download_id, "paused");
        Ok(())
    } else {
        Err("Download not found".to_string())
    }
}

fn is_process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[tauri::command]
pub async fn resume_download(
    app: AppHandle,
    download_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut downloads = state.active_downloads.lock().await;
    if let Some(dl) = downloads.get_mut(&download_id) {
        if dl.paused {
            if is_process_alive(dl.pid) {
                // Process still alive, send SIGCONT
                let result = unsafe { libc::kill(dl.pid as i32, libc::SIGCONT) };
                if result != 0 {
                    return Err(format!("SIGCONT failed: {}", std::io::Error::last_os_error()));
                }
                dl.paused = false;
            } else {
                // Process dead — re-download using --continue (yt-dlp resumes partial files)
                let db = state.db.lock().await;
                let download = crate::db::queries::get_download(&db, download_id)
                    .map_err(|e| format!("DB read failed: {}", e))?;
                drop(db);

                let ytdlp_path_lock = state.ytdlp_path.lock().await;
                let bin = crate::ytdlp::binary::detect_binary(ytdlp_path_lock.as_deref())?;

                let config = DownloadConfig {
                    ytdlp_path: bin.path.to_string_lossy().to_string(),
                    url: download.url.clone(),
                    format: download.format.unwrap_or("best".to_string()),
                    quality: download.quality.unwrap_or("best".to_string()),
                    output_dir: {
                        let db2 = state.db.lock().await;
                        let dir = crate::db::queries::get_setting(&db2, "download_dir")
                            .ok().flatten()
                            .unwrap_or_else(|| "~/Downloads/YTDown/".to_string());
                        drop(db2);
                        if dir.starts_with("~/") {
                            let home = dirs::home_dir().unwrap_or_default();
                            home.join(&dir[2..]).to_string_lossy().to_string()
                        } else {
                            dir
                        }
                    },
                    output_template: "%(title)s.%(ext)s".to_string(),
                    embed_thumbnail: false, embed_metadata: false,
                    write_subs: false, embed_subs: false,
                    embed_chapters: false, sponsorblock: false,
                    custom_format: None, cookie_browser: None, cookie_file: None,
                };

                let new_pid = crate::ytdlp::process::start_download(app, download_id, config).await?;
                dl.pid = new_pid;
                dl.paused = false;

                let db = state.db.lock().await;
                let _ = crate::db::queries::update_download_pid(&db, download_id, Some(new_pid as i64));
                let _ = crate::db::queries::update_download_status(&db, download_id, "downloading");
            }
        }
        Ok(())
    } else {
        Err("Download not found".to_string())
    }
}
