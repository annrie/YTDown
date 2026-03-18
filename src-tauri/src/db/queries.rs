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

// === Playlists ===

pub fn list_playlists(conn: &Connection) -> SqlResult<Vec<Playlist>> {
    let mut stmt = conn.prepare("SELECT id, name, description, created_at, updated_at FROM playlists ORDER BY name")?;
    let rows = stmt.query_map([], |row| Ok(Playlist {
        id: row.get(0)?, name: row.get(1)?, description: row.get(2)?,
        created_at: row.get(3)?, updated_at: row.get(4)?,
    }))?;
    rows.collect()
}

pub fn create_playlist(conn: &Connection, name: &str) -> SqlResult<i64> {
    conn.execute("INSERT INTO playlists (name) VALUES (?1)", params![name])?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_playlist(conn: &Connection, id: i64) -> SqlResult<()> {
    conn.execute("DELETE FROM playlists WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn add_playlist_item(conn: &Connection, playlist_id: i64, download_id: Option<i64>, url: Option<&str>) -> SqlResult<i64> {
    let max_order: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), 0) FROM playlist_items WHERE playlist_id = ?1",
        params![playlist_id], |r| r.get(0)
    ).unwrap_or(0);
    conn.execute(
        "INSERT INTO playlist_items (playlist_id, download_id, url, sort_order) VALUES (?1, ?2, ?3, ?4)",
        params![playlist_id, download_id, url, max_order + 1],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_playlist_items(conn: &Connection, playlist_id: i64) -> SqlResult<Vec<PlaylistItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, playlist_id, download_id, url, sort_order, added_at
         FROM playlist_items WHERE playlist_id = ?1 ORDER BY sort_order"
    )?;
    let rows = stmt.query_map(params![playlist_id], |row| Ok(PlaylistItem {
        id: row.get(0)?, playlist_id: row.get(1)?, download_id: row.get(2)?,
        url: row.get(3)?, sort_order: row.get(4)?, added_at: row.get(5)?,
    }))?;
    rows.collect()
}

// === URL Lists ===

pub fn list_url_lists(conn: &Connection) -> SqlResult<Vec<UrlList>> {
    let mut stmt = conn.prepare("SELECT id, name, created_at, updated_at FROM url_lists ORDER BY name")?;
    let rows = stmt.query_map([], |row| Ok(UrlList {
        id: row.get(0)?, name: row.get(1)?, created_at: row.get(2)?, updated_at: row.get(3)?,
    }))?;
    rows.collect()
}

pub fn create_url_list(conn: &Connection, name: &str) -> SqlResult<i64> {
    conn.execute("INSERT INTO url_lists (name) VALUES (?1)", params![name])?;
    Ok(conn.last_insert_rowid())
}

pub fn add_url_to_list(conn: &Connection, list_id: i64, url: &str) -> SqlResult<i64> {
    let max_order: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), 0) FROM url_list_items WHERE list_id = ?1",
        params![list_id], |r| r.get(0)
    ).unwrap_or(0);
    conn.execute(
        "INSERT INTO url_list_items (list_id, url, sort_order) VALUES (?1, ?2, ?3)",
        params![list_id, url, max_order + 1],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn import_url_list_from_file(conn: &Connection, list_id: i64, file_path: &str) -> SqlResult<usize> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| rusqlite::Error::InvalidParameterName(format!("File read error: {}", e)))?;
    let mut count = 0;
    for line in content.lines() {
        let url = line.trim();
        if !url.is_empty() && !url.starts_with('#') {
            add_url_to_list(conn, list_id, url)?;
            count += 1;
        }
    }
    Ok(count)
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
