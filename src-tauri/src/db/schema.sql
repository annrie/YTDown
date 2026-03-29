-- schema.sql
CREATE TABLE IF NOT EXISTS downloads (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  url               TEXT NOT NULL,
  title             TEXT,
  channel           TEXT,
  channel_id        TEXT,
  channel_url       TEXT,
  site              TEXT,
  thumbnail_url     TEXT,
  format            TEXT,
  quality           TEXT,
  file_path         TEXT,
  file_size         INTEGER,
  bytes_downloaded  INTEGER DEFAULT 0,
  duration          INTEGER,
  status            TEXT NOT NULL DEFAULT 'pending',
  progress          REAL DEFAULT 0,
  pid               INTEGER,
  error_message     TEXT,
  metadata_json     TEXT,
  created_at        DATETIME DEFAULT CURRENT_TIMESTAMP,
  completed_at      DATETIME,
  is_favorite       BOOLEAN DEFAULT 0
);

CREATE VIRTUAL TABLE IF NOT EXISTS downloads_fts USING fts5(
  title, channel, url, site,
  content='downloads',
  content_rowid='id'
);

-- FTS sync triggers
CREATE TRIGGER IF NOT EXISTS downloads_ai AFTER INSERT ON downloads BEGIN
  INSERT INTO downloads_fts(rowid, title, channel, url, site)
  VALUES (new.id, new.title, new.channel, new.url, new.site);
END;

CREATE TRIGGER IF NOT EXISTS downloads_ad AFTER DELETE ON downloads BEGIN
  INSERT INTO downloads_fts(downloads_fts, rowid, title, channel, url, site)
  VALUES ('delete', old.id, old.title, old.channel, old.url, old.site);
END;

CREATE TRIGGER IF NOT EXISTS downloads_au AFTER UPDATE ON downloads BEGIN
  INSERT INTO downloads_fts(downloads_fts, rowid, title, channel, url, site)
  VALUES ('delete', old.id, old.title, old.channel, old.url, old.site);
  INSERT INTO downloads_fts(rowid, title, channel, url, site)
  VALUES (new.id, new.title, new.channel, new.url, new.site);
END;

CREATE TABLE IF NOT EXISTS settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS auto_classify_rules (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  rule_type  TEXT NOT NULL,
  pattern    TEXT NOT NULL,
  target_dir TEXT NOT NULL,
  priority   INTEGER DEFAULT 0,
  enabled    BOOLEAN DEFAULT 1
);

-- Default settings
INSERT OR IGNORE INTO settings (key, value) VALUES
  ('download_dir', '~/Downloads/YTDown/'),
  ('filename_template', '%(title)s.%(ext)s'),
  ('concurrent_downloads', '3'),
  ('default_video_format', 'mp4'),
  ('default_video_quality', 'best'),
  ('default_audio_format', 'mp3'),
  ('embed_thumbnail', 'true'),
  ('embed_metadata', 'true'),
  ('write_subs', 'false'),
  ('embed_subs', 'false'),
  ('embed_chapters', 'true'),
  ('sponsorblock', 'false'),
  ('cookie_browser', 'none'),
  ('cookie_file', ''),
  ('ytdlp_path', 'auto'),
  ('theme', 'system'),
  ('auto_classify', 'false');

CREATE TABLE IF NOT EXISTS url_history (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  type       TEXT NOT NULL CHECK(type IN ('video', 'image')),
  url        TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(type, url)
);

-- Image download feature
CREATE TABLE IF NOT EXISTS image_sessions (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  source_url    TEXT NOT NULL,
  site_name     TEXT,
  image_count   INTEGER DEFAULT 0,
  output_dir    TEXT NOT NULL,
  created_at    DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS images (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id    INTEGER NOT NULL REFERENCES image_sessions(id) ON DELETE CASCADE,
  original_url  TEXT NOT NULL,
  file_path     TEXT,
  filename      TEXT,
  width         INTEGER,
  height        INTEGER,
  file_size     INTEGER,
  format        TEXT,
  status        TEXT DEFAULT 'pending',
  created_at    DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS schedules (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  name         TEXT NOT NULL,
  url          TEXT NOT NULL,
  cron_expr    TEXT NOT NULL,
  options_json TEXT NOT NULL,
  is_active    INTEGER NOT NULL DEFAULT 1,
  is_channel   INTEGER NOT NULL DEFAULT 0,
  last_error   TEXT,
  fail_count   INTEGER NOT NULL DEFAULT 0,
  is_running   INTEGER NOT NULL DEFAULT 0,
  last_run_at  TEXT,
  next_run_at  TEXT,
  created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS download_presets (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  name            TEXT NOT NULL UNIQUE,
  format          TEXT NOT NULL,
  quality         TEXT NOT NULL,
  output_dir      TEXT NOT NULL,
  embed_thumbnail INTEGER NOT NULL DEFAULT 0,
  embed_metadata  INTEGER NOT NULL DEFAULT 0,
  write_subs      INTEGER NOT NULL DEFAULT 0,
  embed_subs      INTEGER NOT NULL DEFAULT 0,
  embed_chapters  INTEGER NOT NULL DEFAULT 0,
  sponsorblock    INTEGER NOT NULL DEFAULT 0,
  created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
