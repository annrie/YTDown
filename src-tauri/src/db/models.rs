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
