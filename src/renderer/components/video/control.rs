//! 程序控制与事件：`VideoContext` 的 play / pause / stop / seek / playbackRate / 全屏，
//! 以及宿主每帧来取的组件事件（play / pause / ended / timeupdate / loadedmetadata /
//! seekcomplete / fullscreenchange）。
//!
//! 依据官方 component/video.html 与 api/media/video/VideoContext.html：
//! - `timeupdate` 播放中每 250ms 一次，`detail = {currentTime, duration}`（秒）；
//! - `ended` 播到末尾（`loop` 时不发，与 HTML 媒体元素一致）；
//! - `loadedmetadata` 的 `detail = {width, height, duration}`；加载失败发 `error`（`{errMsg}`）；
//! - `seekcomplete` 的 `detail = {position}`；
//! - `fullscreenchange` 的 `detail = {fullScreen, direction}`，`direction` 是 `vertical` / `horizontal`。
//!
//! 事件按「上次报给逻辑层的状态」做差：播放器自己按墙上时钟走，宿主只管每帧问一次。
//! `playbackRate` 不等于 1 时不放声音（内置音频是整段样本直出，不能变速不变调）。
use super::*;
use serde_json::{json, Value};

/// timeupdate 的间隔（官方：250ms 一次）
const TIMEUPDATE_MS: u128 = 250;

/// 已经报给逻辑层的状态
#[derive(Default)]
pub(super) struct Reported {
    meta: bool,
    playing: bool,
    last_timeupdate: Option<Instant>,
    /// 命令（seek / 全屏）当场产生的事件，等宿主来取
    pending: Vec<(&'static str, Value)>,
}

/// `VideoContext` 的方法
#[derive(Clone, Debug, PartialEq)]
pub enum VideoCommand {
    Play,
    Pause,
    /// 暂停并回到开头
    Stop,
    /// 跳到第几秒
    Seek(f64),
    PlaybackRate(f64),
    /// `requestFullScreen({direction})`：0 竖屏、90 / -90 横屏，不给按画面宽高比判断
    RequestFullScreen(Option<i32>),
    ExitFullScreen,
    /// `sendDanmu({text, color})`
    SendDanmu(String, String),
}

thread_local! {
    /// 正在全屏的视频 (src, direction)。全屏画面画在所有页面层之上（compositor 读它），
    /// 一个引擎同时只会有一个；thread_local 的理由同 `HOST_OWNS_VIDEO`
    static FULLSCREEN: std::cell::RefCell<Option<(String, &'static str)>> = const { std::cell::RefCell::new(None) };
}

/// 正在全屏的视频 (src, direction)
pub fn fullscreen_video() -> Option<(String, &'static str)> {
    FULLSCREEN.with(|f| f.borrow().clone())
}

/// 换页 / 销毁时收掉全屏（不发事件：页面已经没了）
pub fn clear_fullscreen() {
    FULLSCREEN.with(|f| *f.borrow_mut() = None);
}

impl VideoPlayer {
    /// 当前播放位置（秒）
    pub fn current_time(&self) -> f64 {
        let base = self.frames.get(self.play_start_frame).map(|f| f.timestamp).unwrap_or(self.seek_pos);
        let t = match (self.is_playing, self.play_start_time) {
            (true, Some(t0)) => base + t0.elapsed().as_secs_f64() * self.rate,
            _ => self.frames.get(self.current_frame).map(|f| f.timestamp).unwrap_or(self.seek_pos),
        };
        if self.duration > 0.0 { t.clamp(0.0, self.duration) } else { t.max(0.0) }
    }

    /// 推进播放状态（有画面帧时与绘制同一条路径；没解帧时按时长判断播完）
    fn advance(&mut self) {
        if !self.frames.is_empty() {
            let _ = self.get_current_frame();
            return;
        }
        if self.is_playing && self.duration > 0.0 && self.current_time() >= self.duration {
            if self.loop_play {
                self.seek_pos = 0.0;
                self.play_start_time = Some(Instant::now());
            } else {
                self.seek_pos = self.duration;
                self.is_playing = false;
                self.play_start_time = None;
                self.ended = true;
            }
        }
    }

    fn seek_to(&mut self, pos: f64) {
        let pos = if self.duration > 0.0 { pos.clamp(0.0, self.duration) } else { pos.max(0.0) };
        let playing = self.is_playing;
        if playing {
            self.pause();
        }
        self.seek_pos = pos;
        // 落到 pos 之前最近的一帧
        self.current_frame = self.frames.iter().rposition(|f| f.timestamp <= pos + 1e-6).unwrap_or(0);
        self.ended = false;
        if playing {
            self.play();
        }
        self.reported.pending.push(("seekcomplete", json!({ "position": pos })));
    }

    fn apply(&mut self, src: &str, cmd: &VideoCommand) {
        match cmd {
            VideoCommand::SendDanmu(text, color) => {
                super::extras::send_danmu_at(src, text, color, self.current_time());
            }
            VideoCommand::Play => {
                // 还没加载完：缓冲中（bindwaiting），加载完会接着播
                if !self.is_loaded && self.load_error.is_none() {
                    self.reported.pending.push(("waiting", json!({})));
                }
                if self.ended {
                    // 播完了再 play：从头开始（与 HTML 媒体元素一致）
                    self.current_frame = 0;
                    self.seek_pos = 0.0;
                    self.ended = false;
                }
                self.play();
            }
            VideoCommand::Pause => self.pause(),
            VideoCommand::Stop => {
                self.pause();
                self.seek_to(0.0);
                // stop 不算一次 seek
                self.reported.pending.retain(|(n, _)| *n != "seekcomplete");
            }
            VideoCommand::Seek(p) => self.seek_to(*p),
            VideoCommand::PlaybackRate(r) => {
                let playing = self.is_playing;
                if playing {
                    self.pause();
                }
                self.rate = r.clamp(0.5, 2.0);
                if playing {
                    self.play();
                }
            }
            VideoCommand::RequestFullScreen(dir) => {
                let direction = match dir {
                    Some(0) => "vertical",
                    Some(_) => "horizontal",
                    None if self.width > self.height => "horizontal",
                    None => "vertical",
                };
                let was = fullscreen_video().map(|(s, _)| s == src).unwrap_or(false);
                FULLSCREEN.with(|f| *f.borrow_mut() = Some((src.to_string(), direction)));
                if !was {
                    self.reported.pending.push(("fullscreenchange", json!({ "fullScreen": true, "direction": direction })));
                }
            }
            VideoCommand::ExitFullScreen => {
                if let Some((s, direction)) = fullscreen_video().filter(|(s, _)| s == src) {
                    let _ = s;
                    clear_fullscreen();
                    self.reported.pending.push(("fullscreenchange", json!({ "fullScreen": false, "direction": direction })));
                }
            }
        }
    }

    /// 与上次报给逻辑层的状态做差，产出该发的事件
    fn poll(&mut self) -> Vec<(&'static str, Value)> {
        self.advance();
        let mut out = std::mem::take(&mut self.reported.pending);
        // 加载失败：`binderror` 报一次（detail.errMsg，与微信同字段）
        if let Some(err) = self.load_error.as_ref().filter(|_| !self.reported.meta) {
            self.reported.meta = true;
            out.push(("error", json!({ "errMsg": err })));
            return out;
        }
        if self.is_loaded && !self.reported.meta {
            self.reported.meta = true;
            out.insert(0, ("loadedmetadata", json!({ "width": self.width, "height": self.height, "duration": self.duration })));
            // 整段预解码：加载完就是全部缓冲好了（bindprogress 的 buffered 是百分比）
            out.insert(1, ("progress", json!({ "buffered": 100 })));
        }
        let now = Instant::now();
        if self.is_playing != self.reported.playing {
            self.reported.playing = self.is_playing;
            if self.is_playing {
                out.push(("play", json!({})));
                self.reported.last_timeupdate = None;
            } else if self.ended {
                out.push(("timeupdate", json!({ "currentTime": self.duration, "duration": self.duration })));
                out.push(("ended", json!({})));
            } else {
                out.push(("pause", json!({})));
            }
        }
        if self.is_playing {
            let due = self.reported.last_timeupdate.map(|t| now.duration_since(t).as_millis() >= TIMEUPDATE_MS).unwrap_or(true);
            if due {
                self.reported.last_timeupdate = Some(now);
                out.push(("timeupdate", json!({ "currentTime": self.current_time(), "duration": self.duration })));
            }
        }
        out
    }
}

/// 执行一条 `VideoContext` 命令。返回是否找到了这个视频
pub fn video_command(src: &str, cmd: &VideoCommand) -> bool {
    let Ok(mut players) = get_video_players().lock() else { return false };
    let Some(p) = players.get_mut(src) else { return false };
    p.apply(src, cmd);
    true
}

/// 宿主每帧取一次：这个视频该发的组件事件（按发生顺序）
pub fn video_poll_events(src: &str) -> Vec<(&'static str, Value)> {
    let Ok(mut players) = get_video_players().lock() else { return Vec::new() };
    players.get_mut(src).map(|p| p.poll()).unwrap_or_default()
}

/// 有没有视频还有事件没报（播放中要按 250ms 报 timeupdate；刚加载完要报 loadedmetadata）。
/// 宿主据此决定要不要遍历布局树找 `<video>`，静止页面不做这件事
pub fn video_events_pending() -> bool {
    let Ok(players) = get_video_players().lock() else { return false };
    players.values().any(|p| {
        p.is_playing
            || p.is_playing != p.reported.playing
            || !p.reported.pending.is_empty()
            || ((p.is_loaded || p.load_error.is_some()) && !p.reported.meta)
    })
}

/// 当前位置与时长（秒），测试 / 诊断用
pub fn video_position(src: &str) -> Option<(f64, f64)> {
    let players = get_video_players().lock().ok()?;
    players.get(src).map(|p| (p.current_time(), p.duration))
}
