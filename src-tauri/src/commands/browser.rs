use std::io::Write;
use std::process::{Command, Stdio};

const BROWSERS: &[&str] = &[
    "Safari", "Google Chrome", "Chromium", "Brave Browser",
    "Arc", "Microsoft Edge", "Vivaldi", "Opera", "Firefox", "Biscuit",
];

const CHROMIUM_BROWSERS: &[&str] = &[
    "Google Chrome", "Chromium", "Brave Browser",
    "Arc", "Microsoft Edge", "Vivaldi", "Opera",
];

/// Get the URL of the frontmost browser tab on macOS.
/// Phase 1: Detect the topmost browser using CGWindowList via Swift (reliable z-order).
/// Phase 2: Extract URL using the appropriate method per browser.
#[tauri::command]
pub async fn get_browser_url() -> Result<String, String> {
    let browser = detect_topmost_browser()?;
    get_url_from_browser(&browser)
}

/// Use Swift + CoreGraphics to get the actual window z-order
/// and find the topmost browser (skipping YTDown itself).
fn detect_topmost_browser() -> Result<String, String> {
    let browser_list = BROWSERS.iter()
        .map(|b| format!("\"{}\"", b))
        .collect::<Vec<_>>()
        .join(", ");

    let swift_code = format!(r#"
import CoreGraphics
let browsers: Set<String> = [{browsers}]
let options: CGWindowListOption = [.optionOnScreenOnly, .excludeDesktopElements]
if let list = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] {{
    for w in list {{
        if let name = w["kCGWindowOwnerName"] as? String,
           let layer = w["kCGWindowLayer"] as? Int,
           layer == 0, browsers.contains(name) {{
            print(name)
            break
        }}
    }}
}}
"#, browsers = browser_list);

    let output = Command::new("swift")
        .args(["-e", &swift_code])
        .output()
        .map_err(|e| format!("ブラウザ検出エラー: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("ブラウザ検出エラー: {}", err));
    }

    let browser = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if browser.is_empty() {
        Err("実行中のブラウザが見つかりません".to_string())
    } else {
        Ok(browser)
    }
}

/// Get URL from the detected browser using the appropriate method.
fn get_url_from_browser(browser: &str) -> Result<String, String> {
    let url_script = if browser == "Safari" {
        r#"tell application "Safari"
    if (count of windows) is 0 then error "Safariにウィンドウがありません"
    return URL of current tab of front window
end tell"#.to_string()
    } else if CHROMIUM_BROWSERS.contains(&browser) {
        format!(
            "using terms from application \"Google Chrome\"\n\
                tell application \"{b}\"\n\
                    if (count of windows) is 0 then error \"{b}にウィンドウがありません\"\n\
                    return URL of active tab of front window\n\
                end tell\n\
            end using terms from", b = browser)
    } else {
        // Firefox, Biscuit, etc.
        return get_url_via_ui_scripting(browser);
    };

    let result = run_osascript(&url_script);

    // If Chromium AppleScript failed, fall back to UI scripting
    if result.is_err() && CHROMIUM_BROWSERS.contains(&browser) {
        return get_url_via_ui_scripting(browser);
    }

    result
}

/// Fallback: get URL via UI scripting (Cmd+L, Cmd+A, Cmd+C).
/// Works for Firefox, Biscuit, and any browser with a standard address bar.
fn get_url_via_ui_scripting(browser: &str) -> Result<String, String> {
    let script = format!(
        "set prevClip to the clipboard\n\
        tell application \"System Events\"\n\
            tell process \"{b}\"\n\
                set frontmost to true\n\
                delay 0.2\n\
                keystroke \"l\" using command down\n\
                delay 0.15\n\
                keystroke \"a\" using command down\n\
                delay 0.1\n\
                keystroke \"c\" using command down\n\
                delay 0.15\n\
                key code 53\n\
            end tell\n\
        end tell\n\
        set theURL to (the clipboard) as text\n\
        set the clipboard to prevClip\n\
        return theURL", b = browser);

    run_osascript(&script)
}

/// Run an AppleScript via stdin and return stdout or an error.
fn run_osascript(script: &str) -> Result<String, String> {
    let mut child = Command::new("osascript")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("osascript実行エラー: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(script.as_bytes())
            .map_err(|e| format!("スクリプト書き込みエラー: {}", e))?;
    }

    let output = child.wait_with_output()
        .map_err(|e| format!("osascript待機エラー: {}", e))?;

    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if url.is_empty() {
            Err("URLを取得できませんでした".to_string())
        } else {
            Ok(url)
        }
    } else {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("URL取得エラー: {}", err))
    }
}
