//! 控制条的交互状态（显隐与自动收起、拖进度、静音）与「这个视频在哪放」的抽象。
//! 内置播放器（按 `src` 认）与同层渲染（宿主的播放器，按 `host_feed::video_key` 认）共用这一份，
//! 页面里的控制条与全屏画面（`fullscreen.rs`）也共用。
//!
//! 与微信 `<video>` 对齐的行为（官方 component/video.html）：
//! - 点画面（不在按钮上）切换控制条显隐，派发 `controlstoggle {show}`；
//! - 播放中 [`AUTO_HIDE_MS`] 没碰就自动收起（同样派发 `controlstoggle`）；暂停、播完时控制条回来；
//! - 拖进度条：圆点跟手、时间跟着变，松手才 seek（一次）；点进度条直接跳；
//! - 静音按钮（`show-mute-btn`）切换静音，之后属性 `muted` 再变才以属性为准。
//!
//! 时钟用宿主 pump 给的毫秒数（[`tick`]），不用墙上时钟：同一串 pump 时间得到同一串画面，
//! 逐字节对比的测试才站得住。
use super::*;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};

/// 播放中多久没碰就收起控制条（毫秒）
pub const AUTO_HIDE_MS: u64 = 3000;

/// 画控制条要的播放状态（两种播放器各自的状态折成这一份）
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Status {
    pub playing: bool,
    /// 起播过（封面只在没起播时盖着）
    pub started: bool,
    pub ended: bool,
    /// 秒
    pub current: f64,
    pub duration: f64,
    /// 画面尺寸（全屏不给方向时按它判断横竖）
    pub width: u32,
    pub height: u32,
    pub muted: bool,
}

/// 一个 `<video>`：键（`id`，没有就是 `src`）、地址、属性
#[derive(Clone, Debug, Default)]
pub struct VideoRef {
    pub key: String,
    pub src: String,
    pub attrs: HashMap<String, String>,
}

/// 一帧画面：宿主交来的共享一份，内置播放器的拷一份
pub enum FrameData {
    Host(Arc<host_feed::HostFrame>),
    Owned(Vec<u8>, u32, u32),
}

impl FrameData {
    pub fn rgba(&self) -> (&[u8], u32, u32) {
        match self {
            FrameData::Host(f) => (&f.rgba, f.width, f.height),
            FrameData::Owned(d, w, h) => (d, *w, *h),
        }
    }
}

/// 布尔属性：写了且不是 false 就是真，没写取 `default`
pub fn attr_flag(attrs: &HashMap<String, String>, k: &str, default: bool) -> bool {
    attrs.get(k).map(|s| !matches!(s.trim(), "false" | "{{false}}" | "0")).unwrap_or(default)
}

impl VideoRef {
    pub fn from_attrs(attrs: &HashMap<String, String>) -> Self {
        Self {
            key: host_feed::video_key(attrs),
            src: attrs.get("src").map(|s| s.trim().to_string()).unwrap_or_default(),
            attrs: attrs.clone(),
        }
    }

    pub fn flag(&self, k: &str, default: bool) -> bool {
        attr_flag(&self.attrs, k, default)
    }

    pub fn status(&self) -> Status {
        let muted = effective_muted(&self.key, self.flag("muted", false));
        if host_feed::host_feeds_video() {
            let s = host_feed::state(&self.key);
            let attr_duration = self.attrs.get("duration").and_then(|s| s.trim().parse::<f64>().ok()).unwrap_or(0.0);
            return Status {
                playing: s.playing,
                started: s.started,
                ended: s.ended,
                current: s.current,
                duration: if s.duration > 0.0 { s.duration } else { attr_duration },
                width: s.width,
                height: s.height,
                muted,
            };
        }
        let started = extras::video_started(&self.src);
        let attr_duration = || self.attrs.get("duration").and_then(|s| s.trim().parse::<f64>().ok()).unwrap_or(0.0);
        let Ok(players) = get_video_players().lock() else { return Status::default() };
        let Some(p) = players.get(&self.src) else { return Status { muted, duration: attr_duration(), ..Status::default() } };
        Status {
            playing: p.is_playing,
            started,
            ended: p.ended,
            current: p.current_time(),
            // 还不知道时长时用属性 `duration`（官方：指定视频时长）
            duration: if p.duration > 0.0 { p.duration } else { attr_duration() },
            width: p.width,
            height: p.height,
            muted,
        }
    }

    /// 播放 / 暂停 / seek …：同层渲染时攒给宿主的播放器（`videoCommand`），否则交给内置播放器
    pub fn command(&self, cmd: &VideoCommand) {
        if !host_feed::host_feeds_video() {
            video_command(&self.src, cmd);
            return;
        }
        let (name, extra) = match cmd {
            VideoCommand::Play => ("play", None),
            VideoCommand::Pause => ("pause", None),
            VideoCommand::Stop => ("stop", None),
            VideoCommand::Seek(p) => ("seek", Some(("position", *p))),
            VideoCommand::PlaybackRate(r) => ("playbackRate", Some(("rate", *r))),
            // 全屏画在引擎里（fullscreen.rs），弹幕同层渲染还不支持
            _ => return,
        };
        let mut out = json!({ "key": self.key, "command": name });
        if let Some((k, v)) = extra {
            out[k] = json!(v);
        }
        host_feed::queue_command(out);
    }

    pub fn frame(&self) -> Option<FrameData> {
        if host_feed::host_feeds_video() {
            return host_feed::frame(&self.key).map(FrameData::Host);
        }
        get_video_frame(&self.src).map(|(d, w, h)| FrameData::Owned(d, w, h))
    }

    /// 静音按钮：内置播放器当场换；同层渲染经 `videos` 事件里的 `muted` 属性交给宿主（[`feed_generation`] 变了，
    /// 下一帧重发那份列表）
    pub fn set_muted(&self, muted: bool) {
        with(&self.key, |u| {
            u.muted = Some(muted);
            u.seq = u.seq.wrapping_add(1);
        });
        FEED_GEN.with(|g| g.set(g.get().wrapping_add(1)));
        if !host_feed::host_feeds_video() {
            set_builtin_muted(&self.src, muted);
        }
    }
}

fn set_builtin_muted(src: &str, muted: bool) {
    let Ok(mut players) = get_video_players().lock() else { return };
    let Some(p) = players.get_mut(src) else { return };
    if p.muted == muted {
        return;
    }
    p.muted = muted;
    if muted {
        VideoPlayer::stop_audio_static();
    } else if p.is_playing {
        p.start_audio();
    }
}

#[derive(Default)]
struct Ui {
    src: String,
    /// 控制条收起了（默认显示）
    hidden: bool,
    /// 上次有人碰它的时刻（pump 时钟，毫秒）：播放中从这一刻起算自动收起
    touched_ms: u64,
    /// 上一次 tick 时在不在播（起播 / 暂停的那一刻要动控制条）
    was_playing: bool,
    /// 用户按过静音按钮（None = 跟着属性 `muted`）
    muted: Option<bool>,
    /// 画面上的东西变一次加一（同层渲染按它只重画那个框）
    seq: u32,
}

/// 正在拖的进度条
#[derive(Clone, Debug)]
struct Drag {
    video: VideoRef,
    /// 轨道的横向范围（与手指同一个坐标系）
    x0: f32,
    w: f32,
    ratio: f64,
    /// 真的拖过（不然松手不 seek：那一下是点击，走点击那条路）
    moved: bool,
}

thread_local! {
    static UI: RefCell<HashMap<String, Ui>> = RefCell::new(HashMap::new());
    static NOW: Cell<u64> = const { Cell::new(0) };
    /// 按下时落在进度条上：横着拖起来就是它
    static ARMED: RefCell<Option<Drag>> = const { RefCell::new(None) };
    static DRAG: RefCell<Option<Drag>> = const { RefCell::new(None) };
    /// 引擎自己产生、要派发给节点的组件事件 (键, 名字, detail)
    static EVENTS: RefCell<Vec<(String, &'static str, Value)>> = const { RefCell::new(Vec::new()) };
    /// 要宿主重报 `videos` 列表的东西（静音、全屏）变一次加一
    static FEED_GEN: Cell<u64> = const { Cell::new(0) };
    /// 上一次 [`take_changed`] 时所有视频的版本号之和（内置播放器据此整帧重画）
    static LAST_SUM: Cell<u64> = const { Cell::new(0) };
}

fn with<R>(key: &str, f: impl FnOnce(&mut Ui) -> R) -> R {
    UI.with(|m| f(m.borrow_mut().entry(key.to_string()).or_default()))
}

pub fn now_ms() -> u64 {
    NOW.with(|n| n.get())
}

/// 画 / 点之前登记一下（自动收起要知道它在不在播）
pub fn note(video: &VideoRef) {
    with(&video.key, |u| {
        if u.src != video.src {
            u.src = video.src.clone();
        }
    });
}

pub(super) fn queue_event(key: &str, name: &'static str, detail: Value) {
    EVENTS.with(|e| e.borrow_mut().push((key.to_string(), name, detail)));
}

/// 取走引擎自己产生的组件事件（`controlstoggle` / `fullscreenchange`）
pub fn take_events() -> Vec<(String, &'static str, Value)> {
    EVENTS.with(|e| std::mem::take(&mut *e.borrow_mut()))
}

/// 控制条显示着（拖进度时总是显示）
pub fn controls_shown(key: &str) -> bool {
    if drag_ratio(key).is_some() {
        return true;
    }
    UI.with(|m| m.borrow().get(key).map(|u| !u.hidden).unwrap_or(true))
}

fn set_hidden(key: &str, hidden: bool) {
    let changed = with(key, |u| {
        u.touched_ms = now_ms();
        if u.hidden == hidden {
            return false;
        }
        u.hidden = hidden;
        u.seq = u.seq.wrapping_add(1);
        true
    });
    if changed {
        queue_event(key, "controlstoggle", json!({ "show": !hidden }));
    }
}

/// 点了画面：控制条显隐切换
pub fn toggle(key: &str) {
    let hidden = !controls_shown(key);
    set_hidden(key, !hidden);
}

/// 点了控制条上的按钮：控制条留着，自动收起重新计时
pub fn poke(key: &str) {
    set_hidden(key, false);
}

/// 用户按过的静音（没按过就是属性）
pub fn effective_muted(key: &str, attr: bool) -> bool {
    UI.with(|m| m.borrow().get(key).and_then(|u| u.muted)).unwrap_or(attr)
}

/// 静音 / 全屏变一次加一：同层渲染时宿主要重新拿 `videos` 列表
pub fn feed_generation() -> u64 {
    FEED_GEN.with(|g| g.get())
}

pub(super) fn bump_feed_generation() {
    FEED_GEN.with(|g| g.set(g.get().wrapping_add(1)));
}

pub(super) fn ui_seq(key: &str) -> u32 {
    UI.with(|m| m.borrow().get(key).map(|u| u.seq).unwrap_or(0))
}

/// 同层渲染的视频框「该不该重画」的版本号：宿主的帧 / 状态 + 控制条。全屏中的那个冻住
/// （它画在全屏层里，页面里那一框看不见，不必跟着每帧重画）
pub fn mark_seq(key: &str) -> Option<u32> {
    if let Some(frozen) = fullscreen::frozen_mark(key) {
        return Some(frozen);
    }
    live_mark_seq(key)
}

pub(super) fn live_mark_seq(key: &str) -> Option<u32> {
    let s = host_feed::seq(key)?;
    Some(s.wrapping_add(ui_seq(key).wrapping_mul(0x9E37)))
}

/// 每次 pump：记下时钟；起播开始计时、暂停 / 播完叫回控制条、到点自动收起。
/// 返回控制条的样子有没有变（内置播放器那边要重画；同层渲染按 [`mark_seq`] 只重画那一框）
pub fn tick(now: u64) -> bool {
    NOW.with(|n| n.set(now));
    let keys: Vec<(String, String)> = UI.with(|m| m.borrow().iter().map(|(k, u)| (k.clone(), u.src.clone())).collect());
    for (key, src) in keys {
        let playing = VideoRef { key: key.clone(), src, attrs: HashMap::new() }.status().playing;
        let (was, hidden, touched) = UI.with(|m| m.borrow().get(&key).map(|u| (u.was_playing, u.hidden, u.touched_ms)).unwrap_or_default());
        if playing != was {
            with(&key, |u| u.was_playing = playing);
            if playing {
                // 起播（点的 / VideoContext / autoplay）：从这一刻起算自动收起
                with(&key, |u| u.touched_ms = now);
            } else if hidden {
                set_hidden(&key, false);
            }
            continue;
        }
        let dragging = drag_ratio(&key).is_some();
        if playing && !hidden && !dragging && now.saturating_sub(touched) >= AUTO_HIDE_MS {
            set_hidden(&key, true);
        }
    }
    take_changed()
}

fn take_changed() -> bool {
    let sum: u64 = UI.with(|m| m.borrow().values().map(|u| u.seq as u64).sum());
    LAST_SUM.with(|l| l.replace(sum) != sum)
}

/// 还有事没办完：有事件没派、或者播放中等着自动收起（宿主据此每帧来 tick）
pub fn pending() -> bool {
    if EVENTS.with(|e| !e.borrow().is_empty()) {
        return true;
    }
    UI.with(|m| m.borrow().values().any(|u| u.was_playing && !u.hidden))
}

// ── 拖进度条 ──

/// 按下点落在进度条上：先记下（横着拖起来才算，竖着划照样滚页面）
pub fn arm_seek(video: VideoRef, x0: f32, w: f32) {
    ARMED.with(|a| *a.borrow_mut() = Some(Drag { video, x0, w, ratio: 0.0, moved: false }));
}

pub fn disarm_seek() {
    ARMED.with(|a| *a.borrow_mut() = None);
}

/// 手势锁定为横向、交给进度条
pub fn seek_begin(x: f32) -> bool {
    let Some(d) = ARMED.with(|a| a.borrow_mut().take()) else { return false };
    DRAG.with(|s| *s.borrow_mut() = Some(d));
    seek_update(x)
}

/// 拖动推进：圆点跟手、时间跟着变（还不 seek）
pub fn seek_update(x: f32) -> bool {
    let key = DRAG.with(|s| {
        let mut s = s.borrow_mut();
        let d = s.as_mut()?;
        d.ratio = if d.w > 0.0 { ((x - d.x0) / d.w).clamp(0.0, 1.0) as f64 } else { 0.0 };
        d.moved = true;
        Some(d.video.key.clone())
    });
    let Some(key) = key else { return false };
    with(&key, |u| {
        u.seq = u.seq.wrapping_add(1);
        u.touched_ms = now_ms();
    });
    true
}

/// 松手 / 取消：跳到松手的位置（一次）。返回是否真的拖过（拖过就不再算点击）
pub fn seek_end() -> bool {
    disarm_seek();
    let Some(d) = DRAG.with(|s| s.borrow_mut().take()) else { return false };
    with(&d.video.key, |u| {
        u.seq = u.seq.wrapping_add(1);
        u.touched_ms = now_ms();
    });
    if d.moved {
        let st = d.video.status();
        if st.duration > 0.0 {
            d.video.command(&VideoCommand::Seek(st.duration * d.ratio));
        }
    }
    d.moved
}

pub fn seek_dragging() -> bool {
    DRAG.with(|s| s.borrow().as_ref().map(|d| d.moved).unwrap_or(false))
}

/// 正在拖这个视频的进度条：拖到的比例
pub fn drag_ratio(key: &str) -> Option<f64> {
    DRAG.with(|s| s.borrow().as_ref().filter(|d| d.video.key == key).map(|d| d.ratio))
}

/// 换页：控制条状态、拖动与没派的事件跟着页面走
pub fn clear() {
    UI.with(|m| m.borrow_mut().clear());
    ARMED.with(|a| *a.borrow_mut() = None);
    DRAG.with(|s| *s.borrow_mut() = None);
    EVENTS.with(|e| e.borrow_mut().clear());
    LAST_SUM.with(|l| l.set(0));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn video(key: &str) -> VideoRef {
        VideoRef { key: key.into(), src: format!("/{key}.mp4"), attrs: HashMap::new() }
    }

    #[test]
    fn 点画面切换显隐并派发_controlstoggle() {
        clear();
        tick(1000);
        let v = video("ui-toggle");
        note(&v);
        assert!(controls_shown(&v.key));
        toggle(&v.key);
        assert!(!controls_shown(&v.key));
        toggle(&v.key);
        let ev: Vec<_> = take_events().into_iter().map(|(_, n, d)| format!("{n} {d}")).collect();
        assert_eq!(ev, vec![r#"controlstoggle {"show":false}"#, r#"controlstoggle {"show":true}"#]);
        assert!(tick(1016), "显隐变了：内置播放器那边要重画");
        assert!(!tick(1032), "没变：不重画");
        clear();
    }

    #[test]
    fn 拖进度只在松手时_seek_一次() {
        clear();
        let v = video("ui-seek");
        arm_seek(v.clone(), 100.0, 200.0);
        assert!(seek_begin(150.0));
        assert_eq!(drag_ratio(&v.key), Some(0.25));
        assert!(controls_shown(&v.key), "拖着的时候控制条一定在");
        seek_update(400.0);
        assert_eq!(drag_ratio(&v.key), Some(1.0), "拖出轨道夹到头");
        assert!(seek_dragging());
        assert!(seek_end());
        assert_eq!(drag_ratio(&v.key), None);
        assert!(!seek_dragging());
        // 只按下没拖：松手不算拖过
        arm_seek(v, 100.0, 200.0);
        assert!(!seek_end());
        clear();
    }
}
