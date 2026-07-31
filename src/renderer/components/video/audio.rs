//! 音轨解码与播放：AAC 解码、输出设备、随播放状态起停
//!
//! `VideoPlayer` 的一片，由 `video/mod.rs` 组合。**纯搬迁**：从 1064 行的
//! components/video.rs 按职责切开，一行逻辑没改。
use super::*;

impl VideoPlayer {
    /// 解码音频
    pub(super) fn decode_audio(&mut self, path: &PathBuf) -> Result<(), String> {
        let file = File::open(path).map_err(|e| format!("Cannot open: {}", e))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        
        let mut hint = Hint::new();
        hint.with_extension("mp4");
        
        // symphonia 0.6：`get_probe().format(..)` 换成 `probe(..)` 且直接给出
        // `FormatReader`（不再有 `ProbedFormat` 包装）；轨道的编解码参数改成
        // `Option<CodecParameters>` 的枚举（音频/视频/字幕分开），所以「找音频轨」
        // 从「codec != CODEC_TYPE_NULL」变成「参数是 Audio 这一支」。
        let mut format = symphonia::default::get_probe()
            .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
            .map_err(|e| format!("Probe error: {}", e))?;

        // 查找音频轨道
        let (track_id, audio_params) = format
            .tracks()
            .iter()
            .find_map(|t| match &t.codec_params {
                Some(CodecParameters::Audio(p)) => Some((t.id, p.clone())),
                _ => None,
            })
            .ok_or("No audio track")?;

        let sample_rate = audio_params.sample_rate.unwrap_or(44100);
        let channels = audio_params.channels.as_ref().map(|c| c.count() as u16).unwrap_or(2);
        
        println!("   Audio: {} Hz, {} channels", sample_rate, channels);
        
        // 创建解码器
        let mut decoder = symphonia::default::get_codecs()
            .make_audio_decoder(&audio_params, &AudioDecoderOptions::default())
            .map_err(|e| format!("Decoder error: {}", e))?;
        
        let mut audio_buffer = AudioBuffer::new();
        audio_buffer.sample_rate = sample_rate;
        audio_buffer.channels = channels;
        
        // 解码所有音频包
        // symphonia 0.6：`next_packet()` 返回 `Result<Option<Packet>>`（`Ok(None)` 就是读完了，
        // 不再靠「IoError == UnexpectedEof」判结尾）；解码结果是 `GenericAudioBufferRef`，
        // 交错成 f32 用 `copy_to_vec_interleaved`，不必自己建 `SampleBuffer`。
        let mut interleaved: Vec<f32> = Vec::new();
        loop {
            let packet = match format.next_packet() {
                Ok(Some(p)) => p,
                Ok(None) => break,
                Err(_) => break,
            };

            if packet.track_id != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    interleaved.clear();
                    decoded.copy_to_vec_interleaved(&mut interleaved);
                    audio_buffer.samples.extend_from_slice(&interleaved);
                }
                Err(_) => continue,
            }
        }
        
        if !audio_buffer.samples.is_empty() {
            let duration_secs = audio_buffer.samples.len() as f64 
                / (audio_buffer.sample_rate as f64 * audio_buffer.channels as f64);
            println!("✅ Audio loaded: {:.1}s ({} samples)", duration_secs, audio_buffer.samples.len());
            self.audio_buffer = Some(audio_buffer);
        }
        
        Ok(())
    }

    /// 开始播放音频（设备层在 video_audio.rs，移动端按 `audio` 特性关掉）
    pub(super) fn start_audio(&self) {
        if self.muted { return; }
        let audio_buffer = match &self.audio_buffer {
            Some(ab) => ab,
            None => return,
        };
        video_audio::stop();
        // 从当前帧的时间戳开始播：前面那段样本要跳过
        let skip_time = self.frames.get(self.current_frame)
            .map(|f| f.timestamp)
            .unwrap_or(0.0);
        let skip = (skip_time * audio_buffer.sample_rate as f64 * audio_buffer.channels as f64) as usize;
        let samples: Vec<f32> = if skip < audio_buffer.samples.len() {
            audio_buffer.samples[skip..].to_vec()
        } else {
            audio_buffer.samples.clone()
        };
        video_audio::play(samples, audio_buffer.sample_rate, audio_buffer.channels);
    }

    pub(super) fn stop_audio_static() {
        video_audio::stop();
    }

    pub(super) fn restart_audio(&self) {
        video_audio::stop();
        let Some(audio_buffer) = &self.audio_buffer else { return };
        video_audio::play(
            audio_buffer.samples.clone(),
            audio_buffer.sample_rate,
            audio_buffer.channels,
        );
    }
}
