use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
pub enum ManagedBy {
    Homebrew,
    Bundled,
    PackageManager,
    Manual,
}

pub struct YtdlpBinary {
    pub path: PathBuf,
    pub version: String,
    pub managed_by: ManagedBy,
}

/// Binary name varies by platform
fn binary_name() -> &'static str {
    if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    }
}

/// Classify how the binary is managed based on its path
fn classify_managed_by(path_str: &str) -> ManagedBy {
    if path_str.contains("Cellar") || path_str.contains("homebrew") || path_str.contains("Homebrew")
    {
        ManagedBy::Homebrew
    } else if cfg!(windows) && (path_str.contains("chocolatey") || path_str.contains("scoop")) {
        ManagedBy::PackageManager
    } else if cfg!(target_os = "linux")
        && (path_str.starts_with("/usr/bin/") || path_str.starts_with("/usr/local/bin/"))
    {
        ManagedBy::PackageManager
    } else {
        ManagedBy::Manual
    }
}

/// Detect yt-dlp binary following priority: manual path > system PATH > well-known paths > bundled
pub fn detect_binary(manual_path: Option<&str>) -> Result<YtdlpBinary, String> {
    // 1. Manual path
    if let Some(path) = manual_path {
        if path != "auto" {
            let pb = PathBuf::from(path);
            if pb.exists() {
                let version = get_version(&pb)?;
                return Ok(YtdlpBinary {
                    path: pb,
                    version,
                    managed_by: ManagedBy::Manual,
                });
            }
            return Err(format!("Manual yt-dlp path not found: {}", path));
        }
    }

    // 2. System PATH (using `which` crate for cross-platform support)
    if let Ok(path) = which::which("yt-dlp") {
        if path.exists() {
            let path_str = path.to_string_lossy().to_string();
            let version = get_version(&path)?;
            let managed_by = classify_managed_by(&path_str);
            return Ok(YtdlpBinary {
                path,
                version,
                managed_by,
            });
        }
    }

    // 3. Well-known paths (GUI apps don't inherit shell PATH)
    for known_path in well_known_paths() {
        let pb = PathBuf::from(&known_path);
        if pb.exists() {
            if let Ok(version) = get_version(&pb) {
                let managed_by = classify_managed_by(&known_path);
                return Ok(YtdlpBinary {
                    path: pb,
                    version,
                    managed_by,
                });
            }
        }
    }

    // 4. Bundled
    let bundled = bundled_binary_path();
    if bundled.exists() {
        let version = get_version(&bundled)?;
        return Ok(YtdlpBinary {
            path: bundled,
            version,
            managed_by: ManagedBy::Bundled,
        });
    }

    Err("yt-dlp not found. Use the install button or install manually.".to_string())
}

/// Well-known installation paths per platform
fn well_known_paths() -> Vec<String> {
    let name = binary_name();

    if cfg!(target_os = "macos") {
        vec![
            format!("/usr/local/bin/{}", name),
            format!("/opt/homebrew/bin/{}", name),
            format!("/usr/bin/{}", name),
        ]
    } else if cfg!(windows) {
        let mut paths = vec![
            format!("C:\\Program Files\\yt-dlp\\{}", name),
            format!("C:\\ProgramData\\chocolatey\\bin\\{}", name),
        ];
        // %LOCALAPPDATA%\Microsoft\WinGet\Packages (winget)
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            paths.push(format!("{}\\Microsoft\\WinGet\\Links\\{}", local, name));
        }
        // ~/scoop/shims/yt-dlp.exe
        if let Some(home) = dirs::home_dir() {
            paths.push(format!("{}\\scoop\\shims\\{}", home.display(), name));
        }
        paths
    } else {
        // Linux
        vec![
            format!("/usr/local/bin/{}", name),
            format!("/usr/bin/{}", name),
            format!("/snap/bin/{}", name),
        ]
    }
}

/// Path where the bundled binary is stored
fn bundled_binary_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("YTDown")
        .join("bin")
        .join(binary_name())
}

fn get_version(path: &PathBuf) -> Result<String, String> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Download URL for the current platform
fn download_url() -> &'static str {
    if cfg!(target_os = "macos") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
    } else if cfg!(windows) {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    } else {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux"
    }
}

/// Download yt-dlp binary to app's bundled location
pub fn download_ytdlp_binary() -> Result<PathBuf, String> {
    let target_path = bundled_binary_path();
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create bin dir: {}", e))?;
    }

    let url = download_url();
    let response = reqwest::blocking::get(url).map_err(|e| format!("Download failed: {}", e))?;
    let bytes = response
        .bytes()
        .map_err(|e| format!("Failed to read response: {}", e))?;

    std::fs::write(&target_path, &bytes).map_err(|e| format!("Failed to write binary: {}", e))?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&target_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    Ok(target_path)
}

/// Fetch the latest yt-dlp version tag from GitHub releases API
pub fn fetch_latest_github_version() -> Result<String, String> {
    let response = reqwest::blocking::Client::new()
        .get("https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest")
        .header("User-Agent", "YTDown")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .map_err(|e| format!("Failed to fetch GitHub releases: {}", e))?;

    let body = response
        .text()
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    json["tag_name"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "tag_name not found in GitHub response".to_string())
}
