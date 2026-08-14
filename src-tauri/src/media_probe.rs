use reqwest::Client;
use scraper::{Html, Selector};
use serde::Serialize;
use std::collections::HashSet;
use url::Url;

/// iframe巡回の上限（1件ごとに最大20秒のfetchが走るため）
const MAX_IFRAME_PROBES: usize = 3;
/// HTML読み込みの上限バイト数（巨大レスポンスでのメモリ膨張防止）
const MAX_HTML_BYTES: usize = 3 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct MediaProbeResult {
    pub original_url: String,
    pub media_url: String,
    pub referer: Option<String>,
    pub source: String,
    /// ページから抽出したタイトル（og:title / twitter:title / <title>）
    pub title: Option<String>,
    /// ページから抽出したサムネイル（og:image / twitter:image）
    pub thumbnail: Option<String>,
}

/// ページHTMLから抽出したメタ情報（og:title / og:image 等。直接メディアURLには存在しない）
#[derive(Debug, Clone, Default, Serialize)]
pub struct PageMetadata {
    pub title: Option<String>,
    pub thumbnail: Option<String>,
}

pub fn is_direct_media_url(url: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };
    let path = parsed.path().to_ascii_lowercase();
    matches!(
        path.rsplit('.').next(),
        Some("m3u8" | "mpd" | "mp4" | "m4v" | "mov" | "webm" | "mkv")
    )
}

/// 共有HTTPクライアント。probe用の穏当なUAとタイムアウト設定
fn build_client(user_agent: Option<&str>) -> Result<Client, String> {
    let ua = user_agent
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) YTDown/0.2 Safari/537.36");
    Client::builder()
        .user_agent(ua)
        .timeout(std::time::Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))
}

pub async fn resolve_embedded_media(
    page_url: &str,
    user_agent: Option<&str>,
) -> Result<Option<MediaProbeResult>, String> {
    if is_direct_media_url(page_url) {
        return Ok(Some(MediaProbeResult {
            original_url: page_url.to_string(),
            media_url: page_url.to_string(),
            referer: None,
            source: "direct".to_string(),
            title: None,
            thumbnail: None,
        }));
    }

    let base_url = match Url::parse(page_url) {
        Ok(url) if matches!(url.scheme(), "http" | "https") => url,
        _ => return Ok(None),
    };

    let client = build_client(user_agent)?;

    let Some(html) = fetch_html(&client, page_url).await? else {
        return Ok(None);
    };
    let page_meta = collect_page_metadata(&html, &base_url);
    let candidates = collect_media_candidates(&html, &base_url);
    let Some(media_url) = candidates.first().cloned() else {
        for iframe_url in collect_iframe_urls(&html, &base_url)
            .into_iter()
            .take(MAX_IFRAME_PROBES)
        {
            let iframe_base_url = match Url::parse(&iframe_url) {
                Ok(url) => url,
                Err(_) => continue,
            };
            let Some(iframe_html) = fetch_html(&client, &iframe_url).await? else {
                continue;
            };
            let iframe_candidates = collect_media_candidates(&iframe_html, &iframe_base_url);
            if let Some(media_url) = iframe_candidates.first().cloned() {
                // iframe内のメタも見て、親ページで取れなかった分を補完する
                let iframe_meta = collect_page_metadata(&iframe_html, &iframe_base_url);
                return Ok(Some(MediaProbeResult {
                    original_url: page_url.to_string(),
                    media_url,
                    referer: Some(iframe_url),
                    source: "iframe".to_string(),
                    title: page_meta.title.or(iframe_meta.title),
                    thumbnail: page_meta.thumbnail.or(iframe_meta.thumbnail),
                }));
            }
        }
        return Ok(None);
    };

    Ok(Some(MediaProbeResult {
        original_url: page_url.to_string(),
        media_url,
        referer: Some(page_url.to_string()),
        source: "embedded".to_string(),
        title: page_meta.title,
        thumbnail: page_meta.thumbnail,
    }))
}

/// og:title / twitter:title / <title>、og:image / twitter:image を抽出する
fn collect_page_metadata(html: &str, base_url: &Url) -> PageMetadata {
    let document = Html::parse_document(html);

    let meta_content = |selectors: &[&str]| -> Option<String> {
        for selector in selectors {
            let Ok(parsed) = Selector::parse(selector) else {
                continue;
            };
            if let Some(value) = document
                .select(&parsed)
                .find_map(|el| el.value().attr("content"))
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
            {
                return Some(value.to_string());
            }
        }
        None
    };

    let title = meta_content(&[
        "meta[property='og:title'][content]",
        "meta[name='twitter:title'][content]",
    ])
    .or_else(|| {
        Selector::parse("title").ok().and_then(|selector| {
            document
                .select(&selector)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .filter(|value| !value.is_empty())
        })
    });

    let thumbnail = meta_content(&[
        "meta[property='og:image'][content]",
        "meta[property='og:image:url'][content]",
        "meta[name='twitter:image'][content]",
    ])
    .and_then(|value| base_url.join(value.trim()).ok().map(|url| url.to_string()));

    PageMetadata { title, thumbnail }
}

async fn fetch_html(client: &Client, url: &str) -> Result<Option<String>, String> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("ページを取得できませんでした: {e}"))?;

    if !response.status().is_success() {
        return Ok(None);
    }

    if let Some(content_type) = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
    {
        let content_type = content_type.to_ascii_lowercase();
        if !content_type.contains("html")
            && !content_type.contains("xml")
            && !content_type.starts_with("text/")
        {
            return Ok(None);
        }
    }

    let mut body: Vec<u8> = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("HTML read error: {e}"))?
    {
        body.extend_from_slice(&chunk);
        if body.len() >= MAX_HTML_BYTES {
            break;
        }
    }
    Ok(Some(String::from_utf8_lossy(&body).into_owned()))
}

fn collect_media_candidates(html: &str, base_url: &Url) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();
    let document = Html::parse_document(html);

    for selector in [
        "video[src]",
        "source[src]",
        "a[href]",
        "meta[property='og:video'][content]",
        "meta[property='og:video:url'][content]",
        "meta[name='twitter:player:stream'][content]",
    ] {
        let Ok(parsed_selector) = Selector::parse(selector) else {
            continue;
        };
        for element in document.select(&parsed_selector) {
            let value = element
                .value()
                .attr("src")
                .or_else(|| element.value().attr("href"))
                .or_else(|| element.value().attr("content"));
            if let Some(url) = value.and_then(|value| normalize_candidate_url(value, base_url)) {
                push_candidate(url, &mut seen, &mut candidates);
            }
        }
    }

    for url in scan_media_urls(html, base_url) {
        push_candidate(url, &mut seen, &mut candidates);
    }

    candidates.sort_by_key(|url| media_priority(url));
    candidates
}

fn collect_iframe_urls(html: &str, base_url: &Url) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut urls = Vec::new();
    let document = Html::parse_document(html);
    let Ok(selector) = Selector::parse("iframe[src], embed[src]") else {
        return urls;
    };

    for element in document.select(&selector) {
        let Some(src) = element.value().attr("src") else {
            continue;
        };
        if let Some(url) = normalize_candidate_url(src, base_url) {
            if seen.insert(url.clone()) {
                urls.push(url);
            }
        }
    }

    urls
}

fn push_candidate(url: String, seen: &mut HashSet<String>, candidates: &mut Vec<String>) {
    if is_direct_media_url(&url) && seen.insert(url.clone()) {
        candidates.push(url);
    }
}

fn media_priority(url: &str) -> u8 {
    let lower = url.to_ascii_lowercase();
    if lower.contains(".m3u8") {
        0
    } else if lower.contains(".mpd") {
        1
    } else {
        2
    }
}

fn normalize_candidate_url(value: &str, base_url: &Url) -> Option<String> {
    let cleaned = clean_url_fragment(value);
    if cleaned.is_empty() || cleaned.starts_with("blob:") || cleaned.starts_with("data:") {
        return None;
    }
    base_url.join(&cleaned).ok().map(|url| url.to_string())
}

fn clean_url_fragment(value: &str) -> String {
    value
        .trim()
        .trim_matches(|c| matches!(c, '"' | '\'' | '`' | ')' | '(' | '[' | ']'))
        .replace("\\/", "/")
        .replace("&amp;", "&")
        .replace("\\u0026", "&")
}

fn scan_media_urls(html: &str, base_url: &Url) -> Vec<String> {
    let normalized = html.replace("\\/", "/").replace("&amp;", "&");
    // ASCII小文字化はバイト長を変えないため、検索indexはnormalizedへそのまま使える
    let lowercased = normalized.to_ascii_lowercase();
    let mut urls = Vec::new();
    for marker in [".m3u8", ".mpd", ".mp4", ".m4v", ".mov", ".webm", ".mkv"] {
        let mut search_from = 0;
        while let Some(relative_index) = lowercased[search_from..].find(marker) {
            let marker_index = search_from + relative_index;
            let start = find_url_start(&normalized, marker_index);
            let end = find_url_end(&normalized, marker_index + marker.len());
            if start < end {
                if let Some(url) = normalize_candidate_url(&normalized[start..end], base_url) {
                    urls.push(url);
                }
            }
            search_from = marker_index + marker.len();
        }
    }
    urls
}

fn find_url_start(text: &str, marker_index: usize) -> usize {
    let bytes = text.as_bytes();
    let mut index = marker_index;
    while index > 0 {
        let c = bytes[index - 1] as char;
        if c.is_ascii_whitespace() || matches!(c, '"' | '\'' | '`' | '<' | '>' | '(' | ')' | '[') {
            break;
        }
        index -= 1;
    }
    index
}

fn find_url_end(text: &str, start_index: usize) -> usize {
    let bytes = text.as_bytes();
    let mut index = start_index;
    while index < bytes.len() {
        let c = bytes[index] as char;
        if c.is_ascii_whitespace() || matches!(c, '"' | '\'' | '`' | '<' | '>' | ')' | ']') {
            break;
        }
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_relative_hls_source() {
        let base = Url::parse("https://example.com/watch/123").unwrap();
        let urls = collect_media_candidates(
            r#"<video><source src="/streams/master.m3u8?token=abc&amp;x=1"></video>"#,
            &base,
        );
        assert_eq!(
            urls.first().map(String::as_str),
            Some("https://example.com/streams/master.m3u8?token=abc&x=1")
        );
    }

    #[test]
    fn detects_escaped_script_url() {
        let base = Url::parse("https://example.com/watch/123").unwrap();
        let urls = collect_media_candidates(
            r#"{"file":"https:\/\/cdn.example.com\/video\/master.m3u8"}"#,
            &base,
        );
        assert_eq!(
            urls.first().map(String::as_str),
            Some("https://cdn.example.com/video/master.m3u8")
        );
    }

    #[test]
    fn collects_relative_iframe_urls() {
        let base = Url::parse("https://example.com/watch/123").unwrap();
        let urls = collect_iframe_urls(r#"<iframe src="/player/abc"></iframe>"#, &base);

        assert_eq!(
            urls.first().map(String::as_str),
            Some("https://example.com/player/abc")
        );
    }

    #[test]
    fn extracts_og_title_and_image() {
        let base = Url::parse("https://example.com/watch/123").unwrap();
        let meta = collect_page_metadata(
            r#"<head>
                <meta property="og:title" content="面白い動画">
                <meta property="og:image" content="/thumb/cover.jpg">
                <title>ページタイトル</title>
            </head>"#,
            &base,
        );
        assert_eq!(meta.title.as_deref(), Some("面白い動画"));
        assert_eq!(
            meta.thumbnail.as_deref(),
            Some("https://example.com/thumb/cover.jpg")
        );
    }

    #[test]
    fn falls_back_to_title_tag() {
        let base = Url::parse("https://example.com/watch/123").unwrap();
        let meta = collect_page_metadata(r#"<head><title>素のタイトル</title></head>"#, &base);
        assert_eq!(meta.title.as_deref(), Some("素のタイトル"));
        assert_eq!(meta.thumbnail, None);
    }
}
