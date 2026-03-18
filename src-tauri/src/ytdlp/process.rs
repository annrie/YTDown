use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tauri::{AppHandle, Emitter, Manager};

use super::parser::{parse_progress_line, parse_video_info, VideoInfo};

pub struct DownloadConfig {
    pub ytdlp_path: String,
    pub url: String,
    pub format: String,
    pub quality: String,
    pub output_dir: String,
    pub output_template: String,
    pub embed_thumbnail: bool,
    pub embed_metadata: bool,
    pub write_subs: bool,
    pub embed_subs: bool,
    pub embed_chapters: bool,
    pub sponsorblock: bool,
    pub custom_format: Option<String>,
    pub cookie_browser: Option<String>,
    pub cookie_file: Option<String>,
    /// "single" = first item only, "all" = entire playlist (unused, kept for compatibility)
    #[allow(dead_code)]
    pub playlist_mode: String,
    // Advanced options
    pub restrict_filenames: bool,
    pub no_overwrites: bool,
    pub geo_bypass: bool,
    pub rate_limit: Option<String>,
    pub sub_lang: Option<String>,
    pub convert_subs: Option<String>,
    pub merge_output_format: Option<String>,
    pub recode_video: Option<String>,
    pub retries: u32,
    pub proxy: Option<String>,
    pub extra_args: Vec<String>,
}

/// Fetch video info (formats, metadata) without downloading
pub async fn fetch_info(
    ytdlp_path: &str,
    url: &str,
    cookie_browser: Option<&str>,
    cookie_file: Option<&str>,
) -> Result<VideoInfo, String> {
    let mut args = vec!["--dump-json", "--no-download", "--no-playlist"];
    let browser_owned;
    let file_owned;
    if let Some(browser) = cookie_browser {
        args.push("--cookies-from-browser");
        browser_owned = browser.to_string();
        args.push(&browser_owned);
    }
    if let Some(file) = cookie_file {
        args.push("--cookies");
        file_owned = file.to_string();
        args.push(&file_owned);
    }
    args.push(url);

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        Command::new(ytdlp_path)
            .args(&args)
            .output(),
    )
    .await
    .map_err(|_| "yt-dlp の情報取得がタイムアウトしました（30秒）".to_string())?
    .map_err(|e| format!("Failed to execute yt-dlp: {}", e))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("yt-dlp error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&result.stdout);
    parse_video_info(&stdout)
}

/// Playlist item info returned by fetch_playlist_items
#[derive(Debug, Clone, serde::Serialize)]
pub struct PlaylistItemInfo {
    pub url: String,
    pub title: Option<String>,
    pub channel: Option<String>,
    pub channel_id: Option<String>,
    pub channel_url: Option<String>,
    pub site: Option<String>,
    pub thumbnail_url: Option<String>,
    pub duration: Option<i64>,
}

/// Fetch all items in a playlist using --flat-playlist --dump-json
/// Falls back to --dump-json --yes-playlist if --flat-playlist fails
pub async fn fetch_playlist_items(
    ytdlp_path: &str,
    url: &str,
    cookie_browser: Option<&str>,
    cookie_file: Option<&str>,
) -> Result<Vec<PlaylistItemInfo>, String> {
    // Build cookie args
    let mut cookie_args: Vec<String> = Vec::new();
    if let Some(browser) = cookie_browser {
        cookie_args.extend(["--cookies-from-browser".to_string(), browser.to_string()]);
    }
    if let Some(file) = cookie_file {
        cookie_args.extend(["--cookies".to_string(), file.to_string()]);
    }

    // Strategy 1: --flat-playlist --dump-json (fast, metadata only)
    let items = try_fetch_playlist(ytdlp_path, url, &cookie_args, true).await;
    if let Ok(ref list) = items {
        if !list.is_empty() {
            return items;
        }
    }

    // Strategy 2: --dump-json --yes-playlist (slower, full info per item)
    let items = try_fetch_playlist(ytdlp_path, url, &cookie_args, false).await;
    if let Ok(ref list) = items {
        if !list.is_empty() {
            return items;
        }
    }

    items
}

async fn try_fetch_playlist(
    ytdlp_path: &str,
    url: &str,
    cookie_args: &[String],
    flat: bool,
) -> Result<Vec<PlaylistItemInfo>, String> {
    let mut args: Vec<String> = if flat {
        vec!["--flat-playlist".to_string(), "--dump-json".to_string(), "--yes-playlist".to_string()]
    } else {
        vec!["--dump-json".to_string(), "--yes-playlist".to_string(), "--no-download".to_string()]
    };
    args.extend(cookie_args.iter().cloned());
    args.push(url.to_string());

    let timeout_secs = if flat { 60 } else { 120 };
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        Command::new(ytdlp_path)
            .args(&args)
            .output(),
    )
    .await
    .map_err(|_| "プレイリスト情報の取得がタイムアウトしました".to_string())?
    .map_err(|e| format!("Failed to execute yt-dlp: {}", e))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("yt-dlp error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&result.stdout);
    Ok(parse_playlist_json(&stdout))
}

fn parse_playlist_json(stdout: &str) -> Vec<PlaylistItemInfo> {
    let mut items = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            let video_url = v["url"].as_str()
                .or_else(|| v["webpage_url"].as_str())
                .map(|s| s.to_string());
            if let Some(url) = video_url {
                items.push(PlaylistItemInfo {
                    url,
                    title: v["title"].as_str().map(|s| s.to_string()),
                    channel: v["channel"].as_str().or(v["uploader"].as_str()).map(|s| s.to_string()),
                    channel_id: v["channel_id"].as_str().map(|s| s.to_string()),
                    channel_url: v["channel_url"].as_str().map(|s| s.to_string()),
                    site: v["ie_key"].as_str().or(v["extractor_key"].as_str()).map(|s| s.to_string()),
                    thumbnail_url: v["thumbnail"].as_str().or(v["thumbnails"].as_array()
                        .and_then(|a| a.last())
                        .and_then(|t| t["url"].as_str())).map(|s| s.to_string()),
                    duration: v["duration"].as_f64().map(|d| d as i64),
                });
            }
        }
    }
    items
}

/// Start a download, emitting progress events
pub async fn start_download(
    app: AppHandle,
    download_id: i64,
    config: DownloadConfig,
) -> Result<u32, String> {
    let mut args = vec![
        "--newline".to_string(),
        "--progress".to_string(),
        "--no-playlist".to_string(),
        "--print".to_string(),
        "before_dl:YTDOWN_TITLE:%(title)s".to_string(),
    ];

    // Format selection
    if let Some(ref custom) = config.custom_format {
        args.extend(["-f".to_string(), custom.clone()]);
    } else {
        let format_str = build_format_string(&config.format, &config.quality);
        args.extend(["-f".to_string(), format_str]);
    }

    // Output
    let output_path = format!("{}/{}", config.output_dir, config.output_template);
    args.extend(["-o".to_string(), output_path]);

    // Post-process options
    if config.embed_thumbnail { args.push("--embed-thumbnail".to_string()); }
    if config.embed_metadata { args.push("--embed-metadata".to_string()); }
    if config.write_subs { args.extend(["--write-subs".to_string(), "--write-auto-subs".to_string()]); }
    if config.embed_subs { args.push("--embed-subs".to_string()); }
    if config.embed_chapters { args.push("--embed-chapters".to_string()); }
    if config.sponsorblock { args.push("--sponsorblock-remove".to_string()); }

    // Cookies
    if let Some(ref browser) = config.cookie_browser {
        if browser != "none" {
            args.extend(["--cookies-from-browser".to_string(), browser.clone()]);
        }
    }
    if let Some(ref cookie_file) = config.cookie_file {
        if !cookie_file.is_empty() {
            args.extend(["--cookies".to_string(), cookie_file.clone()]);
        }
    }

    // Advanced options
    if config.restrict_filenames { args.push("--restrict-filenames".to_string()); }
    if config.no_overwrites { args.push("--no-overwrites".to_string()); }
    if config.geo_bypass { args.push("--geo-bypass".to_string()); }
    if let Some(ref limit) = config.rate_limit {
        args.extend(["-r".to_string(), limit.clone()]);
    }
    if let Some(ref lang) = config.sub_lang {
        args.extend(["--sub-lang".to_string(), lang.clone()]);
    }
    if let Some(ref fmt) = config.convert_subs {
        args.extend(["--convert-subs".to_string(), fmt.clone()]);
    }
    if let Some(ref fmt) = config.merge_output_format {
        args.extend(["--merge-output-format".to_string(), fmt.clone()]);
    }
    if let Some(ref fmt) = config.recode_video {
        args.extend(["--recode-video".to_string(), fmt.clone()]);
    }
    if config.retries != 10 {
        args.extend(["--retries".to_string(), config.retries.to_string()]);
    }
    if let Some(ref proxy) = config.proxy {
        args.extend(["--proxy".to_string(), proxy.clone()]);
    }
    // Extra custom args
    args.extend(config.extra_args.iter().cloned());

    args.push(config.url);

    let mut child = Command::new(&config.ytdlp_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn yt-dlp: {}", e))?;

    let pid = child.id().ok_or("Failed to get PID")?;

    // Stream stdout and stderr in parallel for progress
    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
    let app_clone = app.clone();
    let output_dir = config.output_dir.clone();

    // Collect stderr file paths in a shared container
    let stderr_file_path = std::sync::Arc::new(tokio::sync::Mutex::new(None::<String>));
    let stderr_path_clone = stderr_file_path.clone();

    // Read stderr concurrently
    let stderr_task = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(path) = extract_file_path(&line) {
                *stderr_path_clone.lock().await = Some(path);
            }
        }
    });

    tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        let mut final_file_path: Option<String> = None;
        let mut current_title: Option<String> = None;
        let mut title_saved = false;

        while let Ok(Some(line)) = lines.next_line().await {
            // Extract title from --print "before_dl:YTDOWN_TITLE:%(title)s"
            if let Some(title) = line.strip_prefix("YTDOWN_TITLE:") {
                let title = title.trim();
                if !title.is_empty() && title != "NA" {
                    current_title = Some(title.to_string());
                    // Save title to DB immediately (once)
                    if !title_saved {
                        if let Some(state) = app_clone.try_state::<crate::state::AppState>() {
                            if let Ok(db) = state.db.try_lock() {
                                let _ = crate::db::queries::update_download_title(&db, download_id, title);
                            }
                        }
                        title_saved = true;
                    }
                    // Emit title update event immediately
                    let event = serde_json::json!({
                        "download_id": download_id,
                        "percent": 0.0,
                        "speed_bps": 0,
                        "speed_str": "",
                        "eta_secs": 0,
                        "eta_str": "",
                        "downloaded_bytes": 0,
                        "total_bytes": null,
                        "status": "downloading",
                        "title": title,
                    });
                    let _ = app_clone.emit("download-progress", event);
                }
            }

            // Detect file path from Destination/Merger lines
            if let Some(path) = extract_file_path(&line) {
                if current_title.is_none() {
                    if let Some(filename) = std::path::Path::new(&path).file_stem() {
                        current_title = Some(filename.to_string_lossy().to_string());
                    }
                }
                final_file_path = Some(path);
            }

            if let Some(progress) = parse_progress_line(&line) {
                let mut event = serde_json::json!({
                    "download_id": download_id,
                    "percent": progress.percent,
                    "speed_bps": progress.speed_bps,
                    "speed_str": progress.speed_str,
                    "eta_secs": progress.eta_secs,
                    "eta_str": progress.eta_str,
                    "downloaded_bytes": progress.downloaded_bytes,
                    "total_bytes": progress.total_bytes,
                    "status": "downloading",
                });
                if let Some(ref t) = current_title {
                    event["title"] = serde_json::json!(t);
                }
                let _ = app_clone.emit("download-progress", event);
            }
        }

        // Wait for stderr task to finish
        let _ = stderr_task.await;
        if final_file_path.is_none() {
            final_file_path = stderr_file_path.lock().await.clone();
        }

        // Wait for the child process to finish
        if let Ok(status) = child.wait().await {
            let final_status = if status.success() { "completed" } else { "error" };
            let event = serde_json::json!({
                "download_id": download_id,
                "percent": if status.success() { 100.0 } else { 0.0 },
                "status": final_status,
            });
            let _ = app_clone.emit("download-progress", event);

            // Update DB status and file_path
            if let Some(state) = app_clone.try_state::<crate::state::AppState>() {
                if let Ok(db) = state.db.try_lock() {
                    let _ = crate::db::queries::update_download_status(&db, download_id, final_status);

                    // Save file_path if detected and download succeeded
                    if status.success() {
                        if let Some(ref path) = final_file_path {
                            let file_size = std::fs::metadata(path).ok().map(|m| m.len() as i64);
                            let _ = crate::db::queries::update_download_file_path(&db, download_id, path, file_size);
                        } else {
                            // Fallback: try to find the file in output_dir
                            if let Some(path) = find_latest_file(&output_dir) {
                                let file_size = std::fs::metadata(&path).ok().map(|m| m.len() as i64);
                                let _ = crate::db::queries::update_download_file_path(&db, download_id, &path, file_size);
                            }
                        }
                    }
                }
            }
        }
    });

    Ok(pid)
}

/// Extract file path from yt-dlp output lines like:
/// `[Merger] Merging formats into "/path/to/file.mkv"`
/// `[download] Destination: /path/to/file.mp4`
/// `[ExtractAudio] Destination: /path/to/file.mp3`
fn extract_file_path(line: &str) -> Option<String> {
    // [Merger] Merging formats into "path"
    if line.contains("[Merger]") || line.contains("[Muxer]") {
        if let Some(start) = line.find('"') {
            if let Some(end) = line.rfind('"') {
                if end > start {
                    return Some(line[start + 1..end].to_string());
                }
            }
        }
    }
    // [ExtractAudio] Destination: path
    if line.contains("[ExtractAudio]") && line.contains("Destination:") {
        if let Some(idx) = line.find("Destination:") {
            let path = line[idx + 12..].trim();
            if !path.is_empty() {
                return Some(path.to_string());
            }
        }
    }
    // [download] Destination: path (last resort, may be intermediate)
    if line.contains("[download]") && line.contains("Destination:") {
        if let Some(idx) = line.find("Destination:") {
            let path = line[idx + 12..].trim();
            if !path.is_empty() {
                return Some(path.to_string());
            }
        }
    }
    None
}

/// Fallback: find the most recently modified file in the output directory
fn find_latest_file(dir: &str) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| {
            // Skip partial download files
            let name = e.file_name().to_string_lossy().to_string();
            !name.ends_with(".part") && !name.ends_with(".ytdl")
        })
        .max_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()))
        .map(|e| e.path().to_string_lossy().to_string())
}

fn build_format_string(format: &str, quality: &str) -> String {
    match format {
        "mp3" | "m4a" | "flac" | "wav" | "opus" => {
            "bestaudio/best".to_string()
        }
        _ => {
            let height = match quality {
                "4k" | "2160" => "2160",
                "1080" => "1080",
                "720" => "720",
                "480" => "480",
                _ => return "bestvideo+bestaudio/best".to_string(),
            };
            format!("bestvideo[height<={}]+bestaudio/best", height)
        }
    }
}
