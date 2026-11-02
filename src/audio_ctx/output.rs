//! 音频上声卡：每个 InnerAudioContext 一个 rodio `Player`，共用一个设备 sink 的混音器。
//!
//! 与 `<video>` 的 `video_audio` 分开：那边一次只播一段（换视频就停），这里要多路同时出声
//! （背景音乐 + 点击音效是常见写法）。移动端不带 `audio` 特性：有 `game-audio` 时借小游戏的混音器与设备出声；
//! 两个都没有时是空实现（播放时钟照走、事件照发，只是不出声）。从前移动端一律是空实现，手机上小程序放音频没有声音。

#[cfg(feature = "audio")]
mod backend {
    use rodio::stream::MixerDeviceSink;
    use rodio::{ChannelCount, Player, SampleRate, Source};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::sync::Arc;

    thread_local! {
        // 设备 sink 不是 Send，与 JS 运行时同线程
        static DEVICE: RefCell<Option<MixerDeviceSink>> = const { RefCell::new(None) };
        static PLAYERS: RefCell<HashMap<u32, Player>> = RefCell::new(HashMap::new());
    }

    struct Slice {
        samples: Arc<Vec<f32>>,
        pos: usize,
        rate: u32,
        channels: u16,
    }

    impl Iterator for Slice {
        type Item = f32;
        fn next(&mut self) -> Option<f32> {
            let s = *self.samples.get(self.pos)?;
            self.pos += 1;
            Some(s)
        }
    }

    impl Source for Slice {
        fn current_span_len(&self) -> Option<usize> {
            Some(self.samples.len().saturating_sub(self.pos))
        }
        fn channels(&self) -> ChannelCount {
            ChannelCount::new(self.channels).unwrap_or(ChannelCount::new(2).unwrap())
        }
        fn sample_rate(&self) -> SampleRate {
            SampleRate::new(self.rate).unwrap_or(SampleRate::new(44100).unwrap())
        }
        fn total_duration(&self) -> Option<std::time::Duration> {
            None
        }
    }

    pub fn start(id: u32, samples: Arc<Vec<f32>>, rate: u32, channels: u16, from_sec: f64, volume: f32, speed: f32) {
        stop(id);
        let ok = DEVICE.with(|d| {
            let mut d = d.borrow_mut();
            if d.is_none() {
                match rodio::stream::DeviceSinkBuilder::open_default_sink() {
                    Ok(mut dev) => {
                        dev.log_on_drop(false);
                        *d = Some(dev);
                    }
                    Err(e) => {
                        if std::env::var("MINI_AUDIO_LOG").is_ok() {
                            eprintln!("🔈 没有可用的音频设备：{e:?}（播放时钟照走，只是不出声）");
                        }
                        return None;
                    }
                }
            }
            d.as_ref().map(|dev| Player::connect_new(dev.mixer()))
        });
        let Some(player) = ok else { return };
        let pos = ((from_sec.max(0.0) * rate as f64) as usize) * channels.max(1) as usize;
        player.append(Slice { samples, pos, rate, channels });
        player.set_volume(volume);
        player.set_speed(speed);
        player.play();
        PLAYERS.with(|p| p.borrow_mut().insert(id, player));
    }

    pub fn stop(id: u32) {
        PLAYERS.with(|p| {
            if let Some(pl) = p.borrow_mut().remove(&id) {
                pl.stop();
            }
        });
    }

    pub fn set_volume(id: u32, v: f32) {
        PLAYERS.with(|p| {
            if let Some(pl) = p.borrow().get(&id) {
                pl.set_volume(v);
            }
        });
    }

    pub fn stop_all() {
        PLAYERS.with(|p| {
            for (_, pl) in p.borrow_mut().drain() {
                pl.stop();
            }
        });
    }

    pub fn tick() {}
}

/// 移动端（没有 rodio、有小游戏的音频输出）：借小游戏的混音器 + 设备（Android AAudio / iOS cpal，`game/audio/sink.rs`）。
/// 设备第一次开播时才打开（不放音频的小程序不占音频流）
#[cfg(all(not(feature = "audio"), feature = "game-audio"))]
mod backend {
    use crate::game::audio::sink::Sink;
    use std::cell::RefCell;
    use std::sync::Arc;

    thread_local! {
        // 与 JS 运行时同线程；Android 的 AAudio 流句柄与 iOS 的 cpal 流都不是 Send
        static SINK: RefCell<Option<Sink>> = const { RefCell::new(None) };
    }

    pub fn start(id: u32, samples: Arc<Vec<f32>>, rate: u32, channels: u16, from_sec: f64, volume: f32, speed: f32) {
        SINK.with(|s| {
            let mut s = s.borrow_mut();
            let sink = s.get_or_insert_with(|| {
                let k = Sink::open();
                if std::env::var("MINI_AUDIO_LOG").is_ok() {
                    eprintln!("🔈 小程序音频输出：{}", k.note);
                }
                k
            });
            sink.start(id, samples, rate, channels, from_sec, volume, speed);
        });
    }

    fn with(f: impl FnOnce(&mut Sink)) {
        SINK.with(|s| {
            if let Some(k) = s.borrow_mut().as_mut() {
                f(k);
            }
        });
    }

    pub fn stop(id: u32) {
        with(|k| k.stop(id));
    }

    pub fn set_volume(id: u32, v: f32) {
        with(|k| k.set_volume(id, v));
    }

    /// 全停并关掉设备（小程序实例销毁）：留着的话退出小程序之后声音还在放
    pub fn stop_all() {
        SINK.with(|s| {
            if let Some(mut k) = s.borrow_mut().take() {
                k.stop_all();
            }
        });
    }

    pub fn tick() {
        with(|k| {
            if let Some(l) = k.tick() {
                eprintln!("{l}");
            }
        });
    }
}

#[cfg(all(not(feature = "audio"), not(feature = "game-audio")))]
mod backend {
    use std::sync::Arc;
    pub fn start(_id: u32, _s: Arc<Vec<f32>>, _r: u32, _c: u16, _from: f64, _v: f32, _speed: f32) {}
    pub fn stop(_id: u32) {}
    pub fn set_volume(_id: u32, _v: f32) {}
    pub fn stop_all() {}
    pub fn tick() {}
}

pub use backend::{set_volume, start, stop, stop_all, tick};
