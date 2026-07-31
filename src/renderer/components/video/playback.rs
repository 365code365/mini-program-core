//! 播放状态机：按时间轴取当前帧、播放/暂停
//!
//! `VideoPlayer` 的一片，由 `video/mod.rs` 组合。**纯搬迁**：从 1064 行的
//! components/video.rs 按职责切开，一行逻辑没改。
use super::*;

impl VideoPlayer {
    pub fn new(src: &str) -> Self {
        Self {
            src: src.to_string(),
            file_path: None,
            frames: Vec::new(),
            current_frame: 0,
            fps: 24.0,
            duration: 0.0,
            width: 0,
            height: 0,
            is_playing: false,
            is_loaded: false,
            play_start_time: None,
            play_start_frame: 0,
            loop_play: false,
            load_error: None,
            muted: false,
            audio_buffer: None,
        }
    }

    /// 获取当前帧
    pub fn get_current_frame(&mut self) -> Option<&VideoFrame> {
        if !self.is_loaded || self.frames.is_empty() {
            return None;
        }
        
        if self.is_playing {
            if let Some(start_time) = self.play_start_time {
                let elapsed = start_time.elapsed().as_secs_f64();
                let start_timestamp = self.frames.get(self.play_start_frame)
                    .map(|f| f.timestamp)
                    .unwrap_or(0.0);
                let current_time = start_timestamp + elapsed;
                
                // 找到对应时间的帧
                let mut target_frame = self.play_start_frame;
                for (i, frame) in self.frames.iter().enumerate().skip(self.play_start_frame) {
                    if frame.timestamp <= current_time {
                        target_frame = i;
                    } else {
                        break;
                    }
                }
                
                self.current_frame = target_frame;
                
                // 检查是否播放完毕
                if self.current_frame >= self.frames.len() - 1 {
                    if self.loop_play {
                        self.current_frame = 0;
                        self.play_start_frame = 0;
                        self.play_start_time = Some(Instant::now());
                        self.restart_audio();
                    } else {
                        self.current_frame = self.frames.len() - 1;
                        self.is_playing = false;
                        Self::stop_audio_static();
                    }
                }
            }
        }
        
        self.frames.get(self.current_frame)
    }

    /// 播放
    pub fn play(&mut self) {
        if self.is_loaded && !self.is_playing {
            self.is_playing = true;
            self.play_start_time = Some(Instant::now());
            self.play_start_frame = self.current_frame;
            self.start_audio();
        }
    }

    /// 暂停
    pub fn pause(&mut self) {
        self.is_playing = false;
        self.play_start_time = None;
        Self::stop_audio_static();
    }
}
