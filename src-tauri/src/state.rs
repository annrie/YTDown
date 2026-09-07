use crate::db::queries;
use crate::ytdlp::binary::{self, YtdlpBinary};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_cron_scheduler::JobScheduler;
use uuid::Uuid;

pub struct ActiveDownload {
    #[allow(dead_code)]
    pub download_id: i64,
    pub pid: u32,
    pub paused: bool,
}

pub struct AppState {
    pub db: Arc<Mutex<rusqlite::Connection>>,
    pub active_downloads: Arc<Mutex<HashMap<i64, ActiveDownload>>>,
    pub scheduler: Arc<Mutex<JobScheduler>>,
    pub reserved_schedule_ids: Arc<Mutex<HashSet<i64>>>,
    pub running_schedule_pids: Arc<Mutex<HashMap<i64, u32>>>,
    pub cancelled_schedule_ids: Arc<Mutex<HashSet<i64>>>,
    pub schedule_job_ids: Arc<Mutex<HashMap<i64, Uuid>>>,
}

impl AppState {
    pub fn new(db: rusqlite::Connection, scheduler: JobScheduler) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            active_downloads: Arc::new(Mutex::new(HashMap::new())),
            scheduler: Arc::new(Mutex::new(scheduler)),
            reserved_schedule_ids: Arc::new(Mutex::new(HashSet::new())),
            running_schedule_pids: Arc::new(Mutex::new(HashMap::new())),
            cancelled_schedule_ids: Arc::new(Mutex::new(HashSet::new())),
            schedule_job_ids: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Settings-table key holding the user's manual yt-dlp path ("auto" = auto-detect).
    pub const YTDLP_PATH_SETTING_KEY: &'static str = "ytdlp_path";

    /// Resolve the yt-dlp binary for a command: read the manual path from the settings DB
    /// under a scoped lock, then run detection (manual > PATH > well-known > bundled)
    /// off the async runtime because it spawns `yt-dlp --version`.
    /// A missing or unreadable setting is treated as "auto" (same policy as cookie settings).
    /// Callers must not hold `self.db` while awaiting this.
    pub async fn resolve_ytdlp_binary(&self) -> Result<YtdlpBinary, String> {
        let raw = {
            let db = self.db.lock().await;
            queries::get_setting(&db, Self::YTDLP_PATH_SETTING_KEY).ok().flatten()
        }; // db lock dropped here
        let manual = binary::manual_path_from_setting(raw.as_deref());
        tokio::task::spawn_blocking(move || binary::detect_binary(manual.as_deref()))
            .await
            .map_err(|e| format!("Task error: {}", e))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Fresh AppState backed by a real SQLite file under `dir` (schema applied by `init_db`).
    async fn test_state(dir: &PathBuf) -> AppState {
        let conn = crate::db::init_db(dir).expect("init_db");
        let sched = JobScheduler::new().await.expect("scheduler");
        AppState::new(conn, sched)
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ytdown-state-test-{}-{}", std::process::id(), name));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    async fn set_ytdlp_path(state: &AppState, value: &str) {
        let db = state.db.lock().await;
        queries::set_setting(&db, AppState::YTDLP_PATH_SETTING_KEY, value).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn resolve_uses_manual_path_stored_in_settings_db() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("manual");
        let script = dir.join("yt-dlp-fake");
        std::fs::write(&script, "#!/bin/sh\necho db-1.0\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let state = test_state(&dir).await;
        set_ytdlp_path(&state, script.to_str().unwrap()).await;

        let bin = state.resolve_ytdlp_binary().await.unwrap();
        assert_eq!(bin.path, script);
        assert_eq!(bin.version, "db-1.0");
        assert!(matches!(bin.managed_by, binary::ManagedBy::Manual));

        drop(state);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn resolve_reports_missing_manual_path_stored_in_settings_db() {
        let dir = temp_dir("missing");
        let state = test_state(&dir).await;
        set_ytdlp_path(&state, "/nonexistent/ytdown/yt-dlp").await;

        let err = state.resolve_ytdlp_binary().await.unwrap_err();
        assert!(err.contains("Manual yt-dlp path not found"), "unexpected error: {err}");

        drop(state);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
