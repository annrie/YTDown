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
    pub ytdlp_path: Arc<Mutex<Option<String>>>,
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
            ytdlp_path: Arc::new(Mutex::new(None)),
            scheduler: Arc::new(Mutex::new(scheduler)),
            reserved_schedule_ids: Arc::new(Mutex::new(HashSet::new())),
            running_schedule_pids: Arc::new(Mutex::new(HashMap::new())),
            cancelled_schedule_ids: Arc::new(Mutex::new(HashSet::new())),
            schedule_job_ids: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
