//! 预览层里的视频（`wx.previewMedia` 的 `type: 'video'`）：当前那一项是视频就直接播。
//!
//! 两种播放器与页面里的 `<video>` 是同一套，只是这一路不对应任何节点：
//! - **同层渲染**（SDK，创建选项 `sameLayer` 含 `video`）：引擎在 `videos` 事件里多报一路
//!   [`VIDEO_KEY`]（`engine_video_feed.rs`），宿主照常建播放器、按这个键交帧（`host_feed`），
//!   播放器的事件按这个键回来，只更新状态（没有节点可派发）；
//! - **内置播放器**（桌面窗体 / 没开同层渲染）：`components::video` 的软解，按地址认。
//!
//! 切走 / 关掉预览：同层渲染时从 `videos` 里去掉这一路，宿主释放播放器；内置播放器暂停。
//! 宿主自己盖原生播放器的模式（`nativeComponents` 含 `video`）不在这里接：那时只画封面。
use crate::renderer::components::{self as c, host_feed};
use std::cell::{Cell, RefCell};
use std::sync::Arc;

/// 同层渲染时这一路视频的键（与页面里的视频不会撞：节点的键是 id / src）
pub const VIDEO_KEY: &str = "__preview_video";

thread_local! {
    /// 正在放的那一项的地址
    static ACTIVE: RefCell<Option<String>> = const { RefCell::new(None) };
    /// ACTIVE 每变一次加一（引擎据此重新数 `videos`）
    static GEN: Cell<u64> = const { Cell::new(0) };
    /// 上一次画进预览的那一帧的版本号（同层渲染）
    static DRAWN: Cell<Option<u32>> = const { Cell::new(None) };
}

/// 一帧画面（RGBA）
pub(super) enum Frame {
    Host(Arc<host_feed::HostFrame>),
    Builtin(Vec<u8>, u32, u32),
}

impl Frame {
    pub(super) fn rgba(&self) -> (&[u8], u32, u32) {
        match self {
            Frame::Host(f) => (&f.rgba, f.width, f.height),
            Frame::Builtin(d, w, h) => (d, *w, *h),
        }
    }
}

/// 预览层当前该放的视频（同层渲染时引擎把它报给宿主）
pub fn active() -> Option<String> {
    ACTIVE.with(|a| a.borrow().clone())
}

pub fn generation() -> u64 {
    GEN.with(Cell::get)
}

/// 换成放 `url`（None = 不放）。内置播放器在这里起播 / 暂停；同层渲染的由宿主按 `videos` 建 / 放
pub(super) fn set_active(url: Option<&str>) {
    let old = active();
    if old.as_deref() == url {
        return;
    }
    if let Some(o) = old.as_deref().filter(|_| !host_feed::host_feeds_video()) {
        if c::is_video_playing(o) {
            c::toggle_video_play(o);
        }
    }
    if let Some(u) = url.filter(|_| !host_feed::host_feeds_video()) {
        c::get_or_create_player(u, true, false);
        if !c::is_video_playing(u) {
            c::toggle_video_play(u);
        }
    }
    ACTIVE.with(|a| *a.borrow_mut() = url.map(str::to_string));
    GEN.with(|g| g.set(g.get() + 1));
    DRAWN.with(|d| d.set(None));
}

/// 当前这一帧画面（还没解出来给 None：画封面）
pub(super) fn frame(url: &str) -> Option<Frame> {
    if host_feed::host_feeds_video() {
        let f = host_feed::frame(VIDEO_KEY)?;
        DRAWN.with(|d| d.set(host_feed::seq(VIDEO_KEY)));
        return Some(Frame::Host(f));
    }
    c::get_video_frame(url).map(|(d, w, h)| Frame::Builtin(d, w, h))
}

/// 宿主读回画面时按这个尺寸缩（设备像素）
pub(super) fn note_box(w: u32, h: u32) {
    if host_feed::host_feeds_video() {
        host_feed::note_box(VIDEO_KEY, w as f32, h as f32);
    }
}

pub(super) fn playing(url: &str) -> bool {
    if host_feed::host_feeds_video() {
        return host_feed::state(VIDEO_KEY).playing;
    }
    c::is_video_playing(url)
}

/// 有新画面要合成（同层渲染：版本号变了；内置播放器：在播就每帧取）
pub(super) fn has_new_frame(url: &str) -> bool {
    if host_feed::host_feeds_video() {
        return host_feed::seq(VIDEO_KEY) != DRAWN.with(Cell::get);
    }
    c::is_video_playing(url)
}

/// 轻点画面：播 / 停
pub(super) fn toggle(url: &str) {
    if host_feed::host_feeds_video() {
        let cmd = if playing(url) { "pause" } else { "play" };
        host_feed::queue_command(serde_json::json!({ "key": VIDEO_KEY, "command": cmd }));
        return;
    }
    c::toggle_video_play(url);
}
