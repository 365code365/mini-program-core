//! MP4 / M4A 的编辑表（`moov/trak/edts/elst`）：AAC 编码器在开头垫的「预滚」帧（常见 1024 或 2112）与结尾的填充，
//! 靠编辑表告诉播放器从第几帧开始、放多长。symphonia 0.6 的 isomp4 读取器不看编辑表，解出来会多一截、整体后移
//! （GT-AU5 实测 1 秒 48k 的 m4a：49152 帧、偏 1024 帧；Chrome 与 ffmpeg 都按编辑表裁成 48000 帧）。
//! 这里只取第一条正常（media_time ≥ 0）的条目：(开头跳过的秒数, 保留的秒数)。多段编辑（剪辑拼接）不处理。

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

fn u64_at(b: &[u8], o: usize) -> Option<u64> {
    Some(u64::from_be_bytes(b.get(o..o + 8)?.try_into().ok()?))
}

/// 在 `b[from..to]` 里逐个盒子找名字为 `name` 的，返回它的内容区间
fn find(b: &[u8], from: usize, to: usize, name: &[u8; 4]) -> Option<(usize, usize)> {
    let mut at = from;
    while at + 8 <= to {
        let mut size = u32_at(b, at)? as usize;
        let mut head = 8;
        if size == 1 {
            size = u64_at(b, at + 8)? as usize;
            head = 16;
        } else if size == 0 {
            size = to - at;
        }
        if size < head || at + size > to {
            return None;
        }
        if &b[at + 4..at + 8] == name {
            return Some((at + head, at + size));
        }
        at += size;
    }
    None
}

/// mvhd / mdhd 的时间刻度
fn timescale(b: &[u8], body: (usize, usize)) -> Option<u32> {
    let v = *b.get(body.0)?;
    u32_at(b, body.0 + if v == 1 { 20 } else { 12 })
}

/// (开头跳过的秒数, 保留的秒数)。不是 MP4、没有编辑表、或编辑表不正常时 None
pub fn trim(b: &[u8]) -> Option<(f64, f64)> {
    if b.get(4..8)? != b"ftyp" {
        return None;
    }
    let moov = find(b, 0, b.len(), b"moov")?;
    let movie_scale = timescale(b, find(b, moov.0, moov.1, b"mvhd")?)? as f64;
    // 第一个音频轨：hdlr 的 handler_type = soun
    let mut at = moov.0;
    while let Some(trak) = find(b, at, moov.1, b"trak") {
        at = trak.1;
        let Some(mdia) = find(b, trak.0, trak.1, b"mdia") else { continue };
        let Some(hdlr) = find(b, mdia.0, mdia.1, b"hdlr") else { continue };
        if b.get(hdlr.0 + 8..hdlr.0 + 12) != Some(b"soun") {
            continue;
        }
        let media_scale = timescale(b, find(b, mdia.0, mdia.1, b"mdhd")?)? as f64;
        let edts = find(b, trak.0, trak.1, b"edts")?;
        let elst = find(b, edts.0, edts.1, b"elst")?;
        let v = *b.get(elst.0)?;
        let n = u32_at(b, elst.0 + 4)?;
        let mut o = elst.0 + 8;
        for _ in 0..n {
            let (dur, media_time) = if v == 1 {
                (u64_at(b, o)?, u64_at(b, o + 8)? as i64)
            } else {
                (u32_at(b, o)? as u64, u32_at(b, o + 4)? as i32 as i64)
            };
            o += if v == 1 { 20 } else { 12 };
            if media_time < 0 {
                continue; // 空编辑（开头留白）
            }
            if movie_scale <= 0.0 || media_scale <= 0.0 {
                return None;
            }
            return Some((media_time as f64 / media_scale, dur as f64 / movie_scale));
        }
        return None;
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn 不是_mp4_返回_none() {
        assert_eq!(super::trim(b"RIFF\0\0\0\0WAVE"), None);
        assert_eq!(super::trim(&[0u8; 4]), None);
    }
}
