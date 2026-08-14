//! ダウンロード済み動画からサムネイルを生成する。
//!
//! Cloudflare配下のサイトはページのog:imageをサーバサイド取得できないため、
//! 落とした動画そのものから1フレームを抜き出してサムネイルにする（サイト非依存・確実）。
//! 結果は data URI (`data:image/jpeg;base64,...`) として返し、DBの thumbnail_url に
//! そのまま格納する（フロントは `:src` で表示でき、ローカルパスやasset protocolの配線が不要）。

use std::path::Path;
use std::process::Command;

/// 動画ファイルからサムネイルを生成し data URI を返す。失敗時は None。
pub fn generate_thumbnail_data_uri(video_path: &str) -> Option<String> {
    if !Path::new(video_path).exists() {
        return None;
    }
    // 先頭のロゴ/黒画面を避けたいので少し進めた位置から。短い動画は先頭にフォールバック
    for seek in ["10", "0"] {
        if let Some(bytes) = extract_frame_jpeg(video_path, seek) {
            return Some(format!("data:image/jpeg;base64,{}", base64_encode(&bytes)));
        }
    }
    None
}

/// 標準base64エンコード（依存追加を避けるための最小実装）
fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// ffmpegで1フレームをJPEGとして標準出力に取り出す（幅480pxへ縮小）
fn extract_frame_jpeg(video_path: &str, seek_seconds: &str) -> Option<Vec<u8>> {
    let output = Command::new("ffmpeg")
        .args(["-loglevel", "error", "-ss", seek_seconds, "-i"])
        .arg(video_path)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "scale=480:-2",
            "-q:v",
            "3",
            "-f",
            "image2pipe",
            "-vcodec",
            "mjpeg",
            "-",
        ])
        .env("PATH", crate::ytdlp::process::augmented_path_env())
        .output()
        .ok()?;

    if output.status.success() && !output.stdout.is_empty() {
        Some(output.stdout)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::base64_encode;

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }
}
