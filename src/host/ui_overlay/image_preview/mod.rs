//! `wx.previewImage` / `wx.previewMedia` 的全屏预览（官方 api/media/image/wx.previewImage.html、
//! api/media/image/wx.previewMedia.html）。
//!
//! 与 action sheet 同一类宿主浮层：画在合成那一步、GPU 后端整帧直通、状态在线程局部。
//! - `urls` 依次排开，`current`（url 或下标，默认第一张）是打开时那一张；`videos`（previewMedia 带来）
//!   里非空的那几项是视频，翻到就直接播（`video.rs`），轻点播 / 停；
//! - 左右拖动跟手，松手按距离（过四分之一屏）或甩的速度换张，弹簧接住松手速度滑过去（`motion.rs`）；
//!   到头拉不动（有阻力）；滑动中再按住就停在手指下面；
//! - 往下拉跟手缩小、背景变透明，拉够了松手就关，没拉够回弹；
//! - 打开放大淡入、关闭缩小淡出；右上角关闭键，轻点图片也关；多于一张时底部显示「当前 / 总数」。
//! 取舍：不做双指缩放与长按菜单（`showmenu`）；打开不从缩略图的位置飞出来（接口不带缩略图的位置）。

mod chrome;
mod motion;
mod raster;
pub mod video;

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

pub fn on_press(vp: (f32, f32), x: f32, y: f32) -> bool {
    let now = crate::frame_clock::now();
    let top = TOP.with(Cell::get);
    with(|s| {
        let Some(p) = s.as_mut() else { return false };
        if matches!(p.phase, Phase::Closing(..)) {
            return true;
        }
        p.vp = vp;
        let pos = pos_at(p, now);
        let caught = p.slide.is_some();
        // 滑动中途按住：停在手指下面（位置接着按下这一刻的往下算）
        p.slide = None;
        p.back = None;
        let mut track = Tracker::default();
        track.push(Instant::now(), x, y);
        let on_close = chrome::hits_close(vp.0, top, x, y);
        p.drag = Some(Drag { x0: x, y0: y, x, y, axis: None, pos0: pos, caught, on_close, track });
        p.dirty = true;
        true
    })
}

pub fn on_move(x: f32, y: f32) -> bool {
    with(|s| {
        let Some(p) = s.as_mut() else { return false };
        let Some(d) = p.drag.as_mut() else { return true };
        d.x = x;
        d.y = y;
        d.track.push(Instant::now(), x, y);
        if d.axis.is_none() && !d.on_close {
            let (dx, dy) = (x - d.x0, y - d.y0);
            if dx.abs().max(dy.abs()) >= TAP_SLOP {
                // 竖着往下拉、而且不是滑动中途按住的：下拉关闭；其余都算左右翻
                let down = dy > 0.0 && dy.abs() > dx.abs() * 1.2 && !d.caught && d.pos0.fract().abs() < 1e-3;
                d.axis = Some(if down { Axis::Down } else { Axis::X });
                // 手指已经走出去的那段不吞掉：从按下的地方起算（跟手不跳）
            }
        }
        true
    })
}

pub fn on_release(x: f32, y: f32) -> bool {
    let now = crate::frame_clock::now();
    let top = TOP.with(Cell::get);
    let consumed = visible();
    let toggle = with(|s| {
        let Some(p) = s.as_mut() else { return None };
        let Some(mut d) = p.drag.clone() else { return None };
        d.x = x;
        d.y = y;
        p.drag = Some(d.clone());
        let look = look_at(p, now);
        let (vx, vy) = d.track.velocity();
        p.drag = None;
        p.dirty = true;
        match d.axis {
            None => {
                if d.on_close && chrome::hits_close(p.vp.0, top, x, y) {
                    p.phase = Phase::Closing(now, look);
                } else if d.caught {
                    // 滑动中途按住又松开：滑到最近的那张
                    settle(p, look.pos, 0.0, now);
                } else if !p.items[p.index].video.is_empty() {
                    return Some(p.items[p.index].video.clone());
                } else {
                    p.phase = Phase::Closing(now, look);
                }
            }
            Some(Axis::Down) => {
                if d.y - d.y0 > DISMISS_DY || vy > DISMISS_VY {
                    p.phase = Phase::Closing(now, look);
                } else {
                    p.back = Some((now, d.x - d.x0, d.y - d.y0));
                }
            }
            Some(Axis::X) => {
                let dx = d.x - d.x0;
                let target = if dx <= -p.vp.0 * 0.25 || vx < -FLING_VX {
                    (d.pos0.floor() + 1.0).min(p.items.len() as f32 - 1.0)
                } else if dx >= p.vp.0 * 0.25 || vx > FLING_VX {
                    (d.pos0.ceil() - 1.0).max(0.0)
                } else {
                    look.pos.round().clamp(0.0, p.items.len() as f32 - 1.0)
                };
                p.index = target as usize;
                p.slide = Some((now, look.pos - target, -vx / p.vp.0.max(1.0)));
            }
        }
        None
    });
    if let Some(v) = toggle {
        video::toggle(&v);
    }
    sync_video();
    consumed
}

/// 滑到离 `pos` 最近的那张
fn settle(p: &mut Preview, pos: f32, v: f32, now: Instant) {
    let target = pos.round().clamp(0.0, p.items.len() as f32 - 1.0);
    p.index = target as usize;
    p.slide = Some((now, pos - target, v));
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

/// 画进输出缓冲（0x00RRGGBB，行宽 `w`）。没有预览时什么也不写
pub fn render(buf: &mut [u32], w: u32, h: u32, sf: f32, text: Option<&TextRenderer>) {
    let Some(p) = with(|s| s.clone()) else { return };
    let look = FRAME_LOOK.with(|f| f.take()).unwrap_or_else(|| look_at(&p, crate::frame_clock::now()));
    LAST_SIZE.with(|s| s.set((w, h)));
    let (vw, keep) = (w as f32, ((1.0 - look.bg) * 256.0).round().clamp(0.0, 256.0) as u32);
    raster::darken(buf, keep);
    let k = (look.alpha * 256.0).round().clamp(0.0, 256.0) as u32;
    let mut missing = false;
    let mut paused_video = false;
    let lo = look.pos.floor() as isize;
    for i in [lo, lo + 1] {
        let Some(item) = (i >= 0).then(|| p.items.get(i as usize)).flatten() else { continue };
        let off = (i as f32 - look.pos) * vw;
        if off.abs() >= vw {
            continue;
        }
        // 当前那张（离正中最近）带着开合 / 下拉的缩放与位移
        let near = (i as f32 - look.pos).abs() < 0.5;
        let (scale, tx, ty) = if near { (look.scale, look.tx * sf, look.ty * sf) } else { (1.0, 0.0, 0.0) };
        let place = |fw: u32, fh: u32| {
            let (dw, dh) = (fw as f32 * scale, fh as f32 * scale);
            ((vw - dw) / 2.0 + off + tx, (h as f32 - dh) / 2.0 + ty, dw, dh)
        };
        let frame = (!item.video.is_empty() && video::active().as_deref() == Some(item.video.as_str()))
            .then(|| {
                video::note_box(w, h);
                video::frame(&item.video)
            })
            .flatten();
        if let Some(f) = &frame {
            let (data, fw0, fh0) = f.rgba();
            let (fw, fh) = raster::fit_size(fw0, fh0, w, h);
            raster::blit(buf, w, h, &raster::Src::Rgba(data, fw0, fh0), place(fw, fh), k);
        } else if let Some(f) = raster::fitted(&item.url, w, h) {
            raster::blit(buf, w, h, &raster::Src::Px(&f.px, f.w, f.h), place(f.w, f.h), k);
        } else if !item.url.is_empty() {
            // 后台还在缩（或图还没解出来）：有原图就拿原图凑合，缩好了下一帧换上
            missing = true;
            if let Some((data, iw, ih)) = crate::renderer::components::image_rgba(&item.url) {
                let (fw, fh) = raster::fit_size(iw, ih, w, h);
                raster::blit(buf, w, h, &raster::Src::Rgba(&data, iw, ih), place(fw, fh), k);
            }
        }
        if near && !item.video.is_empty() {
            paused_video = !video::playing(&item.video);
        }
    }
    let shown = look.pos.round().clamp(0.0, (p.items.len() - 1) as f32) as usize;
    let insets = (TOP.with(Cell::get), BOTTOM.with(Cell::get));
    chrome::draw(buf, (w, h, sf), text, insets, (shown, p.items.len()), paused_video, look.bg);
    with(|s| {
        if let Some(x) = s.as_mut() {
            x.dirty = false;
            x.missing = missing;
            // 拖动的换算（一屏多宽、下拉缩放按多高）按画面的逻辑尺寸；按下时宿主也会给一份
            x.vp = (w as f32 / sf, h as f32 / sf);
        }
    });
}
