use tauri::{AppHandle, State};
use crate::state::AppState;
use crate::db::{queries, models::Schedule};
use crate::scheduler;

#[tauri::command]
pub async fn create_schedule(
    app: AppHandle,
    name: String,
    url: String,
    cron_expr: String,
    options_json: String,
    is_channel: bool,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let id = {
        let db = state.db.lock().await;
        queries::insert_schedule(&db, &name, &url, &cron_expr, &options_json, is_channel)
            .map_err(|e| e.to_string())?
    };
    let app2 = app.clone();
    let cron = cron_expr.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = scheduler::register_job(&app2, id, &cron).await {
            eprintln!("[YTDown] register_job failed for id={id}: {e}");
        }
    });
    Ok(id)
}

#[tauri::command]
pub async fn update_schedule(
    app: AppHandle,
    id: i64,
    name: String,
    url: String,
    cron_expr: String,
    options_json: String,
    is_channel: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let db = state.db.lock().await;
        queries::update_schedule(&db, id, &name, &url, &cron_expr, &options_json, is_channel)
            .map_err(|e| e.to_string())?;
    }
    let app2 = app.clone();
    let cron = cron_expr.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = scheduler::register_job(&app2, id, &cron).await {
            eprintln!("[YTDown] register_job failed for id={id}: {e}");
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn delete_schedule(
    id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().await;
    queries::delete_schedule(&db, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_schedule(
    app: AppHandle,
    id: i64,
    is_active: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cron_expr = {
        let db = state.db.lock().await;
        queries::toggle_schedule(&db, id, is_active).map_err(|e| e.to_string())?;
        queries::get_schedule(&db, id).map(|s| s.cron_expr).map_err(|e| e.to_string())?
    };
    if is_active {
        scheduler::register_job(&app, id, &cron_expr).await?;
    } else {
        scheduler::cancel_job(&app, id).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn list_schedules(
    state: State<'_, AppState>,
) -> Result<Vec<Schedule>, String> {
    let db = state.db.lock().await;
    queries::list_schedules(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_schedule(
    id: i64,
    state: State<'_, AppState>,
) -> Result<Schedule, String> {
    let db = state.db.lock().await;
    queries::get_schedule(&db, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_schedule_now(
    id: i64,
    app: AppHandle,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    scheduler::execute_schedule(&app, id).await;
    Ok(())
}
