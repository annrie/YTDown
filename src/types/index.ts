// === Database Models ===

export interface Download {
  id: number
  url: string
  title: string | null
  channel: string | null
  channel_id: string | null
  channel_url: string | null
  site: string | null
  thumbnail_url: string | null
  format: string | null
  quality: string | null
  file_path: string | null
  file_size: number | null
  bytes_downloaded: number
  duration: number | null
  status: DownloadStatus
  progress: number
  pid: number | null
  error_message: string | null
  metadata_json: string | null
  created_at: string
  completed_at: string | null
  is_favorite: boolean
}

export type DownloadStatus =
  | 'pending'
  | 'downloading'
  | 'paused'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'error'

export interface Setting {
  key: string
  value: string
}

export interface AutoClassifyRule {
  id: number
  rule_type: 'site' | 'format' | 'date'
  pattern: string
  target_dir: string
  priority: number
  enabled: boolean
}

// === yt-dlp Types ===

export interface VideoInfo {
  title: string
  channel: string
  channel_id: string | null
  channel_url: string | null
  site: string
  thumbnail_url: string | null
  channel_avatar_url: string | null
  duration: number | null
  subtitle_languages: string[]
  auto_subtitle_languages: string[]
  formats: FormatInfo[]
}

export interface FormatInfo {
  format_id: string
  ext: string
  resolution: string | null
  filesize: number | null
  vcodec: string | null
  acodec: string | null
  quality_label: string | null
}

export type PlaylistMode = 'single' | 'all'

export interface DownloadOptions {
  format: string
  quality: string
  output_dir: string
  embed_thumbnail: boolean
  embed_metadata: boolean
  write_subs: boolean
  embed_subs: boolean
  embed_chapters: boolean
  sponsorblock: boolean
  custom_format: string | null
  playlist_mode: PlaylistMode
  // Advanced yt-dlp options
  restrict_filenames: boolean
  no_overwrites: boolean
  geo_bypass: boolean
  rate_limit: string
  sub_lang: string
  convert_subs: string
  merge_output_format: string
  recode_video: string
  retries: number
  proxy: string
  extra_args: string
}

export interface DownloadProgress {
  download_id: number
  percent: number
  speed_bps: number
  speed_str: string
  eta_secs: number
  eta_str: string
  downloaded_bytes: number
  total_bytes: number | null
  status: 'downloading' | 'paused' | 'post_processing' | 'completed' | 'error'
  playlist_index?: number
  playlist_count?: number
}

export interface YtdlpInfo {
  path: string
  version: string
  update_available: boolean
  managed_by: 'homebrew' | 'bundled' | 'manual'
}

// === UI Types ===

export type ViewMode = 'list' | 'grid' | 'column'

export type SidebarSection =
  | 'downloads-active'
  | 'downloads-completed'
  | 'library-all'
  | 'library-video'
  | 'library-audio'
  | 'images-download'
  | 'images-gallery'
  | 'schedules'
  | 'settings'

export interface AppSettings {
  download_dir: string
  filename_template: string
  concurrent_downloads: number
  default_video_format: string
  default_video_quality: string
  default_audio_format: string
  embed_thumbnail: boolean
  embed_metadata: boolean
  write_subs: boolean
  embed_subs: boolean
  embed_chapters: boolean
  sponsorblock: boolean
  cookie_browser: string
  cookie_file: string
  ytdlp_path: string
  theme: 'system' | 'light' | 'dark'
  auto_classify: boolean
  // Appearance
  background_image_light: string
  background_image_dark: string
  background_opacity: number
  // Advanced yt-dlp options
  restrict_filenames: boolean
  no_overwrites: boolean
  geo_bypass: boolean
  rate_limit: string
  sub_lang: string
  convert_subs: string
  merge_output_format: string
  recode_video: string
  retries: number
  proxy: string
  extra_args: string
}

// Image Download Feature
export interface ScrapedImage {
  url: string
  width: number | null
  height: number | null
  alt: string | null
}

export interface ImageToDownload {
  url: string
  filename_hint: string | null
}

export interface ImageSession {
  id: number
  source_url: string
  site_name: string | null
  image_count: number
  output_dir: string
  created_at: string
}

export interface ImageRecord {
  id: number
  session_id: number
  original_url: string
  file_path: string | null
  filename: string | null
  width: number | null
  height: number | null
  file_size: number | null
  format: string | null
  status: 'pending' | 'downloading' | 'completed' | 'failed'
  created_at: string
}

export interface ImageDownloadProgress {
  session_id: number
  image_index: number
  total_images: number
  current_url: string
  percent: number
  status: 'downloading' | 'completed' | 'failed'
  error_message: string | null
}

export interface UrlHistoryEntry {
  id: number
  url: string
  created_at: string
}

export interface Schedule {
  id: number
  name: string
  url: string
  cron_expr: string
  options_json: string
  is_active: boolean
  is_channel: boolean
  last_error: string | null
  fail_count: number
  is_running: boolean
  last_run_at: string | null
  next_run_at: string | null
  last_run_status: 'completed' | 'no_new' | 'stopped' | null
  created_at: string
}

export interface Preset {
  id: number
  name: string
  format: string
  quality: string
  output_dir: string       // snake_case: matches Rust serde output
  embed_thumbnail: boolean
  embed_metadata: boolean
  write_subs: boolean
  embed_subs: boolean
  embed_chapters: boolean
  sponsorblock: boolean
  created_at: string
}
