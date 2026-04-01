use crate::db::{models::Schedule, queries};
use crate::scheduler;
use crate::state::AppState;
use tauri::{AppHandle, Emitter, State};

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
    let is_active = {
        let db = state.db.lock().await;
        let schedule = queries::get_schedule(&db, id).map_err(|e| e.to_string())?;
        schedule.is_active
    };
    {
        let db = state.db.lock().await;
        queries::update_schedule(&db, id, &name, &url, &cron_expr, &options_json, is_channel)
            .map_err(|e| e.to_string())?;
    }
    if is_active {
        let app2 = app.clone();
        let cron = cron_expr.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = scheduler::register_job(&app2, id, &cron).await {
                eprintln!("[YTDown] register_job failed for id={id}: {e}");
            }
        });
    } else {
        let app2 = app.clone();
        tauri::async_runtime::spawn(async move {
            scheduler::cancel_job(&app2, id).await;
        });
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_schedule(
    id: i64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    scheduler::cancel_job(&app, id).await;
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
        queries::get_schedule(&db, id)
            .map(|s| s.cron_expr)
            .map_err(|e| e.to_string())?
    };
    if is_active {
        scheduler::register_job(&app, id, &cron_expr).await?;
    } else {
        scheduler::cancel_job(&app, id).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn list_schedules(state: State<'_, AppState>) -> Result<Vec<Schedule>, String> {
    let db = state.db.lock().await;
    queries::list_schedules(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_schedule(id: i64, state: State<'_, AppState>) -> Result<Schedule, String> {
    let db = state.db.lock().await;
    queries::get_schedule(&db, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_schedule_now(
    id: i64,
    app: AppHandle,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    scheduler::execute_schedule(&app, id).await
}

#[tauri::command]
pub async fn stop_schedule_run(
    id: i64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let pid = {
        let pids = state.running_schedule_pids.lock().await;
        pids.get(&id).copied()
    };
    if let Some(pid) = pid {
        let mut cancelled = state.cancelled_schedule_ids.lock().await;
        cancelled.insert(id);
        drop(cancelled);
        crate::commands::download::kill_process(pid)?;
        let mut pids = state.running_schedule_pids.lock().await;
        pids.remove(&id);

        let db = state.db.lock().await;
        let next_run_at = crate::db::queries::get_schedule(&db, id)
            .ok()
            .and_then(|schedule| crate::scheduler::compute_next_run(&schedule.cron_expr));
        let _ = crate::db::queries::record_schedule_interrupted(&db, id, next_run_at.as_deref());
        drop(db);
        let _ = app.emit("schedule-updated", id);
    }
    Ok(())
}
