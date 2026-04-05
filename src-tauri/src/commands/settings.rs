use crate::db::{models::Setting, queries};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
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

// ─── Export / Import ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct ScheduleExport {
    name: String,
    url: String,
    cron_expr: String,
    options_json: String,
    is_active: bool,
    is_channel: bool,
}

#[derive(Serialize, Deserialize)]
struct ExportData {
    version: String,
    exported_at: String,
    settings: Vec<Setting>,
    presets: Vec<crate::db::models::Preset>,
    rules: Vec<crate::db::models::AutoClassifyRule>,
    schedules: Vec<ScheduleExport>,
}

#[derive(Serialize)]
pub struct ImportResult {
    pub settings_count: usize,
    pub presets_count: usize,
    pub rules_count: usize,
    pub schedules_count: usize,
}

#[tauri::command]
pub async fn export_settings_to_file(
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().await;

    let settings = queries::get_all_settings(&db).map_err(|e| format!("DB error: {}", e))?;
    let presets = queries::list_presets(&db).map_err(|e| format!("DB error: {}", e))?;
    let rules = queries::list_rules(&db).map_err(|e| format!("DB error: {}", e))?;
    let schedules = queries::list_schedules(&db)
        .map_err(|e| format!("DB error: {}", e))?
        .into_iter()
        .map(|s| ScheduleExport {
            name: s.name,
            url: s.url,
            cron_expr: s.cron_expr,
            options_json: s.options_json,
            is_active: s.is_active,
            is_channel: s.is_channel,
        })
        .collect();

    let data = ExportData {
        version: "1".to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        settings,
        presets,
        rules,
        schedules,
    };

    let json = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("Serialization error: {}", e))?;

    std::fs::write(&path, json).map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn import_settings_from_file(
    path: String,
    state: State<'_, AppState>,
) -> Result<ImportResult, String> {
    let json =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;

    let data: ExportData =
        serde_json::from_str(&json).map_err(|e| format!("Invalid backup file: {}", e))?;

    if data.version != "1" {
        return Err(format!("Unsupported backup version: {}", data.version));
    }

    let db = state.db.lock().await;

    // Apply settings
    for s in &data.settings {
        queries::set_setting(&db, &s.key, &s.value)
            .map_err(|e| format!("Failed to apply setting {}: {}", s.key, e))?;
    }

    // Replace presets — clear existing then insert
    db.execute("DELETE FROM download_presets", [])
        .map_err(|e| format!("Failed to clear presets: {}", e))?;
    for p in &data.presets {
        queries::insert_preset(
            &db,
            &p.name,
            &p.format,
            &p.quality,
            &p.output_dir,
            p.embed_thumbnail,
            p.embed_metadata,
            p.write_subs,
            p.embed_subs,
            p.embed_chapters,
            p.sponsorblock,
        )
        .map_err(|e| format!("Failed to insert preset: {}", e))?;
    }

    // Replace rules
    db.execute("DELETE FROM auto_classify_rules", [])
        .map_err(|e| format!("Failed to clear rules: {}", e))?;
    for r in &data.rules {
        queries::create_rule(&db, &r.rule_type, &r.pattern, &r.target_dir, r.priority, r.enabled)
            .map_err(|e| format!("Failed to insert rule: {}", e))?;
    }

    // Replace schedules
    db.execute("DELETE FROM schedules", [])
        .map_err(|e| format!("Failed to clear schedules: {}", e))?;
    for s in &data.schedules {
        queries::insert_schedule(&db, &s.name, &s.url, &s.cron_expr, &s.options_json, s.is_channel)
            .map_err(|e| format!("Failed to insert schedule: {}", e))?;
    }

    Ok(ImportResult {
        settings_count: data.settings.len(),
        presets_count: data.presets.len(),
        rules_count: data.rules.len(),
        schedules_count: data.schedules.len(),
    })
}
