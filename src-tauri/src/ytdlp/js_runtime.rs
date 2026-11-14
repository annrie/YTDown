//! yt-dlp が YouTube の n チャレンジ・署名を解くために使う JavaScript ランタイムの検出。
//!
//! yt-dlp は既定で deno だけを有効にする。deno が無い環境では解読に失敗して
//! 「Only images are available」「Requested format is not available」になるため、
//! アプリ側で deno → node の順に探して `--js-runtimes` を渡す。
//! GUI アプリはシェルの PATH を引き継がないので、バージョン管理ツールの置き場所も探す。
//!
//! 次の場合は何も足さない（yt-dlp の既定の動作に任せる）。
//! - 利用者が追加引数で `--js-runtimes` / `--no-js-runtimes` を指定している
//! - 使っている yt-dlp が `--js-runtimes` を知らない（古い版に未知のオプションを渡して壊さないため）
//! - 最低版（Deno 2.3.0 / Node 22.0.0）を満たすランタイムが見つからない

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// yt-dlp 公式の EJS ガイドが定める最低版。
const MIN_DENO: (u64, u64, u64) = (2, 3, 0);
const MIN_NODE: (u64, u64, u64) = (22, 0, 0);

/// 検出結果の保持時間。起動のたびにプロセスを起動しファイルを走査しないためのもの。
/// 長すぎると、入れ替えた直後に消えたパスを渡してしまうので短くする。
const CACHE_TTL: Duration = Duration::from_secs(60);

/// 先頭に JS ランタイム引数を差し込む。URL などの位置引数より前に置くため。
pub async fn prepend_js_runtime_args(ytdlp_path: &str, args: &mut Vec<String>) {
    let extra = js_runtime_args(ytdlp_path, args).await;
    if !extra.is_empty() {
        args.splice(0..0, extra);
    }
}

/// yt-dlp の起動引数に足す `--js-runtimes` を返す。足す必要がなければ空。
pub async fn js_runtime_args(ytdlp_path: &str, existing: &[String]) -> Vec<String> {
    if has_js_runtime_option(existing) {
        return Vec::new();
    }
    let path = ytdlp_path.to_string();
    // プロセス起動とファイル走査は同期処理なので、非同期ランタイムを止めないよう別スレッドで行う
    tokio::task::spawn_blocking(move || detect_cached(&path))
        .await
        .unwrap_or_default()
}

fn has_js_runtime_option(args: &[String]) -> bool {
    args.iter().any(|arg| {
        arg == "--js-runtimes" || arg == "--no-js-runtimes" || arg.starts_with("--js-runtimes=")
    })
}

type Cache = Mutex<HashMap<String, (Instant, Vec<String>)>>;

fn detect_cached(ytdlp_path: &str) -> Vec<String> {
    static CACHE: std::sync::OnceLock<Cache> = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    if let Ok(guard) = cache.lock() {
        if let Some((at, args)) = guard.get(ytdlp_path) {
            if at.elapsed() < CACHE_TTL {
                return args.clone();
            }
        }
    }
    let args = detect(ytdlp_path);
    if let Ok(mut guard) = cache.lock() {
        guard.insert(ytdlp_path.to_string(), (Instant::now(), args.clone()));
    }
    args
}

fn detect(ytdlp_path: &str) -> Vec<String> {
    if !ytdlp_supports_js_runtimes(ytdlp_path) {
        return Vec::new();
    }
    let home = dirs::home_dir();
    match find_runtime(&search_dirs(), home.as_deref(), &runtime_is_usable) {
        Some((name, path)) => vec![
            "--js-runtimes".to_string(),
            format!("{}:{}", name, path.display()),
        ],
        None => Vec::new(),
    }
}

/// `yt-dlp --help` に `--js-runtimes` があるか。版の番号ではなく、実際のオプション定義で判断する。
fn ytdlp_supports_js_runtimes(ytdlp_path: &str) -> bool {
    let output = Command::new(ytdlp_path)
        .arg("--help")
        .env("PATH", super::process::augmented_path_env())
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).contains("--js-runtimes"),
        Err(_) => false,
    }
}

/// 実際の PATH に、GUI アプリでは抜けがちな macOS の標準の場所を足したもの。
/// 区切り文字はプラットフォームごとに `split_paths` に任せる（Windows は ';'）。
fn search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    #[cfg(not(windows))]
    for extra in ["/usr/local/bin", "/opt/homebrew/bin", "/usr/bin"] {
        let extra = PathBuf::from(extra);
        if !dirs.contains(&extra) {
            dirs.push(extra);
        }
    }
    dirs
}

/// 実行ファイルに `--version` を問い合わせ、最低版を満たすか確かめる。
fn runtime_is_usable(name: &str, path: &Path) -> bool {
    let output = Command::new(path)
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    let Ok(out) = output else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    parse_runtime_version(&String::from_utf8_lossy(&out.stdout))
        .is_some_and(|version| meets_minimum(name, version))
}

fn meets_minimum(name: &str, version: (u64, u64, u64)) -> bool {
    match name {
        "deno" => version >= MIN_DENO,
        "node" => version >= MIN_NODE,
        _ => false,
    }
}

/// "v22.23.2"（node）や "deno 2.3.0 (stable, release, aarch64-apple-darwin)"（deno）の先頭行から
/// 版を取り出す。数字で始まる最初の語を使う。
fn parse_runtime_version(output: &str) -> Option<(u64, u64, u64)> {
    let first_line = output.lines().next()?;
    let token = first_line
        .split_whitespace()
        .map(|t| t.strip_prefix('v').unwrap_or(t))
        .find(|t| t.starts_with(|c: char| c.is_ascii_digit()))?;
    let mut parts = token.split('.').map(|part| {
        let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse::<u64>().ok()
    });
    let major = parts.next()??;
    let minor = parts.next().flatten().unwrap_or(0);
    let patch = parts.next().flatten().unwrap_or(0);
    Some((major, minor, patch))
}

/// 優先順（deno → node）で、`is_usable` を満たす最初の実体を探す。
fn find_runtime(
    path_dirs: &[PathBuf],
    home: Option<&Path>,
    is_usable: &dyn Fn(&str, &Path) -> bool,
) -> Option<(&'static str, PathBuf)> {
    for name in ["deno", "node"] {
        let found = candidates(name, path_dirs, home)
            .into_iter()
            .find(|path| is_usable(name, path));
        if let Some(path) = found {
            return Some((name, path));
        }
    }
    None
}

/// 探す場所の優先順に並べた候補（存在し、実行可能なものだけ）。
fn candidates(base: &str, path_dirs: &[PathBuf], home: Option<&Path>) -> Vec<PathBuf> {
    // Windows では deno.exe / node.exe。macOS・Linux では接尾辞は空
    let name = format!("{}{}", base, std::env::consts::EXE_SUFFIX);
    let mut found: Vec<PathBuf> = Vec::new();
    let mut push = |path: PathBuf| {
        if is_executable_file(&path) && !found.contains(&path) {
            found.push(path);
        }
    };

    // 1. PATH。nodenv / pyenv / asdf の shim はシェルスクリプトなので除く（GUI からは動かない）
    for dir in path_dirs.iter().filter(|dir| !is_shim_dir(dir)) {
        push(dir.join(&name));
    }

    let Some(home) = home else {
        return found;
    };

    // 2. 公式インストーラなどの固定の置き場所
    for dir in [".deno/bin", ".volta/bin", ".local/bin"] {
        push(home.join(dir).join(&name));
    }

    // 3. バージョン管理ツールの実体（新しいバージョンから）。node のみ対象
    if base == "node" {
        for root in [
            home.join(".anyenv/envs/nodenv/versions"),
            home.join(".nodenv/versions"),
            home.join(".nvm/versions/node"),
            home.join(".local/share/fnm/node-versions"),
            home.join(".local/share/mise/installs/node"),
        ] {
            for path in versioned_binaries(&root, &name) {
                push(path);
            }
        }
    }
    found
}

fn is_shim_dir(dir: &Path) -> bool {
    dir.file_name().is_some_and(|n| n == "shims")
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// `root/<version>/bin/<name>`（fnm は `root/<version>/installation/bin/<name>`）を、
/// バージョンの新しい順に返す。
fn versioned_binaries(root: &Path, name: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut versions: Vec<(Vec<u64>, PathBuf)> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let dir = entry.path();
            let key = parse_dir_version(&entry.file_name().to_string_lossy())?;
            let bin = [dir.join("bin").join(name), dir.join("installation/bin").join(name)]
                .into_iter()
                .find(|candidate| is_executable_file(candidate))?;
            Some((key, bin))
        })
        .collect();
    versions.sort_by(|a, b| b.0.cmp(&a.0));
    versions.into_iter().map(|(_, path)| path).collect()
}

/// ディレクトリ名 "22.23.2" や "v22.23.2" を数値の並びにする。並べ替え専用。
/// 数字で始まらない名前（"system" など）は除く。
fn parse_dir_version(name: &str) -> Option<Vec<u64>> {
    let trimmed = name.strip_prefix('v').unwrap_or(name);
    let parts: Option<Vec<u64>> = trimmed.split('.').map(|part| part.parse().ok()).collect();
    parts.filter(|p| !p.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ytdown-jsrt-{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[cfg(unix)]
    fn make_exec(path: &Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(unix)]
    fn make_stub(path: &Path) {
        make_exec(path, "#!/bin/sh\n");
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn any(_: &str, _: &Path) -> bool {
        true
    }

    #[test]
    fn user_specified_js_runtimes_is_respected() {
        assert!(has_js_runtime_option(&strings(&["--js-runtimes", "node:/x/node"])));
        assert!(has_js_runtime_option(&strings(&["--js-runtimes=node"])));
        assert!(has_js_runtime_option(&strings(&["--no-js-runtimes"])));
        assert!(!has_js_runtime_option(&strings(&["--no-playlist"])));
    }

    #[test]
    fn parses_runtime_versions() {
        assert_eq!(parse_runtime_version("v22.23.2\n"), Some((22, 23, 2)));
        assert_eq!(
            parse_runtime_version("deno 2.3.0 (stable, release, aarch64-apple-darwin)\nv8 13.4\n"),
            Some((2, 3, 0))
        );
        assert_eq!(parse_runtime_version("deno 2.4.0-rc.1 (canary)"), Some((2, 4, 0)));
        assert_eq!(parse_runtime_version("garbage"), None);
        assert_eq!(parse_runtime_version(""), None);
    }

    #[test]
    fn enforces_minimum_versions() {
        assert!(meets_minimum("node", (22, 0, 0)));
        assert!(meets_minimum("node", (22, 23, 2)));
        assert!(!meets_minimum("node", (20, 19, 0)));
        assert!(!meets_minimum("node", (18, 20, 8)));
        assert!(meets_minimum("deno", (2, 3, 0)));
        assert!(!meets_minimum("deno", (2, 2, 9)));
        assert!(!meets_minimum("deno", (1, 46, 3)));
        assert!(!meets_minimum("bun", (1, 3, 0)));
    }

    #[test]
    fn dir_versions_sort_numerically() {
        assert_eq!(parse_dir_version("v22.23.2"), Some(vec![22, 23, 2]));
        assert_eq!(parse_dir_version("system"), None);
        assert!(parse_dir_version("9.0.0") < parse_dir_version("22.1.0"));
    }

    #[cfg(unix)]
    #[test]
    fn prefers_deno_over_node() {
        let bin = temp_dir("pref");
        make_stub(&bin.join("deno"));
        make_stub(&bin.join("node"));
        let (name, path) = find_runtime(std::slice::from_ref(&bin), None, &any).unwrap();
        assert_eq!(name, "deno");
        assert_eq!(path, bin.join("deno"));
        let _ = std::fs::remove_dir_all(&bin);
    }

    #[cfg(unix)]
    #[test]
    fn skips_shim_directories() {
        let root = temp_dir("shim");
        let shims = root.join("shims");
        make_stub(&shims.join("node"));
        assert!(find_runtime(&[shims], None, &any).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn tries_newest_version_first_and_falls_through_unusable_ones() {
        let home = temp_dir("nodenv");
        let versions = home.join(".anyenv/envs/nodenv/versions");
        make_stub(&versions.join("9.11.2/bin/node"));
        make_stub(&versions.join("22.23.2/bin/node"));
        make_stub(&versions.join("22.4.0/bin/node"));
        // バージョンの下に node が無いディレクトリは無視される
        std::fs::create_dir_all(versions.join("23.0.0/bin")).unwrap();

        let (_, newest) = find_runtime(&[], Some(&home), &any).unwrap();
        assert_eq!(newest, versions.join("22.23.2/bin/node"));

        // 22.23.2 が使えない（版不足など）なら、次の候補へ進む
        let reject_newest = |_: &str, p: &Path| p != versions.join("22.23.2/bin/node");
        let (_, next) = find_runtime(&[], Some(&home), &reject_newest).unwrap();
        assert_eq!(next, versions.join("22.4.0/bin/node"));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[test]
    fn unsupported_node_on_path_does_not_hide_a_managed_one() {
        let home = temp_dir("pathold");
        let bin = home.join("bin");
        let old = bin.join("node");
        make_stub(&old);
        let managed = home.join(".nvm/versions/node/v22.1.0/bin/node");
        make_stub(&managed);
        // PATH 上の node は版不足とみなす
        let reject_old = |_: &str, p: &Path| p != old;
        let (_, path) = find_runtime(&[bin], Some(&home), &reject_old).unwrap();
        assert_eq!(path, managed);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[test]
    fn finds_deno_in_official_installer_location() {
        let home = temp_dir("deno");
        make_stub(&home.join(".deno/bin/deno"));
        let (name, path) = find_runtime(&[], Some(&home), &any).unwrap();
        assert_eq!(name, "deno");
        assert_eq!(path, home.join(".deno/bin/deno"));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[test]
    fn runtime_is_usable_checks_real_version_output() {
        let dir = temp_dir("usable");
        let new = dir.join("node-new");
        let old = dir.join("node-old");
        let broken = dir.join("node-broken");
        make_exec(&new, "#!/bin/sh\necho v22.23.2\n");
        make_exec(&old, "#!/bin/sh\necho v18.20.4\n");
        make_exec(&broken, "#!/bin/sh\nexit 1\n");
        assert!(runtime_is_usable("node", &new));
        assert!(!runtime_is_usable("node", &old));
        assert!(!runtime_is_usable("node", &broken));
        assert!(!runtime_is_usable("node", &dir.join("missing")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn ytdlp_support_is_detected_from_help_output() {
        let dir = temp_dir("help");
        let new = dir.join("yt-dlp-new");
        let old = dir.join("yt-dlp-old");
        make_exec(&new, "#!/bin/sh\necho '    --js-runtimes RUNTIME[:PATH]  Additional JavaScript runtime'\n");
        make_exec(&old, "#!/bin/sh\necho '    --no-playlist  Download only the video'\n");
        assert!(ytdlp_supports_js_runtimes(&new.to_string_lossy()));
        assert!(!ytdlp_supports_js_runtimes(&old.to_string_lossy()));
        assert!(!ytdlp_supports_js_runtimes(&dir.join("missing").to_string_lossy()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn returns_none_when_nothing_found() {
        let home = temp_dir("empty");
        assert!(find_runtime(&[], Some(&home), &any).is_none());
        let _ = std::fs::remove_dir_all(&home);
    }
}
