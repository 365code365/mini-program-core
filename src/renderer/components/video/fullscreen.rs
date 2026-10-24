//! 全屏播放：控制条上的全屏键、`VideoContext.requestFullScreen({direction})` 之后画在所有页面层之上的那一层。
//! 内置播放器与同层渲染（宿主交帧）共用。
//!
//! 合成器（`compositor::cpu`）在页面、tabBar、fixed 层与 Toast/Modal 之间画它，GPU 后端遇到它整帧直通
//! CPU 合成（与 Modal 同一条路）。画面 contain，黑边；控件与页面里的同一套（`chrome.rs`），外加顶栏的
//! 返回键与 `title`，全屏键换成「退出全屏」。点画面切换控制条、播放中 3 秒自动收起（`ui.rs`）。
//!
//! ## 方向（官方 component/video.html 的 `direction`）
//! `requestFullScreen({direction})` 给了用它，否则用节点的 `direction` 属性，都没有按画面宽高比：
//! 横的视频横屏（90）、竖的竖屏（0）。`90` = 屏幕逆时针转 90 度（手机左侧朝下，原来的右边沿成了上边），
//! `-90` 顺时针。事件里的 `direction` 是 `horizontal` / `vertical`。
//!
//! 取舍：横屏是**引擎把这一层转过来画**，不让宿主转 Activity。转 Activity 会让引擎改尺寸、整页按横屏宽度
//! 重排（综合场景页在一加上要几百毫秒），退出时再排回来、滚动位置也可能变；转画面只动这一层，页面一个像素
//! 不用重画。状态栏 / 导航条由宿主收起（引擎发 `videoFullscreen` 事件，见 `take_host_events`），
//! 系统下拉通知栏仍是竖着的 —— 微信转的是整个界面，这一点不同。
//!
//! 点击：触点先按方向换回画面坐标（[`to_content`]），再交给与页面里同一套命中。
use super::chrome::{self, Flags};
use super::ui::{self, VideoRef};
use super::*;
use serde_json::{json, Value};
use std::cell::RefCell;

struct Fs {
    video: VideoRef,
    /// 0 / 90 / -90
    direction: i32,
    /// 进全屏那一刻页面里那一框的版本号（全屏时它不跟着重画）
    frozen: Option<u32>,
}

/// 一次按下
#[derive(Clone, Copy)]
struct Press {
    u0: f32,
    v0: f32,
    /// 按在进度条上（横着拖起来就是拖进度）
    armed: bool,
    dragging: bool,
}

/// 按下后挪了这么多（画面坐标，逻辑像素）就不再算点击
const TAP_SLOP: f32 = 10.0;
/// 进度条横着挪这么多开始跟手
const DRAG_SLOP: f32 = 4.0;

thread_local! {
    static FS: RefCell<Option<Fs>> = const { RefCell::new(None) };
    /// 给宿主的通知（`videoFullscreen`：收起 / 放出状态栏）
    static NOTICES: RefCell<Vec<Value>> = const { RefCell::new(Vec::new()) };
    static PRESS: RefCell<Option<Press>> = const { RefCell::new(None) };
    /// 画全屏那一层的画布（画面方向的尺寸），每帧复用
    static CANVAS: RefCell<Option<Canvas>> = const { RefCell::new(None) };
}

fn dir_name(d: i32) -> &'static str {
    if d == 0 { "vertical" } else { "horizontal" }
}

/// 正在全屏的视频 (键, `vertical` / `horizontal`)
pub fn fullscreen_video() -> Option<(String, &'static str)> {
    FS.with(|f| f.borrow().as_ref().map(|s| (s.video.key.clone(), dir_name(s.direction))))
}

/// 正在全屏的视频与方向（0 / 90 / -90）
pub fn fullscreen_ref() -> Option<(VideoRef, i32)> {
    FS.with(|f| f.borrow().as_ref().map(|s| (s.video.clone(), s.direction)))
}

fn parse_dir(v: i32) -> Option<i32> {
    matches!(v, 0 | 90 | -90).then_some(v)
}

/// 进全屏。`direction`：`requestFullScreen` 给的（0 / 90 / -90）
pub fn enter(video: VideoRef, direction: Option<i32>) {
    if fullscreen_video().map(|(k, _)| k == video.key).unwrap_or(false) {
        return;
    }
    exit();
    let attr = video.attrs.get("direction").and_then(|s| s.trim().parse::<i32>().ok()).and_then(parse_dir);
    let d = direction.and_then(parse_dir).or(attr).unwrap_or_else(|| {
        let st = video.status();
        // 宿主还没报 loadedmetadata（宽高 0）时看已经交来的那一帧：从前这里直接当竖屏，横的视频全屏也是竖着的
        let (w, h) = if st.width > 0 && st.height > 0 {
            (st.width, st.height)
        } else {
            super::host_feed::frame(&video.key).map(|f| (f.width, f.height)).unwrap_or((0, 0))
        };
        if w > h { 90 } else { 0 }
    });
    let key = video.key.clone();
    let frozen = ui::live_mark_seq(&key);
    FS.with(|f| *f.borrow_mut() = Some(Fs { video, direction: d, frozen }));
    ui::queue_event(&key, "fullscreenchange", json!({ "fullScreen": true, "direction": dir_name(d) }));
    NOTICES.with(|n| n.borrow_mut().push(json!({ "type": "videoFullscreen", "fullScreen": true, "direction": d, "key": key })));
    ui::poke(&key);
    ui::bump_feed_generation();
}

/// 退出全屏（返回键、顶栏的返回、控制条的退出键、`exitFullScreen`）。返回是不是真的退了
pub fn exit() -> bool {
    let Some(s) = FS.with(|f| f.borrow_mut().take()) else { return false };
    PRESS.with(|p| *p.borrow_mut() = None);
    CANVAS.with(|c| *c.borrow_mut() = None);
    let key = s.video.key;
    ui::queue_event(&key, "fullscreenchange", json!({ "fullScreen": false, "direction": dir_name(s.direction) }));
    NOTICES.with(|n| n.borrow_mut().push(json!({ "type": "videoFullscreen", "fullScreen": false, "direction": s.direction, "key": key })));
    ui::bump_feed_generation();
    true
}

/// 换页 / 销毁时收掉全屏（不发组件事件：页面已经没了；宿主那边照样要放出状态栏）
pub fn clear_fullscreen() {
    if let Some(s) = FS.with(|f| f.borrow_mut().take()) {
        NOTICES.with(|n| n.borrow_mut().push(json!({ "type": "videoFullscreen", "fullScreen": false, "direction": s.direction, "key": s.video.key })));
    }
    PRESS.with(|p| *p.borrow_mut() = None);
    CANVAS.with(|c| *c.borrow_mut() = None);
}

pub(super) fn frozen_mark(key: &str) -> Option<u32> {
    FS.with(|f| f.borrow().as_ref().filter(|s| s.video.key == key).and_then(|s| s.frozen))
}

/// 全屏那一层的版本号（画面帧 + 控制条）：变了宿主就要重新合成。没有全屏是 None
pub fn compose_seq() -> Option<u64> {
    let (v, d) = fullscreen_ref()?;
    let frame = host_feed::seq(&v.key).unwrap_or(0) as u64;
    Some((frame << 32) ^ ((ui::ui_seq(&v.key) as u64) << 2) ^ (d as u64 & 3))
}

/// 取走给宿主的通知：`{type:"videoFullscreen", fullScreen, direction, key}`
pub fn take_host_events() -> Vec<Value> {
    NOTICES.with(|n| std::mem::take(&mut *n.borrow_mut()))
}

/// 画面方向的尺寸（横屏时宽高对调）
pub fn content_size(vw: f32, vh: f32, dir: i32) -> (f32, f32) {
    if dir == 0 { (vw, vh) } else { (vh, vw) }
}

/// 屏幕坐标 → 画面坐标（与 [`draw_fullscreen`] 的旋转互逆）
pub fn to_content(x: f32, y: f32, vw: f32, vh: f32, dir: i32) -> (f32, f32) {
    match dir {
        // 逆时针转了 90°：用户看到的上边是屏幕右沿、左边是屏幕上沿
        90 => (y, vw - x),
        -90 => (vh - y, x),
        _ => (x, y),
    }
}

fn full_flags(video: &VideoRef, cw: f32) -> Flags {
    let mut f = Flags::of(&video.attrs, cw);
    // 全屏里退出键总在；进度条除非明说不要
    f.fullscreen_btn = true;
    f.progress = video.flag("show-progress", true);
    f
}

/// 把全屏画面画进输出缓冲（0x00RRGGBB，行宽 `w`）。没有全屏时什么也不做
pub fn draw_fullscreen(buf: &mut [u32], w: u32, h: u32, sf: f32, text: Option<&TextRenderer>) {
    let Some((video, dir)) = fullscreen_ref() else { return };
    let (cw, ch) = if dir == 0 { (w, h) } else { (h, w) };
    CANVAS.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.as_ref().map(|c| (c.width(), c.height())) != Some((cw, ch)) {
            *slot = Some(Canvas::new(cw, ch));
        }
        let Some(canvas) = slot.as_mut() else { return };
        let t0 = std::time::Instant::now();
        paint(canvas, &video, sf.max(0.1), text);
        let t1 = std::time::Instant::now();
        blit_rotated(buf, w, h, canvas, dir);
        let ms = |a: std::time::Instant, b: std::time::Instant| b.duration_since(a).as_secs_f32() * 1000.0;
        COST.with(|c| c.set((ms(t0, t1), ms(t1, std::time::Instant::now()))));
    });
}

thread_local! {
    /// 上一次画全屏那一层的耗时（毫秒）：(画画面与控件, 转进输出缓冲)。`MINI_DAMAGE_LOG` 的 🩹 行带上它
    static COST: std::cell::Cell<(f32, f32)> = const { std::cell::Cell::new((0.0, 0.0)) };
}

/// 上一次画全屏那一层的耗时（毫秒）：(画画面与控件, 转进输出缓冲)
pub fn last_cost() -> (f32, f32) {
    COST.with(|c| c.get())
}

fn paint(canvas: &mut Canvas, video: &VideoRef, u: f32, text: Option<&TextRenderer>) {
    let (cw, ch) = (canvas.width() as f32, canvas.height() as f32);
    ui::note(video);
    let st = video.status();
    match video.frame() {
        Some(f) => {
            let (rgba, fw, fh) = f.rgba();
            // 同层渲染时宿主按 `videos` 里的全屏尺寸读回、已经缩好：整行拷贝，只把四周的黑边重新填一遍
            // （控制条的渐变叠在上面，不重填就一帧比一帧黑）。整块先清一遍要多写一整屏
            if canvas.blit_image_exact(rgba, fw, fh, 0.0, 0.0, cw, ch, "aspectFit", 0.0) {
                letterbox(canvas, fw, fh);
            } else {
                canvas.clear(Color::BLACK);
                canvas.draw_image(rgba, fw, fh, 0.0, 0.0, cw, ch, "aspectFit", 0.0);
            }
        }
        None => canvas.clear(Color::BLACK),
    }
    let flags = full_flags(video, cw / u);
    if ui::controls_shown(&video.key) {
        let title = video.attrs.get("title").map(|s| s.trim()).unwrap_or("");
        chrome::draw_top_bar(canvas, text, 0.0, 0.0, cw, u, title);
        let bar = chrome::bar_layout(0.0, 0.0, cw, ch, &flags, u, true, st.duration);
        chrome::draw_bar(canvas, text, &bar, &st, ui::drag_ratio(&video.key), u, true);
    } else if st.playing && flags.bottom_progress {
        chrome::draw_bottom_progress(canvas, 0.0, 0.0, cw, ch, u, &st);
    }
    if !st.playing && flags.center_btn {
        chrome::draw_center(canvas, 0.0, 0.0, cw, ch, u);
    }
}

/// 居中的 fw×fh 画面四周填黑（落点与 `blit_image_exact` 的取整一致：偏移四舍五入到整像素）
fn letterbox(canvas: &mut Canvas, fw: u32, fh: u32) {
    let (cw, ch) = (canvas.width() as i32, canvas.height() as i32);
    let ox = ((cw - fw as i32) as f32 / 2.0).round() as i32;
    let oy = ((ch - fh as i32) as f32 / 2.0).round() as i32;
    let (x1, y1) = ((ox + fw as i32).min(cw), (oy + fh as i32).min(ch));
    let (ox, oy) = (ox.max(0), oy.max(0));
    let black = Color::BLACK;
    for y in 0..ch {
        if y < oy || y >= y1 {
            canvas.fill_span(0, cw, y, black);
        } else {
            canvas.fill_span(0, ox, y, black);
            canvas.fill_span(x1, cw, y, black);
        }
    }
}

/// 画面方向的画布 → 屏幕方向的输出缓冲。分块转（32×32）：按行读一边、按列写另一边，
/// 不分块的话每个像素都换一条缓存行
fn blit_rotated(buf: &mut [u32], w: u32, h: u32, canvas: &Canvas, dir: i32) {
    let (w, h) = (w as usize, h as usize);
    let src = canvas.pixels();
    let cw = canvas.width() as usize;
    let conv = |c: &Color| ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32;
    if dir == 0 {
        for (d, s) in buf.iter_mut().zip(src) {
            *d = conv(s);
        }
        return;
    }
    // 一块 T×T：先把画布那一侧的 T 段（每段 T 个、连续）读进栈上的小块，再转着写进输出的 T 行
    // （每行 T 个、连续）。两边都是整段切片，不再逐像素算下标、查越界 —— 一加上全屏一帧从 15ms 降下来的大头
    const T: usize = 32;
    let mut tile = [[0u32; T]; T];
    for by in (0..h).step_by(T) {
        let ye = (by + T).min(h);
        for bx in (0..w).step_by(T) {
            let xe = (bx + T).min(w);
            let (tw, th) = (xe - bx, ye - by);
            // 90：屏幕 (x, y) = 画面 (u = y, v = W-1-x)；-90：屏幕 (x, y) = 画面 (u = H-1-y, v = x)。
            // 这一块要的画面行 v、每行要的那段 u
            let (v0, u0) = if dir == 90 { (w - xe, by) } else { (bx, h - ye) };
            for (r, row) in tile.iter_mut().take(tw).enumerate() {
                let s = (v0 + r) * cw + u0;
                for (t, c) in row.iter_mut().zip(&src[s..s + th]) {
                    *t = conv(c);
                }
            }
            for j in 0..th {
                let y = by + j;
                let dst = &mut buf[y * w + bx..y * w + xe];
                if dir == 90 {
                    // 画面行 v = W-1-x → 块里第 (xe-1-x) 行，第 j 列
                    for (i, d) in dst.iter_mut().enumerate() {
                        *d = tile[tw - 1 - i][j];
                    }
                } else {
                    // 画面行 v = x → 块里第 i 行；u = H-1-y → 第 (ye-1-y) 列
                    for (i, d) in dst.iter_mut().enumerate() {
                        *d = tile[i][th - 1 - j];
                    }
                }
            }
        }
    }
}

// ── 触摸（屏幕的逻辑坐标；vw / vh 是整个画面的逻辑尺寸） ──

/// 按下。全屏时这串触摸都归它，返回 true
pub fn fullscreen_press(x: f32, y: f32, vw: f32, vh: f32) -> bool {
    let Some((video, dir)) = fullscreen_ref() else { return false };
    let (u, v) = to_content(x, y, vw, vh, dir);
    let (cw, ch) = content_size(vw, vh, dir);
    let mut armed = false;
    if ui::controls_shown(&video.key) {
        let st = video.status();
        let bar = chrome::bar_layout(0.0, 0.0, cw, ch, &full_flags(&video, cw), 1.0, true, st.duration);
        if let (Some(_), Some((x0, tw))) = (bar.track_ratio(u, v, 1.0), bar.track) {
            ui::arm_seek(video, x0, tw);
            armed = true;
        }
    }
    PRESS.with(|p| *p.borrow_mut() = Some(Press { u0: u, v0: v, armed, dragging: false }));
    true
}

/// 移动：按在进度条上横着拖就跟手
pub fn fullscreen_move(x: f32, y: f32, vw: f32, vh: f32) -> bool {
    let Some((_, dir)) = fullscreen_ref() else { return false };
    let (u, _) = to_content(x, y, vw, vh, dir);
    PRESS.with(|p| {
        let mut p = p.borrow_mut();
        let Some(p) = p.as_mut() else { return };
        if p.dragging {
            ui::seek_update(u);
        } else if p.armed && (u - p.u0).abs() > DRAG_SLOP {
            p.dragging = ui::seek_begin(u);
        }
    });
    true
}

/// 抬起：拖过进度条就 seek，否则当一次点击
pub fn fullscreen_release(x: f32, y: f32, vw: f32, vh: f32) -> bool {
    let Some((video, dir)) = fullscreen_ref() else { return false };
    let press = PRESS.with(|p| p.borrow_mut().take());
    if press.map(|p| p.dragging).unwrap_or(false) {
        ui::seek_end();
        return true;
    }
    ui::disarm_seek();
    let (u, v) = to_content(x, y, vw, vh, dir);
    if let Some(p) = press {
        if (u - p.u0).hypot(v - p.v0) > TAP_SLOP {
            return true;
        }
    }
    let (cw, ch) = content_size(vw, vh, dir);
    tap(&video, u, v, cw, ch);
    true
}

/// 这串触摸被系统打断：拖到一半的进度照样落下去
pub fn fullscreen_cancel() {
    if PRESS.with(|p| p.borrow_mut().take()).map(|p| p.dragging).unwrap_or(false) {
        ui::seek_end();
    }
    ui::disarm_seek();
}

fn tap(video: &VideoRef, u: f32, v: f32, cw: f32, ch: f32) {
    let st = video.status();
    let flags = full_flags(video, cw);
    let key = &video.key;
    if ui::controls_shown(key) {
        if chrome::back_hit(0.0, 0.0, 1.0, u, v) {
            exit();
            return;
        }
        let bar = chrome::bar_layout(0.0, 0.0, cw, ch, &flags, 1.0, true, st.duration);
        if bar.contains(u, v) {
            if bar.on_play(u, v) {
                video.command(if st.playing { &VideoCommand::Pause } else { &VideoCommand::Play });
            } else if bar.on_mute(u, v) {
                video.set_muted(!st.muted);
            } else if bar.on_fullscreen(u, v) {
                exit();
                return;
            } else if let (Some(r), true) = (bar.track_ratio(u, v, 1.0), st.duration > 0.0) {
                video.command(&VideoCommand::Seek(st.duration * r));
            }
            ui::poke(key);
            return;
        }
    }
    if !st.playing && flags.center_btn && chrome::center_hit(0.0, 0.0, cw, ch, 1.0, u, v) {
        video.command(&VideoCommand::Play);
        ui::poke(key);
        return;
    }
    ui::toggle(key);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 画面坐标 → 屏幕坐标（测试里验证与 to_content 互逆）
    fn to_screen(u: f32, v: f32, vw: f32, vh: f32, dir: i32) -> (f32, f32) {
        match dir {
            90 => (vw - v, u),
            -90 => (v, vh - u),
            _ => (u, v),
        }
    }

    #[test]
    fn 横屏的触点换算与画法互逆() {
        for dir in [0, 90, -90] {
            for (u, v) in [(10.0, 20.0), (600.0, 300.0), (333.0, 1.0)] {
                let (x, y) = to_screen(u, v, 375.0, 667.0, dir);
                let (u2, v2) = to_content(x, y, 375.0, 667.0, dir);
                assert!((u - u2).abs() < 1e-3 && (v - v2).abs() < 1e-3, "{dir}: ({u},{v}) → ({x},{y}) → ({u2},{v2})");
            }
        }
        // 90：画面左上角（返回键）在屏幕右上角
        assert_eq!(to_screen(0.0, 0.0, 375.0, 667.0, 90), (375.0, 0.0));
        // -90：画面左上角在屏幕左下角
        assert_eq!(to_screen(0.0, 0.0, 375.0, 667.0, -90), (0.0, 667.0));
    }

    #[test]
    fn 旋转拷贝与触点换算一致() {
        // 3×2 的屏幕，横屏画面 2×3：画面 (u, v) 的颜色编码成 u*16+v，看落到屏幕的哪
        for dir in [90, -90] {
            let mut c = Canvas::new(2, 3);
            for v in 0..3u32 {
                for u in 0..2u32 {
                    c.set_pixel(u as i32, v as i32, Color::new(0, 0, (u * 16 + v) as u8, 255));
                }
            }
            let mut buf = vec![0u32; 6];
            blit_rotated(&mut buf, 3, 2, &c, dir);
            for y in 0..2 {
                for x in 0..3 {
                    // 像素中心换算到画面
                    let (u, v) = to_content(x as f32 + 0.5, y as f32 + 0.5, 3.0, 2.0, dir);
                    let want = (u.floor() as u32) * 16 + v.floor() as u32;
                    assert_eq!(buf[y * 3 + x], want, "{dir}: 屏幕 ({x},{y})");
                }
            }
        }
    }
}
