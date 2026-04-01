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
    drop(db);

    process::fetch_info(
        &binary.path.to_string_lossy(),
        &url,
        cookie_browser.as_deref(),
        cookie_file.as_deref(),
    )
    .await
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
