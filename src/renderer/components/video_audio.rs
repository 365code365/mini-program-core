//! `<video>` 的**音频设备**层：把解码好的 PCM 样本送到系统声卡。
//!
//! 单独拆出来有两个原因：
//!
//! 1. **移动端不能编进 rodio**。`rodio` 要打开系统音频设备（桌面走 CoreAudio/WASAPI/ALSA），
//!    在 iOS/Android 上应该由宿主用系统播放器接管（省电、走硬解、遵守系统音频焦点）。
//!    所以这里按 `audio` 特性分成两套实现，关掉特性时是空实现，
//!    `--no-default-features` 就能为 iOS/Android 编译整个引擎。
//! 2. `video.rs` 已经 1100 多行，解码、绘制、音频混在一起，动一处要读全篇。
//!
//! 对外只有两个函数：[`play`] / [`stop`]。解码出来的样本一律是交错排列的 `f32`。

/// 开始播放一段 PCM（会先停掉正在播的那段）
pub fn play(samples: Vec<f32>, sample_rate: u32, channels: u16) {
    backend::play(samples, sample_rate, channels);
}

/// 停止播放
pub fn stop() {
    backend::stop();
}

/// 当前构建是否真的能出声（宿主/诊断用）
pub fn available() -> bool {
    cfg!(feature = "audio")
}

// ───────────────────────── 有 `audio` 特性：真的出声 ─────────────────────────
#[cfg(feature = "audio")]
mod backend {
    // rodio 0.22：`Sink` 改名 `Player`，`OutputStream::try_default()` 换成
    // 「设备 sink 构建器 + mixer」两段式；`Source` 的 span/声道/采样率也换了类型。
    use rodio::stream::MixerDeviceSink;
    use rodio::{ChannelCount, Player, SampleRate, Source};

    // thread_local：设备 sink 不是 Send
    thread_local! {
        static AUDIO_STREAM: std::cell::RefCell<Option<(MixerDeviceSink, Player)>> =
            const { std::cell::RefCell::new(None) };
    }

    pub(super) fn play(samples: Vec<f32>, sample_rate: u32, channels: u16) {
        stop();
        match rodio::stream::DeviceSinkBuilder::open_default_sink() {
            Ok(mut device) => {
                // rodio 0.22 在 DeviceSink 析构时会打一行提示，正常停播也会触发 ——
                // 对宿主日志来说是纯噪声，关掉。
                device.log_on_drop(false);
                let player = Player::connect_new(device.mixer());
                player.append(SamplesSource::new(samples, sample_rate, channels));
                player.play();
                AUDIO_STREAM.with(|cell| {
                    *cell.borrow_mut() = Some((device, player));
                });
            }
            Err(e) => println!("❌ Audio output error: {:?}", e),
        }
    }

    pub(super) fn stop() {
        AUDIO_STREAM.with(|cell| {
            if let Some((_, ref player)) = *cell.borrow() {
                player.stop();
            }
            *cell.borrow_mut() = None;
        });
    }

    /// 解码后的样本作为音频源
    struct SamplesSource {
        samples: Vec<f32>,
        position: usize,
        sample_rate: u32,
        channels: u16,
    }

    impl SamplesSource {
        fn new(samples: Vec<f32>, sample_rate: u32, channels: u16) -> Self {
            Self { samples, position: 0, sample_rate, channels }
        }
    }

    impl Iterator for SamplesSource {
        type Item = f32;
        fn next(&mut self) -> Option<Self::Item> {
            let s = *self.samples.get(self.position)?;
            self.position += 1;
            Some(s)
        }
    }

    impl Source for SamplesSource {
        // rodio 0.22：`current_frame_len` 改名 `current_span_len`（"frame" 一词让给
        // 「同一时刻各声道的一组样本」这个含义），声道数与采样率也换成了非零类型。
        fn current_span_len(&self) -> Option<usize> {
            Some(self.samples.len() - self.position)
        }
        fn channels(&self) -> ChannelCount {
            ChannelCount::new(self.channels).unwrap_or(ChannelCount::new(2).unwrap())
        }
        fn sample_rate(&self) -> SampleRate {
            SampleRate::new(self.sample_rate).unwrap_or(SampleRate::new(44100).unwrap())
        }
        fn total_duration(&self) -> Option<std::time::Duration> {
            let total = self.samples.len() / (self.channels as usize).max(1);
            Some(std::time::Duration::from_secs_f64(
                total as f64 / self.sample_rate.max(1) as f64,
            ))
        }
    }
}

// ─────────────── 关掉 `audio` 特性：空实现（移动端由宿主播声音）───────────────
#[cfg(not(feature = "audio"))]
mod backend {
    pub(super) fn play(_samples: Vec<f32>, _sample_rate: u32, _channels: u16) {}
    pub(super) fn stop() {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn play_and_stop_never_panic_regardless_of_backend() {
        // 关特性时是空实现；开特性时机器上没有声卡也不能崩（CI 里就没有）
        super::play(vec![0.0; 64], 44100, 2);
        super::stop();
        super::stop(); // 重复 stop 也要安全
    }

    #[test]
    fn availability_matches_the_feature() {
        assert_eq!(super::available(), cfg!(feature = "audio"));
    }
}
