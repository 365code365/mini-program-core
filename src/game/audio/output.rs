//! 输出：真设备（桌面 / iOS 用 cpal 的 CoreAudio 等；Android 用 aaudio.rs 运行时 dlopen 的 AAudio，决策 D-4 = A）或离线（无头 / 设备打不开）。
//!
//! - 设备模式：混音器整个搬进 cpal 的输出回调（音频线程独占，无锁）；采样格式不是 f32 的设备（不少 Android 机给 i16）
//!   在回调里经一块**预先分配**的 f32 缓冲转换；
//! - 离线模式：混音器留在引擎线程，pump 时按帧时钟补齐「到此刻应该输出的帧数」，每次 128 帧（Web Audio 的渲染块大小）。
//!   无头测试据此逐样本可复现；`MINI_AUDIO_DUMP=<文件>` 把输出写成小端 f32（GT-AU2 / AU4 对拍用）。
//!   设备打不开（`MINI_AUDIO_FAULT=open` 可注入）同样落到离线：时钟照走、事件照发，只是不出声（GT-AU9）。

use super::mixer::Mixer;
use std::io::Write;
use std::sync::atomic::{AtomicU32, AtomicU64};
use std::sync::atomic::Ordering;
#[cfg(all(feature = "game-audio", not(target_os = "android")))]
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
    /// 上次读走以来输出样本 |s| 的最大值（f32 的位；非负浮点的位按整数比大小与按数值比一致，所以能 fetch_max）。
    /// 诊断「设备在跑、为什么没声」：峰值 0 = 混音器出的是静音，峰值有 = 数据交给系统了，问题在会话 / 路由 / 静音键
    pub peak_bits: AtomicU32,
}

impl Stats {
    /// 回调里记峰值（一块几百个样本求最大值，不分配、不加锁）
    pub fn note_peak(&self, out: &[f32]) {
        let m = out.iter().fold(0f32, |m, s| m.max(s.abs()));
        self.peak_bits.fetch_max(m.to_bits(), Ordering::Relaxed);
    }

    /// 读走峰值并清零（引擎线程）
    pub fn take_peak(&self) -> f32 {
        f32::from_bits(self.peak_bits.swap(0, Ordering::Relaxed))
    }
}

pub enum Output {
    #[cfg(all(feature = "game-audio", not(target_os = "android")))]
    Device { _stream: cpal::Stream, stats: Arc<Stats>, watch: Watch },
    #[cfg(all(feature = "game-audio", target_os = "android"))]
    Android(super::aaudio::Out),
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
/// `fault`：创建选项 fault 或 `MINI_AUDIO_FAULT`（前者优先）。`stall@N` = 设备第 N 次回调之后不再计数（看门狗看到的就是
/// 「系统停了 I/O」，验它会重启）；`open` = 当作设备打不开
pub fn open<E>(offline: bool, fault: Option<&str>, mut make: impl FnMut(u32, usize) -> (Mixer, E)) -> (Opened, E) {
    let fault = fault.map(str::to_string).or_else(|| std::env::var("MINI_AUDIO_FAULT").ok()).unwrap_or_default();
    let _ = &fault;
    #[cfg(feature = "game-audio")]
    if !offline {
        if fault == "open" {
            return offline_out(make, "设备打开失败（故障注入 MINI_AUDIO_FAULT=open），静音模式".into());
        }
        #[cfg(target_os = "android")]
        return match super::aaudio::open(&mut make) {
            Ok((out, ends, note)) => {
                let (rate, channels) = (out.rate, out.channels);
                (Opened { out: Output::Android(out), rate, channels, note }, ends)
            }
            Err(e) => offline_out(make, format!("{e}，静音模式")),
        };
        #[cfg(not(target_os = "android"))]
        let dev = match device::open() {
            Ok(d) => d,
            Err(e) => return offline_out(make, format!("没有可用的输出设备（{e}），静音模式")),
        };
        #[cfg(not(target_os = "android"))]
        let (rate, channels) = (dev.rate, dev.channels);
        #[cfg(not(target_os = "android"))]
        let (mixer, ends) = make(rate, channels);
        #[cfg(not(target_os = "android"))]
        let stall = fault.strip_prefix("stall@").and_then(|n| n.parse().ok());
        #[cfg(not(target_os = "android"))]
        return match dev.start(mixer, stall) {
            Ok((stream, stats, note)) => (Opened { out: Output::Device { _stream: stream, stats, watch: Watch::default() }, rate, channels, note }, ends),
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

    /// 设备模式的维护（Android：设备断开重开、延迟估计；iOS / 桌面：输出停了就重启）；有要记的日志就返回。
    /// `quiet` = 引擎这边本来就不要声音（后台 / 系统打断中）：这时候设备停着是对的，不去拉它
    pub fn poll(&mut self, quiet: bool) -> Option<String> {
        let _ = quiet;
        match self {
            #[cfg(all(feature = "game-audio", target_os = "android"))]
            Output::Android(o) => o.poll(),
            #[cfg(all(feature = "game-audio", not(target_os = "android")))]
            Output::Device { _stream, stats, watch } => {
                use cpal::traits::StreamTrait;
                let now = std::time::Instant::now();
                let stalled_ms = watch.check(stats.callbacks.load(Ordering::Relaxed), now, quiet)?;
                // pause 再 play = AudioUnitStop + Start：cpal 记着「在放」时单调 play 什么都不做，而系统停掉的 I/O 它不知道
                let _ = _stream.pause();
                let r = _stream.play();
                watch.restarts += 1;
                // 前几次都报，之后每 30 次报一次（会话一直起不来时别刷屏）
                (watch.restarts <= 3 || watch.restarts % 30 == 0).then(|| match r {
                    Ok(()) => format!("🔊 音频输出 {stalled_ms}ms 没拉数据（系统停了 I/O：会话被停用 / 打断 / 路由变了），已重启（第 {} 次）", watch.restarts),
                    Err(e) => format!("⚠️ 音频输出停了，重启失败（第 {} 次）：{e}", watch.restarts),
                })
            }
            _ => None,
        }
    }

    fn device_stats(&self) -> Option<&Stats> {
        match self {
            #[cfg(all(feature = "game-audio", not(target_os = "android")))]
            Output::Device { stats, .. } => Some(stats),
            #[cfg(all(feature = "game-audio", target_os = "android"))]
            Output::Android(o) => Some(&o.stats),
            _ => None,
        }
    }

    /// 进后台停设备回调、回前台恢复（离线模式什么都不做）
    pub fn set_paused(&mut self, paused: bool) {
        match self {
            #[cfg(all(feature = "game-audio", target_os = "android"))]
            Output::Android(o) => o.set_paused(paused),
            #[cfg(all(feature = "game-audio", not(target_os = "android")))]
            Output::Device { _stream, watch, .. } => {
                use cpal::traits::StreamTrait;
                if paused { let _ = _stream.pause(); } else { let _ = _stream.play(); }
                // 回前台：从现在起重新计时，别把后台那段算成「停了」
                watch.reset();
            }
            _ => {}
        }
        let _ = paused;
    }

    /// 关掉设备，之后不再出声（实例销毁 / 中毒；离线模式什么都不做）。不等 Drop 的原因：
    /// 销毁要先拆 JS 运行时（大游戏几十毫秒），设备等到最后才关，退出那一下还响一小截；中毒的实例整个 forget，等不到 Drop
    pub fn close(&mut self) {
        match self {
            #[cfg(all(feature = "game-audio", target_os = "android"))]
            Output::Android(o) => o.close(),
            #[cfg(all(feature = "game-audio", not(target_os = "android")))]
            Output::Device { _stream, .. } => {
                use cpal::traits::StreamTrait;
                let _ = _stream.pause();
            }
            _ => {}
        }
    }

    /// 欠载次数（Android 有，其它 0）
    pub fn xruns(&self) -> i32 {
        match self {
            #[cfg(all(feature = "game-audio", target_os = "android"))]
            Output::Android(o) => o.xruns(),
            _ => 0,
        }
    }

    /// 已经输出的帧数
    pub fn frames(&self) -> u64 {
        if let Some(s) = self.device_stats() {
            return s.frames.load(Ordering::Relaxed);
        }
        match self {
            Output::Offline { done, .. } => *done,
            #[allow(unreachable_patterns)]
            _ => 0,
        }
    }

    /// 看门狗：(重启过几次, 回调多少毫秒没涨)；离线 / Android (0, 0)
    pub fn watch_state(&self) -> (u32, u128) {
        match self {
            #[cfg(all(feature = "game-audio", not(target_os = "android")))]
            Output::Device { watch, .. } => (watch.restarts, watch.since.map(|t| t.elapsed().as_millis()).unwrap_or(0)),
            _ => (0, 0),
        }
    }

    /// 上次读走以来的输出峰值（离线 0）
    pub fn take_peak(&self) -> f32 {
        self.device_stats().map(Stats::take_peak).unwrap_or(0.0)
    }

    /// (延迟 µs, 回调块帧数, 回调次数)；离线全 0
    pub fn stats(&self) -> (u32, u32, u64) {
        match self.device_stats() {
            Some(s) => (s.latency_us.load(Ordering::Relaxed), s.block_frames.load(Ordering::Relaxed), s.callbacks.load(Ordering::Relaxed)),
            None => (0, 0, 0),
        }
    }

    pub fn is_device(&self) -> bool {
        !matches!(self, Output::Offline { .. })
    }
}

/// 设备输出的看门狗：回调次数超过 [`Watch::STALL_MS`] 没涨（引擎又不是故意静音）就说它停了。
///
/// 为什么要有：iOS 上 RemoteIO 会被系统停掉而 cpal 不知道 —— 宿主停用了音频会话（从前 GameAudioSession 改类别前先
/// setActive(false)，正好在引擎开设备之后：真机上设备回调 32 次、0.68 秒后就再也不来，背景音乐和音效全没声）、
/// 来电 / 闹钟打断、媒体服务重置。停了之后 cpal 的 `play()` 是空操作（它记着「在放」），只能 pause + play 重启
#[derive(Default)]
pub struct Watch {
    last_cb: u64,
    since: Option<std::time::Instant>,
    pub restarts: u32,
}

impl Watch {
    pub const STALL_MS: u128 = 600;

    fn reset(&mut self) {
        self.since = None;
    }

    /// 停了多少毫秒（够判定「停了」时返回，并从现在重新计时）；`quiet` 时不判、只记下当前值
    fn check(&mut self, cb: u64, now: std::time::Instant, quiet: bool) -> Option<u128> {
        if quiet || cb != self.last_cb || self.since.is_none() {
            self.last_cb = cb;
            self.since = Some(now);
            return None;
        }
        let ms = now.duration_since(self.since?).as_millis();
        if ms < Self::STALL_MS {
            return None;
        }
        self.since = Some(now);
        Some(ms)
    }
}

#[cfg(test)]
mod watch_tests {
    use super::Watch;
    use std::time::{Duration, Instant};

    #[test]
    fn 回调不涨才算停_故意静音时不算() {
        let t0 = Instant::now();
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        let mut w = Watch::default();
        assert_eq!(w.check(10, at(0), false), None, "第一次只记下");
        assert_eq!(w.check(20, at(500), false), None, "在涨");
        assert_eq!(w.check(20, at(900), false), None, "停了 400ms，还不够");
        assert_eq!(w.check(20, at(1200), false), Some(700), "停了 700ms：判定");
        assert_eq!(w.check(20, at(1500), false), None, "判过之后重新计时");
        assert_eq!(w.check(20, at(1900), false), Some(700));
        // 后台 / 打断中：设备停着是对的
        assert_eq!(w.check(20, at(5000), true), None);
        assert_eq!(w.check(20, at(5300), false), None, "从安静状态出来重新计时");
        w.reset();
        assert_eq!(w.check(20, at(9000), false), None, "reset 之后第一次只记下");
    }
}

#[cfg(all(feature = "game-audio", not(target_os = "android")))]
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
        /// `stall`：故障注入，第 N 次回调之后不再计数（见 [`super::open`]）
        pub fn start(self, mixer: Mixer, stall: Option<u64>) -> Result<(cpal::Stream, Arc<Stats>, String), String> {
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
                        st.note_peak(out);
                        note_cb(&st, info, out.len() / ch, stall);
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
                                st.note_peak(s);
                                for (o, v) in part.iter_mut().zip(s.iter()) {
                                    *o = (v * 32767.0) as i16;
                                }
                            }
                            note_cb(&st, info, out.len() / ch, stall);
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

    fn note_cb(st: &Stats, info: &cpal::OutputCallbackInfo, frames: usize, stall: Option<u64>) {
        // 故障注入 stall@N：之后不再计数，看门狗看到的就是「系统停了 I/O」
        if stall.is_some_and(|n| st.callbacks.load(Ordering::Relaxed) >= n) {
            return;
        }
        let ts = info.timestamp();
        if let Some(d) = ts.playback.duration_since(&ts.callback) {
            st.latency_us.store(d.as_micros().min(u32::MAX as u128) as u32, Ordering::Relaxed);
        }
        st.block_frames.store(frames as u32, Ordering::Relaxed);
        st.callbacks.fetch_add(1, Ordering::Relaxed);
        st.frames.fetch_add(frames as u64, Ordering::Relaxed);
    }
}
