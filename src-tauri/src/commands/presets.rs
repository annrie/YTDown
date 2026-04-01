use crate::db::{models::Preset, queries};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn list_presets(state: State<'_, AppState>) -> Result<Vec<Preset>, String> {
    let presets = {
        let db = state.db.lock().await;
        queries::list_presets(&db).map_err(|e| e.to_string())?
    };
    Ok(presets)
}

#[tauri::command]
pub async fn create_preset(
    name: String,
    format: String,
    quality: String,
    output_dir: String,
    embed_thumbnail: bool,
    embed_metadata: bool,
    write_subs: bool,
    embed_subs: bool,
    embed_chapters: bool,
    sponsorblock: bool,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let id = {
        let db = state.db.lock().await;
        queries::insert_preset(
            &db,
            &name,
            &format,
            &quality,
            &output_dir,
            embed_thumbnail,
            embed_metadata,
            write_subs,
            embed_subs,
            embed_chapters,
            sponsorblock,
        )
        .map_err(|e| e.to_string())?
    };
    Ok(id)
}

#[tauri::command]
pub async fn update_preset(
    id: i64,
    name: String,
    format: String,
    quality: String,
    output_dir: String,
    embed_thumbnail: bool,
    embed_metadata: bool,
    write_subs: bool,
    embed_subs: bool,
    embed_chapters: bool,
    sponsorblock: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let db = state.db.lock().await;
        queries::update_preset(
            &db,
            id,
            &name,
            &format,
            &quality,
            &output_dir,
            embed_thumbnail,
            embed_metadata,
            write_subs,
            embed_subs,
            embed_chapters,
            sponsorblock,
        )
        .map_err(|e| e.to_string())?
    };
    Ok(())
}

#[tauri::command]
pub async fn delete_preset(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    {
        let db = state.db.lock().await;
        queries::delete_preset(&db, id).map_err(|e| e.to_string())?
    };
    Ok(())
}
