use crate::state::AppState;
use crate::ytdlp::{binary, process};
use tauri::State;

#[tauri::command]
pub async fn fetch_formats(
    url: String,
    state: State<'_, AppState>,
) -> Result<crate::ytdlp::parser::VideoInfo, String> {
    let ytdlp_path = state.ytdlp_path.lock().await;
    let path_clone = ytdlp_path.clone();
    drop(ytdlp_path);

    let binary = tokio::task::spawn_blocking(move || binary::detect_binary(path_clone.as_deref()))
        .await
        .map_err(|e| format!("Task error: {}", e))??;

    // Read cookie settings from DB
    let db = state.db.lock().await;
    let cookie_browser = crate::db::queries::get_setting(&db, "cookie_browser")
        .ok()
        .flatten()
        .filter(|v| v != "none" && !v.is_empty());
    let cookie_file = crate::db::queries::get_setting(&db, "cookie_file")
        .ok()
        .flatten()
        .filter(|v| !v.is_empty());
    let auto_detect_media = crate::db::queries::get_setting(&db, "auto_detect_media")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(true);
    let user_agent = crate::db::queries::get_setting(&db, "http_user_agent")
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty());
    drop(db);

    // probeは特殊サイト向け。YouTubeはyt-dlpのネイティブ抽出に任せる
    let resolved_media = if auto_detect_media && !super::download::is_youtube_url(&url) {
        crate::media_probe::resolve_embedded_media(&url, user_agent.as_deref())
            .await
            .ok()
            .flatten()
    } else {
        None
    };
    let info_url = resolved_media
        .as_ref()
        .map(|media| media.media_url.as_str())
        .unwrap_or(&url);
    let referer = resolved_media
        .as_ref()
        .and_then(|media| media.referer.as_deref());

    let result = process::fetch_info(
        &binary.path.to_string_lossy(),
        info_url,
        cookie_browser.as_deref(),
        cookie_file.as_deref(),
        referer,
        user_agent.as_deref(),
        false,
    )
    .await;

    let result = match result {
        Err(err) if process::is_cloudflare_impersonate_error(&err) => {
            process::fetch_info(
                &binary.path.to_string_lossy(),
                info_url,
                cookie_browser.as_deref(),
                cookie_file.as_deref(),
                referer,
                user_agent.as_deref(),
                true,
            )
            .await
        }
        other => other,
    };

    match result {
        // yt-dlp成功時: probeのog:title/og:imageで弱いメタデータ（"master"等）を補完
        Ok(mut info) => {
            if let Some(media) = resolved_media.as_ref() {
                apply_probe_metadata(&mut info, media, &url);
            }
            Ok(info)
        }
        // yt-dlp失敗時: probeがページからメタデータを掴んでいれば、それだけで
        // タイトル/サムネイルを表示してダウンロード可能にする（.htmlページで403になる特殊サイト向け）。
        // 素の直接m3u8（メタなし）は従来どおりエラー→手動ストリーム欄に委ねる
        Err(err) => {
            match resolved_media
                .as_ref()
                .filter(|m| m.title.is_some() || m.thumbnail.is_some())
            {
                Some(media) => Ok(video_info_from_probe(media, &url)),
                None => Err(err),
            }
        }
    }
}

/// probeのメタデータだけから最小限のVideoInfoを組み立てる（yt-dlpが情報取得に失敗した場合の表示用）
fn video_info_from_probe(
    media: &crate::media_probe::MediaProbeResult,
    original_url: &str,
) -> crate::ytdlp::parser::VideoInfo {
    let host = url::Url::parse(original_url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_default();
    let title = media
        .title
        .clone()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| if host.is_empty() { "動画".to_string() } else { host.clone() });

    crate::ytdlp::parser::VideoInfo {
        title,
        channel: String::new(),
        channel_id: None,
        channel_url: None,
        site: host,
        thumbnail_url: media.thumbnail.clone(),
        channel_avatar_url: None,
        duration: None,
        upload_date: None,
        view_count: None,
        chapters: Vec::new(),
        subtitle_languages: Vec::new(),
        auto_subtitle_languages: Vec::new(),
        formats: Vec::new(),
    }
}

/// probeのタイトル/サムネイルを、yt-dlp結果が貧弱なときだけ上書きする
fn apply_probe_metadata(
    info: &mut crate::ytdlp::parser::VideoInfo,
    media: &crate::media_probe::MediaProbeResult,
    original_url: &str,
) {
    if let Some(title) = media.title.as_ref().filter(|t| !t.trim().is_empty()) {
        if is_weak_title(&info.title, original_url) {
            info.title = title.clone();
        }
    }
    if info.thumbnail_url.is_none() {
        if let Some(thumb) = media.thumbnail.as_ref().filter(|t| !t.trim().is_empty()) {
            info.thumbnail_url = Some(thumb.clone());
        }
    }
}

/// yt-dlpのタイトルが「ファイル名そのまま」等の無意味な値かを判定する
fn is_weak_title(title: &str, original_url: &str) -> bool {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return true;
    }
    // m3u8のファイル名由来（master / video / index / playlist 等）は無意味とみなす
    const GENERIC: &[&str] = &["master", "video", "index", "playlist", "manifest", "chunklist"];
    if GENERIC.contains(&trimmed.to_ascii_lowercase().as_str()) {
        return true;
    }
    // URL末尾のファイル名（拡張子なし）と一致するならファイル名流用
    if let Some(stem) = original_url
        .rsplit('/')
        .next()
        .and_then(|seg| seg.split(['.', '?']).next())
    {
        if !stem.is_empty() && stem.eq_ignore_ascii_case(trimmed) {
            return true;
        }
    }
    false
}

#[tauri::command]
pub async fn fetch_channel_info(
    url: String,
    state: State<'_, AppState>,
) -> Result<crate::ytdlp::parser::VideoInfo, String> {
    let ytdlp_path = state.ytdlp_path.lock().await;
    let path_clone = ytdlp_path.clone();
    drop(ytdlp_path);

    let binary = tokio::task::spawn_blocking(move || binary::detect_binary(path_clone.as_deref()))
        .await
        .map_err(|e| format!("Task error: {}", e))??;

    let db = state.db.lock().await;
    let cookie_browser = crate::db::queries::get_setting(&db, "cookie_browser")
        .ok()
        .flatten()
        .filter(|v| v != "none" && !v.is_empty());
    let cookie_file = crate::db::queries::get_setting(&db, "cookie_file")
        .ok()
        .flatten()
        .filter(|v| !v.is_empty());
    drop(db);

    let mut args = vec!["-J", "--flat-playlist", "--playlist-items", "1"];

    let browser_owned;
    let file_owned;
    if let Some(browser) = cookie_browser.as_deref() {
        args.push("--cookies-from-browser");
        browser_owned = browser.to_string();
        args.push(&browser_owned);
    }
    if let Some(file) = cookie_file.as_deref() {
        args.push("--cookies");
        file_owned = file.to_string();
        args.push(&file_owned);
    }
    args.push(&url);

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::process::Command::new(&binary.path)
            .args(&args)
            .env("PATH", process::augmented_path_env())
            .output(),
    )
    .await
    .map_err(|_| "Youtube情報の取得がタイムアウトしました。".to_string())?
    .map_err(|e| format!("Failed to execute yt-dlp: {}", e))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("yt-dlp error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&result.stdout);
    crate::ytdlp::parser::parse_video_info(&stdout)
}
