use tauri::{AppHandle, State};
use serde::Deserialize;
use crate::state::{ActiveDownload, AppState};
use crate::ytdlp::{binary, process::DownloadConfig};

/// YouTube URL判定
fn is_youtube_url(url: &str) -> bool {
    url.contains("youtube.com/") || url.contains("youtu.be/") || url.contains("youtube.com/shorts/")
}

/// YouTube → チャンネル名フォルダ、その他 → フラット
fn output_template_for(url: &str) -> String {
    if is_youtube_url(url) {
        "%(channel)s/%(title)s.%(ext)s".to_string()
    } else {
        "%(title)s.%(ext)s".to_string()
    }
}

/// Read cookie settings from DB (scoped lock)
async fn get_cookie_settings(state: &AppState) -> (Option<String>, Option<String>) {
    let db = state.db.lock().await;
    let cookie_browser = crate::db::queries::get_setting(&db, "cookie_browser")
        .ok().flatten()
        .filter(|v| v != "none" && !v.is_empty());
    let cookie_file = crate::db::queries::get_setting(&db, "cookie_file")
        .ok().flatten()
        .filter(|v| !v.is_empty());
    (cookie_browser, cookie_file)
}

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
    #[serde(default = "default_playlist_mode")]
    pub playlist_mode: String,
    // Advanced yt-dlp options
    #[serde(default)]
    pub restrict_filenames: bool,
    #[serde(default)]
    pub no_overwrites: bool,
    #[serde(default)]
    pub geo_bypass: bool,
    #[serde(default)]
    pub rate_limit: String,
    #[serde(default)]
    pub sub_lang: String,
    #[serde(default)]
    pub convert_subs: String,
    #[serde(default)]
    pub merge_output_format: String,
    #[serde(default)]
    pub recode_video: String,
    #[serde(default = "default_retries")]
    pub retries: u32,
    #[serde(default)]
    pub proxy: String,
    #[serde(default)]
    pub extra_args: String,
}

fn default_retries() -> u32 { 10 }

fn non_empty(s: String) -> Option<String> {
    if s.trim().is_empty() { None } else { Some(s) }
}

fn parse_extra_args(s: &str) -> Vec<String> {
    s.split_whitespace().map(|s| s.to_string()).collect()
}

fn default_playlist_mode() -> String {
    "single".to_string()
}

// ── Cross-platform process control ──────────────────────────────────

/// Terminate a process by PID
fn kill_process(pid: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        if result != 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() != Some(libc::ESRCH) {
                return Err(format!("SIGTERM failed: {}", err));
            }
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        use std::process::Command;
        Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output()
            .map_err(|e| format!("taskkill failed: {}", e))?;
        Ok(())
    }
}

/// Suspend (pause) a process by PID
fn suspend_process(pid: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid as i32, libc::SIGSTOP) };
        if result != 0 {
            return Err(format!("SIGSTOP failed: {}", std::io::Error::last_os_error()));
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        // Windows does not have SIGSTOP; use undocumented NtSuspendProcess or
        // fall back to a simple error for now (resume will re-download)
        Err("Windows does not support pausing downloads. Cancel and restart instead.".to_string())
    }
}

/// Resume a suspended process by PID
fn resume_process(pid: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid as i32, libc::SIGCONT) };
        if result != 0 {
            return Err(format!("SIGCONT failed: {}", std::io::Error::last_os_error()));
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        let _ = pid;
        Err("Windows does not support resuming suspended processes.".to_string())
    }
}

/// Check if a process is still running
fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(windows)]
    {
        use std::process::Command;
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output()
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.contains(&pid.to_string())
            })
            .unwrap_or(false)
    }
}

// ── Tauri commands ──────────────────────────────────────────────────

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    url: String,
    options: DownloadOptions,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let bin = {
        let ytdlp_path_lock = state.ytdlp_path.lock().await;
        binary::detect_binary(ytdlp_path_lock.as_deref())?
    }; // ytdlp_path_lock dropped here

    // Insert download record to DB
    let download_id = {
        let db = state.db.lock().await;
        crate::db::queries::insert_download(
            &db, &url, None, None, None, None, None, None,
            Some(&options.format), Some(&options.quality), None,
        ).map_err(|e| format!("DB insert failed: {}", e))?
    }; // db lock dropped here

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

    let (cookie_browser, cookie_file) = get_cookie_settings(&state).await;
    let output_template = output_template_for(&url);

    let extra_args = parse_extra_args(&options.extra_args);

    let config = DownloadConfig {
        ytdlp_path: bin.path.to_string_lossy().to_string(),
        url,
        format: options.format,
        quality: options.quality,
        output_dir,
        output_template,
        embed_thumbnail: options.embed_thumbnail,
        embed_metadata: options.embed_metadata,
        write_subs: options.write_subs,
        embed_subs: options.embed_subs,
        embed_chapters: options.embed_chapters,
        sponsorblock: options.sponsorblock,
        custom_format: options.custom_format,
        cookie_browser,
        cookie_file,
        playlist_mode: options.playlist_mode,
        restrict_filenames: options.restrict_filenames,
        no_overwrites: options.no_overwrites,
        geo_bypass: options.geo_bypass,
        rate_limit: non_empty(options.rate_limit),
        sub_lang: non_empty(options.sub_lang),
        convert_subs: non_empty(options.convert_subs),
        merge_output_format: non_empty(options.merge_output_format),
        recode_video: non_empty(options.recode_video),
        retries: options.retries,
        proxy: non_empty(options.proxy),
        extra_args,
    };

    let pid = crate::ytdlp::process::start_download(app, download_id, config).await?;

    // Track active download (scoped lock)
    {
        let mut downloads = state.active_downloads.lock().await;
        downloads.insert(download_id, ActiveDownload {
            download_id,
            pid,
            paused: false,
        });
    }

    Ok(download_id)
}

#[tauri::command]
pub async fn cancel_download(
    download_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut downloads = state.active_downloads.lock().await;
    if let Some(dl) = downloads.remove(&download_id) {
        kill_process(dl.pid)?;
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
        suspend_process(dl.pid)?;
        dl.paused = true;
        let db = state.db.lock().await;
        let _ = crate::db::queries::update_download_status(&db, download_id, "paused");
        Ok(())
    } else {
        Err("Download not found".to_string())
    }
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
                // Process still alive, send resume signal
                resume_process(dl.pid)?;
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
                    output_template: output_template_for(&download.url),
                    embed_thumbnail: {
                        let db_t = state.db.lock().await;
                        let v = crate::db::queries::get_setting(&db_t, "embed_thumbnail")
                            .ok().flatten().map(|v| v == "true").unwrap_or(true);
                        drop(db_t);
                        v
                    },
                    embed_metadata: {
                        let db_t = state.db.lock().await;
                        let v = crate::db::queries::get_setting(&db_t, "embed_metadata")
                            .ok().flatten().map(|v| v == "true").unwrap_or(true);
                        drop(db_t);
                        v
                    },
                    write_subs: false, embed_subs: false,
                    embed_chapters: false, sponsorblock: false,
                    custom_format: None,
                    cookie_browser: {
                        let db3 = state.db.lock().await;
                        let v = crate::db::queries::get_setting(&db3, "cookie_browser")
                            .ok().flatten().filter(|v| v != "none" && !v.is_empty());
                        drop(db3);
                        v
                    },
                    cookie_file: {
                        let db4 = state.db.lock().await;
                        let v = crate::db::queries::get_setting(&db4, "cookie_file")
                            .ok().flatten().filter(|v| !v.is_empty());
                        drop(db4);
                        v
                    },
                    playlist_mode: "single".to_string(),
                    restrict_filenames: false,
                    no_overwrites: false,
                    geo_bypass: false,
                    rate_limit: None,
                    sub_lang: None,
                    convert_subs: None,
                    merge_output_format: None,
                    recode_video: None,
                    retries: 10,
                    proxy: None,
                    extra_args: Vec::new(),
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

/// Fetch playlist items without starting any downloads
#[tauri::command]
pub async fn fetch_playlist_items(
    url: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::ytdlp::process::PlaylistItemInfo>, String> {
    let ytdlp_path_lock = state.ytdlp_path.lock().await;
    let bin = binary::detect_binary(ytdlp_path_lock.as_deref())?;
    let ytdlp_path = bin.path.to_string_lossy().to_string();
    drop(ytdlp_path_lock);

    let (cookie_browser, cookie_file) = get_cookie_settings(&state).await;
    let items = crate::ytdlp::process::fetch_playlist_items(
        &ytdlp_path, &url, cookie_browser.as_deref(), cookie_file.as_deref(),
    ).await?;
    if items.is_empty() {
        return Err("プレイリストにアイテムが見つかりません".to_string());
    }
    Ok(items)
}
