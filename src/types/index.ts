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

export interface Playlist {
  id: number
  name: string
  description: string | null
  created_at: string
  updated_at: string
}

export interface PlaylistItem {
  id: number
  playlist_id: number
  download_id: number | null
  url: string | null
  sort_order: number
  added_at: string
}

export interface UrlList {
  id: number
  name: string
  created_at: string
  updated_at: string
}

export interface UrlListItem {
  id: number
  list_id: number
  url: string
  title: string | null
  sort_order: number
  added_at: string
}

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
  duration: number | null
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
  | 'playlist'
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
