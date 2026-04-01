use super::models::*;
use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};

// === Downloads ===

pub fn insert_download(
    conn: &Connection,
    url: &str,
    title: Option<&str>,
    channel: Option<&str>,
    channel_id: Option<&str>,
    channel_url: Option<&str>,
    site: Option<&str>,
    thumbnail_url: Option<&str>,
    format: Option<&str>,
    quality: Option<&str>,
    duration: Option<i64>,
) -> SqlResult<i64> {
    conn.execute(
        "INSERT INTO downloads (url, title, channel, channel_id, channel_url, site, thumbnail_url, format, quality, duration, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'pending')",
        params![url, title, channel, channel_id, channel_url, site, thumbnail_url, format, quality, duration],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn insert_completed_download(
    conn: &Connection,
    url: &str,
    title: Option<&str>,
    channel: Option<&str>,
    channel_id: Option<&str>,
    channel_url: Option<&str>,
    site: Option<&str>,
    thumbnail_url: Option<&str>,
    format: Option<&str>,
    quality: Option<&str>,
    duration: Option<i64>,
    file_path: &str,
    file_size: Option<i64>,
    created_at: &str,
    completed_at: &str,
) -> SqlResult<i64> {
    conn.execute(
        "INSERT INTO downloads (
            url, title, channel, channel_id, channel_url, site, thumbnail_url,
            format, quality, file_path, file_size, duration, status, created_at, completed_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7,
            ?8, ?9, ?10, ?11, ?12, 'completed', ?13, ?14
         )",
        params![
            url,
            title,
            channel,
            channel_id,
            channel_url,
            site,
            thumbnail_url,
            format,
            quality,
            file_path,
            file_size,
            duration,
            created_at,
            completed_at,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn has_download_for_file_path(conn: &Connection, file_path: &str) -> SqlResult<bool> {
    let existing = conn
        .query_row(
            "SELECT 1 FROM downloads WHERE file_path = ?1 LIMIT 1",
            params![file_path],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    Ok(existing.is_some())
}

pub fn get_download(conn: &Connection, id: i64) -> SqlResult<Download> {
    conn.query_row(
        "SELECT id, url, title, channel, channel_id, channel_url, site, thumbnail_url,
                format, quality, file_path, file_size, bytes_downloaded, duration,
                status, progress, pid, error_message, metadata_json,
                created_at, completed_at, is_favorite
         FROM downloads WHERE id = ?1",
        params![id],
        |row| {
            Ok(Download {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                channel: row.get(3)?,
                channel_id: row.get(4)?,
                channel_url: row.get(5)?,
                site: row.get(6)?,
                thumbnail_url: row.get(7)?,
                format: row.get(8)?,
                quality: row.get(9)?,
                file_path: row.get(10)?,
                file_size: row.get(11)?,
                bytes_downloaded: row.get(12)?,
                duration: row.get(13)?,
                status: row.get(14)?,
                progress: row.get(15)?,
                pid: row.get(16)?,
                error_message: row.get(17)?,
                metadata_json: row.get(18)?,
                created_at: row.get(19)?,
                completed_at: row.get(20)?,
                is_favorite: row.get(21)?,
            })
        },
    )
}

pub fn update_download_status(conn: &Connection, id: i64, status: &str) -> SqlResult<()> {
    let completed_at = if status == "completed" {
        Some(chrono::Utc::now().to_rfc3339())
    } else {
        None
    };
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
pub fn update_download_progress(
    conn: &Connection,
    id: i64,
    progress: f64,
    bytes_downloaded: i64,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE downloads SET progress = ?1, bytes_downloaded = ?2 WHERE id = ?3",
        params![progress, bytes_downloaded, id],
    )?;
    Ok(())
}

pub fn update_download_title(conn: &Connection, id: i64, title: &str) -> SqlResult<()> {
    conn.execute(
        "UPDATE downloads SET title = ?1 WHERE id = ?2",
        params![title, id],
    )?;
    Ok(())
}

pub fn update_download_pid(conn: &Connection, id: i64, pid: Option<i64>) -> SqlResult<()> {
    conn.execute(
        "UPDATE downloads SET pid = ?1 WHERE id = ?2",
        params![pid, id],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn update_download_file_path(
    conn: &Connection,
    id: i64,
    path: &str,
    size: Option<i64>,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE downloads SET file_path = ?1, file_size = ?2 WHERE id = ?3",
        params![path, size, id],
    )?;
    Ok(())
}

pub fn list_downloads(conn: &Connection, status_filter: Option<&str>) -> SqlResult<Vec<Download>> {
    let sql = match status_filter {
        Some(_) => {
            "SELECT id, url, title, channel, channel_id, channel_url, site, thumbnail_url,
                           format, quality, file_path, file_size, bytes_downloaded, duration,
                           status, progress, pid, error_message, metadata_json,
                           created_at, completed_at, is_favorite
                    FROM downloads WHERE status = ?1 ORDER BY created_at DESC"
        }
        None => {
            "SELECT id, url, title, channel, channel_id, channel_url, site, thumbnail_url,
                        format, quality, file_path, file_size, bytes_downloaded, duration,
                        status, progress, pid, error_message, metadata_json,
                        created_at, completed_at, is_favorite
                 FROM downloads ORDER BY created_at DESC"
        }
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
        id: row.get(0)?,
        url: row.get(1)?,
        title: row.get(2)?,
        channel: row.get(3)?,
        channel_id: row.get(4)?,
        channel_url: row.get(5)?,
        site: row.get(6)?,
        thumbnail_url: row.get(7)?,
        format: row.get(8)?,
        quality: row.get(9)?,
        file_path: row.get(10)?,
        file_size: row.get(11)?,
        bytes_downloaded: row.get(12)?,
        duration: row.get(13)?,
        status: row.get(14)?,
        progress: row.get(15)?,
        pid: row.get(16)?,
        error_message: row.get(17)?,
        metadata_json: row.get(18)?,
        created_at: row.get(19)?,
        completed_at: row.get(20)?,
        is_favorite: row.get(21)?,
    })
}

pub fn search_downloads(conn: &Connection, query: &str) -> SqlResult<Vec<Download>> {
    let mut stmt = conn.prepare(
        "SELECT d.id, d.url, d.title, d.channel, d.channel_id, d.channel_url, d.site,
                d.thumbnail_url, d.format, d.quality, d.file_path, d.file_size,
                d.bytes_downloaded, d.duration, d.status, d.progress, d.pid,
                d.error_message, d.metadata_json, d.created_at, d.completed_at, d.is_favorite
         FROM downloads_fts f JOIN downloads d ON f.rowid = d.id
         WHERE downloads_fts MATCH ?1 ORDER BY rank",
    )?;
    let rows = stmt.query_map(params![query], row_to_download)?;
    rows.collect()
}

pub fn toggle_favorite(conn: &Connection, id: i64) -> SqlResult<bool> {
    let current: bool = conn.query_row(
        "SELECT is_favorite FROM downloads WHERE id = ?1",
        params![id],
        |r| r.get(0),
    )?;
    let new_val = !current;
    conn.execute(
        "UPDATE downloads SET is_favorite = ?1 WHERE id = ?2",
        params![new_val, id],
    )?;
    Ok(new_val)
}

// === Settings ===

pub fn get_setting(conn: &Connection, key: &str) -> SqlResult<Option<String>> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .optional()
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
    let rows = stmt.query_map([], |row| {
        Ok(Setting {
            key: row.get(0)?,
            value: row.get(1)?,
        })
    })?;
    rows.collect()
}

// === Auto-Classify Rules ===

#[allow(dead_code)]
pub fn list_rules(conn: &Connection) -> SqlResult<Vec<AutoClassifyRule>> {
    let mut stmt = conn.prepare(
        "SELECT id, rule_type, pattern, target_dir, priority, enabled FROM auto_classify_rules ORDER BY priority DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(AutoClassifyRule {
            id: row.get(0)?,
            rule_type: row.get(1)?,
            pattern: row.get(2)?,
            target_dir: row.get(3)?,
            priority: row.get(4)?,
            enabled: row.get(5)?,
        })
    })?;
    rows.collect()
}

#[allow(dead_code)]
pub fn create_rule(
    conn: &Connection,
    rule_type: &str,
    pattern: &str,
    target_dir: &str,
    priority: i64,
) -> SqlResult<i64> {
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
    conn.execute(
        "DELETE FROM url_history WHERE type = ?1",
        params![history_type],
    )?;
    Ok(())
}

// === Schedules ===

pub fn insert_schedule(
    conn: &Connection,
    name: &str,
    url: &str,
    cron_expr: &str,
    options_json: &str,
    is_channel: bool,
) -> SqlResult<i64> {
    conn.execute(
        "INSERT INTO schedules (name, url, cron_expr, options_json, is_channel)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![name, url, cron_expr, options_json, is_channel as i64],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_schedule(conn: &Connection, id: i64) -> SqlResult<Schedule> {
    conn.query_row(
        "SELECT id, name, url, cron_expr, options_json, is_active, is_channel,
                last_error, fail_count, is_running, last_run_at, next_run_at,
                last_run_status, created_at
         FROM schedules WHERE id = ?1",
        params![id],
        |row| {
            Ok(Schedule {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                cron_expr: row.get(3)?,
                options_json: row.get(4)?,
                is_active: row.get::<_, i64>(5)? != 0,
                is_channel: row.get::<_, i64>(6)? != 0,
                last_error: row.get(7)?,
                fail_count: row.get(8)?,
                is_running: row.get::<_, i64>(9)? != 0,
                last_run_at: row.get(10)?,
                next_run_at: row.get(11)?,
                last_run_status: row.get(12)?,
                created_at: row.get(13)?,
            })
        },
    )
}

pub fn list_schedules(conn: &Connection) -> SqlResult<Vec<Schedule>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, url, cron_expr, options_json, is_active, is_channel,
                last_error, fail_count, is_running, last_run_at, next_run_at,
                last_run_status, created_at
         FROM schedules ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Schedule {
            id: row.get(0)?,
            name: row.get(1)?,
            url: row.get(2)?,
            cron_expr: row.get(3)?,
            options_json: row.get(4)?,
            is_active: row.get::<_, i64>(5)? != 0,
            is_channel: row.get::<_, i64>(6)? != 0,
            last_error: row.get(7)?,
            fail_count: row.get(8)?,
            is_running: row.get::<_, i64>(9)? != 0,
            last_run_at: row.get(10)?,
            next_run_at: row.get(11)?,
            last_run_status: row.get(12)?,
            created_at: row.get(13)?,
        })
    })?;
    rows.collect()
}

pub fn list_active_schedules(conn: &Connection) -> SqlResult<Vec<Schedule>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, url, cron_expr, options_json, is_active, is_channel,
                last_error, fail_count, is_running, last_run_at, next_run_at,
                last_run_status, created_at
         FROM schedules WHERE is_active = 1 ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Schedule {
            id: row.get(0)?,
            name: row.get(1)?,
            url: row.get(2)?,
            cron_expr: row.get(3)?,
            options_json: row.get(4)?,
            is_active: row.get::<_, i64>(5)? != 0,
            is_channel: row.get::<_, i64>(6)? != 0,
            last_error: row.get(7)?,
            fail_count: row.get(8)?,
            is_running: row.get::<_, i64>(9)? != 0,
            last_run_at: row.get(10)?,
            next_run_at: row.get(11)?,
            last_run_status: row.get(12)?,
            created_at: row.get(13)?,
        })
    })?;
    rows.collect()
}

pub fn update_schedule(
    conn: &Connection,
    id: i64,
    name: &str,
    url: &str,
    cron_expr: &str,
    options_json: &str,
    is_channel: bool,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE schedules SET name=?1, url=?2, cron_expr=?3, options_json=?4, is_channel=?5
         WHERE id=?6",
        params![name, url, cron_expr, options_json, is_channel as i64, id],
    )?;
    Ok(())
}

pub fn delete_schedule(conn: &Connection, id: i64) -> SqlResult<()> {
    conn.execute("DELETE FROM schedules WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn toggle_schedule(conn: &Connection, id: i64, is_active: bool) -> SqlResult<()> {
    conn.execute(
        "UPDATE schedules SET is_active = ?1 WHERE id = ?2",
        params![is_active as i64, id],
    )?;
    Ok(())
}

/// アプリ起動時に全スケジュールの is_running をリセット（クラッシュ対策）
pub fn reset_all_running_schedules(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "UPDATE schedules SET is_running = 0 WHERE is_running = 1",
        [],
    )?;
    Ok(())
}

pub fn set_schedule_running(conn: &Connection, id: i64, is_running: bool) -> SqlResult<()> {
    conn.execute(
        "UPDATE schedules SET is_running = ?1 WHERE id = ?2",
        params![is_running as i64, id],
    )?;
    Ok(())
}

pub fn record_schedule_success(
    conn: &Connection,
    id: i64,
    last_run_at: &str,
    next_run_at: Option<&str>,
    status: &str,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE schedules SET fail_count=0, last_error=NULL, is_running=0,
                              last_run_at=?1, next_run_at=?2, last_run_status=?3 WHERE id=?4",
        params![last_run_at, next_run_at, status, id],
    )?;
    Ok(())
}

pub fn record_schedule_failure(
    conn: &Connection,
    id: i64,
    error: &str,
    next_run_at: Option<&str>,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE schedules SET fail_count = fail_count + 1, last_error=?1,
                              is_running=0, next_run_at=?2, last_run_status=NULL WHERE id=?3",
        params![error, next_run_at, id],
    )?;
    Ok(())
}

pub fn record_schedule_interrupted(
    conn: &Connection,
    id: i64,
    next_run_at: Option<&str>,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE schedules SET is_running=0, last_error=NULL,
                              next_run_at=?1, last_run_status='stopped' WHERE id=?2",
        params![next_run_at, id],
    )?;
    Ok(())
}

pub fn disable_schedule(conn: &Connection, id: i64) -> SqlResult<()> {
    conn.execute("UPDATE schedules SET is_active=0 WHERE id=?1", params![id])?;
    Ok(())
}

pub fn list_overdue_schedules(conn: &Connection, now: &str) -> SqlResult<Vec<Schedule>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, url, cron_expr, options_json, is_active, is_channel,
                last_error, fail_count, is_running, last_run_at, next_run_at,
                last_run_status, created_at
         FROM schedules WHERE is_active=1 AND next_run_at IS NOT NULL AND next_run_at < ?1",
    )?;
    let rows = stmt.query_map(params![now], |row| {
        Ok(Schedule {
            id: row.get(0)?,
            name: row.get(1)?,
            url: row.get(2)?,
            cron_expr: row.get(3)?,
            options_json: row.get(4)?,
            is_active: row.get::<_, i64>(5)? != 0,
            is_channel: row.get::<_, i64>(6)? != 0,
            last_error: row.get(7)?,
            fail_count: row.get(8)?,
            is_running: row.get::<_, i64>(9)? != 0,
            last_run_at: row.get(10)?,
            next_run_at: row.get(11)?,
            last_run_status: row.get(12)?,
            created_at: row.get(13)?,
        })
    })?;
    rows.collect()
}

pub fn update_schedule_next_run(
    conn: &Connection,
    id: i64,
    next_run_at: Option<&str>,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE schedules SET next_run_at=?1 WHERE id=?2",
        params![next_run_at, id],
    )?;
    Ok(())
}

// ── Presets ──────────────────────────────────────────────────────────────

pub fn list_presets(conn: &Connection) -> SqlResult<Vec<crate::db::models::Preset>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, format, quality, output_dir,
                embed_thumbnail, embed_metadata, write_subs, embed_subs,
                embed_chapters, sponsorblock, created_at
         FROM download_presets ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(crate::db::models::Preset {
            id: row.get(0)?,
            name: row.get(1)?,
            format: row.get(2)?,
            quality: row.get(3)?,
            output_dir: row.get(4)?,
            embed_thumbnail: row.get::<_, i64>(5)? != 0,
            embed_metadata: row.get::<_, i64>(6)? != 0,
            write_subs: row.get::<_, i64>(7)? != 0,
            embed_subs: row.get::<_, i64>(8)? != 0,
            embed_chapters: row.get::<_, i64>(9)? != 0,
            sponsorblock: row.get::<_, i64>(10)? != 0,
            created_at: row.get(11)?,
        })
    })?;
    rows.collect()
}

pub fn insert_preset(
    conn: &Connection,
    name: &str,
    format: &str,
    quality: &str,
    output_dir: &str,
    embed_thumbnail: bool,
    embed_metadata: bool,
    write_subs: bool,
    embed_subs: bool,
    embed_chapters: bool,
    sponsorblock: bool,
) -> SqlResult<i64> {
    conn.execute(
        "INSERT INTO download_presets
         (name, format, quality, output_dir, embed_thumbnail, embed_metadata,
          write_subs, embed_subs, embed_chapters, sponsorblock)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            name,
            format,
            quality,
            output_dir,
            embed_thumbnail as i64,
            embed_metadata as i64,
            write_subs as i64,
            embed_subs as i64,
            embed_chapters as i64,
            sponsorblock as i64,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_preset(
    conn: &Connection,
    id: i64,
    name: &str,
    format: &str,
    quality: &str,
    output_dir: &str,
    embed_thumbnail: bool,
    embed_metadata: bool,
    write_subs: bool,
    embed_subs: bool,
    embed_chapters: bool,
    sponsorblock: bool,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE download_presets SET
         name=?1, format=?2, quality=?3, output_dir=?4,
         embed_thumbnail=?5, embed_metadata=?6, write_subs=?7,
         embed_subs=?8, embed_chapters=?9, sponsorblock=?10
         WHERE id=?11",
        params![
            name,
            format,
            quality,
            output_dir,
            embed_thumbnail as i64,
            embed_metadata as i64,
            write_subs as i64,
            embed_subs as i64,
            embed_chapters as i64,
            sponsorblock as i64,
            id,
        ],
    )?;
    Ok(())
}

pub fn delete_preset(conn: &Connection, id: i64) -> SqlResult<()> {
    conn.execute("DELETE FROM download_presets WHERE id = ?1", params![id])?;
    Ok(())
}
