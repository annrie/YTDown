use std::io::Write;
use std::process::{Command, Stdio};

const BROWSERS: &[&str] = &[
    "Safari",
    "Google Chrome",
    "Chromium",
    "Brave Browser",
    "Arc",
    "Microsoft Edge",
    "Vivaldi",
    "Opera",
    "Firefox",
    "Biscuit",
];

const CHROMIUM_BROWSERS: &[&str] = &[
    "Google Chrome",
    "Chromium",
    "Brave Browser",
    "Arc",
    "Microsoft Edge",
    "Vivaldi",
    "Opera",
];

/// 取得結果。どのブラウザから取ったかをUI側で明示するためブラウザ名も返す
#[derive(serde::Serialize)]
pub struct BrowserUrl {
    pub browser: String,
    pub url: String,
    /// ページタイトル（取得できた場合。ファイル名の初期値に使える）
    pub title: Option<String>,
}

/// Get the URL of the frontmost browser tab.
/// macOS: Detect the topmost browser via CGWindowList (native), then extract URL.
/// Other platforms: Not yet supported.
#[tauri::command]
pub async fn get_browser_url() -> Result<BrowserUrl, String> {
    #[cfg(not(target_os = "macos"))]
    {
        return Err(
            "ブラウザからのURL取得はmacOSのみ対応しています。URLを直接入力してください。"
                .to_string(),
        );
    }

    #[cfg(target_os = "macos")]
    {
        let (browser, pid) = detect_topmost_browser()?;
        let url = get_url_from_browser(&browser, pid)?;
        let title = get_title_from_browser(&browser, pid);
        Ok(BrowserUrl {
            browser,
            url,
            title,
        })
    }
}

/// ブラウザから現在ページのタイトルを取得する（best-effort、失敗時None）。
/// AppleScript対応ブラウザはタブ名、それ以外はAXツリーのAXWebArea/AXTitleから読む。
#[cfg(target_os = "macos")]
fn get_title_from_browser(browser: &str, pid: i32) -> Option<String> {
    let script = if browser == "Safari" {
        Some(
            "tell application \"Safari\"\n\
                    if (count of windows) is 0 then return \"\"\n\
                    return name of current tab of front window\n\
                end tell"
                .to_string(),
        )
    } else if CHROMIUM_BROWSERS.contains(&browser) {
        Some(format!(
            "using terms from application \"Google Chrome\"\n\
                tell application \"{b}\"\n\
                    if (count of windows) is 0 then return \"\"\n\
                    return title of active tab of front window\n\
                end tell\n\
            end using terms from",
            b = browser
        ))
    } else {
        None
    };

    let title = match script {
        Some(script) => run_osascript(&script).ok(),
        None => get_title_via_ax(pid).ok(),
    };
    title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

/// macOS Accessibility API (HIServices) の最小FFI
#[cfg(target_os = "macos")]
mod ax {
    use core_foundation::base::CFTypeRef;
    use core_foundation::string::CFStringRef;
    use std::os::raw::c_void;

    pub type AXUIElementRef = *const c_void;
    pub type AXError = i32;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        pub fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
        pub fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> AXError;
        pub fn AXUIElementSetAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: CFTypeRef,
        ) -> AXError;
    }
}

/// CoreGraphicsのウィンドウ一覧（前面→背面順）から最前面のブラウザ名とpidを特定する。
/// swift等の開発ツールチェーンに依存しない（Xcode非搭載の配布先でも動く）。
#[cfg(target_os = "macos")]
fn detect_topmost_browser() -> Result<(String, i32), String> {
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::window::{
        copy_window_info, kCGNullWindowID, kCGWindowLayer, kCGWindowListExcludeDesktopElements,
        kCGWindowListOptionOnScreenOnly, kCGWindowOwnerName, kCGWindowOwnerPID,
    };

    let window_list = copy_window_info(
        kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
        kCGNullWindowID,
    )
    .ok_or_else(|| "ブラウザ検出エラー: ウィンドウ一覧を取得できませんでした".to_string())?;

    let (owner_key, layer_key, pid_key) = unsafe {
        (
            CFString::wrap_under_get_rule(kCGWindowOwnerName),
            CFString::wrap_under_get_rule(kCGWindowLayer),
            CFString::wrap_under_get_rule(kCGWindowOwnerPID),
        )
    };

    for item in window_list.iter() {
        let dict = unsafe {
            CFDictionary::<CFString, CFType>::wrap_under_get_rule(*item as CFDictionaryRef)
        };
        let layer = dict
            .find(&layer_key)
            .and_then(|value| value.downcast::<CFNumber>())
            .and_then(|number| number.to_i32());
        if layer != Some(0) {
            continue;
        }
        let Some(owner) = dict
            .find(&owner_key)
            .and_then(|value| value.downcast::<CFString>())
            .map(|name| name.to_string())
        else {
            continue;
        };
        if BROWSERS.contains(&owner.as_str()) {
            let Some(pid) = dict
                .find(&pid_key)
                .and_then(|value| value.downcast::<CFNumber>())
                .and_then(|number| number.to_i32())
            else {
                continue;
            };
            return Ok((owner, pid));
        }
    }

    Err("実行中のブラウザが見つかりません".to_string())
}

/// Accessibility APIでブラウザのWebエリアから現在ページのURLを直接読む。
/// アドレスバーを持たないブラウザ（Biscuit等のElectron系）でも動作し、
/// キー操作・クリップボードを一切使わない。要アクセシビリティ権限。
#[cfg(target_os = "macos")]
fn get_url_via_ax(pid: i32) -> Result<String, String> {
    use core_foundation::array::CFArray;
    use core_foundation::base::{CFType, CFTypeRef, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::string::CFString;
    use core_foundation::url::CFURL;

    const MAX_DEPTH: u32 = 30;
    const MAX_CHILDREN: usize = 60;

    fn copy_attr(el: &CFType, name: &CFString) -> Option<CFType> {
        let mut value: CFTypeRef = std::ptr::null();
        let err = unsafe {
            ax::AXUIElementCopyAttributeValue(
                el.as_CFTypeRef() as ax::AXUIElementRef,
                name.as_concrete_TypeRef(),
                &mut value,
            )
        };
        if err == 0 && !value.is_null() {
            Some(unsafe { CFType::wrap_under_create_rule(value) })
        } else {
            None
        }
    }

    fn role_of(el: &CFType, role_key: &CFString) -> Option<String> {
        copy_attr(el, role_key)
            .and_then(|value| value.downcast::<CFString>())
            .map(|role| role.to_string())
    }

    fn cf_to_url_string(value: &CFType) -> Option<String> {
        value
            .downcast::<CFURL>()
            .map(|url| url.get_string().to_string())
            .or_else(|| value.downcast::<CFString>().map(|s| s.to_string()))
    }

    fn is_http_url(url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    /// DFSでhttp(s)のURLを持つWebエリアを探す。
    /// file:// のWebエリアはElectronシェルUI（Biscuit等）なので、
    /// その内側に入れ子になったゲストWebエリア（実際のサイト）まで降りる
    fn find_http_web_url(
        el: &CFType,
        depth: u32,
        keys: &(CFString, CFString, CFString), // (role, children, url)
    ) -> Option<String> {
        if depth > MAX_DEPTH {
            return None;
        }
        let (role_key, children_key, url_key) = keys;
        if role_of(el, role_key).as_deref() == Some("AXWebArea") {
            if let Some(url) = copy_attr(el, url_key).as_ref().and_then(cf_to_url_string) {
                if is_http_url(&url) {
                    return Some(url);
                }
            }
        }
        let children = copy_attr(el, children_key)?.downcast::<CFArray>()?;
        for item in children.iter().take(MAX_CHILDREN) {
            let child = unsafe { CFType::wrap_under_get_rule(*item as CFTypeRef) };
            if let Some(found) = find_http_web_url(&child, depth + 1, keys) {
                return Some(found);
            }
        }
        None
    }

    let role_key = CFString::from_static_string("AXRole");
    let children_key = CFString::from_static_string("AXChildren");
    let url_key = CFString::from_static_string("AXURL");

    let app_ref = unsafe { ax::AXUIElementCreateApplication(pid) };
    if app_ref.is_null() {
        return Err("AX: アプリ要素を取得できませんでした".to_string());
    }
    let app = unsafe { CFType::wrap_under_create_rule(app_ref as CFTypeRef) };

    // Electron系はアシスト技術の接続を検知するまでAXツリーを公開しないことがあるため明示的に有効化
    let manual_ax = CFString::from_static_string("AXManualAccessibility");
    unsafe {
        ax::AXUIElementSetAttributeValue(
            app.as_CFTypeRef() as ax::AXUIElementRef,
            manual_ax.as_concrete_TypeRef(),
            CFBoolean::true_value().as_CFTypeRef(),
        );
    }

    let keys = (role_key, children_key, url_key);

    // 1) フォーカス中の要素から親方向へ辿り、http(s)のWebエリアを探す（最も内側＝実サイト優先）。
    //    file://のWebエリアはElectronシェルUIなので採用しない
    let focused_key = CFString::from_static_string("AXFocusedUIElement");
    let parent_key = CFString::from_static_string("AXParent");
    if let Some(mut cursor) = copy_attr(&app, &focused_key) {
        for _ in 0..60 {
            if role_of(&cursor, &keys.0).as_deref() == Some("AXWebArea") {
                if let Some(url) = copy_attr(&cursor, &keys.2).as_ref().and_then(cf_to_url_string)
                {
                    if is_http_url(&url) {
                        return Ok(url);
                    }
                }
            }
            match copy_attr(&cursor, &parent_key) {
                Some(parent) => cursor = parent,
                None => break,
            }
        }
    }

    // 2) 前面ウィンドウからDFSでhttp(s)のWebエリアを探す
    let focused_window_key = CFString::from_static_string("AXFocusedWindow");
    let windows_key = CFString::from_static_string("AXWindows");
    let window = copy_attr(&app, &focused_window_key).or_else(|| {
        copy_attr(&app, &windows_key)
            .and_then(|value| value.downcast::<CFArray>())
            .and_then(|windows| {
                windows
                    .iter()
                    .next()
                    .map(|item| unsafe { CFType::wrap_under_get_rule(*item as CFTypeRef) })
            })
    });
    if let Some(url) = window.and_then(|window| find_http_web_url(&window, 0, &keys)) {
        return Ok(url);
    }

    Err("AX: 表示中ページのURLが見つかりませんでした".to_string())
}

/// Accessibility APIで表示中ページのタイトル（AXWebArea/AXTitle）を読む。
/// URL取得と同じく、http(s)のWebエリア（Electronシェルではなく実サイト）のタイトルを返す。
#[cfg(target_os = "macos")]
fn get_title_via_ax(pid: i32) -> Result<String, String> {
    use core_foundation::array::CFArray;
    use core_foundation::base::{CFType, CFTypeRef, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::string::CFString;
    use core_foundation::url::CFURL;

    const MAX_DEPTH: u32 = 30;
    const MAX_CHILDREN: usize = 60;

    fn copy_attr(el: &CFType, name: &CFString) -> Option<CFType> {
        let mut value: CFTypeRef = std::ptr::null();
        let err = unsafe {
            ax::AXUIElementCopyAttributeValue(
                el.as_CFTypeRef() as ax::AXUIElementRef,
                name.as_concrete_TypeRef(),
                &mut value,
            )
        };
        if err == 0 && !value.is_null() {
            Some(unsafe { CFType::wrap_under_create_rule(value) })
        } else {
            None
        }
    }
    fn str_attr(el: &CFType, key: &CFString) -> Option<String> {
        copy_attr(el, key)
            .and_then(|v| v.downcast::<CFString>())
            .map(|s| s.to_string())
    }
    fn url_attr(el: &CFType, key: &CFString) -> Option<String> {
        copy_attr(el, key).and_then(|v| {
            v.downcast::<CFURL>()
                .map(|u| u.get_string().to_string())
                .or_else(|| v.downcast::<CFString>().map(|s| s.to_string()))
        })
    }
    fn is_http(url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    let role_key = CFString::from_static_string("AXRole");
    let children_key = CFString::from_static_string("AXChildren");
    let url_key = CFString::from_static_string("AXURL");
    let title_key = CFString::from_static_string("AXTitle");

    // http(s)のWebエリアなら、そのAXTitleを返す
    fn title_of_http_web_area(
        el: &CFType,
        keys: &(CFString, CFString, CFString, CFString), // role, children, url, title
    ) -> Option<String> {
        let (role_key, _, url_key, title_key) = keys;
        if str_attr(el, role_key).as_deref() == Some("AXWebArea") {
            if url_attr(el, url_key).as_deref().is_some_and(is_http) {
                return str_attr(el, title_key).filter(|t| !t.trim().is_empty());
            }
        }
        None
    }

    fn dfs_title(
        el: &CFType,
        depth: u32,
        keys: &(CFString, CFString, CFString, CFString),
    ) -> Option<String> {
        if depth > MAX_DEPTH {
            return None;
        }
        if let Some(title) = title_of_http_web_area(el, keys) {
            return Some(title);
        }
        let children = copy_attr(el, &keys.1)?.downcast::<CFArray>()?;
        for item in children.iter().take(MAX_CHILDREN) {
            let child = unsafe { CFType::wrap_under_get_rule(*item as CFTypeRef) };
            if let Some(title) = dfs_title(&child, depth + 1, keys) {
                return Some(title);
            }
        }
        None
    }

    let app_ref = unsafe { ax::AXUIElementCreateApplication(pid) };
    if app_ref.is_null() {
        return Err("AX: アプリ要素を取得できませんでした".to_string());
    }
    let app = unsafe { CFType::wrap_under_create_rule(app_ref as CFTypeRef) };
    let manual_ax = CFString::from_static_string("AXManualAccessibility");
    unsafe {
        ax::AXUIElementSetAttributeValue(
            app.as_CFTypeRef() as ax::AXUIElementRef,
            manual_ax.as_concrete_TypeRef(),
            CFBoolean::true_value().as_CFTypeRef(),
        );
    }

    let keys = (role_key, children_key, url_key, title_key);

    // 1) フォーカス中の要素から親方向へ
    let focused_key = CFString::from_static_string("AXFocusedUIElement");
    let parent_key = CFString::from_static_string("AXParent");
    if let Some(mut cursor) = copy_attr(&app, &focused_key) {
        for _ in 0..60 {
            if let Some(title) = title_of_http_web_area(&cursor, &keys) {
                return Ok(title);
            }
            match copy_attr(&cursor, &parent_key) {
                Some(parent) => cursor = parent,
                None => break,
            }
        }
    }

    // 2) 前面ウィンドウからDFS
    let focused_window_key = CFString::from_static_string("AXFocusedWindow");
    let windows_key = CFString::from_static_string("AXWindows");
    let window = copy_attr(&app, &focused_window_key).or_else(|| {
        copy_attr(&app, &windows_key)
            .and_then(|value| value.downcast::<CFArray>())
            .and_then(|windows| {
                windows
                    .iter()
                    .next()
                    .map(|item| unsafe { CFType::wrap_under_get_rule(*item as CFTypeRef) })
            })
    });
    if let Some(title) = window.and_then(|w| dfs_title(&w, 0, &keys)) {
        return Ok(title);
    }

    Err("AX: タイトルが見つかりませんでした".to_string())
}

/// Get URL from the detected browser using the appropriate method.
#[cfg(target_os = "macos")]
fn get_url_from_browser(browser: &str, pid: i32) -> Result<String, String> {
    let url_script = if browser == "Safari" {
        r#"tell application "Safari"
    if (count of windows) is 0 then error "Safariにウィンドウがありません"
    return URL of current tab of front window
end tell"#
            .to_string()
    } else if CHROMIUM_BROWSERS.contains(&browser) {
        format!(
            "using terms from application \"Google Chrome\"\n\
                tell application \"{b}\"\n\
                    if (count of windows) is 0 then error \"{b}にウィンドウがありません\"\n\
                    return URL of active tab of front window\n\
                end tell\n\
            end using terms from",
            b = browser
        )
    } else {
        // Firefox, Biscuit等: アドレスバーを持たないブラウザがあるため、
        // まずAXツリーからURLを読み、失敗時のみキー操作にフォールバック
        return get_url_via_ax(pid).or_else(|_| get_url_via_ui_scripting(browser));
    };

    let result = run_osascript(&url_script);

    // If Chromium AppleScript failed, fall back to AX, then UI scripting
    if result.is_err() && CHROMIUM_BROWSERS.contains(&browser) {
        return get_url_via_ax(pid).or_else(|_| get_url_via_ui_scripting(browser));
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
        return theURL",
        b = browser
    );

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
        stdin
            .write_all(script.as_bytes())
            .map_err(|e| format!("スクリプト書き込みエラー: {}", e))?;
    }

    let output = child
        .wait_with_output()
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
        Err(format!("URL取得エラー: {}", friendly_osascript_error(&err)))
    }
}

/// TCC（macOSプライバシー権限）起因の定型エラーを、対処先が分かるメッセージへ変換する
fn friendly_osascript_error(err: &str) -> String {
    // System Eventsのキーストローク送信拒否（エラー1002）
    if err.contains("(1002)")
        || err.contains("キー操作の送信は許可されません")
        || err.contains("not allowed to send keystrokes")
    {
        return "アクセシビリティ権限が必要です。システム設定 > プライバシーとセキュリティ > \
                アクセシビリティ で YTDown（開発実行時は起動元のターミナルアプリ）を有効にして、\
                もう一度お試しください。"
            .to_string();
    }
    // Apple Events送信の拒否（エラー-1743、オートメーション権限）
    if err.contains("-1743")
        || err.contains("Not authorized to send Apple events")
        || err.contains("Apple Eventsを送信する権限がありません")
    {
        return "オートメーション権限が必要です。システム設定 > プライバシーとセキュリティ > \
                オートメーション で YTDown からブラウザおよび System Events への制御を許可して、\
                もう一度お試しください。"
            .to_string();
    }
    truncate_error(err)
}

/// UIアラートに流す外部コマンドのエラー出力を丸める（大量出力がUIを覆う事故の防止）
fn truncate_error(err: &str) -> String {
    const MAX_CHARS: usize = 200;
    let mut chars = err.chars();
    let head: String = chars.by_ref().take(MAX_CHARS).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    #[ignore = "GUIセッションと起動中ブラウザが必要（手動スモークテスト用）"]
    fn detects_topmost_browser_on_dev_machine() {
        let result = super::detect_topmost_browser();
        println!("detect_topmost_browser => {result:?}");
    }

    #[test]
    #[ignore = "GUIセッション・起動中ブラウザ・アクセシビリティ権限が必要（手動スモークテスト用）"]
    fn reads_url_via_ax_on_dev_machine() {
        match super::detect_topmost_browser() {
            Ok((browser, pid)) => {
                println!("browser: {browser} (pid {pid})");
                println!("get_url_via_ax => {:?}", super::get_url_via_ax(pid));
            }
            Err(err) => println!("detection failed: {err}"),
        }
    }
}
