use crate::state::AppState;
use crate::ytdlp::binary;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct YtdlpInfo {
    pub path: String,
    pub version: String,
    pub update_available: bool,
    pub latest_version: Option<String>,
    pub managed_by: String,
}

fn managed_by_label(managed_by: &binary::ManagedBy) -> &'static str {
    match managed_by {
        binary::ManagedBy::Homebrew => "homebrew",
        binary::ManagedBy::Bundled => "bundled",
        binary::ManagedBy::PackageManager => "package_manager",
        binary::ManagedBy::Manual => "manual",
    }
}

/// Get current yt-dlp binary info (fast, no network call)
#[tauri::command]
pub async fn get_ytdlp_info(state: State<'_, AppState>) -> Result<YtdlpInfo, String> {
    let bin = state.resolve_ytdlp_binary().await?;
    Ok(YtdlpInfo {
        path: bin.path.to_string_lossy().to_string(),
        version: bin.version,
        update_available: false,
        latest_version: None,
        managed_by: managed_by_label(&bin.managed_by).to_string(),
    })
}

/// Check for yt-dlp updates via GitHub releases API (network call)
/// Returns Some(latest_version) if update available, None if up to date
#[tauri::command]
pub async fn check_ytdlp_update(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let current = state.resolve_ytdlp_binary().await?.version;
    tokio::task::spawn_blocking(move || {
        let latest = binary::fetch_latest_github_version()?;
        if latest != current {
            Ok(Some(latest))
        } else {
            Ok(None)
        }
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

#[tauri::command]
pub async fn install_ytdlp(state: State<'_, AppState>) -> Result<YtdlpInfo, String> {
    let path = tokio::task::spawn_blocking(|| binary::download_ytdlp_binary())
        .await
        .map_err(|e| format!("Task failed: {}", e))??;

    // Re-detect to get full info (a configured manual path still wins, so report the truth)
    let bin = state
        .resolve_ytdlp_binary()
        .await
        .map_err(|_| format!("Installed but failed to detect at: {}", path.display()))?;

    Ok(YtdlpInfo {
        path: bin.path.to_string_lossy().to_string(),
        version: bin.version,
        update_available: false,
        latest_version: None,
        managed_by: managed_by_label(&bin.managed_by).to_string(),
    })
}

/// Update yt-dlp. For bundled: auto-downloads latest. For others: returns helpful message as Err.
#[tauri::command]
pub async fn update_ytdlp(state: State<'_, AppState>) -> Result<String, String> {
    let bin = state.resolve_ytdlp_binary().await?;

    match bin.managed_by {
        binary::ManagedBy::Homebrew => Err(
            "homebrew管理のyt-dlpです。ターミナルで `brew upgrade yt-dlp` を実行してください。"
                .to_string(),
        ),
        binary::ManagedBy::PackageManager => {
            Err("パッケージマネージャ管理のyt-dlpです。手動で更新してください。".to_string())
        }
        binary::ManagedBy::Bundled => {
            tokio::task::spawn_blocking(|| {
                binary::download_ytdlp_binary()?;
                let new_bin = binary::detect_binary(None)?;
                Ok(new_bin.version)
            })
            .await
            .map_err(|e| format!("Task failed: {}", e))?
        }
        binary::ManagedBy::Manual => {
            Err("手動インストールのyt-dlpです。手動で更新してください。".to_string())
        }
    }
}
