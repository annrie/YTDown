use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub title: String,
    pub channel: String,
    pub channel_id: Option<String>,
    pub channel_url: Option<String>,
    pub site: String,
    pub thumbnail_url: Option<String>,
    pub duration: Option<i64>,
    pub formats: Vec<FormatInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    pub format_id: String,
    pub ext: String,
    pub resolution: Option<String>,
    pub filesize: Option<i64>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub quality_label: Option<String>,
}

/// Parse `yt-dlp --dump-json` output into VideoInfo
pub fn parse_video_info(json_str: &str) -> Result<VideoInfo, String> {
    let v: Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let formats = v["formats"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|f| FormatInfo {
                    format_id: f["format_id"].as_str().unwrap_or("").to_string(),
                    ext: f["ext"].as_str().unwrap_or("").to_string(),
                    resolution: f["resolution"].as_str().map(|s| s.to_string()),
                    filesize: f["filesize"].as_i64().or_else(|| f["filesize_approx"].as_i64()),
                    vcodec: f["vcodec"].as_str().map(|s| s.to_string()),
                    acodec: f["acodec"].as_str().map(|s| s.to_string()),
                    quality_label: f["format_note"].as_str().map(|s| s.to_string()),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(VideoInfo {
        title: v["title"].as_str().unwrap_or("Unknown").to_string(),
        channel: v["channel"].as_str().or(v["uploader"].as_str()).unwrap_or("Unknown").to_string(),
        channel_id: v["channel_id"].as_str().map(|s| s.to_string()),
        channel_url: v["channel_url"].as_str().map(|s| s.to_string()),
        site: v["extractor_key"].as_str().unwrap_or("Unknown").to_string(),
        thumbnail_url: v["thumbnail"].as_str().map(|s| s.to_string()),
        duration: v["duration"].as_i64(),
        formats,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressUpdate {
    pub percent: f64,
    pub speed_bps: u64,
    pub speed_str: String,
    pub eta_secs: u64,
    pub eta_str: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

/// Parse a yt-dlp progress line like:
/// [download]  45.2% of 100.00MiB at 5.20MiB/s ETA 00:10
pub fn parse_progress_line(line: &str) -> Option<ProgressUpdate> {
    if !line.contains("[download]") || !line.contains('%') {
        return None;
    }

    let percent = line
        .split_whitespace()
        .find(|s| s.ends_with('%'))
        .and_then(|s| s.trim_end_matches('%').parse::<f64>().ok())
        .unwrap_or(0.0);

    let total_bytes = extract_size(line, "of ");
    let speed_bps = extract_speed(line);
    let speed_str = line
        .split("at ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .unwrap_or("0B/s")
        .to_string();

    let eta_str = line
        .split("ETA ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .unwrap_or("--:--")
        .to_string();
    let eta_secs = parse_eta(&eta_str);

    let downloaded_bytes = total_bytes
        .map(|t| (percent / 100.0 * t as f64) as u64)
        .unwrap_or(0);

    Some(ProgressUpdate {
        percent,
        speed_bps,
        speed_str,
        eta_secs,
        eta_str,
        downloaded_bytes,
        total_bytes,
    })
}

fn extract_size(line: &str, prefix: &str) -> Option<u64> {
    line.split(prefix)
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| parse_size_str(s))
}

fn parse_size_str(s: &str) -> Option<u64> {
    let s = s.trim();
    let multiplier = if s.ends_with("GiB") { 1_073_741_824.0 }
        else if s.ends_with("MiB") { 1_048_576.0 }
        else if s.ends_with("KiB") { 1_024.0 }
        else { 1.0 };
    let num_str = s.trim_end_matches(|c: char| c.is_alphabetic());
    num_str.parse::<f64>().ok().map(|n| (n * multiplier) as u64)
}

fn extract_speed(line: &str) -> u64 {
    line.split("at ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| {
            let s = s.trim_end_matches("/s");
            parse_size_str(s)
        })
        .unwrap_or(0)
}

/// Parse playlist item lines like:
/// `[download] Downloading item 3 of 15`
/// `[download] Downloading video 3 of 15`
#[allow(dead_code)]
pub fn parse_playlist_progress(line: &str) -> Option<(u32, u32)> {
    if !line.contains("[download]") || !line.contains(" of ") {
        return None;
    }
    // Match "Downloading item N of M" or "Downloading video N of M"
    let lower = line.to_lowercase();
    if !lower.contains("downloading item") && !lower.contains("downloading video") {
        return None;
    }
    let parts: Vec<&str> = line.split_whitespace().collect();
    // Find the index pattern: ... N of M
    for (i, part) in parts.iter().enumerate() {
        if *part == "of" && i > 0 && i + 1 < parts.len() {
            let current = parts[i - 1].parse::<u32>().ok()?;
            let total = parts[i + 1].parse::<u32>().ok()?;
            return Some((current, total));
        }
    }
    None
}

fn parse_eta(eta: &str) -> u64 {
    let parts: Vec<&str> = eta.split(':').collect();
    match parts.len() {
        2 => {
            let min = parts[0].parse::<u64>().unwrap_or(0);
            let sec = parts[1].parse::<u64>().unwrap_or(0);
            min * 60 + sec
        }
        3 => {
            let hr = parts[0].parse::<u64>().unwrap_or(0);
            let min = parts[1].parse::<u64>().unwrap_or(0);
            let sec = parts[2].parse::<u64>().unwrap_or(0);
            hr * 3600 + min * 60 + sec
        }
        _ => 0,
    }
}
