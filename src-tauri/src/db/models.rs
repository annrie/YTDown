use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    pub id: i64,
    pub url: String,
    pub title: Option<String>,
    pub channel: Option<String>,
    pub channel_id: Option<String>,
    pub channel_url: Option<String>,
    pub site: Option<String>,
    pub thumbnail_url: Option<String>,
    pub format: Option<String>,
    pub quality: Option<String>,
    pub file_path: Option<String>,
    pub file_size: Option<i64>,
    pub bytes_downloaded: i64,
    pub duration: Option<i64>,
    pub status: String,
    pub progress: f64,
    pub pid: Option<i64>,
    pub error_message: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlHistoryEntry {
    pub id: i64,
    pub url: String,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoClassifyRule {
    pub id: i64,
    pub rule_type: String,
    pub pattern: String,
    pub target_dir: String,
    pub priority: i64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub cron_expr: String,
    pub options_json: String,
    pub is_active: bool,
    pub is_channel: bool,
    pub last_error: Option<String>,
    pub fail_count: i64,
    pub is_running: bool,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub last_run_status: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub url: String,
    pub title: Option<String>,
    pub site: Option<String>,
    pub file_path: Option<String>,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoPresetRule {
    pub id: i64,
    pub domain: String,
    pub preset_id: i64,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub id: i64,
    pub name: String,
    pub format: String,
    pub quality: String,
    pub output_dir: String,
    pub embed_thumbnail: bool,
    pub embed_metadata: bool,
    pub write_subs: bool,
    pub embed_subs: bool,
    pub embed_chapters: bool,
    pub sponsorblock: bool,
    pub created_at: String,
}
