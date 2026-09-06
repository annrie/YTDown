use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum ManagedBy {
    Homebrew,
    Bundled,
    PackageManager,
    Manual,
}

#[derive(Debug)]
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

/// Normalize the raw `ytdlp_path` setting value.
/// `None`, "auto", empty, or whitespace-only mean auto-detect. Only a leading "~/" is
/// expanded (`~` alone or `~user/...` are passed through untouched).
pub fn manual_path_from_setting(raw: Option<&str>) -> Option<String> {
    let trimmed = raw?.trim();
    if trimmed.is_empty() || trimmed == "auto" {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return Some(home.join(rest).to_string_lossy().to_string());
        }
    }
    Some(trimmed.to_string())
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

    // 2. System PATH (using `which` crate for cross-platform support).
    // A broken binary on PATH falls through to well-known paths / bundled.
    if let Ok(path) = which::which("yt-dlp") {
        match get_version(&path) {
            Ok(version) => {
                let path_str = path.to_string_lossy().to_string();
                let managed_by = classify_managed_by(&path_str);
                return Ok(YtdlpBinary {
                    path,
                    version,
                    managed_by,
                });
            }
            Err(e) => eprintln!("[YTDown] yt-dlp on PATH is unusable, falling back: {e}"),
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

    // Contract: the frontend (DownloadDialog `isYtdlpMissing`) matches the "yt-dlp not found"
    // prefix to offer the bundled install. Keep the prefix stable.
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

/// Upper bound for the stderr excerpt included in `get_version` errors (shown in the UI).
const MAX_STDERR_DETAIL_CHARS: usize = 300;

/// How long `yt-dlp --version` may take before the probe is abandoned. A user-configured
/// wrapper that blocks must not hang every command that resolves the binary.
const VERSION_PROBE_TIMEOUT: Duration = Duration::from_secs(15);

/// After the direct child has exited, how long descendants may keep the pipes open before
/// the whole group is taken down. A real yt-dlp never leaves background jobs behind, so
/// this only matters for sloppy wrapper scripts.
const PIPE_GRACE_AFTER_EXIT: Duration = Duration::from_secs(1);

fn get_version(path: &PathBuf) -> Result<String, String> {
    get_version_with_timeout(path, VERSION_PROBE_TIMEOUT)
}

fn get_version_with_timeout(path: &PathBuf, timeout: Duration) -> Result<String, String> {
    let mut cmd = Command::new(path);
    cmd.arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Give the probe its own process group so a wrapper script's descendants can be taken
    // down with it (they would otherwise keep our pipes open indefinitely).
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to run yt-dlp at {}: {}", path.display(), e))?;
    let deadline = Instant::now() + timeout;

    // Drain both pipes on helper threads so a chatty child can't stall on a full pipe
    // while we poll for exit.
    let stdout_rx = spawn_pipe_reader(child.stdout.take());
    let stderr_rx = spawn_pipe_reader(child.stderr.take());

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Ok(None) => {
                kill_process_tree(&mut child);
                return Err(format!(
                    "yt-dlp at {} did not answer --version within {:?}",
                    path.display(),
                    timeout
                ));
            }
            Err(e) => {
                kill_process_tree(&mut child);
                return Err(format!(
                    "Failed to wait for yt-dlp at {}: {}",
                    path.display(),
                    e
                ));
            }
        }
    };
    let stdout = collect_pipe(&stdout_rx, &mut child, path)?;
    let stderr = collect_pipe(&stderr_rx, &mut child, path)?;

    if !status.success() {
        // Keep the message short: the last non-empty stderr line is usually the real cause
        // (Python tracebacks end with the exception), capped so it stays readable in the UI.
        let detail: String = stderr
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .map(|line| line.trim().chars().take(MAX_STDERR_DETAIL_CHARS).collect())
            .unwrap_or_default();
        return Err(if detail.is_empty() {
            format!("yt-dlp at {} failed ({})", path.display(), status)
        } else {
            format!("yt-dlp at {} failed ({}): {}", path.display(), status, detail)
        });
    }
    let version = stdout.trim().to_string();
    if version.is_empty() {
        return Err(format!("yt-dlp at {} returned an empty version", path.display()));
    }
    Ok(version)
}

fn spawn_pipe_reader<R: Read + Send + 'static>(pipe: Option<R>) -> Receiver<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut buf);
        }
        let _ = tx.send(String::from_utf8_lossy(&buf).into_owned());
    });
    rx
}

/// Collect a pipe reader's output once the direct child has exited. If a descendant the
/// wrapper left behind still holds the pipe open, kill the whole group and drain what
/// was written before that.
fn collect_pipe(
    rx: &Receiver<String>,
    child: &mut Child,
    path: &PathBuf,
) -> Result<String, String> {
    if let Ok(output) = rx.recv_timeout(PIPE_GRACE_AFTER_EXIT) {
        return Ok(output);
    }
    kill_process_tree(child);
    rx.recv_timeout(PIPE_GRACE_AFTER_EXIT).map_err(|_| {
        format!(
            "yt-dlp at {} left its --version output open beyond {:?} after exiting",
            path.display(),
            PIPE_GRACE_AFTER_EXIT
        )
    })
}

/// Kill the probe and, on unix, everything in its process group. Also reaps the child so
/// no zombie is left behind. (On Windows only the direct child is killed.)
fn kill_process_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        // `process_group(0)` made the child the group leader, so -pid addresses the group.
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_path_none_for_auto_empty_and_whitespace() {
        assert_eq!(manual_path_from_setting(None), None);
        assert_eq!(manual_path_from_setting(Some("auto")), None);
        assert_eq!(manual_path_from_setting(Some("")), None);
        assert_eq!(manual_path_from_setting(Some("   ")), None);
    }

    #[test]
    fn manual_path_trims_and_keeps_absolute_path() {
        assert_eq!(
            manual_path_from_setting(Some("  /usr/local/bin/yt-dlp \n")),
            Some("/usr/local/bin/yt-dlp".to_string())
        );
    }

    #[test]
    fn manual_path_expands_home_prefix() {
        let home = dirs::home_dir().expect("home dir");
        let expected = home.join(".local/bin/yt-dlp").to_string_lossy().to_string();
        assert_eq!(manual_path_from_setting(Some("~/.local/bin/yt-dlp")), Some(expected));
    }

    /// Executable shell script in its own temp dir, removed on drop so test runs leave nothing
    /// behind. One dir per script keeps parallel tests from racing on a shared directory.
    #[cfg(unix)]
    struct TempScript(PathBuf);

    #[cfg(unix)]
    impl Drop for TempScript {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
            if let Some(dir) = self.0.parent() {
                let _ = std::fs::remove_dir(dir);
            }
        }
    }

    #[cfg(unix)]
    fn write_script(name: &str, body: &str) -> TempScript {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!(
            "ytdown-binary-test-{}-{}",
            std::process::id(),
            name
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        TempScript(path)
    }

    #[cfg(unix)]
    #[test]
    fn get_version_returns_trimmed_stdout_on_success() {
        let script = write_script("ok.sh", "#!/bin/sh\necho 2026.01.01\n");
        assert_eq!(get_version(&script.0).unwrap(), "2026.01.01");
    }

    #[cfg(unix)]
    #[test]
    fn get_version_errors_on_nonzero_exit_and_includes_stderr() {
        let script = write_script("fail.sh", "#!/bin/sh\necho 'command not found' >&2\nexit 127\n");
        let err = get_version(&script.0).unwrap_err();
        assert!(err.contains("failed ("), "unexpected error: {err}");
        assert!(err.contains("command not found"), "stderr missing: {err}");
    }

    #[cfg(unix)]
    #[test]
    fn get_version_error_keeps_only_last_stderr_line() {
        let script = write_script(
            "traceback.sh",
            "#!/bin/sh\necho 'Traceback (most recent call last):' >&2\necho '  File x' >&2\necho \"ModuleNotFoundError: No module named 'yt_dlp'\" >&2\nexit 1\n",
        );
        let err = get_version(&script.0).unwrap_err();
        assert!(err.contains("ModuleNotFoundError"), "last line missing: {err}");
        assert!(!err.contains("Traceback"), "earlier lines should be dropped: {err}");
    }

    #[cfg(unix)]
    #[test]
    fn get_version_error_omits_detail_when_stderr_is_empty() {
        let script = write_script("silent-fail.sh", "#!/bin/sh\nexit 3\n");
        let err = get_version(&script.0).unwrap_err();
        assert!(err.contains("failed ("), "unexpected error: {err}");
        assert!(err.ends_with(')'), "should not end with a dangling separator: {err}");
    }

    #[cfg(unix)]
    #[test]
    fn get_version_times_out_on_hanging_binary() {
        let script = write_script("hang.sh", "#!/bin/sh\nexec sleep 30\n");
        let started = Instant::now();
        let err = get_version_with_timeout(&script.0, Duration::from_millis(300)).unwrap_err();
        assert!(err.contains("did not answer"), "unexpected error: {err}");
        assert!(started.elapsed() < Duration::from_secs(5), "timeout was not enforced");
    }

    #[cfg(unix)]
    #[test]
    fn get_version_is_bounded_when_a_descendant_keeps_the_pipe_open() {
        // The wrapper exits at once but leaves a background job holding stdout/stderr.
        // The direct child's budget is the production one (its first exec can take seconds
        // on a loaded macOS); the point is that we never wait for the 30s background job.
        let script = write_script("bg.sh", "#!/bin/sh\nsleep 30 &\necho 2026.01.01\nexit 0\n");
        let started = Instant::now();
        let version = get_version_with_timeout(&script.0, VERSION_PROBE_TIMEOUT).unwrap();
        assert_eq!(version, "2026.01.01");
        assert!(started.elapsed() < Duration::from_secs(20), "pipe wait was not bounded");
    }

    #[cfg(unix)]
    #[test]
    fn get_version_errors_on_empty_output() {
        let script = write_script("empty.sh", "#!/bin/sh\nexit 0\n");
        let err = get_version(&script.0).unwrap_err();
        assert!(err.contains("empty version"), "unexpected error: {err}");
    }

    #[cfg(unix)]
    #[test]
    fn detect_binary_uses_manual_path() {
        let script = write_script("manual.sh", "#!/bin/sh\necho manual-1.0\n");
        let bin = detect_binary(Some(script.0.to_str().unwrap())).unwrap();
        assert_eq!(bin.path, script.0);
        assert_eq!(bin.version, "manual-1.0");
        assert!(matches!(bin.managed_by, ManagedBy::Manual));
    }

    #[test]
    fn detect_binary_errors_when_manual_path_missing() {
        let err = detect_binary(Some("/nonexistent/ytdown/yt-dlp")).unwrap_err();
        assert!(err.contains("Manual yt-dlp path not found"), "unexpected error: {err}");
    }
}
