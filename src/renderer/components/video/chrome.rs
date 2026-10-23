//! 控制条的几何与画法（页面里的视频与全屏画面共用；画与点用同一套算式，点哪就是画在哪的那个）。
//!
//! 照微信 `<video>` 的默认控件排（官方 component/video.html 的属性都认）：
//!
//! ```text
//! [▶/❚❚] 00:12 ━━━━━●───────── 00:46 [🔇] [⛶]
//! ```
//! - `show-play-btn`（默认开）：左边的播放 / 暂停键；
//! - `show-progress`：时间与进度条。没写时宽度大于 240 才有（官方原话「若不设置，宽度大于240时才会显示」），
//!   没有进度条时只留「当前 / 时长」；
//! - `show-mute-btn`（默认关）、`show-fullscreen-btn`（默认开）：右边两个键；
//! - `show-center-play-btn`（默认开）：暂停 / 没起播时画面正中一个播放键；
//! - `show-bottom-progress`（默认开）：控制条收起时最底下一条 2px 的进度。
//!
//! 坐标：调用方给框（自己坐标系里的 x / y / w / h）与 `u`（一个逻辑像素等于几个单位）：
//! 画在页面画布上 `u` 是像素比，点击命中用逻辑坐标 `u = 1`。
use super::ui::{attr_flag, Status};
use crate::geometry::Rect as GeoRect;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path};
use std::collections::HashMap;

/// 控制条高（逻辑像素）：页面里 / 全屏
pub const BAR_H: f32 = 40.0;
pub const FULL_BAR_H: f32 = 48.0;
/// 全屏顶栏（返回键 + 标题）
pub const TOP_BAR_H: f32 = 48.0;
const PAD: f32 = 8.0;
/// 按钮的点击区宽
const BTN: f32 = 32.0;
/// 「00:00」占的宽（12px 字实测 33，留点余量；不按实际字宽排，点击命中没有字体也能算）
const TIME_W: f32 = 38.0;
const TIME_W_LONG: f32 = 56.0;
/// 中间播放键的半径 / 点击半径
const CENTER_R: f32 = 25.0;
const CENTER_HIT_R: f32 = 36.0;

/// 哪些控件要画（属性折算好）
#[derive(Clone, Copy, Debug)]
pub struct Flags {
    pub controls: bool,
    pub play_btn: bool,
    pub center_btn: bool,
    pub progress: bool,
    pub fullscreen_btn: bool,
    pub mute_btn: bool,
    pub bottom_progress: bool,
}

impl Flags {
    /// `w`：视频框宽（逻辑像素，`show-progress` 没写时按它定）
    pub fn of(attrs: &HashMap<String, String>, w: f32) -> Self {
        let f = |k: &str, d: bool| attr_flag(attrs, k, d);
        let center_pos = attrs.get("play-btn-position").map(|s| s.trim() == "center").unwrap_or(false);
        Self {
            controls: f("controls", true),
            play_btn: f("show-play-btn", true) && !center_pos,
            center_btn: f("show-center-play-btn", true) || center_pos,
            progress: if attrs.contains_key("show-progress") { f("show-progress", true) } else { w > 240.0 },
            fullscreen_btn: f("show-fullscreen-btn", true),
            mute_btn: f("show-mute-btn", false),
            bottom_progress: f("show-bottom-progress", true),
        }
    }
}

/// 控制条里各个东西的位置（调用方坐标系）
#[derive(Clone, Copy, Debug)]
pub struct Bar {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub play: Option<GeoRect>,
    /// 当前时间（没有进度条时是「当前 / 时长」）的左边
    pub cur_x: f32,
    /// 进度条的横向范围 (x0, 宽)
    pub track: Option<(f32, f32)>,
    /// 时长的左边（有进度条才有）
    pub dur_x: Option<f32>,
    pub mute: Option<GeoRect>,
    pub fullscreen: Option<GeoRect>,
}

impl Bar {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }

    /// 落在进度条上（竖向整条控制条都算，横向两端各放宽半个圆点）：返回比例
    pub fn track_ratio(&self, x: f32, y: f32, u: f32) -> Option<f64> {
        let (x0, w) = self.track?;
        let slop = 8.0 * u;
        (self.contains(x, y) && x >= x0 - slop && x <= x0 + w + slop && w > 0.0).then(|| ((x - x0) / w).clamp(0.0, 1.0) as f64)
    }
}

fn hit(r: &Option<GeoRect>, x: f32, y: f32) -> bool {
    r.map(|r| x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height).unwrap_or(false)
}

impl Bar {
    pub fn on_play(&self, x: f32, y: f32) -> bool {
        hit(&self.play, x, y)
    }
    pub fn on_mute(&self, x: f32, y: f32) -> bool {
        hit(&self.mute, x, y)
    }
    pub fn on_fullscreen(&self, x: f32, y: f32) -> bool {
        hit(&self.fullscreen, x, y)
    }
}

/// 时长过了一小时显示成 `h:mm:ss`（时间那一格要宽一些）
fn long_time(duration: f64) -> bool {
    duration >= 3600.0
}

/// 框 (x, y, w, h) 底部那条控制条
pub fn bar_layout(x: f32, y: f32, w: f32, h: f32, flags: &Flags, u: f32, fullscreen: bool, duration: f64) -> Bar {
    let bh = if fullscreen { FULL_BAR_H } else { BAR_H } * u;
    let by = y + h - bh;
    let tw = if long_time(duration) { TIME_W_LONG } else { TIME_W } * u;
    let rect = |rx: f32| Some(GeoRect::new(rx, by, BTN * u, bh));
    let mut left = x + PAD * u;
    let play = flags.play_btn.then(|| {
        let r = rect(left);
        left += BTN * u;
        r
    }).flatten();
    let cur_x = left + 4.0 * u;
    let mut right = x + w - PAD * u;
    let fullscreen_r = flags.fullscreen_btn.then(|| {
        right -= BTN * u;
        rect(right)
    }).flatten();
    let mute = flags.mute_btn.then(|| {
        right -= BTN * u;
        rect(right)
    }).flatten();
    let (track, dur_x) = if flags.progress {
        let dur_x = right - 4.0 * u - tw;
        let x0 = cur_x + tw + PAD * u;
        let x1 = dur_x - PAD * u;
        ((x1 - x0 > 12.0 * u).then_some((x0, x1 - x0)), Some(dur_x))
    } else {
        (None, None)
    };
    Bar { x, y: by, w, h: bh, play, cur_x, track, dur_x, mute, fullscreen: fullscreen_r }
}

/// 中间播放键：圆心与点击半径
pub fn center_hit(x: f32, y: f32, w: f32, h: f32, u: f32, px: f32, py: f32) -> bool {
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let r = CENTER_HIT_R * u;
    (px - cx).powi(2) + (py - cy).powi(2) <= r * r
}

/// 全屏顶栏的返回键（左上角）
pub fn back_hit(x: f32, y: f32, u: f32, px: f32, py: f32) -> bool {
    px >= x && px <= x + 64.0 * u && py >= y && py <= y + TOP_BAR_H * u
}

/// 默认控件的进度轨道（逻辑像素，相对视频左边）：测试与诊断用，算式同 [`bar_layout`]
pub fn progress_track(w: f32) -> (f32, f32) {
    let flags = Flags::of(&HashMap::new(), w.max(241.0));
    let bar = bar_layout(0.0, 0.0, w, 100.0, &flags, 1.0, false, 0.0);
    bar.track.unwrap_or((0.0, 0.0))
}

pub fn format_time(seconds: f64, long: bool) -> String {
    let s = seconds.max(0.0) as u64;
    if long {
        format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    } else {
        format!("{:02}:{:02}", s / 60, s % 60)
    }
}

fn fill(canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, c: Color) {
    canvas.draw_rect(&GeoRect::new(x, y, w, h), &Paint::new().with_color(c).with_style(PaintStyle::Fill));
}

fn aa(c: Color) -> Paint {
    Paint::new().with_color(c).with_style(PaintStyle::Fill).with_anti_alias(true)
}

/// 竖向渐变的黑（`top_alpha` → `bottom_alpha`），逐行填（一行一次整行填充，比路径快得多）
fn gradient(canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, top_alpha: f32, bottom_alpha: f32) {
    let rows = h.ceil().max(1.0) as i32;
    for i in 0..rows {
        let t = (i as f32 + 0.5) / rows as f32;
        let a = (top_alpha + (bottom_alpha - top_alpha) * t).clamp(0.0, 1.0);
        let rh = (h - i as f32).min(1.0);
        fill(canvas, x, y + i as f32, w, rh, Color::new(0, 0, 0, (a * 255.0).round() as u8));
    }
}

/// 粗线段（四边形填充；`draw_line` 是 1px 的 Wu 线，不认线宽）
fn thick(canvas: &mut Canvas, (x0, y0): (f32, f32), (x1, y1): (f32, f32), width: f32) {
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = (dx * dx + dy * dy).sqrt().max(1e-3);
    let (nx, ny) = (-dy / len * width / 2.0, dx / len * width / 2.0);
    let mut p = Path::new();
    p.move_to(x0 + nx, y0 + ny);
    p.line_to(x1 + nx, y1 + ny);
    p.line_to(x1 - nx, y1 - ny);
    p.line_to(x0 - nx, y0 - ny);
    p.close();
    canvas.draw_path(&p, &aa(Color::WHITE));
}

fn triangle(canvas: &mut Canvas, cx: f32, cy: f32, s: f32) {
    let mut p = Path::new();
    p.move_to(cx - s * 0.4, cy - s * 0.5);
    p.line_to(cx - s * 0.4, cy + s * 0.5);
    p.line_to(cx + s * 0.5, cy);
    p.close();
    canvas.draw_path(&p, &aa(Color::WHITE));
}

fn pause_bars(canvas: &mut Canvas, cx: f32, cy: f32, s: f32) {
    let (bw, gap) = (s * 0.22, s * 0.28);
    fill(canvas, cx - gap / 2.0 - bw, cy - s / 2.0, bw, s, Color::WHITE);
    fill(canvas, cx + gap / 2.0, cy - s / 2.0, bw, s, Color::WHITE);
}

/// 扬声器（静音时带一个叉）
fn speaker(canvas: &mut Canvas, cx: f32, cy: f32, s: f32, muted: bool, u: f32) {
    let mut p = Path::new();
    p.move_to(cx - s * 0.5, cy - s * 0.18);
    p.line_to(cx - s * 0.25, cy - s * 0.18);
    p.line_to(cx + s * 0.05, cy - s * 0.45);
    p.line_to(cx + s * 0.05, cy + s * 0.45);
    p.line_to(cx - s * 0.25, cy + s * 0.18);
    p.line_to(cx - s * 0.5, cy + s * 0.18);
    p.close();
    canvas.draw_path(&p, &aa(Color::WHITE));
    let t = 1.6 * u;
    if muted {
        let (x0, d) = (cx + s * 0.2, s * 0.22);
        thick(canvas, (x0, cy - d), (x0 + 2.0 * d, cy + d), t);
        thick(canvas, (x0, cy + d), (x0 + 2.0 * d, cy - d), t);
    } else {
        // 两道声波：短竖线近似
        thick(canvas, (cx + s * 0.22, cy - s * 0.16), (cx + s * 0.22, cy + s * 0.16), t);
        thick(canvas, (cx + s * 0.42, cy - s * 0.32), (cx + s * 0.42, cy + s * 0.32), t);
    }
}

/// 全屏键：四个角往外的折角；`exit` 时往里（退出全屏）
fn corners(canvas: &mut Canvas, cx: f32, cy: f32, s: f32, exit: bool, u: f32) {
    let (t, arm) = (1.8 * u, s * 0.28);
    let h = s / 2.0 - t / 2.0;
    for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        // 进全屏：折角顶点在外角，两条臂沿边朝里；退出：顶点靠近中心，两条臂朝外伸到边上
        let (vx, vy, out) = if exit {
            (cx + sx * (h - arm), cy + sy * (h - arm), 1.0)
        } else {
            (cx + sx * h, cy + sy * h, -1.0)
        };
        let hx = vx + sx * out * arm;
        fill(canvas, vx.min(hx) - t / 2.0, vy - t / 2.0, (hx - vx).abs() + t, t, Color::WHITE);
        let hy = vy + sy * out * arm;
        fill(canvas, vx - t / 2.0, vy.min(hy) - t / 2.0, t, (hy - vy).abs() + t, Color::WHITE);
    }
}

/// 进度：轨道（半透明）+ 已播（白）+ 圆点
fn track(canvas: &mut Canvas, x0: f32, w: f32, cy: f32, ratio: f64, u: f32, dragging: bool) {
    let th = 2.0 * u;
    fill(canvas, x0, cy - th / 2.0, w, th, Color::new(255, 255, 255, 90));
    let px = x0 + w * ratio.clamp(0.0, 1.0) as f32;
    fill(canvas, x0, cy - th / 2.0, px - x0, th, Color::WHITE);
    let r = if dragging { 8.0 } else { 6.0 } * u;
    canvas.draw_circle(px, cy, r, &aa(Color::WHITE));
}

/// 控制条。`drag`：正在拖进度时拖到的比例（圆点与当前时间跟着它）
#[allow(clippy::too_many_arguments)]
pub fn draw_bar(canvas: &mut Canvas, tr: Option<&TextRenderer>, bar: &Bar, st: &Status, drag: Option<f64>, u: f32, fullscreen: bool) {
    gradient(canvas, bar.x, bar.y, bar.w, bar.h, 0.0, 0.55);
    let cy = bar.y + bar.h / 2.0;
    let icon = 16.0 * u;
    if let Some(p) = bar.play {
        let cx = p.x + p.width / 2.0;
        if st.playing {
            pause_bars(canvas, cx, cy, icon * 0.9);
        } else {
            triangle(canvas, cx, cy, icon);
        }
    }
    if let Some(m) = bar.mute {
        speaker(canvas, m.x + m.width / 2.0, cy, icon, st.muted, u);
    }
    if let Some(f) = bar.fullscreen {
        corners(canvas, f.x + f.width / 2.0, cy, icon * 0.9, fullscreen, u);
    }
    let long = long_time(st.duration);
    let ratio = drag.unwrap_or(if st.duration > 0.0 { st.current / st.duration } else { 0.0 });
    let current = if drag.is_some() { st.duration * ratio } else { st.current };
    if let Some((x0, w)) = bar.track {
        track(canvas, x0, w, cy, ratio, u, drag.is_some());
    }
    let Some(tr) = tr else { return };
    let size = if fullscreen { 13.0 } else { 12.0 } * u;
    let baseline = cy + size * 0.36;
    let white = Paint::new().with_color(Color::WHITE);
    match bar.dur_x {
        Some(dx) => {
            tr.draw_text(canvas, &format_time(current, long), bar.cur_x, baseline, size, &white);
            tr.draw_text(canvas, &format_time(st.duration, long), dx, baseline, size, &white);
        }
        None => {
            let s = format!("{} / {}", format_time(current, long), format_time(st.duration, long));
            tr.draw_text(canvas, &s, bar.cur_x, baseline, size, &white);
        }
    }
}

/// 中间的播放键（半透明黑圆 + 白三角）
pub fn draw_center(canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, u: f32) {
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    canvas.draw_circle(cx, cy, CENTER_R * u, &aa(Color::new(0, 0, 0, 128)));
    triangle(canvas, cx + 2.0 * u, cy, 18.0 * u);
}

/// 控制条收起时最底下那条细进度
pub fn draw_bottom_progress(canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, u: f32, st: &Status) {
    let th = 2.0 * u;
    let ratio = if st.duration > 0.0 { (st.current / st.duration).clamp(0.0, 1.0) } else { 0.0 };
    fill(canvas, x, y + h - th, w, th, Color::new(255, 255, 255, 70));
    fill(canvas, x, y + h - th, w * ratio as f32, th, Color::WHITE);
}

/// 全屏顶栏：渐变 + 返回箭头 + 标题（`title` 属性）
pub fn draw_top_bar(canvas: &mut Canvas, tr: Option<&TextRenderer>, x: f32, y: f32, w: f32, u: f32, title: &str) {
    let h = TOP_BAR_H * u;
    gradient(canvas, x, y, w, h, 0.55, 0.0);
    let (cx, cy, arm) = (x + 22.0 * u, y + h / 2.0, 7.0 * u);
    thick(canvas, (cx + arm * 0.6, cy - arm), (cx - arm * 0.4, cy), 2.2 * u);
    thick(canvas, (cx - arm * 0.4, cy), (cx + arm * 0.6, cy + arm), 2.2 * u);
    if let (Some(tr), false) = (tr, title.is_empty()) {
        let size = 16.0 * u;
        tr.draw_text(canvas, title, x + 44.0 * u, cy + size * 0.36, size, &Paint::new().with_color(Color::WHITE));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs(kv: &[(&str, &str)]) -> HashMap<String, String> {
        kv.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn 默认控件的排法() {
        let f = Flags::of(&HashMap::new(), 300.0);
        let b = bar_layout(0.0, 0.0, 300.0, 200.0, &f, 1.0, false, 46.0);
        assert_eq!((b.y, b.h), (160.0, 40.0));
        assert!(b.on_play(20.0, 180.0) && !b.on_play(60.0, 180.0));
        assert!(b.on_fullscreen(280.0, 180.0), "默认有全屏键，在最右");
        assert!(b.mute.is_none(), "静音键默认关");
        let (x0, w) = b.track.unwrap();
        assert_eq!((x0, w), progress_track(300.0));
        assert_eq!(b.track_ratio(x0 + w / 2.0, 180.0, 1.0), Some(0.5));
        assert_eq!(b.track_ratio(x0 + w / 2.0, 150.0, 1.0), None, "控制条上面不算");
    }

    #[test]
    fn 窄视频不写_show_progress_就没有进度条() {
        let b = bar_layout(0.0, 0.0, 200.0, 150.0, &Flags::of(&HashMap::new(), 200.0), 1.0, false, 10.0);
        assert!(b.track.is_none() && b.dur_x.is_none());
        let b = bar_layout(0.0, 0.0, 200.0, 150.0, &Flags::of(&attrs(&[("show-progress", "true")]), 200.0), 1.0, false, 10.0);
        assert!(b.track.is_some(), "写了 show-progress 就画");
        let f = Flags::of(&attrs(&[("show-mute-btn", "true"), ("show-fullscreen-btn", "false"), ("show-play-btn", "false")]), 300.0);
        let b = bar_layout(0.0, 0.0, 300.0, 200.0, &f, 1.0, false, 10.0);
        assert!(b.play.is_none() && b.fullscreen.is_none() && b.on_mute(280.0, 180.0));
    }

    #[test]
    fn 时间格式() {
        assert_eq!(format_time(46.6, false), "00:46");
        assert_eq!(format_time(3725.0, true), "1:02:05");
    }
}
