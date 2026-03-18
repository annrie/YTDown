use crate::ytdlp::{binary, process};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn fetch_formats(
    url: String,
    state: State<'_, AppState>,
) -> Result<crate::ytdlp::parser::VideoInfo, String> {
    let ytdlp_path = state.ytdlp_path.lock().await;
    let path_clone = ytdlp_path.clone();
    drop(ytdlp_path);

    let binary = tokio::task::spawn_blocking(move || {
        binary::detect_binary(path_clone.as_deref())
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
    ?;

    // Read cookie settings from DB
    let db = state.db.lock().await;
    let cookie_browser = crate::db::queries::get_setting(&db, "cookie_browser")
        .ok().flatten().filter(|v| v != "none" && !v.is_empty());
    let cookie_file = crate::db::queries::get_setting(&db, "cookie_file")
        .ok().flatten().filter(|v| !v.is_empty());
    drop(db);

    process::fetch_info(
        &binary.path.to_string_lossy(), &url,
        cookie_browser.as_deref(), cookie_file.as_deref(),
    ).await
}
