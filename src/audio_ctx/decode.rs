//! 把一个音频源读成交错的 f32 样本：本地 / 包内 / `wxfile://` 路径直接读，http(s) 先下载。
//!
//! 容器与编码交给 symphonia（mp4/m4a 的 AAC、mp3，与 `<video>` 的音轨同一套）；
//! WAV（PCM 16/24/32 位整型与 32 位浮点）自己解析 —— 格式简单，不值得为它再开 symphonia 的两个特性。

use std::sync::Arc;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::codecs::CodecParameters;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// 解码结果
#[derive(Debug, Clone)]
pub struct Pcm {
    pub samples: Arc<Vec<f32>>,
    pub rate: u32,
    pub channels: u16,
}

impl Pcm {
    pub fn duration(&self) -> f64 {
        self.samples.len() as f64 / (self.rate.max(1) as f64 * self.channels.max(1) as f64)
    }
}

/// 失败时给 (errCode, 原因)：10002 网络、10003 文件、10004 格式（微信 InnerAudioContext.onError 的码表）
pub fn load(src: &str) -> Result<Pcm, (i32, String)> {
    let (bytes, ext) = if src.starts_with("http://") || src.starts_with("https://") {
        let (status, _h, body) = crate::net::fetch_bytes("GET", src, &[], Vec::new(), std::time::Duration::from_secs(60))
            .map_err(|e| (10002, e))?;
        if !(200..300).contains(&status) {
            return Err((10002, format!("HTTP {status}")));
        }
        (body, crate::wxfile::ext_of_url(src))
    } else {
        let path = crate::assets::resolve(src).ok_or((10003, format!("file not found: {src}")))?;
        let bytes = std::fs::read(&path).map_err(|e| (10003, e.to_string()))?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        (bytes, ext)
    };
    decode_bytes(bytes, &ext)
}

/// 已经读进内存的音频（WAV 自己解析，其余交给 symphonia）；失败码 10004（格式）
pub fn decode_bytes(bytes: Vec<u8>, ext: &str) -> Result<Pcm, (i32, String)> {
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return wav(&bytes).ok_or((10004, "unsupported wav".into()));
    }
    decode(bytes, ext).map_err(|e| (10004, e))
}

fn decode(bytes: Vec<u8>, ext: &str) -> Result<Pcm, String> {
    let edit = super::mp4_edit::trim(&bytes);
    let mss = MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes)), Default::default());
    let mut hint = Hint::new();
    if !ext.is_empty() {
        hint.with_extension(ext);
    }
    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
        .map_err(|e| format!("probe: {e}"))?;
    let (track_id, params) = format
        .tracks()
        .iter()
        .find_map(|t| match &t.codec_params {
            Some(CodecParameters::Audio(p)) => Some((t.id, p.clone())),
            _ => None,
        })
        .ok_or("no audio track")?;
    let rate = params.sample_rate.unwrap_or(44100);
    let channels = params.channels.as_ref().map(|c| c.count() as u16).unwrap_or(2);
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .map_err(|e| format!("decoder: {e}"))?;
    let mut out = Vec::new();
    let mut chunk = Vec::new();
    while let Ok(Some(packet)) = format.next_packet() {
        if packet.track_id != track_id {
            continue;
        }
        if let Ok(decoded) = decoder.decode(&packet) {
            chunk.clear();
            decoded.copy_to_vec_interleaved(&mut chunk);
            // 编码器延迟 / 填充（mp3 的 LAME 头、ogg 的 granule）symphonia 的解码器自己按包的 trim_start / trim_end 裁过了，
            // 这里再裁就多裁一截（GT-AU5 实测 mp3 少 478 帧）
            out.extend_from_slice(&chunk);
        }
    }
    // MP4 的编辑表（AAC 的预滚帧，isomp4 读取器不处理，见 mp4_edit.rs）
    if let Some((skip, keep)) = edit {
        let ch = channels.max(1) as usize;
        let total = out.len() / ch;
        let a = ((skip * rate as f64).round() as usize).min(total);
        let e = (a + (keep * rate as f64).round() as usize).min(total);
        if keep > 0.0 && e > a {
            out = out[a * ch..e * ch].to_vec();
        }
    }
    if out.is_empty() {
        return Err("no audio samples".into());
    }
    Ok(Pcm { samples: Arc::new(out), rate, channels })
}

/// RIFF/WAVE：fmt 块 + data 块
fn wav(b: &[u8]) -> Option<Pcm> {
    let mut at = 12;
    let (mut fmt, mut channels, mut rate, mut bits) = (0u16, 0u16, 0u32, 0u16);
    while at + 8 <= b.len() {
        let id = &b[at..at + 4];
        let len = u32::from_le_bytes([b[at + 4], b[at + 5], b[at + 6], b[at + 7]]) as usize;
        let body = b.get(at + 8..(at + 8 + len).min(b.len()))?;
        if id == b"fmt " && body.len() >= 16 {
            fmt = u16::from_le_bytes([body[0], body[1]]);
            channels = u16::from_le_bytes([body[2], body[3]]);
            rate = u32::from_le_bytes([body[4], body[5], body[6], body[7]]);
            bits = u16::from_le_bytes([body[14], body[15]]);
            if fmt == 0xFFFE && body.len() >= 26 {
                fmt = u16::from_le_bytes([body[24], body[25]]); // WAVE_FORMAT_EXTENSIBLE 的子格式
            }
        } else if id == b"data" {
            let samples: Vec<f32> = match (fmt, bits) {
                (1, 8) => body.iter().map(|&s| (s as f32 - 128.0) / 128.0).collect(),
                (1, 16) => body.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0).collect(),
                (1, 24) => body
                    .chunks_exact(3)
                    .map(|c| (((c[2] as i32) << 24 | (c[1] as i32) << 16 | (c[0] as i32) << 8) >> 8) as f32 / 8_388_608.0)
                    .collect(),
                (1, 32) => body.chunks_exact(4).map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32 / 2_147_483_648.0).collect(),
                (3, 32) => body.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect(),
                _ => return None,
            };
            if channels == 0 || rate == 0 {
                return None;
            }
            return Some(Pcm { samples: Arc::new(samples), rate, channels });
        }
        at += 8 + len + (len & 1);
    }
    None
}

/// 测试与夹具用：生成一段 16 位 PCM 的 WAV
pub fn make_wav(rate: u32, channels: u16, seconds: f32) -> Vec<u8> {
    let frames = (rate as f32 * seconds) as usize;
    let mut data = Vec::with_capacity(frames * channels as usize * 2);
    for i in 0..frames {
        let v = ((i as f32 / rate as f32 * 440.0 * std::f32::consts::TAU).sin() * 8000.0) as i16;
        for _ in 0..channels {
            data.extend_from_slice(&v.to_le_bytes());
        }
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * channels as u32 * 2).to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_generated_wav() {
        let pcm = wav(&make_wav(8000, 2, 0.5)).unwrap();
        assert_eq!(pcm.rate, 8000);
        assert_eq!(pcm.channels, 2);
        assert!((pcm.duration() - 0.5).abs() < 1e-3);
    }

    #[test]
    fn missing_file_is_a_file_error() {
        assert_eq!(load("/definitely/not/here.mp3").unwrap_err().0, 10003);
    }
}
