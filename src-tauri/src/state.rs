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
