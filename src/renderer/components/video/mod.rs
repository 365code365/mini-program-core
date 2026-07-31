//! video 组件 - 视频播放器
//! 
//! 使用 symphonia 解码音频，手动解析 MP4 容器，openh264 解码 H.264 视频
//! 
//! 属性：
//! - src: 视频资源地址
//! - autoplay: 是否自动播放
//! - loop: 是否循环播放
//! - muted: 是否静音
//! - controls: 是否显示控制条

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use taffy::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;
use std::path::PathBuf;
use std::fs::File;

// 音频设备层拆到 video_audio.rs（移动端按 `audio` 特性关掉，见该文件注释）
use super::video_audio;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::codecs::CodecParameters;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::formats::probe::Hint;

// H.264 解码
#[cfg(feature = "h264")]
use openh264::decoder::Decoder as H264Decoder;


/// 视频帧数据
pub struct VideoFrame {
    pub data: Vec<u8>,  // RGBA 数据
    pub width: u32,
    pub height: u32,
    pub timestamp: f64, // 秒
}

/// 音频样本缓冲
struct AudioBuffer {
    samples: Vec<f32>,
    sample_rate: u32,
    channels: u16,
}

impl AudioBuffer {
    fn new() -> Self {
        Self {
            samples: Vec::new(),
            sample_rate: 44100,
            channels: 2,
        }
    }
}

/// 视频播放器状态
pub struct VideoPlayer {
    pub src: String,
    pub file_path: Option<PathBuf>,
    pub frames: Vec<VideoFrame>,
    pub current_frame: usize,
    pub fps: f64,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub is_playing: bool,
    pub is_loaded: bool,
    pub play_start_time: Option<Instant>,
    pub play_start_frame: usize,
    pub loop_play: bool,
    pub load_error: Option<String>,
    pub muted: bool,
    // 音频数据
    audio_buffer: Option<AudioBuffer>,
}

// ── `VideoPlayer` 的实现按职责分片 ──
/// 音轨解码与播放
mod audio;
/// H.264 解码与像素转换
mod decode;
/// MP4 容器解析
mod mp4;
/// 播放状态机
mod playback;

/// 全局视频播放器缓存
static VIDEO_PLAYERS: OnceLock<Arc<Mutex<HashMap<String, VideoPlayer>>>> = OnceLock::new();

fn get_video_players() -> &'static Arc<Mutex<HashMap<String, VideoPlayer>>> {
    VIDEO_PLAYERS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

/// 获取或创建视频播放器
pub fn get_or_create_player(src: &str, autoplay: bool, loop_play: bool) -> bool {
    let players = get_video_players();
    let mut players_guard = players.lock().unwrap();
    
    if !players_guard.contains_key(src) {
        let mut player = VideoPlayer::new(src);
        player.loop_play = loop_play;
        
        match player.load() {
            Ok(_) => {
                if autoplay {
                    player.play();
                }
                players_guard.insert(src.to_string(), player);
                return true;
            }
            Err(e) => {
                println!("❌ Failed to load video: {}", e);
                let mut player = VideoPlayer::new(src);
                player.load_error = Some(e);
                players_guard.insert(src.to_string(), player);
                return false;
            }
        }
    }
    true
}

/// 获取视频当前帧
pub fn get_video_frame(src: &str) -> Option<(Vec<u8>, u32, u32)> {
    let players = get_video_players();
    let mut players_guard = players.lock().ok()?;
    
    if let Some(player) = players_guard.get_mut(src) {
        if let Some(frame) = player.get_current_frame() {
            return Some((frame.data.clone(), frame.width, frame.height));
        }
    }
    None
}

/// 检查视频是否正在播放
pub fn is_video_playing(src: &str) -> bool {
    let players = get_video_players();
    if let Ok(players_guard) = players.lock() {
        if let Some(player) = players_guard.get(src) {
            return player.is_playing;
        }
    }
    false
}

/// 播放/暂停视频
pub fn toggle_video_play(src: &str) {
    let players = get_video_players();
    if let Ok(mut players_guard) = players.lock() {
        if let Some(player) = players_guard.get_mut(src) {
            if player.is_playing {
                player.pause();
            } else {
                player.play();
            }
        }
    }
}

/// 获取视频进度信息
pub fn get_video_progress(src: &str) -> Option<(f64, f64)> {
    let players = get_video_players();
    if let Ok(players_guard) = players.lock() {
        if let Some(player) = players_guard.get(src) {
            if player.is_loaded && !player.frames.is_empty() {
                let current_time = player.frames.get(player.current_frame)
                    .map(|f| f.timestamp)
                    .unwrap_or(0.0);
                return Some((current_time, player.duration));
            }
        }
    }
    None
}

/// 检查是否有任何视频正在播放
pub fn has_playing_video() -> bool {
    let players = get_video_players();
    if let Ok(players_guard) = players.lock() {
        for player in players_guard.values() {
            if player.is_playing {
                return true;
            }
        }
    }
    false
}

pub struct VideoComponent;

impl VideoComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        // 替换元素：作者写死的尺寸就是它的最小尺寸，别被兄弟压没。
        // 必须在合成默认尺寸**之前**调用 —— CSS 语义里「指定尺寸」只算作者写的那个，
        // 引擎给 textarea 之类补的默认高度不算，那种情况仍应允许被父级压缩。
        super::pin_replaced_min_size(&mut ts);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let src = node.get_attr("src").unwrap_or("");
        let autoplay = node.get_attr("autoplay").map(|v| v == "true" || v == "{{true}}").unwrap_or(false);
        let loop_play = node.get_attr("loop").map(|v| v == "true" || v == "{{true}}").unwrap_or(false);
        
        let default_width = 300.0;
        let default_height = 225.0;
        
        if dim_is_auto(ts.size.width) {
            ts.size.width = length(default_width * sf);
        }
        if dim_is_auto(ts.size.height) {
            ts.size.height = length(default_height * sf);
        }
        
        ns.background_color = Some(Color::BLACK);
        ns.border_radius = 4.0 * sf;
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        if !src.is_empty() {
            get_or_create_player(src, autoplay, loop_play);
        }
        
        Some(RenderNode {
            tag: "video".into(),
            text: src.into(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        x: f32, y: f32, w: f32, h: f32, sf: f32
    ) {
        let style = &node.style;
        let radius = style.border_radius;
        let src = &node.text;
        
        let bg_paint = Paint::new()
            .with_color(Color::BLACK)
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        
        if radius > 0.0 {
            let mut path = Path::new();
            path.add_round_rect(x, y, w, h, radius);
            canvas.draw_path(&path, &bg_paint);
        } else {
            canvas.draw_rect(&GeoRect::new(x, y, w, h), &bg_paint);
        }
        
        if !src.is_empty() {
            if let Some((frame_data, frame_w, frame_h)) = get_video_frame(src) {
                canvas.draw_image(&frame_data, frame_w, frame_h, x, y, w, h, "aspectFit", radius);
                Self::draw_controls(canvas, text_renderer, src, x, y, w, h, sf);
                return;
            }
        }
        
        Self::draw_placeholder(canvas, x, y, w, h, sf);
    }
    
    fn draw_placeholder(canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        let btn_size = 50.0 * sf;
        
        let bg_paint = Paint::new()
            .with_color(Color::new(0, 0, 0, 128))
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        canvas.draw_circle(cx, cy, btn_size / 2.0, &bg_paint);
        
        let tri_size = btn_size * 0.35;
        let mut path = Path::new();
        path.move_to(cx - tri_size * 0.4, cy - tri_size * 0.6);
        path.line_to(cx - tri_size * 0.4, cy + tri_size * 0.6);
        path.line_to(cx + tri_size * 0.6, cy);
        path.close();
        
        let tri_paint = Paint::new()
            .with_color(Color::WHITE)
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        canvas.draw_path(&path, &tri_paint);
    }
    
    fn draw_controls(
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        src: &str,
        x: f32, y: f32, w: f32, h: f32, sf: f32
    ) {
        let bar_height = 36.0 * sf;
        let bar_y = y + h - bar_height;
        
        let bg_paint = Paint::new()
            .with_color(Color::new(0, 0, 0, 160))
            .with_style(PaintStyle::Fill);
        canvas.draw_rect(&GeoRect::new(x, bar_y, w, bar_height), &bg_paint);
        
        let is_playing = is_video_playing(src);
        let btn_size = 24.0 * sf;
        let btn_x = x + 12.0 * sf;
        let btn_y = bar_y + (bar_height - btn_size) / 2.0;
        
        let btn_paint = Paint::new()
            .with_color(Color::WHITE)
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        
        if is_playing {
            let bar_w = 4.0 * sf;
            let bar_h = btn_size * 0.6;
            let gap = 6.0 * sf;
            canvas.draw_rect(&GeoRect::new(
                btn_x + (btn_size - gap - bar_w * 2.0) / 2.0,
                btn_y + (btn_size - bar_h) / 2.0,
                bar_w, bar_h
            ), &btn_paint);
            canvas.draw_rect(&GeoRect::new(
                btn_x + (btn_size + gap - bar_w * 2.0) / 2.0 + bar_w,
                btn_y + (btn_size - bar_h) / 2.0,
                bar_w, bar_h
            ), &btn_paint);
        } else {
            let tri_size = btn_size * 0.5;
            let mut path = Path::new();
            let cx = btn_x + btn_size / 2.0;
            let cy = btn_y + btn_size / 2.0;
            path.move_to(cx - tri_size * 0.3, cy - tri_size * 0.5);
            path.line_to(cx - tri_size * 0.3, cy + tri_size * 0.5);
            path.line_to(cx + tri_size * 0.5, cy);
            path.close();
            canvas.draw_path(&path, &btn_paint);
        }
        
        if let Some((current, duration)) = get_video_progress(src) {
            let progress_x = btn_x + btn_size + 12.0 * sf;
            let progress_w = w - progress_x - 80.0 * sf - x;
            let progress_h = 4.0 * sf;
            let progress_y = bar_y + (bar_height - progress_h) / 2.0;
            
            let track_paint = Paint::new()
                .with_color(Color::new(255, 255, 255, 80))
                .with_style(PaintStyle::Fill);
            canvas.draw_rect(&GeoRect::new(progress_x, progress_y, progress_w, progress_h), &track_paint);
            
            let progress = if duration > 0.0 { current / duration } else { 0.0 };
            let fill_paint = Paint::new()
                .with_color(Color::from_hex(0x07C160))
                .with_style(PaintStyle::Fill);
            canvas.draw_rect(&GeoRect::new(progress_x, progress_y, progress_w * progress as f32, progress_h), &fill_paint);
            
            if let Some(tr) = text_renderer {
                let time_text = format!("{} / {}", Self::format_time(current), Self::format_time(duration));
                let time_x = progress_x + progress_w + 8.0 * sf;
                let time_y = bar_y + bar_height / 2.0 - 6.0 * sf;
                let text_paint = Paint::new().with_color(Color::WHITE);
                tr.draw_text(canvas, &time_text, time_x, time_y, 11.0 * sf, &text_paint);
            }
        }
    }
    
    fn format_time(seconds: f64) -> String {
        let mins = (seconds / 60.0) as u32;
        let secs = (seconds % 60.0) as u32;
        format!("{:02}:{:02}", mins, secs)
    }
}
