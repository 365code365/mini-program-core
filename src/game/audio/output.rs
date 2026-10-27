//! 输出：真设备（cpal：Android AAudio、iOS / macOS CoreAudio，决策 D-4 = A）或离线（无头 / 设备打不开）。
//!
//! - 设备模式：混音器整个搬进 cpal 的输出回调（音频线程独占，无锁）；采样格式不是 f32 的设备（不少 Android 机给 i16）
//!   在回调里经一块**预先分配**的 f32 缓冲转换；
//! - 离线模式：混音器留在引擎线程，pump 时按帧时钟补齐「到此刻应该输出的帧数」，每次 128 帧（Web Audio 的渲染块大小）。
//!   无头测试据此逐样本可复现；`MINI_AUDIO_DUMP=<文件>` 把输出写成小端 f32（GT-AU2 / AU4 对拍用）。
//!   设备打不开（`MINI_AUDIO_FAULT=open` 可注入）同样落到离线：时钟照走、事件照发，只是不出声（GT-AU9）。

use super::mixer::Mixer;
use std::io::Write;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

/// 离线模式的采样率（与 Chrome OfflineAudioContext 对拍时固定 48k）
pub const OFFLINE_RATE: u32 = 48_000;
const BLOCK: usize = 128;

/// 设备回调写的统计（引擎线程读）
#[derive(Default)]
pub struct Stats {
    /// 最近一次回调「回调时刻 → 预计出声时刻」（微秒）
    pub latency_us: AtomicU32,
    /// 最近一次回调的帧数
    pub block_frames: AtomicU32,
    pub callbacks: AtomicU64,
    /// 已输出帧数（播放时钟）
    pub frames: AtomicU64,
}

pub enum Output {
    #[cfg(feature = "game-audio")]
    Device { _stream: cpal::Stream, stats: Arc<Stats> },
    Offline { mixer: Box<Mixer>, done: u64, origin_us: Option<u64>, buf: Vec<f32>, dump: Option<std::fs::File>, channels: usize },
}

pub struct Opened {
    pub out: Output,
    pub rate: u32,
    /// 声道数（日志 / 诊断）
    #[allow(dead_code)]
    pub channels: usize,
    /// 说明（日志用）：用的是什么设备 / 为什么落到离线
    pub note: String,
}

/// `make(rate, channels)` 在知道设备格式之后建混音器与引擎线程那一侧的队列端点 `E`。
/// 设备启动失败时混音器已经交给 cpal 了拿不回来：再调一次 `make` 给离线模式建一套新的
pub fn open<E>(offline: bool, mut make: impl FnMut(u32, usize) -> (Mixer, E)) -> (Opened, E) {
    #[cfg(feature = "game-audio")]
    if !offline {
        if std::env::var("MINI_AUDIO_FAULT").as_deref() == Ok("open") {
            return offline_out(make, "设备打开失败（故障注入 MINI_AUDIO_FAULT=open），静音模式".into());
        }
        let dev = match device::open() {
            Ok(d) => d,
            Err(e) => return offline_out(make, format!("没有可用的输出设备（{e}），静音模式")),
        };
        let (rate, channels) = (dev.rate, dev.channels);
        let (mixer, ends) = make(rate, channels);
        return match dev.start(mixer) {
            Ok((stream, stats, note)) => (Opened { out: Output::Device { _stream: stream, stats }, rate, channels, note }, ends),
            Err(e) => offline_out(make, format!("设备启动失败（{e}），静音模式")),
        };
    }
    let _ = offline;
    offline_out(make, "离线模式（无头 / 没编进 game-audio）".into())
}

fn offline_out<E>(mut make: impl FnMut(u32, usize) -> (Mixer, E), note: String) -> (Opened, E) {
    let (m, ends) = make(OFFLINE_RATE, 2);
    let dump = std::env::var("MINI_AUDIO_DUMP").ok().and_then(|p| std::fs::File::create(p).ok());
    let out = Output::Offline { mixer: Box::new(m), done: 0, origin_us: None, buf: vec![0.0; BLOCK * 2], dump, channels: 2 };
    (Opened { out, rate: OFFLINE_RATE, channels: 2, note }, ends)
}

impl Output {
    /// 离线模式：补齐到帧时刻 `t_us` 应该输出的帧（设备模式什么都不做）
    pub fn advance(&mut self, t_us: u64, rate: u32) {
        if let Output::Offline { mixer, done, origin_us, buf, dump, channels } = self {
            let origin = *origin_us.get_or_insert(t_us);
            let target = (t_us.saturating_sub(origin) as u128 * rate as u128 / 1_000_000) as u64;
            while *done + BLOCK as u64 <= target {
                mixer.render(buf);
                *done += BLOCK as u64;
                if let Some(f) = dump {
                    let bytes: Vec<u8> = buf.iter().flat_map(|s| s.to_le_bytes()).collect();
                    let _ = f.write_all(&bytes);
                }
            }
            let _ = channels;
        }
    }

    /// 已经输出的帧数
    pub fn frames(&self) -> u64 {
        match self {
            #[cfg(feature = "game-audio")]
            Output::Device { stats, .. } => stats.frames.load(Ordering::Relaxed),
            Output::Offline { done, .. } => *done,
        }
    }

    /// (延迟 µs, 回调块帧数, 回调次数)；离线全 0
    pub fn stats(&self) -> (u32, u32, u64) {
        match self {
            #[cfg(feature = "game-audio")]
            Output::Device { stats, .. } => (
                stats.latency_us.load(Ordering::Relaxed),
                stats.block_frames.load(Ordering::Relaxed),
                stats.callbacks.load(Ordering::Relaxed),
            ),
            Output::Offline { .. } => (0, 0, 0),
        }
    }

    pub fn is_device(&self) -> bool {
        !matches!(self, Output::Offline { .. })
    }
}

#[cfg(feature = "game-audio")]
mod device {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    /// 回调一次最多处理的帧数（i16 设备的转换缓冲按它预分配；回调要的更多就分几段）
    const MAX_BLOCK: usize = 4096;

    pub struct Dev {
        device: cpal::Device,
        config: cpal::StreamConfig,
        format: cpal::SampleFormat,
        pub rate: u32,
        pub channels: usize,
    }

    pub fn open() -> Result<Dev, String> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("default_output_device 为空")?;
        let sup = device.default_output_config().map_err(|e| e.to_string())?;
        let format = sup.sample_format();
        let config: cpal::StreamConfig = sup.into();
        Ok(Dev { rate: config.sample_rate, channels: config.channels as usize, device, config, format })
    }

    impl Dev {
        pub fn start(self, mixer: Mixer) -> Result<(cpal::Stream, Arc<Stats>, String), String> {
            let stats = Arc::new(Stats::default());
            let st = stats.clone();
            let ch = self.channels;
            let err = |e: cpal::StreamError| eprintln!("🔊 音频输出出错：{e}");
            let note = format!("设备 {} Hz × {} 声道，{:?}", self.rate, ch, self.format);
            let mut mixer = Box::new(mixer);
            let stream = match self.format {
                cpal::SampleFormat::F32 => self.device.build_output_stream(
                    &self.config,
                    move |out: &mut [f32], info: &cpal::OutputCallbackInfo| {
                        mixer.render(out);
                        note_cb(&st, info, out.len() / ch);
                    },
                    err,
                    None,
                ),
                cpal::SampleFormat::I16 => {
                    let mut scratch = vec![0f32; MAX_BLOCK * ch];
                    self.device.build_output_stream(
                        &self.config,
                        move |out: &mut [i16], info: &cpal::OutputCallbackInfo| {
                            for part in out.chunks_mut(MAX_BLOCK * ch) {
                                let s = &mut scratch[..part.len()];
                                mixer.render(s);
                                for (o, v) in part.iter_mut().zip(s.iter()) {
                                    *o = (v * 32767.0) as i16;
                                }
                            }
                            note_cb(&st, info, out.len() / ch);
                        },
                        err,
                        None,
                    )
                }
                f => return Err(format!("不支持的采样格式 {f:?}")),
            };
            let stream = stream.map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, stats, note))
        }
    }

    fn note_cb(st: &Stats, info: &cpal::OutputCallbackInfo, frames: usize) {
        let ts = info.timestamp();
        if let Some(d) = ts.playback.duration_since(&ts.callback) {
            st.latency_us.store(d.as_micros().min(u32::MAX as u128) as u32, Ordering::Relaxed);
        }
        st.block_frames.store(frames as u32, Ordering::Relaxed);
        st.callbacks.fetch_add(1, Ordering::Relaxed);
        st.frames.fetch_add(frames as u64, Ordering::Relaxed);
    }
}
