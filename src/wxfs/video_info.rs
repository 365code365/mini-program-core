//! `wx.getVideoInfo` 的原生侧：直接读 MP4 / MOV 的盒子结构（`moov` → `mvhd` / `trak` → `tkhd` / `mdhd` /
//! `stsd` / `stts`），不解码画面。
//!
//! 回包与微信同形（官方 api/media/video/wx.getVideoInfo）：`orientation`（`up` / `down` / `left` / `right`，
//! 来自视频轨 `tkhd` 的变换矩阵）、`type`（容器格式）、`duration`（秒）、`size`（**kB**）、`width` / `height`
//! （编码尺寸，不随方向交换 —— 微信也是）、`fps`、`bitrate`（kbps）。
//!
//! 只认 ISO BMFF（mp4 / mov / m4v / 3gp）：别的容器给 `{err}`。`moov` 在文件尾（没做 faststart）也找得到 ——
//! 顶层盒子按长度跳着走，不把整个文件读进内存。

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct VideoInfo {
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub bitrate_kbps: f64,
    pub size_kb: f64,
    pub orientation: &'static str,
    pub kind: String,
    pub codec: String,
}

/// `{orientation,type,duration,size,width,height,fps,bitrate}` 或 `{err}`
pub fn info_json(src: &str) -> String {
    let r = crate::assets::resolve(src).ok_or_else(|| format!("file not found: {src}")).and_then(|p| parse_file(&p));
    match r {
        Ok(v) => serde_json::json!({
            "orientation": v.orientation,
            "type": v.kind,
            "duration": v.duration,
            "size": v.size_kb,
            "width": v.width,
            "height": v.height,
            "fps": v.fps,
            "bitrate": v.bitrate_kbps,
            "codec": v.codec,
        })
        .to_string(),
        Err(e) => serde_json::json!({ "err": e }).to_string(),
    }
}

pub fn parse_file(path: &std::path::Path) -> Result<VideoInfo, String> {
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    let len = f.metadata().map_err(|e| e.to_string())?.len();
    let mut kind = String::new();
    let mut moov = None;
    let mut pos = 0u64;
    // 顶层盒子：找 ftyp（格式）与 moov（元信息）
    while pos + 8 <= len {
        f.seek(SeekFrom::Start(pos)).map_err(|e| e.to_string())?;
        let (size, name, header) = read_box_header(&mut f, len - pos)?;
        if size == 0 {
            break;
        }
        match &name {
            b"ftyp" => {
                let mut brand = [0u8; 4];
                f.read_exact(&mut brand).map_err(|e| e.to_string())?;
                kind = brand_kind(&brand).to_string();
            }
            b"moov" => {
                let body = (size - header).min(64 * 1024 * 1024) as usize;
                let mut buf = vec![0u8; body];
                f.read_exact(&mut buf).map_err(|e| e.to_string())?;
                moov = Some(buf);
            }
            _ => {}
        }
        pos += size;
    }
    if kind.is_empty() {
        return Err("不支持的视频格式（只认 mp4 / mov）".into());
    }
    let moov = moov.ok_or("视频里没有 moov（文件不完整？）")?;
    let mut v = parse_moov(&moov)?;
    v.kind = kind;
    v.size_kb = (len as f64 / 1024.0 * 100.0).round() / 100.0;
    if v.duration > 0.0 {
        v.bitrate_kbps = ((len as f64 * 8.0 / 1000.0 / v.duration) * 100.0).round() / 100.0;
    }
    Ok(v)
}

fn brand_kind(brand: &[u8; 4]) -> &'static str {
    match brand {
        b"qt  " => "mov",
        b"3gp4" | b"3gp5" | b"3gp6" | b"3g2a" => "3gp",
        b"M4V " | b"M4VH" | b"M4VP" => "m4v",
        _ => "mp4",
    }
}

/// (盒子总长, 名字, 头长)。`size == 1` 是 64 位长度，`0` 表示到文件尾
fn read_box_header<R: Read>(r: &mut R, remaining: u64) -> Result<(u64, [u8; 4], u64), String> {
    let mut h = [0u8; 8];
    r.read_exact(&mut h).map_err(|e| e.to_string())?;
    let mut size = u32::from_be_bytes([h[0], h[1], h[2], h[3]]) as u64;
    let name = [h[4], h[5], h[6], h[7]];
    let mut header = 8;
    if size == 1 {
        let mut l = [0u8; 8];
        r.read_exact(&mut l).map_err(|e| e.to_string())?;
        size = u64::from_be_bytes(l);
        header = 16;
    } else if size == 0 {
        size = remaining;
    }
    if size < header {
        return Err("视频文件结构损坏".into());
    }
    Ok((size, name, header))
}

/// 内存里的一串盒子：按名字迭代 (名字, 内容)
fn boxes(data: &[u8]) -> Vec<([u8; 4], &[u8])> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 8 <= data.len() {
        let mut size = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        let name = [data[i + 4], data[i + 5], data[i + 6], data[i + 7]];
        let mut header = 8;
        if size == 1 && i + 16 <= data.len() {
            size = u64::from_be_bytes(data[i + 8..i + 16].try_into().unwrap()) as usize;
            header = 16;
        } else if size == 0 {
            size = data.len() - i;
        }
        if size < header || i + size > data.len() {
            break;
        }
        out.push((name, &data[i + header..i + size]));
        i += size;
    }
    out
}

fn child<'a>(data: &'a [u8], name: &[u8; 4]) -> Option<&'a [u8]> {
    boxes(data).into_iter().find(|(n, _)| n == name).map(|(_, b)| b)
}

fn be32(b: &[u8], at: usize) -> u64 {
    b.get(at..at + 4).map(|s| u32::from_be_bytes(s.try_into().unwrap()) as u64).unwrap_or(0)
}

fn be64(b: &[u8], at: usize) -> u64 {
    b.get(at..at + 8).map(|s| u64::from_be_bytes(s.try_into().unwrap())).unwrap_or(0)
}

/// `mvhd` / `mdhd`：(timescale, duration)，version 0 是 32 位、1 是 64 位
fn header_times(b: &[u8]) -> (u64, u64) {
    match b.first() {
        Some(1) => (be32(b, 20), be64(b, 24)),
        _ => (be32(b, 12), be32(b, 16)),
    }
}

fn parse_moov(moov: &[u8]) -> Result<VideoInfo, String> {
    let mut v = VideoInfo { orientation: "up", ..Default::default() };
    if let Some(mvhd) = child(moov, b"mvhd") {
        let (ts, d) = header_times(mvhd);
        if ts > 0 {
            v.duration = d as f64 / ts as f64;
        }
    }
    for (name, trak) in boxes(moov) {
        if &name != b"trak" {
            continue;
        }
        let Some(mdia) = child(trak, b"mdia") else { continue };
        let is_video = child(mdia, b"hdlr").map(|h| h.get(8..12) == Some(b"vide")).unwrap_or(false);
        if !is_video {
            continue;
        }
        if let Some(tkhd) = child(trak, b"tkhd") {
            // version 0：矩阵在 40、宽高在 76 / 80；version 1 各往后 12
            let off = if tkhd.first() == Some(&1) { 12 } else { 0 };
            let m = |i: usize| be32(tkhd, 40 + off + i * 4) as u32 as i32;
            v.orientation = orientation(m(0), m(1), m(3), m(4));
            v.width = (be32(tkhd, 76 + off) >> 16) as u32;
            v.height = (be32(tkhd, 80 + off) >> 16) as u32;
        }
        let (ts, media_dur) = child(mdia, b"mdhd").map(header_times).unwrap_or((0, 0));
        let stbl = child(mdia, b"minf").and_then(|m| child(m, b"stbl"));
        if let Some(stbl) = stbl {
            if let Some(stsd) = child(stbl, b"stsd") {
                // stsd：version/flags(4) + 条目数(4) + 第一条（长度 4 + 4cc）
                if let Some(cc) = stsd.get(12..16) {
                    v.codec = String::from_utf8_lossy(cc).trim().to_string();
                }
                // 宽高没写在 tkhd 里（极少见）时取视觉样本描述里的
                if v.width == 0 {
                    v.width = stsd.get(40..42).map(|s| u16::from_be_bytes([s[0], s[1]]) as u32).unwrap_or(0);
                    v.height = stsd.get(42..44).map(|s| u16::from_be_bytes([s[0], s[1]]) as u32).unwrap_or(0);
                }
            }
            if let Some(stts) = child(stbl, b"stts") {
                let n = be32(stts, 4) as usize;
                let samples: u64 = (0..n.min(100_000)).map(|i| be32(stts, 8 + i * 8)).sum();
                if ts > 0 && media_dur > 0 {
                    v.fps = ((samples as f64 * ts as f64 / media_dur as f64) * 100.0).round() / 100.0;
                }
            }
        }
        if v.duration <= 0.0 && ts > 0 {
            v.duration = media_dur as f64 / ts as f64;
        }
        break;
    }
    if v.width == 0 && v.height == 0 {
        return Err("视频里没有画面轨".into());
    }
    v.duration = (v.duration * 1000.0).round() / 1000.0;
    Ok(v)
}

/// 16.16 定点矩阵 [a b; c d] → 方向（只认 90° 的倍数）
fn orientation(a: i32, b: i32, c: i32, d: i32) -> &'static str {
    let one = 1 << 16;
    match (a, b, c, d) {
        (0, x, y, 0) if x == one && y == -one => "right",
        (0, x, y, 0) if x == -one && y == one => "left",
        (x, 0, 0, y) if x == -one && y == -one => "down",
        _ => "up",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_matrix_maps_to_orientation() {
        let one = 1 << 16;
        assert_eq!(orientation(one, 0, 0, one), "up");
        assert_eq!(orientation(0, one, -one, 0), "right");
        assert_eq!(orientation(0, -one, one, 0), "left");
        assert_eq!(orientation(-one, 0, 0, -one), "down");
    }

    #[test]
    fn sample_mp4_has_sane_metadata() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sample/sample-app/assets/oceans.mp4");
        if !p.is_file() {
            return;
        }
        let v = parse_file(&p).expect("解析 oceans.mp4");
        assert_eq!(v.kind, "mp4");
        assert!(v.width > 0 && v.height > 0, "{v:?}");
        assert!(v.duration > 1.0, "{v:?}");
        assert!(v.fps > 10.0 && v.fps < 121.0, "{v:?}");
        assert!(v.bitrate_kbps > 0.0 && v.size_kb > 0.0);
    }

    #[test]
    fn non_mp4_is_rejected() {
        let p = std::env::temp_dir().join("mini-render-not-video.bin");
        std::fs::write(&p, b"definitely not a video file at all").unwrap();
        assert!(parse_file(&p).is_err());
        std::fs::remove_file(&p).ok();
    }
}
