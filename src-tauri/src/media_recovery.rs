//! PNG偽装HLS（turbosplayer / turboviplay 系）で生成された再生不能MP4の復元。
//!
//! これらのサイトは各TSセグメントの先頭に 1x1 のダミーPNG + 0xFF パディングを付けて
//! 配信する。yt-dlp/ffmpeg はセグメントを画像と誤認し、mdat内に
//! `[PNGヘッダ][パディング][本物のMPEG-TS]` が連なった再生不能ファイルを吐く。
//!
//! ダウンロード完了後にこのモジュールを通し、mdat先頭がPNG署名の場合のみ
//! 各セグメントからTS本体を彫り出して結合し、ffmpegでremuxして原ファイルを置き換える。
//! 通常の動画は mdat がPNG署名で始まらないため、即座に false を返して素通りする。

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::process::Command;

const PNG_SIG: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
/// MPEG-TSパケットの同期バイトと固定長。3連続で188バイト整列していればTS開始とみなす
const TS_SYNC: u8 = 0x47;
const TS_PACKET: usize = 188;

/// PNG偽装MP4なら復元して原ファイルを置き換える。復元したら Ok(true)。
/// 通常ファイル・非対象は Ok(false)。エラーは呼び出し側でログするだけで良い
/// （復元失敗時も原ファイルは温存される）。
pub fn recover_if_png_disguised(path: &Path) -> Result<bool, String> {
    let Some((mdat_start, mdat_len)) = find_mdat(path)? else {
        return Ok(false);
    };
    if !mdat_starts_with_png(path, mdat_start)? {
        return Ok(false);
    }

    let mut file = File::open(path).map_err(|e| format!("open failed: {e}"))?;
    file.seek(SeekFrom::Start(mdat_start))
        .map_err(|e| format!("seek failed: {e}"))?;
    let mut mdat = vec![0u8; mdat_len];
    file.read_exact(&mut mdat)
        .map_err(|e| format!("mdat read failed: {e}"))?;

    let markers = find_png_markers(&mdat);
    if markers.is_empty() {
        return Ok(false);
    }

    // セグメントごとにTS本体を彫り出して結合
    let ts_path = path.with_extension("recover.ts");
    {
        use std::io::Write;
        let mut out = File::create(&ts_path).map_err(|e| format!("temp create failed: {e}"))?;
        let mut wrote_any = false;
        for window in markers.windows(2).chain(std::iter::once(
            [*markers.last().unwrap(), mdat.len()].as_slice(),
        )) {
            let (seg_start, seg_end) = (window[0], window[1]);
            if let Some(ts_start) = find_ts_start(&mdat[seg_start..seg_end]) {
                out.write_all(&mdat[seg_start + ts_start..seg_end])
                    .map_err(|e| format!("temp write failed: {e}"))?;
                wrote_any = true;
            }
        }
        if !wrote_any {
            let _ = std::fs::remove_file(&ts_path);
            return Ok(false);
        }
    }

    // ffmpegでremux（再エンコードなし）。成功時のみ原ファイルを置き換える
    let out_tmp = path.with_extension("recover.mp4");
    let remux = remux_ts_to_mp4(&ts_path, &out_tmp);
    let _ = std::fs::remove_file(&ts_path);
    remux?;

    std::fs::rename(&out_tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&out_tmp);
        format!("replace failed: {e}")
    })?;
    Ok(true)
}

/// トップレベルboxを走査して最初のmdatの (ペイロード開始オフセット, ペイロード長) を返す
fn find_mdat(path: &Path) -> Result<Option<(u64, usize)>, String> {
    let mut file = File::open(path).map_err(|e| format!("open failed: {e}"))?;
    let file_len = file
        .metadata()
        .map_err(|e| format!("metadata failed: {e}"))?
        .len();

    let mut offset: u64 = 0;
    while offset + 8 <= file_len {
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| format!("seek failed: {e}"))?;
        let mut header = [0u8; 8];
        if file.read_exact(&mut header).is_err() {
            break;
        }
        let size32 = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
        let box_type = &header[4..8];

        // box全体サイズとヘッダ長を決定（32bit / 64bit / EOFまで の3形式）
        let (box_size, header_len) = match size32 {
            1 => {
                let mut ext = [0u8; 8];
                file.read_exact(&mut ext)
                    .map_err(|e| format!("largesize read failed: {e}"))?;
                (u64::from_be_bytes(ext), 16u64)
            }
            0 => (file_len - offset, 8u64),
            n => (n as u64, 8u64),
        };
        if box_size < header_len {
            break; // 壊れたbox
        }

        if box_type == b"mdat" {
            let payload_start = offset + header_len;
            let payload_len = (box_size - header_len).min(file_len - payload_start);
            return Ok(Some((payload_start, payload_len as usize)));
        }
        offset += box_size;
    }
    Ok(None)
}

fn mdat_starts_with_png(path: &Path, mdat_start: u64) -> Result<bool, String> {
    let mut file = File::open(path).map_err(|e| format!("open failed: {e}"))?;
    file.seek(SeekFrom::Start(mdat_start))
        .map_err(|e| format!("seek failed: {e}"))?;
    let mut head = [0u8; 8];
    if file.read_exact(&mut head).is_err() {
        return Ok(false);
    }
    Ok(head == PNG_SIG)
}

/// バイト列内のPNG署名の出現位置（=各セグメント先頭）をすべて返す
fn find_png_markers(data: &[u8]) -> Vec<usize> {
    let mut markers = Vec::new();
    let mut pos = 0;
    while pos + PNG_SIG.len() <= data.len() {
        if data[pos..pos + PNG_SIG.len()] == PNG_SIG {
            markers.push(pos);
            pos += PNG_SIG.len();
        } else {
            pos += 1;
        }
    }
    markers
}

/// セグメント内でMPEG-TSが始まる位置（188バイト整列で0x47が3連続）を探す
fn find_ts_start(segment: &[u8]) -> Option<usize> {
    let limit = segment.len().checked_sub(2 * TS_PACKET + 1)?;
    (0..=limit).find(|&i| {
        segment[i] == TS_SYNC
            && segment[i + TS_PACKET] == TS_SYNC
            && segment[i + 2 * TS_PACKET] == TS_SYNC
    })
}

/// TSをmp4へストリームコピーremux（映像+音声のみ採用、重複ストリームは捨てる）
fn remux_ts_to_mp4(ts_path: &Path, out_path: &Path) -> Result<(), String> {
    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-fflags",
            "+genpts",
            "-i",
        ])
        .arg(ts_path)
        .args([
            "-map",
            "0:v:0",
            "-map",
            "0:a:0?",
            "-c",
            "copy",
            "-movflags",
            "+faststart",
        ])
        .arg(out_path)
        .env("PATH", crate::ytdlp::process::augmented_path_env())
        .output()
        .map_err(|e| format!("ffmpeg spawn failed: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let _ = std::fs::remove_file(out_path);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("ffmpeg remux failed: {}", stderr.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_png_markers_and_ts_start() {
        // [PNG署名][パディング0xFF×3][TSパケット×3] を1セグメントとして組む
        let mut seg = Vec::new();
        seg.extend_from_slice(&PNG_SIG);
        seg.extend_from_slice(&[0xFF, 0xFF, 0xFF]);
        let ts_offset = seg.len();
        for _ in 0..3 {
            seg.push(TS_SYNC);
            seg.extend_from_slice(&[0u8; TS_PACKET - 1]);
        }
        assert_eq!(find_png_markers(&seg), vec![0]);
        assert_eq!(find_ts_start(&seg), Some(ts_offset));
    }

    #[test]
    fn ignores_data_without_ts_sync() {
        let seg = [0x00u8; 100];
        assert_eq!(find_ts_start(&seg), None);
    }

    /// 実際のPNG偽装ファイルに対するエンドツーエンド検証（要ffmpeg・実ファイル）。
    /// `cargo test recovers_real_disguised_file -- --ignored --nocapture`
    #[test]
    #[ignore = "実ファイル(/Volumes/Logitec2/soko/master.mp4)とffmpegが必要"]
    fn recovers_real_disguised_file() {
        let src = Path::new("/Volumes/Logitec2/soko/master.mp4");
        if !src.exists() {
            println!("skip: source not found");
            return;
        }
        let tmp = std::env::temp_dir().join("ytdown_recover_test.mp4");
        std::fs::copy(src, &tmp).expect("copy");

        // 復元前はmdatがPNG署名で始まる
        let (start, _) = find_mdat(&tmp).expect("find_mdat").expect("has mdat");
        assert!(mdat_starts_with_png(&tmp, start).unwrap());

        let recovered = recover_if_png_disguised(&tmp).expect("recover");
        assert!(recovered, "should report recovered");

        // 復元後は正常なmp4（mdatがPNG署名で始まらない）
        let (start2, _) = find_mdat(&tmp).expect("find_mdat2").expect("has mdat2");
        assert!(!mdat_starts_with_png(&tmp, start2).unwrap());
        let size = std::fs::metadata(&tmp).unwrap().len();
        println!("recovered size: {size}");
        assert!(size > 1_000_000);
        let _ = std::fs::remove_file(&tmp);
    }
}
