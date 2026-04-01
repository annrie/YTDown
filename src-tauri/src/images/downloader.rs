use image::ImageFormat;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct ImageToDownload {
    pub url: String,
    pub filename_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadedImage {
    pub original_url: String,
    pub file_path: String,
    pub filename: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub file_size: u64,
    pub format: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub session_id: i64,
    pub image_index: usize,
    pub total_images: usize,
    pub current_url: String,
    pub percent: f64,
    pub status: String,
    pub error_message: Option<String>,
}

pub async fn download_and_save(
    client: &Client,
    img: &ImageToDownload,
    output_dir: &Path,
    target_format: Option<&str>,
    index: usize,
) -> Result<DownloadedImage, String> {
    let response = client
        .get(&img.url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Read error: {e}"))?;

    let original_filename = img.filename_hint.as_deref().unwrap_or("").to_string();
    let filename = if original_filename.is_empty() {
        format!("image_{index:04}")
    } else {
        Path::new(&original_filename)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("image_{index:04}"))
    };

    let (final_bytes, format_str, width, height) = match image::load_from_memory(&bytes) {
        Ok(img_data) => {
            let w = img_data.width();
            let h = img_data.height();
            if let Some("webp") = target_format {
                let mut buf = std::io::Cursor::new(Vec::new());
                match img_data.write_to(&mut buf, ImageFormat::WebP) {
                    Ok(_) => (buf.into_inner(), "webp".to_string(), Some(w), Some(h)),
                    Err(_) => {
                        let ext = guess_extension(&bytes);
                        (bytes.to_vec(), ext, Some(w), Some(h))
                    }
                }
            } else {
                let ext = guess_extension(&bytes);
                (bytes.to_vec(), ext, Some(w), Some(h))
            }
        }
        Err(_) => {
            let ext = guess_extension(&bytes);
            (bytes.to_vec(), ext, None, None)
        }
    };

    fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Cannot create directory: {e}"))?;

    let full_filename = format!("{filename}.{format_str}");
    let file_path = output_dir.join(&full_filename);
    let file_path = ensure_unique_path(file_path);
    let final_filename = file_path.file_name().unwrap().to_string_lossy().to_string();

    let file_size = final_bytes.len() as u64;
    fs::write(&file_path, &final_bytes)
        .await
        .map_err(|e| format!("Write error: {e}"))?;

    Ok(DownloadedImage {
        original_url: img.url.clone(),
        file_path: file_path.to_string_lossy().to_string(),
        filename: final_filename,
        width,
        height,
        file_size,
        format: format_str,
    })
}

fn guess_extension(bytes: &[u8]) -> String {
    if bytes.starts_with(b"\x89PNG") {
        "png".to_string()
    } else if bytes.starts_with(b"\xFF\xD8\xFF") {
        "jpg".to_string()
    } else if bytes.starts_with(b"GIF8") {
        "gif".to_string()
    } else if bytes.starts_with(b"RIFF") && bytes.len() > 12 && &bytes[8..12] == b"WEBP" {
        "webp".to_string()
    } else {
        "bin".to_string()
    }
}

fn ensure_unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }
    let stem = path.file_stem().unwrap().to_string_lossy().to_string();
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let parent = path.parent().unwrap();
    for i in 1..1000 {
        let new_path = parent.join(format!("{stem}_{i}.{ext}"));
        if !new_path.exists() {
            return new_path;
        }
    }
    path
}

pub fn create_download_client() -> Result<Client, String> {
    Client::builder()
        .user_agent("Mozilla/5.0 (compatible; YTDown/0.2)")
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Client error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_extension_png() {
        assert_eq!(guess_extension(b"\x89PNG\r\n\x1a\n"), "png");
    }
    #[test]
    fn test_guess_extension_jpg() {
        assert_eq!(guess_extension(b"\xFF\xD8\xFF\xE0"), "jpg");
    }
    #[test]
    fn test_guess_extension_unknown() {
        assert_eq!(guess_extension(b"unknown"), "bin");
    }
    #[test]
    fn test_webp_conversion_works() {
        let img = image::RgbImage::new(2, 2);
        let dynamic_img = image::DynamicImage::ImageRgb8(img);
        let mut buf = std::io::Cursor::new(Vec::new());
        let result = dynamic_img.write_to(&mut buf, image::ImageFormat::WebP);
        assert!(result.is_ok(), "WebP encoding must work");
        assert!(!buf.into_inner().is_empty());
    }
}
