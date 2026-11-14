//! yt-dlp が YouTube の n チャレンジ・署名を解くために使う JavaScript ランタイムの検出。
//!
//! yt-dlp は既定で deno だけを有効にする。deno が無い環境では解読に失敗して
//! 「Only images are available」「Requested format is not available」になるため、
//! アプリ側で deno → node の順に探して `--js-runtimes` を渡す。
//! GUI アプリはシェルの PATH を引き継がないので、バージョン管理ツールの置き場所も探す。

use std::path::{Path, PathBuf};

/// yt-dlp の起動引数に足す `--js-runtimes` を返す。
/// 利用者が自分で `--js-runtimes` / `--no-js-runtimes` を指定済みなら、何も足さない（空を返す）。
/// 見つからないときも空を返す（yt-dlp の既定の動作に任せる）。
pub fn js_runtime_args(existing: &[String]) -> Vec<String> {
    if has_js_runtime_option(existing) {
        return Vec::new();
    }
    detect_cached()
}

/// 検出結果の保持時間。起動のたびにファイルシステムを走査しないためのもの。
/// 長すぎると、node を入れ替えた直後に消えたパスを渡してしまうので短くする。
const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(60);

fn detect_cached() -> Vec<String> {
    use std::sync::Mutex;
    use std::time::Instant;
    static CACHE: Mutex<Option<(Instant, Vec<String>)>> = Mutex::new(None);

    if let Ok(guard) = CACHE.lock() {
        if let Some((at, args)) = guard.as_ref() {
            if at.elapsed() < CACHE_TTL {
                return args.clone();
            }
        }
    }
    let args = detect();
    if let Ok(mut guard) = CACHE.lock() {
        *guard = Some((Instant::now(), args.clone()));
    }
    args
}

fn detect() -> Vec<String> {
    let home = dirs::home_dir();
    match find_runtime(&search_dirs(), home.as_deref()) {
        Some((name, path)) => vec![
            "--js-runtimes".to_string(),
            format!("{}:{}", name, path.display()),
        ],
        None => Vec::new(),
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

/// 先頭に JS ランタイム引数を差し込む。URL などの位置引数より前に置くため。
pub fn prepend_js_runtime_args(args: &mut Vec<String>) {
    let extra = js_runtime_args(args);
    if !extra.is_empty() {
        args.splice(0..0, extra);
    }
}

fn has_js_runtime_option(args: &[String]) -> bool {
    args.iter().any(|arg| {
        arg == "--js-runtimes"
            || arg == "--no-js-runtimes"
            || arg.starts_with("--js-runtimes=")
    })
}

/// 優先順（deno → node）で、実行できる実体を探す。
fn find_runtime(path_dirs: &[PathBuf], home: Option<&Path>) -> Option<(&'static str, PathBuf)> {
    for name in ["deno", "node"] {
        if let Some(path) = find_executable(name, path_dirs, home) {
            return Some((name, path));
        }
    }
    None
}

fn find_executable(base: &str, path_dirs: &[PathBuf], home: Option<&Path>) -> Option<PathBuf> {
    // Windows では deno.exe / node.exe。macOS・Linux では接尾辞は空
    let exe_name = format!("{}{}", base, std::env::consts::EXE_SUFFIX);
    let name = exe_name.as_str();
    // 1. PATH。nodenv / pyenv / asdf の shim はシェルスクリプトなので除く（GUI からは動かない）
    for dir in path_dirs {
        if is_shim_dir(dir) {
            continue;
        }
        let candidate = dir.join(name);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
    }

    let home = home?;

    // 2. 公式インストーラなどの固定の置き場所
    let fixed = [
        home.join(".deno/bin"),
        home.join(".volta/bin"),
        home.join(".local/bin"),
    ];
    for dir in &fixed {
        let candidate = dir.join(name);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
    }

    // 3. バージョン管理ツールの実体（最新バージョンを選ぶ）。node のみ対象
    if base == "node" {
        let version_roots = [
            home.join(".anyenv/envs/nodenv/versions"),
            home.join(".nodenv/versions"),
            home.join(".nvm/versions/node"),
            home.join(".local/share/fnm/node-versions"),
            home.join(".local/share/mise/installs/node"),
        ];
        for root in &version_roots {
            if let Some(path) = newest_version_binary(root, name) {
                return Some(path);
            }
        }
    }
    None
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

/// バージョン管理ツールの実体から選ぶときの、Node の最低メジャーバージョン。
const MIN_NODE_MAJOR: u64 = 20;

/// `root/<version>/bin/<name>` のうち、バージョンが最も新しいものを返す。
/// fnm は `root/<version>/installation/bin/<name>` なので、そちらも見る。
fn newest_version_binary(root: &Path, name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(root).ok()?;
    let mut versions: Vec<(Vec<u64>, PathBuf)> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let dir = entry.path();
            let key = parse_version(&entry.file_name().to_string_lossy())?;
            // yt-dlp の JS チャレンジの解読は、古い Node では動かない
            if key.first().is_none_or(|major| *major < MIN_NODE_MAJOR) {
                return None;
            }
            let bin = [dir.join("bin").join(name), dir.join("installation/bin").join(name)]
                .into_iter()
                .find(|candidate| is_executable_file(candidate))?;
            Some((key, bin))
        })
        .collect();
    versions.sort_by(|a, b| a.0.cmp(&b.0));
    versions.pop().map(|(_, path)| path)
}

/// "22.23.2" や "v22.23.2" を数値の並びにする。数字で始まらない名前（"system" など）は除く。
fn parse_version(name: &str) -> Option<Vec<u64>> {
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
    fn make_exec(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn user_specified_js_runtimes_is_respected() {
        assert!(js_runtime_args(&strings(&["--js-runtimes", "node:/x/node"])).is_empty());
        assert!(js_runtime_args(&strings(&["--js-runtimes=node"])).is_empty());
        assert!(js_runtime_args(&strings(&["--no-js-runtimes"])).is_empty());
    }

    #[test]
    fn parses_versions_numerically() {
        assert_eq!(parse_version("v22.23.2"), Some(vec![22, 23, 2]));
        assert_eq!(parse_version("9.0.0"), Some(vec![9, 0, 0]));
        assert_eq!(parse_version("system"), None);
        assert!(parse_version("9.0.0") < parse_version("22.1.0"));
    }

    #[cfg(unix)]
    #[test]
    fn prefers_deno_over_node() {
        let bin = temp_dir("pref");
        make_exec(&bin.join("deno"));
        make_exec(&bin.join("node"));
        let (name, path) = find_runtime(std::slice::from_ref(&bin), None).unwrap();
        assert_eq!(name, "deno");
        assert_eq!(path, bin.join("deno"));
        let _ = std::fs::remove_dir_all(&bin);
    }

    #[cfg(unix)]
    #[test]
    fn skips_shim_directories() {
        let root = temp_dir("shim");
        let shims = root.join("shims");
        make_exec(&shims.join("node"));
        assert!(find_runtime(&[shims], None).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn finds_newest_nodenv_version_when_not_on_path() {
        let home = temp_dir("nodenv");
        let versions = home.join(".anyenv/envs/nodenv/versions");
        make_exec(&versions.join("9.11.2/bin/node"));
        make_exec(&versions.join("22.23.2/bin/node"));
        make_exec(&versions.join("22.4.0/bin/node"));
        // バージョンの下に node が無いディレクトリは無視される
        std::fs::create_dir_all(versions.join("23.0.0/bin")).unwrap();
        let (name, path) = find_runtime(&[], Some(&home)).unwrap();
        assert_eq!(name, "node");
        assert_eq!(path, versions.join("22.23.2/bin/node"));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[test]
    fn ignores_node_older_than_minimum() {
        let home = temp_dir("oldnode");
        make_exec(&home.join(".nvm/versions/node/v18.20.0/bin/node"));
        assert!(find_runtime(&[], Some(&home)).is_none());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[test]
    fn finds_deno_in_official_installer_location() {
        let home = temp_dir("deno");
        make_exec(&home.join(".deno/bin/deno"));
        let (name, path) = find_runtime(&[], Some(&home)).unwrap();
        assert_eq!(name, "deno");
        assert_eq!(path, home.join(".deno/bin/deno"));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn returns_none_when_nothing_found() {
        let home = temp_dir("empty");
        assert!(find_runtime(&[], Some(&home)).is_none());
        let _ = std::fs::remove_dir_all(&home);
    }
}
