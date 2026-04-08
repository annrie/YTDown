use crate::db::{models::HistoryEntry, queries};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn list_history(
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<HistoryEntry>, String> {
    let conn = state.db.lock().await;
    queries::list_history(&conn, limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_history_entry(
    id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let conn = state.db.lock().await;
    queries::delete_history_entry(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.db.lock().await;
    queries::clear_history(&conn).map_err(|e| e.to_string())
}
