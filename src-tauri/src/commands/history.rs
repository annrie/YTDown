use crate::db::{models::UrlHistoryEntry, queries};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn save_url_history(
    history_type: String,
    url: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().await;
    queries::save_url_history(&db, &history_type, &url).map_err(|e| format!("DB error: {}", e))
}

#[tauri::command]
pub async fn get_url_history(
    history_type: String,
    state: State<'_, AppState>,
) -> Result<Vec<UrlHistoryEntry>, String> {
    let db = state.db.lock().await;
    queries::get_url_history(&db, &history_type).map_err(|e| format!("DB error: {}", e))
}

#[tauri::command]
pub async fn clear_url_history(
    history_type: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().await;
    queries::clear_url_history(&db, &history_type).map_err(|e| format!("DB error: {}", e))
}
