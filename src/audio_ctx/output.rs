//! 音频上声卡：每个 InnerAudioContext 一个 rodio `Player`，共用一个设备 sink 的混音器。
//!
//! 与 `<video>` 的 `video_audio` 分开：那边一次只播一段（换视频就停），这里要多路同时出声
//! （背景音乐 + 点击音效是常见写法）。关掉 `audio` 特性（移动端）时是空实现：
//! 播放时钟照走、事件照发，只是不出声 —— 移动端的声音应交给宿主的系统播放器。

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
}

#[cfg(not(feature = "audio"))]
mod backend {
    use std::sync::Arc;
    pub fn start(_id: u32, _s: Arc<Vec<f32>>, _r: u32, _c: u16, _from: f64, _v: f32, _speed: f32) {}
    pub fn stop(_id: u32) {}
    pub fn set_volume(_id: u32, _v: f32) {}
}

pub use backend::{set_volume, start, stop};
