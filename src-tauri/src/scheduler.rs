use tauri::{AppHandle, Emitter, Manager};
use tokio_cron_scheduler::Job;
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
        // クラッシュ時に残った is_running フラグをリセット
        let _ = queries::reset_all_running_schedules(&db);
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

    let cron = normalize_cron(&cron);
    let job = Job::new_async(cron.as_str(), move |_uuid, _lock| {
        let app = app_clone.clone();
        Box::pin(async move {
            if let Err(e) = execute_schedule(&app, schedule_id).await {
                eprintln!("[YTDown] execute_schedule failed for id={schedule_id}: {e}");
            }
        })
    })
    .map_err(|e| e.to_string())?;

    let next = compute_next_run(cron_expr);
    {
        let db = state.db.lock().await;
        let _ = queries::update_schedule_next_run(&db, schedule_id, next.as_deref());
    }

    let sched = state.scheduler.lock().await;
    sched.add(job).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// スケジュール実行本体
pub async fn execute_schedule(app: &AppHandle, schedule_id: i64) -> Result<(), String> {
    let state = app.state::<AppState>();

    let schedule = {
        let db = state.db.lock().await;
        queries::get_schedule(&db, schedule_id).map_err(|e| e.to_string())?
    };

    if schedule.is_running {
        return Err(format!("スケジュール {} は実行中です", schedule_id));
    }
    if !schedule.is_active {
        return Err(format!("スケジュール {} は無効化されています", schedule_id));
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
    result.map(|_| ())
}

/// yt-dlp ダウンロード実行・完了後にライブラリDBへ記録
async fn run_download(
    app: &AppHandle,
    schedule: &crate::db::models::Schedule,
) -> Result<(), String> {
    use crate::commands::download::DownloadOptions;

    let options: DownloadOptions = serde_json::from_str(&schedule.options_json)
        .map_err(|e| format!("オプションのパースに失敗: {}", e))?;

    let format = options.format.clone();
    let quality = options.quality.clone();

    let file_path = crate::commands::download::run_download_internal(
        app,
        schedule.url.clone(),
        options,
        schedule.is_channel,
        schedule.last_run_at.clone(),
    )
    .await?;

    // ライブラリDBへ記録
    let title = file_path.as_deref().and_then(|p| {
        std::path::Path::new(p)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    });
    let file_size = file_path.as_deref().and_then(|p| {
        std::fs::metadata(p).ok().map(|m| m.len() as i64)
    });

    let state = app.state::<AppState>();
    let db = state.db.lock().await;
    let dl_id = queries::insert_download(
        &db, &schedule.url, title.as_deref(), None, None, None, None, None,
        Some(&format), Some(&quality), None,
    ).unwrap_or(-1);
    if dl_id > 0 {
        let _ = queries::update_download_status(&db, dl_id, "completed");
        if let Some(ref path) = file_path {
            let _ = queries::update_download_file_path(&db, dl_id, path, file_size);
        }
    }

    Ok(())
}

/// フロントエンドの5フィールドcron式を Rust crate が要求する6フィールドに正規化
/// 例: "0 9 * * *" → "0 0 9 * * *" (先頭に秒フィールド 0 を追加)
fn normalize_cron(expr: &str) -> String {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() == 5 {
        format!("0 {}", expr)
    } else {
        expr.to_string()
    }
}

/// cron式から次回発火時刻を計算 (ISO8601)
pub fn compute_next_run(cron_expr: &str) -> Option<String> {
    use cron::Schedule;
    use std::str::FromStr;

    let normalized = normalize_cron(cron_expr);
    let schedule = Schedule::from_str(&normalized).ok()?;
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
