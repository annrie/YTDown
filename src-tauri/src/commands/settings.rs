use crate::db::{models::Setting, queries};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn set_ytdlp_path(path: String, state: State<'_, AppState>) -> Result<(), String> {
    if path != "auto" && !std::path::Path::new(&path).exists() {
        return Err(format!("Path not found: {}", path));
    }
    let mut ytdlp_path = state.ytdlp_path.lock().await;
    *ytdlp_path = if path == "auto" { None } else { Some(path) };
    Ok(())
}

#[tauri::command]
pub async fn get_all_settings(state: State<'_, AppState>) -> Result<Vec<Setting>, String> {
    let db = state.db.lock().await;
    queries::get_all_settings(&db).map_err(|e| format!("DB error: {}", e))
}

#[tauri::command]
pub async fn get_setting(
    key: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let db = state.db.lock().await;
    queries::get_setting(&db, &key).map_err(|e| format!("DB error: {}", e))
}

#[tauri::command]
pub async fn set_setting(
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().await;
    queries::set_setting(&db, &key, &value).map_err(|e| format!("DB error: {}", e))
}
