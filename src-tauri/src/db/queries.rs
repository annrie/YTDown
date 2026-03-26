use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};
use super::models::*;

// === Downloads ===

pub fn insert_download(conn: &Connection, url: &str, title: Option<&str>, channel: Option<&str>,
    channel_id: Option<&str>, channel_url: Option<&str>, site: Option<&str>,
    thumbnail_url: Option<&str>, format: Option<&str>, quality: Option<&str>,
    duration: Option<i64>) -> SqlResult<i64>
{
    conn.execute(
        "INSERT INTO downloads (url, title, channel, channel_id, channel_url, site, thumbnail_url, format, quality, duration, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'pending')",
        params![url, title, channel, channel_id, channel_url, site, thumbnail_url, format, quality, duration],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_download(conn: &Connection, id: i64) -> SqlResult<Download> {
    conn.query_row(
        "SELECT id, url, title, channel, channel_id, channel_url, site, thumbnail_url,
                format, quality, file_path, file_size, bytes_downloaded, duration,
                status, progress, pid, error_message, metadata_json,
                created_at, completed_at, is_favorite
         FROM downloads WHERE id = ?1",
        params![id],
        |row| Ok(Download {
            id: row.get(0)?, url: row.get(1)?, title: row.get(2)?, channel: row.get(3)?,
            channel_id: row.get(4)?, channel_url: row.get(5)?, site: row.get(6)?,
            thumbnail_url: row.get(7)?, format: row.get(8)?, quality: row.get(9)?,
            file_path: row.get(10)?, file_size: row.get(11)?, bytes_downloaded: row.get(12)?,
            duration: row.get(13)?, status: row.get(14)?, progress: row.get(15)?,
            pid: row.get(16)?, error_message: row.get(17)?, metadata_json: row.get(18)?,
            created_at: row.get(19)?, completed_at: row.get(20)?, is_favorite: row.get(21)?,
        }),
    )
}

pub fn update_download_status(conn: &Connection, id: i64, status: &str) -> SqlResult<()> {
    let completed_at = if status == "completed" { Some(chrono::Utc::now().to_rfc3339()) } else { None };
    conn.execute(
        "UPDATE downloads SET status = ?1, completed_at = ?2 WHERE id = ?3",
        params![status, completed_at, id],
    )?;
    Ok(())
}

pub fn update_download_error(conn: &Connection, id: i64, error_message: &str) -> SqlResult<()> {
    conn.execute(
        "UPDATE downloads SET status = 'error', error_message = ?1 WHERE id = ?2",
        params![error_message, id],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn update_download_progress(conn: &Connection, id: i64, progress: f64, bytes_downloaded: i64) -> SqlResult<()> {
    conn.execute(
        "UPDATE downloads SET progress = ?1, bytes_downloaded = ?2 WHERE id = ?3",
        params![progress, bytes_downloaded, id],
    )?;
    Ok(())
}

pub fn update_download_title(conn: &Connection, id: i64, title: &str) -> SqlResult<()> {
    conn.execute("UPDATE downloads SET title = ?1 WHERE id = ?2", params![title, id])?;
    Ok(())
}

pub fn update_download_pid(conn: &Connection, id: i64, pid: Option<i64>) -> SqlResult<()> {
    conn.execute("UPDATE downloads SET pid = ?1 WHERE id = ?2", params![pid, id])?;
    Ok(())
}

#[allow(dead_code)]
pub fn update_download_file_path(conn: &Connection, id: i64, path: &str, size: Option<i64>) -> SqlResult<()> {
    conn.execute(
        "UPDATE downloads SET file_path = ?1, file_size = ?2 WHERE id = ?3",
        params![path, size, id],
    )?;
    Ok(())
}

pub fn list_downloads(conn: &Connection, status_filter: Option<&str>) -> SqlResult<Vec<Download>> {
    let sql = match status_filter {
        Some(_) => "SELECT id, url, title, channel, channel_id, channel_url, site, thumbnail_url,
                           format, quality, file_path, file_size, bytes_downloaded, duration,
                           status, progress, pid, error_message, metadata_json,
                           created_at, completed_at, is_favorite
                    FROM downloads WHERE status = ?1 ORDER BY created_at DESC",
        None => "SELECT id, url, title, channel, channel_id, channel_url, site, thumbnail_url,
                        format, quality, file_path, file_size, bytes_downloaded, duration,
                        status, progress, pid, error_message, metadata_json,
                        created_at, completed_at, is_favorite
                 FROM downloads ORDER BY created_at DESC",
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = if let Some(status) = status_filter {
        stmt.query_map(params![status], row_to_download)?
    } else {
        stmt.query_map([], row_to_download)?
    };
    rows.collect()
}

fn row_to_download(row: &rusqlite::Row) -> SqlResult<Download> {
    Ok(Download {
        id: row.get(0)?, url: row.get(1)?, title: row.get(2)?, channel: row.get(3)?,
        channel_id: row.get(4)?, channel_url: row.get(5)?, site: row.get(6)?,
        thumbnail_url: row.get(7)?, format: row.get(8)?, quality: row.get(9)?,
        file_path: row.get(10)?, file_size: row.get(11)?, bytes_downloaded: row.get(12)?,
        duration: row.get(13)?, status: row.get(14)?, progress: row.get(15)?,
        pid: row.get(16)?, error_message: row.get(17)?, metadata_json: row.get(18)?,
        created_at: row.get(19)?, completed_at: row.get(20)?, is_favorite: row.get(21)?,
    })
}

pub fn search_downloads(conn: &Connection, query: &str) -> SqlResult<Vec<Download>> {
    let mut stmt = conn.prepare(
        "SELECT d.id, d.url, d.title, d.channel, d.channel_id, d.channel_url, d.site,
                d.thumbnail_url, d.format, d.quality, d.file_path, d.file_size,
                d.bytes_downloaded, d.duration, d.status, d.progress, d.pid,
                d.error_message, d.metadata_json, d.created_at, d.completed_at, d.is_favorite
         FROM downloads_fts f JOIN downloads d ON f.rowid = d.id
         WHERE downloads_fts MATCH ?1 ORDER BY rank"
    )?;
    let rows = stmt.query_map(params![query], row_to_download)?;
    rows.collect()
}

pub fn toggle_favorite(conn: &Connection, id: i64) -> SqlResult<bool> {
    let current: bool = conn.query_row(
        "SELECT is_favorite FROM downloads WHERE id = ?1", params![id], |r| r.get(0)
    )?;
    let new_val = !current;
    conn.execute("UPDATE downloads SET is_favorite = ?1 WHERE id = ?2", params![new_val, id])?;
    Ok(new_val)
}

// === Settings ===

pub fn get_setting(conn: &Connection, key: &str) -> SqlResult<Option<String>> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    ).optional()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_all_settings(conn: &Connection) -> SqlResult<Vec<Setting>> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| Ok(Setting { key: row.get(0)?, value: row.get(1)? }))?;
    rows.collect()
}

// === Auto-Classify Rules ===

#[allow(dead_code)]
pub fn list_rules(conn: &Connection) -> SqlResult<Vec<AutoClassifyRule>> {
    let mut stmt = conn.prepare(
        "SELECT id, rule_type, pattern, target_dir, priority, enabled FROM auto_classify_rules ORDER BY priority DESC"
    )?;
    let rows = stmt.query_map([], |row| Ok(AutoClassifyRule {
        id: row.get(0)?, rule_type: row.get(1)?, pattern: row.get(2)?,
        target_dir: row.get(3)?, priority: row.get(4)?, enabled: row.get(5)?,
    }))?;
    rows.collect()
}

#[allow(dead_code)]
pub fn create_rule(conn: &Connection, rule_type: &str, pattern: &str, target_dir: &str, priority: i64) -> SqlResult<i64> {
    conn.execute(
        "INSERT INTO auto_classify_rules (rule_type, pattern, target_dir, priority) VALUES (?1, ?2, ?3, ?4)",
        params![rule_type, pattern, target_dir, priority],
    )?;
    Ok(conn.last_insert_rowid())
}

// === URL History ===

pub fn save_url_history(conn: &Connection, history_type: &str, url: &str) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO url_history (type, url, created_at) VALUES (?1, ?2, datetime('now'))
         ON CONFLICT(type, url) DO UPDATE SET created_at = datetime('now')",
        params![history_type, url],
    )?;
    conn.execute(
        "DELETE FROM url_history WHERE type = ?1 AND id NOT IN (
           SELECT id FROM url_history WHERE type = ?1 ORDER BY created_at DESC LIMIT 10
         )",
        params![history_type],
    )?;
    Ok(())
}

pub fn get_url_history(conn: &Connection, history_type: &str) -> SqlResult<Vec<UrlHistoryEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, url, created_at FROM url_history WHERE type = ?1 ORDER BY created_at DESC LIMIT 10"
    )?;
    let rows = stmt.query_map(params![history_type], |row| {
        Ok(UrlHistoryEntry {
            id: row.get(0)?,
            url: row.get(1)?,
            created_at: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn clear_url_history(conn: &Connection, history_type: &str) -> SqlResult<()> {
    conn.execute("DELETE FROM url_history WHERE type = ?1", params![history_type])?;
    Ok(())
}
