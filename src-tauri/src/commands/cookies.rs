#[tauri::command]
pub async fn import_cookies_from_browser(browser: String) -> Result<String, String> {
    // Validate browser name
    match browser.as_str() {
        "safari" | "chrome" | "firefox" => Ok(browser),
        _ => Err(format!("Unsupported browser: {}", browser)),
    }
    // NOTE: Actual cookie import is handled by passing --cookies-from-browser to yt-dlp
    // This command just validates and stores the preference
}

#[tauri::command]
pub async fn set_cookie_file(path: String) -> Result<(), String> {
    if !path.is_empty() && !std::path::Path::new(&path).exists() {
        return Err(format!("Cookie file not found: {}", path));
    }
    Ok(())
}

/// Open macOS System Settings to the Full Disk Access privacy pane.
#[tauri::command]
pub async fn open_system_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}

/// Check if the app can access Safari's cookie database.
/// Tries multiple known paths (sandboxed and legacy).
/// Returns true if accessible, false otherwise.
#[tauri::command]
pub async fn check_safari_access() -> bool {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let candidates = [
                // macOS Sonoma+ sandboxed Safari
                home.join("Library/Containers/com.apple.Safari/Data/Library/Cookies/Cookies.binarycookies"),
                // Legacy path (older macOS)
                home.join("Library/Safari/Cookies/Cookies.binarycookies"),
            ];
            return candidates.iter().any(|p| std::fs::File::open(p).is_ok());
        }
        false
    }
    #[cfg(not(target_os = "macos"))]
    {
        true // No FDA concept on other platforms
    }
}
