//! `wx.previewImage` / `wx.previewMedia` 的全屏预览（官方 api/media/image/wx.previewImage.html、
//! api/media/image/wx.previewMedia.html）。
//!
//! 与 action sheet 同一类宿主浮层：画在合成那一步、GPU 后端整帧直通、状态在线程局部。
//! - `urls` 依次排开，`current`（url 或下标，默认第一张）是打开时那一张；`videos`（previewMedia 带来）
//!   里非空的那几项是视频，翻到就直接播（`video.rs`），轻点播 / 停；
//! - 左右拖动跟手，松手按距离（过四分之一屏）或甩的速度换张，弹簧接住松手速度滑过去（`motion.rs`）；
//!   到头拉不动（有阻力）；滑动中再按住就停在手指下面；
//! - 往下拉跟手缩小、背景变透明，拉够了松手就关，没拉够回弹；
//! - 打开放大淡入、关闭缩小淡出；右上角关闭键，轻点图片也关；多于一张时底部显示「当前 / 总数」；
//! - 双指捏合缩放、双击放大 / 还原、放大后单指平移（到边拖出去一截就换张），见 `zoom.rs`。
//!   轻点关闭要等 [`zoom::DOUBLE_TAP_SECS`] 确认不是双击（微信一样有这一下延迟）。
//!   第二根手指由宿主经 [`on_pointer2`] 送来（引擎的页面触摸仍是单指，只有预览认第二根）。
//! 取舍：不做长按菜单（`showmenu`）；打开不从缩略图的位置飞出来（接口不带缩略图的位置）。

mod chrome;
mod draw;
mod input;
mod motion;
mod raster;
pub mod video;
mod zoom;

pub use draw::render;
pub use input::{on_pointer2, on_press, on_move, on_release};

use crate::runtime::MiniApp;
use crate::text::TextRenderer;
use motion::{Look, Tracker};
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::time::Instant;

pub use video::VIDEO_KEY;

/// 位移小于它算轻点（逻辑像素）
const TAP_SLOP: f32 = 8.0;
/// 下拉超过它（或往下甩）松手就关
const DISMISS_DY: f32 = 90.0;
const DISMISS_VY: f32 = 700.0;
/// 甩的速度超过它就换张（逻辑像素 / 秒）
const FLING_VX: f32 = 450.0;
/// 远程图 / 后台解码的图没到时，最多续帧等这么久（秒）
const WAIT_SECS: f32 = 10.0;

#[derive(Clone, Debug)]
struct Item {
    /// 要画的图（视频是封面，可以为空）
    url: String,
    /// 视频地址（图片为空）
    video: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Axis {
    X,
    Down,
    /// 放大着：单指平移
    Pan,
    /// 两指捏合（以及捏完剩下那根手指：不再算点击、不翻页）
    Pinch,
}

#[derive(Clone, Debug)]
struct Drag {
    x0: f32,
    y0: f32,
    x: f32,
    y: f32,
    axis: Option<Axis>,
    /// 按下时第几张在正中（滑动中途按住时是小数）
    pos0: f32,
    /// 按下时还在滑：轻点只是停住，不关
    caught: bool,
    on_close: bool,
    track: Tracker,
    /// 按下（或捏合结束换回单指）那一刻的平移
    pan0: (f32, f32),
}

#[derive(Clone, Copy, Debug)]
enum Phase {
    Opening(Instant),
    Shown,
    Closing(Instant, Look),
}

#[derive(Clone, Debug)]
struct Preview {
    items: Vec<Item>,
    /// 停下来时在正中的那一张
    index: usize,
    phase: Phase,
    drag: Option<Drag>,
    /// 换张的弹簧：起点时刻、相对 index 的偏移（屏）、初速（屏 / 秒）
    slide: Option<(Instant, f32, f32)>,
    /// 下拉没拉够、回弹：起点时刻与起点位移
    back: Option<(Instant, f32, f32)>,
    /// 逻辑视口（按下时更新；画的时候按缓冲尺寸）
    vp: (f32, f32),
    opened: Instant,
    /// 画面要重新合成（输入变了 / 动画终态还没画）
    dirty: bool,
    /// 上一帧有图没到（远程 / 后台解码）：续帧等它
    missing: bool,
    /// 当前那张的缩放平移（换张时归位）
    zoom: zoom::Zoom,
    pinch: Option<zoom::Pinch>,
    /// 第二根手指（逻辑坐标）
    p2: Option<(f32, f32)>,
    /// 捏合刚结束：下一次单指移动从那一刻的位置重新起算（剩下的手指可能是另一根，位置会跳）
    rebase: bool,
    /// 上一次轻点（双击判定）与等着确认不是双击的关闭
    last_tap: Option<(Instant, f32, f32)>,
    pending_close: Option<Instant>,
    /// 当前那张 aspectFit 后的大小（逻辑像素，上一次画的时候记下）
    fit: (f32, f32),
}

thread_local! {
    static PREVIEW: RefCell<Option<Preview>> = const { RefCell::new(None) };
    /// 状态栏高（逻辑像素）：关闭键放在它下面
    static TOP: Cell<f32> = const { Cell::new(20.0) };
    /// 底部安全区（逻辑像素）：「当前 / 总数」放在它上面（三键导航条 / 手势条不压字）
    static BOTTOM: Cell<f32> = const { Cell::new(0.0) };
    /// 这一帧合成时定下的样子（[`covers_page`] 与 [`render`] 用同一份，见 `covers_page`）
    static FRAME_LOOK: Cell<Option<Look>> = const { Cell::new(None) };
    /// 上一次画的缓冲尺寸（预取左右两张时按它缩）
    static LAST_SIZE: Cell<(u32, u32)> = const { Cell::new((0, 0)) };
}

fn with<R>(f: impl FnOnce(&mut Option<Preview>) -> R) -> R {
    PREVIEW.with(|s| f(&mut s.borrow_mut()))
}

pub fn visible() -> bool {
    with(|s| s.is_some())
}

/// 这一帧要重新合成（动画在走、拖动中、有新视频帧、等图）
pub fn animating() -> bool {
    with(|s| s.as_ref().map(needs_compose).unwrap_or(false))
}

/// 宿主要一直出帧（视频在放：帧由宿主的解码器送来，引擎每个 vsync 看一眼有没有新的）
pub fn wants_frames() -> bool {
    animating() || video::active().is_some()
}

pub fn clear() {
    with(|s| *s = None);
    video::set_active(None);
    raster::clear_cache();
}

/// 状态栏高与底部安全区（逻辑像素），宿主每帧给
pub fn set_insets(top: f32, bottom: f32) {
    TOP.with(|t| t.set(top));
    BOTTOM.with(|b| b.set(bottom.max(0.0)));
}

/// 当前那张（测试 / 诊断用）
pub fn current() -> Option<(usize, String)> {
    with(|s| s.as_ref().map(|p| (p.index, p.items[p.index].url.clone())))
}

fn needs_compose(p: &Preview) -> bool {
    let waiting = p.missing && p.opened.elapsed().as_secs_f32() < WAIT_SECS;
    let video = video::active().map(|u| video::has_new_frame(&u)).unwrap_or(false);
    p.dirty || waiting || video || p.drag.is_some() || p.slide.is_some() || p.back.is_some() || !matches!(p.phase, Phase::Shown)
        || p.zoom.animating() || p.pending_close.is_some()
}

/// 第几张在正中（拖动 / 弹簧 / 停住）
fn pos_at(p: &Preview, now: Instant) -> f32 {
    let n = p.items.len() as f32;
    if let Some(d) = p.drag.as_ref().filter(|d| d.axis == Some(Axis::X)) {
        let raw = d.pos0 - (d.x - d.x0) / p.vp.0.max(1.0);
        let max = n - 1.0;
        return if raw < 0.0 { motion::rubber(raw) } else if raw > max { max + motion::rubber(raw - max) } else { raw };
    }
    if let Some((t0, x0, v0)) = p.slide {
        return p.index as f32 + motion::spring(x0, v0, now.saturating_duration_since(t0).as_secs_f32()).0;
    }
    p.drag.as_ref().map(|d| d.pos0).unwrap_or(p.index as f32)
}

/// 这一刻画成什么样
fn look_at(p: &Preview, now: Instant) -> Look {
    let mut l = Look::at(pos_at(p, now));
    if let Some(d) = p.drag.as_ref().filter(|d| d.axis == Some(Axis::Down)) {
        l = l.dragged_down(d.x - d.x0, d.y - d.y0, p.vp.1);
    } else if let Some((t0, tx, ty)) = p.back {
        let k = 1.0 - motion::ease_out_cubic(now.saturating_duration_since(t0).as_secs_f32() / motion::BACK_SECS);
        l = l.dragged_down(tx * k, ty * k, p.vp.1);
    }
    match p.phase {
        Phase::Opening(t0) => l.opening(now.saturating_duration_since(t0).as_secs_f32()),
        Phase::Shown => l,
        Phase::Closing(t0, from) => from.closing(now.saturating_duration_since(t0).as_secs_f32()),
    }
}

/// 每帧：取走 `wx.previewImage` 的请求、推进动画（走完的收掉）、预取左右两张。返回是否要重新合成
pub fn take_requests(app: &mut MiniApp) -> bool {
    let mut changed = false;
    for req in app.take_host_requests("previewImage") {
        changed |= open(&req);
    }
    let now = crate::frame_clock::now();
    let done = with(|s| {
        let Some(p) = s.as_mut() else { return false };
        if let Some((t0, x0, v0)) = p.slide {
            let (x, v) = motion::spring(x0, v0, now.saturating_duration_since(t0).as_secs_f32());
            if motion::spring_done(x, v) {
                p.slide = None;
                p.dirty = true;
            }
        }
        if p.back.is_some_and(|(t0, ..)| now.saturating_duration_since(t0).as_secs_f32() >= motion::BACK_SECS) {
            p.back = None;
            p.dirty = true;
        }
        if p.zoom.tick(now) {
            p.dirty = true;
        }
        // 轻点过了双击的时限还没等来第二下：关
        if p.pending_close.is_some_and(|t| now.saturating_duration_since(t).as_secs_f32() >= zoom::DOUBLE_TAP_SECS) {
            p.pending_close = None;
            p.last_tap = None;
            let look = look_at(p, now);
            p.phase = Phase::Closing(now, look);
        }
        match p.phase {
            Phase::Opening(t0) if now.saturating_duration_since(t0).as_secs_f32() >= motion::OPEN_SECS => {
                p.phase = Phase::Shown;
                p.dirty = true;
            }
            Phase::Closing(t0, _) => return now.saturating_duration_since(t0).as_secs_f32() >= motion::CLOSE_SECS,
            _ => {}
        }
        false
    });
    if done {
        clear();
        return true;
    }
    sync_video();
    prefetch();
    changed || animating()
}

fn open(req: &Value) -> bool {
    let list = |k: &str| -> Vec<String> {
        req.get(k).and_then(Value::as_array).map(|a| a.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect()).unwrap_or_default()
    };
    let (urls, videos) = (list("urls"), list("videos"));
    let items: Vec<Item> =
        urls.iter().enumerate().map(|(i, u)| Item { url: u.clone(), video: videos.get(i).cloned().unwrap_or_default() }).collect();
    if items.is_empty() || items.iter().all(|i| i.url.is_empty() && i.video.is_empty()) {
        return false;
    }
    let index = match req.get("current") {
        Some(Value::Number(n)) => n.as_u64().unwrap_or(0) as usize,
        Some(Value::String(s)) => urls.iter().position(|u| u == s).unwrap_or(0),
        _ => 0,
    }
    .min(items.len() - 1);
    let now = crate::frame_clock::now();
    raster::clear_cache();
    with(|s| {
        *s = Some(Preview {
            items,
            index,
            phase: Phase::Opening(now),
            drag: None,
            slide: None,
            back: None,
            vp: (375.0, 667.0),
            opened: now,
            dirty: true,
            missing: false,
            zoom: zoom::Zoom::default(),
            pinch: None,
            p2: None,
            rebase: false,
            last_tap: None,
            pending_close: None,
            fit: (375.0, 667.0),
        })
    });
    sync_video();
    true
}

/// 当前那一项是视频、而且不在关：放它；否则停
fn sync_video() {
    let want = with(|s| {
        s.as_ref()
            .filter(|p| !matches!(p.phase, Phase::Closing(..)))
            .map(|p| p.items[p.index].video.clone())
            .filter(|v| !v.is_empty())
    });
    video::set_active(want.as_deref());
}

/// 停稳的时候把左右两张交给后台先缩好：拖起来的第一帧不用拿原图凑合
fn prefetch() {
    let (w, h) = LAST_SIZE.with(Cell::get);
    if w == 0 {
        return;
    }
    let next = with(|s| {
        let p = s.as_ref().filter(|p| matches!(p.phase, Phase::Shown) && p.drag.is_none() && p.slide.is_none())?;
        [p.index.wrapping_sub(1), p.index + 1]
            .into_iter()
            .filter_map(|k| p.items.get(k))
            .map(|i| i.url.clone())
            .find(|u| !u.is_empty() && raster::cached(u, w, h).is_none())
    });
    if let Some(u) = next {
        raster::request(&u, w, h);
    }
}

/// 预览停在「黑底不透明」的状态（开完了、没在下拉 / 回弹 / 关）：页面整个看不见，
/// 宿主可以把页面的重画（动图换帧、CSS 动画、setData）留到它露出来再做（`engine_preview.rs`）
pub fn opaque() -> bool {
    with(|s| {
        s.as_ref().is_some_and(|p| {
            matches!(p.phase, Phase::Shown) && p.back.is_none() && !p.drag.as_ref().is_some_and(|d| d.axis == Some(Axis::Down))
        })
    })
}

/// 这一帧黑底不透明、盖满整屏：底下的页面不用先合成（`compositor::cpu::compose_rows`）。
/// 顺带定住这一帧的样子：紧接着的 [`render`] 用同一份 —— 两次各取一次时刻的话，关闭的第一帧
/// 可能这里判「盖满」（页面没合成）、画的时候黑底已经透了一点，压暗的是一块没画过的缓冲
pub fn covers_page() -> bool {
    let look = with(|s| s.as_ref().map(|p| look_at(p, crate::frame_clock::now())));
    FRAME_LOOK.with(|f| f.set(look));
    look.is_some_and(|l| l.bg >= 0.999)
}
