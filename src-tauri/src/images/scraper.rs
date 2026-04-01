use reqwest::Client;
use scraper::{Html, Selector};
use serde::Serialize;
use std::collections::HashSet;
use url::Url;

#[derive(Debug, Clone, Serialize)]
pub struct ScrapedImage {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub alt: Option<String>,
}

pub async fn scrape_images_from_url(
    page_url: &str,
    min_width: u32,
    min_height: u32,
) -> Result<Vec<ScrapedImage>, String> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (compatible; YTDown/0.2)")
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let response = client.get(page_url).send().await.map_err(|e| {
        if e.is_timeout() {
            "タイムアウト: 30秒以内にページを取得できませんでした".to_string()
        } else if e.is_connect() {
            "ネットワークエラー: ページに接続できませんでした".to_string()
        } else {
            format!("ページを取得できませんでした: {e}")
        }
    })?;

    let status = response.status();
    if status == reqwest::StatusCode::FORBIDDEN {
        return Err("アクセスがブロックされました。別のURLを試してください".to_string());
    }
    if !status.is_success() {
        return Err(format!(
            "ページを取得できませんでした（ステータス: {status}）"
        ));
    }

    let html = response
        .text()
        .await
        .map_err(|e| format!("HTML read error: {e}"))?;
    let base_url = Url::parse(page_url).map_err(|e| format!("Invalid URL: {e}"))?;
    let document = Html::parse_document(&html);
    let img_selector = Selector::parse("img").unwrap();

    let mut seen_urls = HashSet::new();
    let mut images = Vec::new();

    for element in document.select(&img_selector) {
        let src = match element.value().attr("src") {
            Some(s) if !s.is_empty() => s,
            _ => continue,
        };
        let absolute_url = match base_url.join(src) {
            Ok(u) => u.to_string(),
            Err(_) => continue,
        };
        if !seen_urls.insert(absolute_url.clone()) {
            continue;
        }
        let width = element
            .value()
            .attr("width")
            .and_then(|w| w.parse::<u32>().ok());
        let height = element
            .value()
            .attr("height")
            .and_then(|h| h.parse::<u32>().ok());
        let alt = element.value().attr("alt").map(|s| s.to_string());

        if let (Some(w), Some(h)) = (width, height) {
            if w < 32 || h < 32 {
                continue;
            }
            if w < min_width && h < min_height {
                continue;
            }
        }

        images.push(ScrapedImage {
            url: absolute_url,
            width,
            height,
            alt,
        });
    }
    Ok(images)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scrape_invalid_url() {
        let result = scrape_images_from_url("not-a-url", 100, 100).await;
        assert!(result.is_err());
    }
}
