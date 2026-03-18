use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

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
}

impl AppState {
    pub fn new(db: rusqlite::Connection) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            active_downloads: Arc::new(Mutex::new(HashMap::new())),
            ytdlp_path: Arc::new(Mutex::new(None)),
        }
    }
}
