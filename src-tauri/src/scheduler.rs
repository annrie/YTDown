use tauri::{AppHandle, Emitter, Manager};
use tokio_cron_scheduler::{Job, JobScheduler};
use chrono::Utc;

use crate::state::AppState;
use crate::db::queries;

/// アプリ起動時・スリープ復帰時のスキップ判定と通知
pub async fn check_overdue_schedules(app: &AppHandle) {
    let state = app.state::<AppState>();
    let now = Utc::now().to_rfc3339();
    let overdue = {
        let db = state.db.lock().await;
        queries::list_overdue_schedules(&db, &now).unwrap_or_default()
    };

    for schedule in overdue {
        let next = compute_next_run(&schedule.cron_expr);
        {
            let db = state.db.lock().await;
            let _ = queries::update_schedule_next_run(&db, schedule.id, next.as_deref());
        }
        send_notification(
            app,
            "スケジュールをスキップしました",
            &format!(
                "「{}」のスケジュールが実行されませんでした（アプリが起動していませんでした）",
                schedule.name
            ),
        );
    }
}

/// 全アクティブスケジュールを JobScheduler に登録する
pub async fn register_all_jobs(app: &AppHandle) {
    let state = app.state::<AppState>();
    let schedules = {
        let db = state.db.lock().await;
        queries::list_active_schedules(&db).unwrap_or_default()
    };

    for schedule in schedules {
        let _ = register_job(app, schedule.id, &schedule.cron_expr).await;
    }
}

/// 1つのスケジュールを JobScheduler に登録する
pub async fn register_job(
    app: &AppHandle,
    schedule_id: i64,
    cron_expr: &str,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let app_clone = app.clone();
    let cron = cron_expr.to_string();

    let job = Job::new_async(cron.as_str(), move |_uuid, _lock| {
        let app = app_clone.clone();
        Box::pin(async move {
            execute_schedule(&app, schedule_id).await;
        })
    })
    .map_err(|e| e.to_string())?;

    let next = compute_next_run(cron_expr);
    {
        let db = state.db.lock().await;
        let _ = queries::update_schedule_next_run(&db, schedule_id, next.as_deref());
    }

    let mut sched = state.scheduler.lock().await;
    sched.add(job).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// スケジュール実行本体
pub async fn execute_schedule(app: &AppHandle, schedule_id: i64) {
    let state = app.state::<AppState>();

    let schedule = {
        let db = state.db.lock().await;
        match queries::get_schedule(&db, schedule_id) {
            Ok(s) => s,
            Err(_) => return,
        }
    };

    if schedule.is_running || !schedule.is_active {
        return;
    }

    {
        let db = state.db.lock().await;
        let _ = queries::set_schedule_running(&db, schedule_id, true);
    }

    let result = run_download(app, &schedule).await;
    let now = Utc::now().to_rfc3339();
    let next = compute_next_run(&schedule.cron_expr);

    {
        let db = state.db.lock().await;
        match result {
            Ok(_) => {
                let _ = queries::record_schedule_success(
                    &db,
                    schedule_id,
                    &now,
                    next.as_deref(),
                );
            }
            Err(ref e) => {
                let _ = queries::record_schedule_failure(
                    &db,
                    schedule_id,
                    e,
                    next.as_deref(),
                );
                if let Ok(s) = queries::get_schedule(&db, schedule_id) {
                    if s.fail_count >= 3 {
                        let _ = queries::disable_schedule(&db, schedule_id);
                        send_notification(
                            app,
                            "スケジュール自動無効化",
                            &format!(
                                "「{}」が3回連続で失敗したため無効化されました",
                                schedule.name
                            ),
                        );
                    }
                }
            }
        }
    }

    let _ = app.emit("schedule-fired", schedule_id);
}

/// yt-dlp ダウンロード実行
async fn run_download(
    app: &AppHandle,
    schedule: &crate::db::models::Schedule,
) -> Result<(), String> {
    use crate::commands::download::DownloadOptions;

    let options: DownloadOptions = serde_json::from_str(&schedule.options_json)
        .map_err(|e| format!("オプションのパースに失敗: {}", e))?;

    crate::commands::download::run_download_internal(
        app,
        schedule.url.clone(),
        options,
        schedule.is_channel,
        schedule.last_run_at.clone(),
    )
    .await
}

/// cron式から次回発火時刻を計算 (ISO8601)
pub fn compute_next_run(cron_expr: &str) -> Option<String> {
    use cron::Schedule;
    use std::str::FromStr;

    let schedule = Schedule::from_str(cron_expr).ok()?;
    let next = schedule.upcoming(Utc).next()?;
    Some(next.to_rfc3339())
}

/// ジョブを JobScheduler から除去する（toggle OFF 時に使用）
pub async fn cancel_job(_app: &AppHandle, _schedule_id: i64) {
    // TODO: AppState に job_ids: Arc<Mutex<HashMap<i64, uuid::Uuid>>> を追加して
    // UUID ベースのキャンセルを実装する
}

/// macOS 通知を送信
/// TODO: mac-notification-sys の macOS 15 SDK 互換性修正後に tauri-plugin-notification を使用する
fn send_notification(_app: &AppHandle, title: &str, body: &str) {
    eprintln!("[YTDown Notification] {}: {}", title, body);
}
