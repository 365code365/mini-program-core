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
use crate::{Canvas, Color, Paint, Rect as GeoRect};
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
    /// 倍速（`VideoContext.playbackRate`，0.5 ~ 2.0）
    pub rate: f64,
    /// 没有画面帧可对时的播放位置（秒）：seek 落点 / 暂停处
    pub seek_pos: f64,
    /// 播到末尾停下了（不是用户暂停）：再 play 从头开始，事件发 `ended` 而不是 `pause`
    pub ended: bool,
    /// 已经报给逻辑层的状态（事件按它做差，见 control.rs）
    reported: control::Reported,
}

// ── `VideoPlayer` 的实现按职责分片 ──
/// 音轨解码与播放
mod audio;
/// 封面、弹幕、进度轨道几何（G-6）
mod extras;
pub use extras::{clear_danmu, send_danmu};
pub use chrome::progress_track;
/// H.264 解码与像素转换
mod decode;
/// MP4 容器解析
mod mp4;
/// 播放状态机
mod playback;
/// 内置播放器的 VideoContext 命令与组件事件
mod control;
pub use control::{video_command, video_events_pending, video_poll_events, video_position, VideoCommand};
/// 全屏画面（画在所有页面层之上，横屏时转过来画）与全屏时的触摸
pub mod fullscreen;
pub use fullscreen::{clear_fullscreen, draw_fullscreen, fullscreen_video};
/// 控制条的显隐 / 自动收起 / 拖进度 / 静音，与「视频在哪放」的抽象（两种播放器共用）
pub mod ui;
/// 控制条的几何与画法（页面里与全屏共用）
pub mod chrome;
/// 同层渲染：宿主解码、帧交给引擎画（创建选项 `sameLayer` 含 `video`）
pub mod host_feed;

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

/// 每次建树时同步属性：`muted` / `loop` 随 setData 变；`initial-time` 只在第一次（还没动过）时生效
fn set_player_options(src: &str, muted: bool, loop_play: bool, initial_time: f64) {
    let Ok(mut players) = get_video_players().lock() else { return };
    let Some(p) = players.get_mut(src) else { return };
    p.loop_play = loop_play;
    if p.muted != muted {
        p.muted = muted;
        if muted {
            VideoPlayer::stop_audio_static();
        } else if p.is_playing {
            p.start_audio();
        }
    }
    if initial_time > 0.0 && !p.is_playing && p.current_frame == 0 && p.seek_pos == 0.0 && !p.ended {
        p.seek_pos = initial_time;
        p.current_frame = p.frames.iter().rposition(|f| f.timestamp <= initial_time + 1e-6).unwrap_or(0);
    }
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

thread_local! {
    /// `<video>` 由宿主的原生播放器承载（创建选项 `nativeComponents` 含 `video`）。
    /// 这时引擎既不解码也不画画面，只留一块黑底（宿主的播放器盖在上面）——
    /// 两边各解一份是白烧电，内置的软解在手机上也远不如系统硬解。
    /// thread_local：引擎实例只在一条线程上跑，并行的测试 / 另一个实例互不影响。
    static HOST_OWNS_VIDEO: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 宿主是否接管 `<video>`（`MiniEngine` 换页重建渲染器时设）
pub fn set_host_owns_video(on: bool) {
    HOST_OWNS_VIDEO.with(|c| c.set(on));
}

fn host_owns_video() -> bool {
    HOST_OWNS_VIDEO.with(|c| c.get())
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
        // 布尔属性：写了且不是 false 就是真（`<video autoplay>` 不带值也算开）
        let flag = |k: &str| {
            node.get_attr(k)
                .map(|v| !matches!(v.trim(), "false" | "{{false}}" | "0"))
                .unwrap_or(false)
        };
        let autoplay = flag("autoplay");
        let loop_play = flag("loop");
        // 用户按过控制条的静音键就以它为准（ui.rs）
        let muted = ui::effective_muted(&host_feed::video_key(&attrs), flag("muted"));
        let initial_time = node.get_attr("initial-time").and_then(|v| v.trim().parse::<f64>().ok()).unwrap_or(0.0);
        
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
        
        // 宿主承载（盖控件）与同层渲染（宿主解码、引擎画）时都不建内置播放器
        if !src.is_empty() && !host_owns_video() && !host_feed::host_feeds_video() {
            get_or_create_player(src, autoplay, loop_play);
            set_player_options(src, muted, loop_play, initial_time);
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
}

/// 画：黑底、画面（内置播放器解出的帧 / 宿主交来的帧）、封面、控制条、中间播放键
mod draw;
