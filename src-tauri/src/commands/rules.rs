use crate::db::{models::AutoClassifyRule, queries};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn list_rules(state: State<'_, AppState>) -> Result<Vec<AutoClassifyRule>, String> {
    let rules = {
        let db = state.db.lock().await;
        queries::list_rules(&db).map_err(|e| e.to_string())?
    };
    Ok(rules)
}

#[tauri::command]
pub async fn create_rule(
    rule_type: String,
    pattern: String,
    target_dir: String,
    priority: i64,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let id = {
        let db = state.db.lock().await;
        queries::create_rule(&db, &rule_type, &pattern, &target_dir, priority, enabled)
            .map_err(|e| e.to_string())?
    };
    Ok(id)
}

#[tauri::command]
pub async fn update_rule(
    id: i64,
    rule_type: String,
    pattern: String,
    target_dir: String,
    priority: i64,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let db = state.db.lock().await;
        queries::update_rule(&db, id, &rule_type, &pattern, &target_dir, priority, enabled)
            .map_err(|e| e.to_string())?
    };
    Ok(())
}

#[tauri::command]
pub async fn delete_rule(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    {
        let db = state.db.lock().await;
        queries::delete_rule(&db, id).map_err(|e| e.to_string())?
    };
    Ok(())
}
