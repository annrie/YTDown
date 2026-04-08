use crate::db::{models::{AutoPresetRule, Preset}, queries};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn list_auto_preset_rules(
    state: State<'_, AppState>,
) -> Result<Vec<AutoPresetRule>, String> {
    let conn = state.db.lock().await;
    queries::list_auto_preset_rules(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_auto_preset_rule(
    domain: String,
    preset_id: i64,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let conn = state.db.lock().await;
    queries::insert_auto_preset_rule(&conn, &domain, preset_id, enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_auto_preset_rule(
    id: i64,
    domain: String,
    preset_id: i64,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let conn = state.db.lock().await;
    queries::update_auto_preset_rule(&conn, id, &domain, preset_id, enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_auto_preset_rule(
    id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let conn = state.db.lock().await;
    queries::delete_auto_preset_rule(&conn, id).map_err(|e| e.to_string())
}

/// URL のドメインに一致する有効なプリセットを返す。なければ None。
#[tauri::command]
pub async fn resolve_preset_for_url(
    url: String,
    state: State<'_, AppState>,
) -> Result<Option<Preset>, String> {
    let domain = extract_domain(&url);
    let Some(domain) = domain else {
        return Ok(None);
    };
    let conn = state.db.lock().await;
    queries::find_preset_for_domain(&conn, &domain).map_err(|e| e.to_string())
}

fn extract_domain(url: &str) -> Option<String> {
    // Simple domain extraction: strip scheme and path
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let host = without_scheme.split('/').next()?;
    // Strip port and www. prefix
    let host = host.split(':').next().unwrap_or(host);
    let host = host.strip_prefix("www.").unwrap_or(host);
    if host.is_empty() { None } else { Some(host.to_lowercase()) }
}
