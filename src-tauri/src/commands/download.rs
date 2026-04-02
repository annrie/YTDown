use crate::state::{ActiveDownload, AppState};
use crate::ytdlp::{
    binary,
    process::{self, DownloadConfig},
};
use chrono::Local;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use tauri::{AppHandle, Emitter, Manager, State};

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
        .ok()
        .flatten()
        .filter(|v| v != "none" && !v.is_empty());
    let cookie_file = crate::db::queries::get_setting(&db, "cookie_file")
        .ok()
        .flatten()
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
    /// チャンネル監視: 初回実行時に既存動画をスキップする
    #[serde(default)]
    pub skip_initial: bool,
}

fn default_retries() -> u32 {
    10
}

fn non_empty(s: String) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

fn parse_extra_args(s: &str) -> Vec<String> {
    s.split_whitespace().map(|s| s.to_string()).collect()
}

fn normalize_channel_download_url(url: &str, is_channel: bool) -> String {
    if !is_channel || !is_youtube_url(url) {
        return url.to_string();
    }

    let Ok(mut parsed) = url::Url::parse(url) else {
        return url.to_string();
    };

    let path = parsed.path().trim_end_matches('/');
    let segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let is_channel_root = match segments.as_slice() {
        [segment] if segment.starts_with('@') => true,
        ["channel", _] | ["user", _] | ["c", _] => true,
        _ => false,
    };
    let already_specific_tab = segments.last().is_some_and(|segment| {
        matches!(
            *segment,
            "videos" | "streams" | "shorts" | "playlists" | "featured"
        )
    });

    if is_channel_root && !already_specific_tab {
        parsed.set_path(&format!("{}/videos", path));
        parsed.to_string()
    } else {
        url.to_string()
    }
}

fn schedule_cutoff_date(last_run_at: &Option<String>, skip_initial: bool) -> Option<String> {
    if let Some(last_run) = last_run_at {
        return Some(
            chrono::DateTime::parse_from_rfc3339(last_run)
                .map(|timestamp| timestamp.with_timezone(&Local).format("%Y%m%d").to_string())
                .unwrap_or_else(|_| {
                    last_run
                        .chars()
                        .take(10)
                        .collect::<String>()
                        .replace('-', "")
                }),
        );
    }

    if skip_initial {
        Some(Local::now().format("%Y%m%d").to_string())
    } else {
        None
    }
}

fn read_schedule_archive_ids(archive_path: &std::path::Path) -> HashSet<String> {
    std::fs::read_to_string(archive_path)
        .ok()
        .map(|contents| {
            contents
                .lines()
                .filter_map(|line| line.split_whitespace().nth(1))
                .map(|id| id.to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
struct LatestChannelEntry {
    id: Option<String>,
    upload_date: Option<String>,
    video_url: Option<String>,
}

#[derive(Debug, Clone)]
struct VideoPublishInfo {
    upload_date: Option<String>,
    timestamp: Option<i64>,
}

fn parse_latest_channel_entry(stdout: &str) -> Option<LatestChannelEntry> {
    let value: Value = serde_json::from_str(stdout).ok()?;
    let entry = value
        .get("entries")
        .and_then(|entries| entries.as_array())
        .and_then(|entries| entries.first())
        .unwrap_or(&value);

    Some(LatestChannelEntry {
        id: entry
            .get("id")
            .and_then(|id| id.as_str())
            .map(|id| id.to_string()),
        upload_date: entry
            .get("upload_date")
            .and_then(|upload_date| upload_date.as_str())
            .map(|upload_date| upload_date.to_string()),
        video_url: entry
            .get("webpage_url")
            .and_then(|webpage_url| webpage_url.as_str())
            .or_else(|| entry.get("url").and_then(|url| url.as_str()))
            .map(|url| url.to_string()),
    })
}

fn latest_entry_is_definitely_not_new(
    latest_entry: &LatestChannelEntry,
    cutoff_date: &str,
    archived_ids: &HashSet<String>,
) -> bool {
    if latest_entry
        .id
        .as_ref()
        .is_some_and(|id| archived_ids.contains(id))
    {
        return true;
    }

    latest_entry
        .upload_date
        .as_deref()
        .is_some_and(|upload_date| upload_date < cutoff_date)
}

async fn fetch_latest_channel_entry(
    ytdlp_path: &str,
    channel_url: &str,
    cookie_browser: Option<&str>,
    cookie_file: Option<&str>,
) -> Result<Option<LatestChannelEntry>, String> {
    let mut args = vec![
        "-J".to_string(),
        "--flat-playlist".to_string(),
        "--playlist-items".to_string(),
        "1".to_string(),
    ];

    if let Some(browser) = cookie_browser {
        args.extend(["--cookies-from-browser".to_string(), browser.to_string()]);
    }
    if let Some(file) = cookie_file {
        args.extend(["--cookies".to_string(), file.to_string()]);
    }
    args.push(channel_url.to_string());

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(45),
        tokio::process::Command::new(ytdlp_path)
            .args(&args)
            .env("PATH", process::augmented_path_env())
            .output(),
    )
    .await
    .map_err(|_| "yt-dlp の最新動画確認がタイムアウトしました".to_string())?
    .map_err(|e| format!("Failed to execute yt-dlp: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp error: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_latest_channel_entry(&stdout))
}

fn last_run_timestamp(last_run_at: &Option<String>) -> Option<i64> {
    last_run_at
        .as_deref()
        .and_then(|last_run| chrono::DateTime::parse_from_rfc3339(last_run).ok())
        .map(|last_run| last_run.timestamp())
}

async fn fetch_video_publish_info(
    ytdlp_path: &str,
    video_url: &str,
    cookie_browser: Option<&str>,
    cookie_file: Option<&str>,
) -> Result<VideoPublishInfo, String> {
    let mut args = vec![
        "--dump-single-json".to_string(),
        "--no-playlist".to_string(),
    ];

    if let Some(browser) = cookie_browser {
        args.extend(["--cookies-from-browser".to_string(), browser.to_string()]);
    }
    if let Some(file) = cookie_file {
        args.extend(["--cookies".to_string(), file.to_string()]);
    }
    args.push(video_url.to_string());

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        tokio::process::Command::new(ytdlp_path)
            .args(&args)
            .env("PATH", process::augmented_path_env())
            .output(),
    )
    .await
    .map_err(|_| "yt-dlp の最新動画メタデータ確認がタイムアウトしました（120秒）".to_string())?
    .map_err(|e| format!("Failed to execute yt-dlp: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp error: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse yt-dlp video metadata: {}", e))?;

    Ok(VideoPublishInfo {
        upload_date: value
            .get("upload_date")
            .and_then(|upload_date| upload_date.as_str())
            .map(|upload_date| upload_date.to_string()),
        timestamp: value
            .get("timestamp")
            .and_then(|timestamp| timestamp.as_i64()),
    })
}

async fn find_recent_channel_video_urls(
    ytdlp_path: &str,
    channel_url: &str,
    cutoff_date: &str,
    cookie_browser: Option<&str>,
    cookie_file: Option<&str>,
    archived_ids: &HashSet<String>,
) -> Result<Vec<String>, String> {
    let mut args = vec![
        "--dump-json".to_string(),
        "--no-download".to_string(),
        "--yes-playlist".to_string(),
        "--playlist-end".to_string(),
        "10".to_string(),
    ];

    if let Some(browser) = cookie_browser {
        args.extend(["--cookies-from-browser".to_string(), browser.to_string()]);
    }
    if let Some(file) = cookie_file {
        args.extend(["--cookies".to_string(), file.to_string()]);
    }
    args.push(channel_url.to_string());

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        tokio::process::Command::new(ytdlp_path)
            .args(&args)
            .env("PATH", process::augmented_path_env())
            .output(),
    )
    .await
    .map_err(|_| "yt-dlp のチャンネル確認がタイムアウトしました".to_string())?
    .map_err(|e| format!("Failed to execute yt-dlp: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp error: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut urls = Vec::new();

    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };

        let Some(upload_date) = value["upload_date"].as_str() else {
            continue;
        };
        if upload_date < cutoff_date {
            break;
        }

        let Some(video_id) = value["id"].as_str() else {
            continue;
        };
        if archived_ids.contains(video_id) {
            continue;
        }

        let video_url = value["webpage_url"]
            .as_str()
            .or_else(|| value["url"].as_str())
            .map(|url| url.to_string());
        if let Some(video_url) = video_url {
            urls.push(video_url);
        }
    }

    Ok(urls)
}

fn schedule_archive_path(app: &AppHandle, schedule_id: i64) -> Result<std::path::PathBuf, String> {
    let archive_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("archives");
    std::fs::create_dir_all(&archive_dir)
        .map_err(|e| format!("Failed to create archive dir: {}", e))?;
    Ok(archive_dir.join(format!("schedule-{}.txt", schedule_id)))
}

fn default_playlist_mode() -> String {
    "single".to_string()
}

fn extract_first_output_line(stdout: &[u8]) -> Option<String> {
    String::from_utf8_lossy(stdout)
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
}

fn is_no_new_channel_result(
    status: &std::process::ExitStatus,
    stdout: &[u8],
    stderr: &[u8],
    is_channel: bool,
) -> bool {
    if !is_channel || status.success() || status.code().is_none() {
        return false;
    }

    let stdout = String::from_utf8_lossy(stdout);
    let stderr = String::from_utf8_lossy(stderr);
    let combined = format!("{}\n{}", stdout, stderr).to_lowercase();

    combined.trim().is_empty()
        || combined.contains("no entries")
        || combined.contains("no video uploads")
        || combined.contains("does not pass filter")
        || combined.contains("playlist does not have")
        || combined.contains("there are no videos")
        || combined.contains("the playlist does not have any videos")
        || combined.contains("the playlist does not have any entries")
}

// ── Cross-platform process control ──────────────────────────────────

/// Terminate a process by PID
pub fn kill_process(pid: u32) -> Result<(), String> {
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
            return Err(format!(
                "SIGSTOP failed: {}",
                std::io::Error::last_os_error()
            ));
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
            return Err(format!(
                "SIGCONT failed: {}",
                std::io::Error::last_os_error()
            ));
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
            &db,
            &url,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&options.format),
            Some(&options.quality),
            None,
        )
        .map_err(|e| format!("DB insert failed: {}", e))?
    }; // db lock dropped here

    // Expand ~ in output_dir
    let output_dir = if options.output_dir.starts_with("~/") {
        let home = dirs::home_dir().unwrap_or_default();
        home.join(&options.output_dir[2..])
            .to_string_lossy()
            .to_string()
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
        recode_video: non_empty(options.recode_video),
        retries: options.retries,
        proxy: non_empty(options.proxy),
        extra_args,
    };

    let pid = crate::ytdlp::process::start_download(app, download_id, config).await?;

    // Track active download (scoped lock)
    {
        let mut downloads = state.active_downloads.lock().await;
        downloads.insert(
            download_id,
            ActiveDownload {
                download_id,
                pid,
                paused: false,
            },
        );
    }

    Ok(download_id)
}

#[tauri::command]
pub async fn cancel_download(download_id: i64, state: State<'_, AppState>) -> Result<(), String> {
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
pub async fn pause_download(download_id: i64, state: State<'_, AppState>) -> Result<(), String> {
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
                            .ok()
                            .flatten()
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
                            .ok()
                            .flatten()
                            .map(|v| v == "true")
                            .unwrap_or(true);
                        drop(db_t);
                        v
                    },
                    embed_metadata: {
                        let db_t = state.db.lock().await;
                        let v = crate::db::queries::get_setting(&db_t, "embed_metadata")
                            .ok()
                            .flatten()
                            .map(|v| v == "true")
                            .unwrap_or(true);
                        drop(db_t);
                        v
                    },
                    write_subs: false,
                    embed_subs: false,
                    embed_chapters: false,
                    sponsorblock: false,
                    custom_format: None,
                    cookie_browser: {
                        let db3 = state.db.lock().await;
                        let v = crate::db::queries::get_setting(&db3, "cookie_browser")
                            .ok()
                            .flatten()
                            .filter(|v| v != "none" && !v.is_empty());
                        drop(db3);
                        v
                    },
                    cookie_file: {
                        let db4 = state.db.lock().await;
                        let v = crate::db::queries::get_setting(&db4, "cookie_file")
                            .ok()
                            .flatten()
                            .filter(|v| !v.is_empty());
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
                    recode_video: None,
                    retries: 10,
                    proxy: None,
                    extra_args: Vec::new(),
                };

                let new_pid =
                    crate::ytdlp::process::start_download(app, download_id, config).await?;
                dl.pid = new_pid;
                dl.paused = false;

                let db = state.db.lock().await;
                let _ =
                    crate::db::queries::update_download_pid(&db, download_id, Some(new_pid as i64));
                let _ = crate::db::queries::update_download_status(&db, download_id, "downloading");
            }
        }
        Ok(())
    } else {
        Err("Download not found".to_string())
    }
}

/// Internal download runner for scheduled downloads.
/// Runs yt-dlp to completion without progress tracking.
/// If `is_channel` is true and `last_run_at` is Some, adds --dateafter filter.
/// Returns the output file path on success (first line of yt-dlp --print output)
pub async fn run_download_internal(
    app: &AppHandle,
    url: String,
    options: DownloadOptions,
    is_channel: bool,
    last_run_at: Option<String>,
    schedule_id: Option<i64>,
) -> Result<Option<String>, String> {
    let state = app.state::<AppState>();
    let normalized_url = normalize_channel_download_url(&url, is_channel);

    let bin = {
        let ytdlp_path_lock = state.ytdlp_path.lock().await;
        crate::ytdlp::binary::detect_binary(ytdlp_path_lock.as_deref())?
    };
    let ytdlp_path = bin.path.to_string_lossy().to_string();

    // Expand ~ in output_dir
    let output_dir = if options.output_dir.starts_with("~/") {
        let home = dirs::home_dir().unwrap_or_default();
        home.join(&options.output_dir[2..])
            .to_string_lossy()
            .to_string()
    } else {
        options.output_dir.clone()
    };
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    let (cookie_browser, cookie_file) = get_cookie_settings(&state).await;
    let output_template = output_template_for(&normalized_url);
    let extra_args = parse_extra_args(&options.extra_args);
    let cutoff_date = if is_channel {
        schedule_cutoff_date(&last_run_at, options.skip_initial)
    } else {
        None
    };
    let last_run_ts = last_run_timestamp(&last_run_at);
    let mut target_urls = vec![normalized_url.clone()];

    if is_channel {
        if let Some(ref cutoff_date) = cutoff_date {
            let archived_ids = schedule_id
                .and_then(|sid| schedule_archive_path(app, sid).ok())
                .map(|path| read_schedule_archive_ids(&path))
                .unwrap_or_default();
            let latest_entry = fetch_latest_channel_entry(
                &ytdlp_path,
                &normalized_url,
                cookie_browser.as_deref(),
                cookie_file.as_deref(),
            )
            .await?;

            if latest_entry.as_ref().is_some_and(|entry| {
                latest_entry_is_definitely_not_new(entry, cutoff_date, &archived_ids)
            }) {
                return Ok(None);
            }

            if let Some(entry) = latest_entry.as_ref() {
                if let Some(video_url) = entry.video_url.as_deref() {
                    // Flat-playlist entries often omit upload_date. For initial channel checks,
                    // fetch the newest video's publish metadata before falling back to the
                    // expensive full channel scan.
                    if entry.upload_date.is_none() || last_run_ts.is_some() {
                        let publish_info = fetch_video_publish_info(
                            &ytdlp_path,
                            video_url,
                            cookie_browser.as_deref(),
                            cookie_file.as_deref(),
                        )
                        .await?;

                        if publish_info
                            .upload_date
                            .as_deref()
                            .is_some_and(|upload_date| upload_date < cutoff_date.as_str())
                        {
                            return Ok(None);
                        }

                        if last_run_ts.is_some_and(|known_last_run_ts| {
                            publish_info
                                .timestamp
                                .is_some_and(|video_ts| video_ts <= known_last_run_ts)
                        }) {
                            return Ok(None);
                        }
                    }
                }
            }

            let recent_urls = find_recent_channel_video_urls(
                &ytdlp_path,
                &normalized_url,
                cutoff_date,
                cookie_browser.as_deref(),
                cookie_file.as_deref(),
                &archived_ids,
            )
            .await?;

            if recent_urls.is_empty() {
                return Ok(None);
            }

            target_urls = recent_urls;
        }
    }

    // Build args (no progress/newline flags — just run to completion)
    let mut args: Vec<String> = Vec::new();

    // Format selection
    if let Some(ref custom) = options.custom_format {
        args.extend(["-f".to_string(), custom.clone()]);
    } else {
        let height = match options.quality.as_str() {
            "4k" | "2160" => "2160",
            "1080" => "1080",
            "720" => "720",
            "480" => "480",
            _ => "",
        };
        let format_str = match options.format.as_str() {
            "mp3" | "m4a" | "flac" | "wav" | "opus" => "bestaudio/best".to_string(),
            "mp4" => {
                if height.is_empty() {
                    "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best".to_string()
                } else {
                    format!("bestvideo[ext=mp4][height<={}]+bestaudio[ext=m4a]/bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best", height)
                }
            }
            "webm" => {
                if height.is_empty() {
                    "bestvideo[ext=webm]+bestaudio[ext=webm]/best[ext=webm]/best".to_string()
                } else {
                    format!("bestvideo[ext=webm][height<={}]+bestaudio[ext=webm]/bestvideo[ext=webm]+bestaudio[ext=webm]/best[ext=webm]/best", height)
                }
            }
            _ => {
                if height.is_empty() {
                    "bestvideo+bestaudio/best".to_string()
                } else {
                    format!("bestvideo[height<={}]+bestaudio/best", height)
                }
            }
        };
        args.extend(["-f".to_string(), format_str]);

        match options.format.as_str() {
            "mp3" | "m4a" | "flac" | "wav" | "opus" => {
                args.push("-x".to_string());
                args.extend(["--audio-format".to_string(), options.format.clone()]);
            }
            _ => {}
        }
    }

    // Output path
    let output_path = std::path::PathBuf::from(&output_dir)
        .join(&output_template)
        .to_string_lossy()
        .to_string();
    args.extend(["-o".to_string(), output_path]);

    // Post-process options
    if options.embed_thumbnail {
        args.push("--write-thumbnail".to_string());
        args.push("--convert-thumbnails".to_string());
        args.push("jpg".to_string());
        args.push("--embed-thumbnail".to_string());
    }
    if is_channel {
        args.push("--no-write-playlist-metafiles".to_string());
    }
    if options.embed_metadata {
        args.push("--embed-metadata".to_string());
    }
    if options.write_subs || options.embed_subs {
        args.extend(["--write-subs".to_string(), "--write-auto-subs".to_string()]);
    }
    if options.embed_subs {
        args.push("--embed-subs".to_string());
    }
    if options.embed_chapters {
        args.push("--embed-chapters".to_string());
    }
    if options.sponsorblock {
        args.push("--sponsorblock-remove".to_string());
    }

    // Cookies
    if let Some(ref browser) = cookie_browser {
        if browser != "none" {
            args.extend(["--cookies-from-browser".to_string(), browser.clone()]);
        }
    }
    if let Some(ref file) = cookie_file {
        if !file.is_empty() {
            args.extend(["--cookies".to_string(), file.clone()]);
        }
    }

    // Advanced options
    if options.restrict_filenames {
        args.push("--restrict-filenames".to_string());
    }
    if options.no_overwrites {
        args.push("--no-overwrites".to_string());
    }
    if options.geo_bypass {
        args.push("--geo-bypass".to_string());
    }
    if let Some(ref limit) = non_empty(options.rate_limit.clone()) {
        args.extend(["-r".to_string(), limit.clone()]);
    }
    if let Some(ref lang) = non_empty(options.sub_lang.clone()) {
        args.extend(["--sub-lang".to_string(), lang.clone()]);
    }
    if let Some(ref fmt) = non_empty(options.convert_subs.clone()) {
        args.extend(["--convert-subs".to_string(), fmt.clone()]);
    }
    if let Some(ref fmt) = non_empty(options.merge_output_format.clone()) {
        args.extend(["--merge-output-format".to_string(), fmt.clone()]);
    }
    if let Some(ref fmt) = non_empty(options.recode_video.clone()) {
        args.extend(["--recode-video".to_string(), fmt.clone()]);
    }
    if options.retries != 10 {
        args.extend(["--retries".to_string(), options.retries.to_string()]);
    }
    if let Some(ref proxy) = non_empty(options.proxy.clone()) {
        args.extend(["--proxy".to_string(), proxy.clone()]);
    }
    args.extend(extra_args.iter().cloned());

    // Channel incremental: keep a per-schedule archive to avoid duplicate downloads.
    if is_channel {
        if let Some(sid) = schedule_id {
            let archive_path = schedule_archive_path(app, sid)?;
            args.extend([
                "--download-archive".to_string(),
                archive_path.to_string_lossy().to_string(),
            ]);
        }
    }

    args.extend(target_urls);

    // Capture output file path via --print
    args.extend(["--print".to_string(), "after_move:filepath".to_string()]);

    let child = tokio::process::Command::new(&ytdlp_path)
        .args(&args)
        .env("PATH", process::augmented_path_env())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn yt-dlp: {}", e))?;

    if let Some(sid) = schedule_id {
        let state = app.state::<crate::state::AppState>();
        {
            let db = state.db.lock().await;
            let _ = crate::db::queries::set_schedule_running(&db, sid, true);
        }
        let _ = app.emit("schedule-updated", sid);

        if let Some(pid) = child.id() {
            let mut pids = state.running_schedule_pids.lock().await;
            pids.insert(sid, pid);
        }
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("Failed to wait for yt-dlp: {}", e))?;

    if let Some(sid) = schedule_id {
        let state = app.state::<crate::state::AppState>();
        let mut pids = state.running_schedule_pids.lock().await;
        pids.remove(&sid);
    }

    let file_path = extract_first_output_line(&output.stdout);

    if output.status.success() {
        Ok(file_path)
    } else if is_no_new_channel_result(&output.status, &output.stdout, &output.stderr, is_channel) {
        Ok(None)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let code = output
            .status
            .code()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "signal".to_string());
        let detail = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else if !stdout.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            "(no output)".to_string()
        };
        Err(format!("yt-dlp failed (exit {}): {}", code, detail))
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
        &ytdlp_path,
        &url,
        cookie_browser.as_deref(),
        cookie_file.as_deref(),
    )
    .await?;
    if items.is_empty() {
        return Err("プレイリストにアイテムが見つかりません".to_string());
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::{
        latest_entry_is_definitely_not_new, parse_latest_channel_entry, LatestChannelEntry,
    };
    use std::collections::HashSet;

    #[test]
    fn parses_latest_channel_entry_from_flat_playlist_json() {
        let stdout = r#"{
          "entries": [
            {
              "id": "video123",
              "webpage_url": "https://www.youtube.com/watch?v=video123",
              "upload_date": "20260331",
              "title": "Latest upload"
            }
          ]
        }"#;

        let entry = parse_latest_channel_entry(stdout).expect("entry should parse");

        assert_eq!(entry.id.as_deref(), Some("video123"));
        assert_eq!(entry.upload_date.as_deref(), Some("20260331"));
        assert_eq!(
            entry.video_url.as_deref(),
            Some("https://www.youtube.com/watch?v=video123")
        );
    }

    #[test]
    fn treats_archived_latest_entry_as_not_new() {
        let mut archived_ids = HashSet::new();
        archived_ids.insert("video123".to_string());

        let entry = LatestChannelEntry {
            id: Some("video123".to_string()),
            upload_date: Some("20260401".to_string()),
            video_url: None,
        };

        assert!(latest_entry_is_definitely_not_new(
            &entry,
            "20260331",
            &archived_ids
        ));
    }

    #[test]
    fn treats_older_latest_entry_as_not_new() {
        let entry = LatestChannelEntry {
            id: Some("video123".to_string()),
            upload_date: Some("20260330".to_string()),
            video_url: None,
        };

        assert!(latest_entry_is_definitely_not_new(
            &entry,
            "20260331",
            &HashSet::new()
        ));
    }
}
