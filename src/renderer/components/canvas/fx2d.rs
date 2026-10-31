//! Canvas 2D 的「不是纯色、普通叠加」那一类绘制：渐变 / 图案填充、阴影、`globalCompositeOperation`。
//!
//! 做法：先把要画的形状（路径填充 / 描边、文字）用白色画进一张同尺寸的**覆盖率蒙版**，再逐像素
//! 取「源颜色」（纯色、线性 / 径向 / 锥形渐变、图案）× 覆盖率 × `globalAlpha`，按合成模式叠到后备缓冲上；
//! 有阴影时先把蒙版模糊、平移、染成阴影色，按同一合成模式先叠一遍（HTML Canvas 的绘制模型）。
//!
//! 纯色 + `source-over` + 无阴影仍走原来的直接光栅化（`Fx::trivial`），既有画面逐字节不变。
//!
//! 渐变的几何在**用户坐标**里给出，画的那一刻按当前矩阵换到设备坐标（与浏览器一致：渐变随 CTM 走）。
//! 径向渐变在非等比缩放下应是椭圆，这里按平均缩放画成圆（取舍）。

use super::*;
use super::fx_composite::{composite, composite_region};

/// 渐变 / 图案（用户坐标）
#[derive(Clone, Debug)]
pub enum Source {
    Linear { p0: (f32, f32), p1: (f32, f32), stops: Vec<(f32, Color)> },
    Radial { c0: (f32, f32), r0: f32, c1: (f32, f32), r1: f32, stops: Vec<(f32, Color)> },
    /// 起始角（弧度，从 x 轴正向顺时针）与圆心
    Conic { angle: f32, c: (f32, f32), stops: Vec<(f32, Color)> },
    Pattern { rgba: Arc<Vec<u8>>, w: u32, h: u32, repeat_x: bool, repeat_y: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Composite {
    #[default]
    SourceOver,
    SourceIn,
    SourceOut,
    SourceAtop,
    DestinationOver,
    DestinationIn,
    DestinationOut,
    DestinationAtop,
    Lighter,
    Copy,
    Xor,
    Multiply,
    Screen,
    Darken,
    Lighten,
    Difference,
}

impl Composite {
    pub fn parse(s: &str) -> Self {
        match s.trim() {
            "source-in" => Self::SourceIn,
            "source-out" => Self::SourceOut,
            "source-atop" => Self::SourceAtop,
            "destination-over" => Self::DestinationOver,
            "destination-in" => Self::DestinationIn,
            "destination-out" => Self::DestinationOut,
            "destination-atop" => Self::DestinationAtop,
            "lighter" => Self::Lighter,
            "copy" => Self::Copy,
            "xor" => Self::Xor,
            "multiply" => Self::Multiply,
            "screen" => Self::Screen,
            "darken" => Self::Darken,
            "lighten" => Self::Lighten,
            "difference" => Self::Difference,
            _ => Self::SourceOver,
        }
    }

    /// 这几种会动到形状**之外**的像素（目标在源覆盖率为 0 处也变）
    pub(super) fn clears_outside(self) -> bool {
        matches!(self, Self::SourceIn | Self::SourceOut | Self::DestinationIn | Self::DestinationAtop | Self::Copy)
    }
}

/// 阴影（`shadowBlur` 是逻辑 px，偏移不随矩阵变 —— HTML 语义）
#[derive(Clone, Copy, Debug)]
pub struct Shadow {
    pub blur: f32,
    pub color: Color,
    pub ox: f32,
    pub oy: f32,
}

/// 默认「没有阴影」= 透明色。**不能** derive：`Color::default()` 是不透明黑，
/// 那样 `Fx::trivial` 永远为假，所有绘制都绕进蒙版路径，抗锯齿边缘差 1 级
/// （gallery 34_canvas 0.6% 像素、最大通道差 1 就是这么来的）。
impl Default for Shadow {
    fn default() -> Self {
        Self { blur: 0.0, color: Color::TRANSPARENT, ox: 0.0, oy: 0.0 }
    }
}

/// 上下文里这部分状态（随 save / restore）
#[derive(Clone, Debug, Default)]
pub struct Fx {
    pub fill: Option<Source>,
    pub stroke: Option<Source>,
    pub shadow: Shadow,
    pub composite: Composite,
}

impl Shadow {
    /// HTML 语义：颜色不透明度为 0、或模糊与偏移都为 0 时不画阴影
    pub fn visible(&self) -> bool {
        self.color.a > 0 && (self.blur > 0.0 || self.ox != 0.0 || self.oy != 0.0)
    }
}

impl Fx {
    /// 纯色 + source-over + 无阴影：走原来的直接光栅化（逐字节与旧基线一致）
    /// drawImage 用：图自己就是源，填充 / 描边样式（渐变、图案）与它无关，只看合成模式与阴影
    pub fn trivial_for_image(&self) -> bool {
        self.composite == Composite::SourceOver && !self.shadow.visible()
    }

    pub fn trivial(&self, stroke: bool) -> bool {
        let src = if stroke { &self.stroke } else { &self.fill };
        src.is_none() && self.composite == Composite::SourceOver && !self.shadow.visible()
    }
}

fn lerp_stops(stops: &[(f32, Color)], t: f32) -> [f32; 4] {
    let c = |c: Color| [c.r as f32, c.g as f32, c.b as f32, c.a as f32];
    let Some(first) = stops.first() else { return [0.0; 4] };
    if t <= first.0 {
        return c(first.1);
    }
    for w in stops.windows(2) {
        let (a, b) = (w[0], w[1]);
        if t <= b.0 {
            let k = if b.0 > a.0 { (t - a.0) / (b.0 - a.0) } else { 1.0 };
            let (ca, cb) = (c(a.1), c(b.1));
            return [0, 1, 2, 3].map(|i| ca[i] + (cb[i] - ca[i]) * k);
        }
    }
    c(stops.last().unwrap().1)
}

/// 换到设备坐标后的源（每像素取色用）
pub(super) enum DeviceSource {
    Solid([f32; 4]),
    Linear { p0: (f32, f32), d: (f32, f32), len2: f32, stops: Vec<(f32, Color)> },
    Radial { c0: (f32, f32), r0: f32, c1: (f32, f32), r1: f32, stops: Vec<(f32, Color)> },
    Conic { angle: f32, c: (f32, f32), stops: Vec<(f32, Color)> },
    Pattern { rgba: Arc<Vec<u8>>, w: u32, h: u32, rx: bool, ry: bool, inv: [f32; 6] },
}

impl DeviceSource {
    /// 像素中心 (x, y) 的颜色（RGBA 0~255，未预乘）。None：图案不重复时落在外面（透明）
    pub(super) fn sample(&self, x: f32, y: f32) -> [f32; 4] {
        match self {
            Self::Solid(c) => *c,
            Self::Linear { p0, d, len2, stops } => {
                let t = if *len2 > 0.0 { ((x - p0.0) * d.0 + (y - p0.1) * d.1) / len2 } else { 0.0 };
                lerp_stops(stops, t)
            }
            Self::Radial { c0, r0, c1, r1, stops } => {
                // 同心近似：按点到（插值后）圆心的距离在 r0..r1 之间取 t（两个圆心重合时就是精确解）
                let cx = c0.0 + (c1.0 - c0.0) * 0.5;
                let cy = c0.1 + (c1.1 - c0.1) * 0.5;
                let (ccx, ccy) = if (c0.0 - c1.0).abs() < 0.01 && (c0.1 - c1.1).abs() < 0.01 { *c1 } else { (cx, cy) };
                let dist = ((x - ccx).powi(2) + (y - ccy).powi(2)).sqrt();
                let t = if (r1 - r0).abs() > 1e-4 { (dist - r0) / (r1 - r0) } else { 0.0 };
                lerp_stops(stops, t)
            }
            Self::Conic { angle, c, stops } => {
                let a = (y - c.1).atan2(x - c.0) - angle;
                let t = a.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
                lerp_stops(stops, t)
            }
            Self::Pattern { rgba, w, h, rx, ry, inv } => {
                let u = inv[0] * x + inv[2] * y + inv[4];
                let v = inv[1] * x + inv[3] * y + inv[5];
                let (w, h) = (*w as f32, *h as f32);
                if (!rx && (u < 0.0 || u >= w)) || (!ry && (v < 0.0 || v >= h)) || w < 1.0 || h < 1.0 {
                    return [0.0; 4];
                }
                let (iu, iv) = (u.rem_euclid(w) as usize, v.rem_euclid(h) as usize);
                let i = (iv * w as usize + iu) * 4;
                [rgba[i] as f32, rgba[i + 1] as f32, rgba[i + 2] as f32, rgba[i + 3] as f32]
            }
        }
    }
}

fn invert(m: [f32; 6]) -> [f32; 6] {
    let det = m[0] * m[3] - m[1] * m[2];
    if det.abs() < 1e-12 {
        return [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    }
    let id = 1.0 / det;
    [
        m[3] * id,
        -m[1] * id,
        -m[2] * id,
        m[0] * id,
        (m[2] * m[5] - m[3] * m[4]) * id,
        (m[1] * m[4] - m[0] * m[5]) * id,
    ]
}

impl Canvas2DContext {
    pub(super) fn device_source(&self, stroke: bool) -> DeviceSource {
        let src = if stroke { &self.fx.stroke } else { &self.fx.fill };
        let solid = if stroke { self.stroke_style } else { self.fill_style };
        let sc = self.avg_scale();
        match src {
            None => DeviceSource::Solid([solid.r as f32, solid.g as f32, solid.b as f32, solid.a as f32]),
            Some(Source::Linear { p0, p1, stops }) => {
                let (a, b) = (self.tp(p0.0, p0.1), self.tp(p1.0, p1.1));
                let d = (b.0 - a.0, b.1 - a.1);
                DeviceSource::Linear { p0: a, d, len2: d.0 * d.0 + d.1 * d.1, stops: stops.clone() }
            }
            Some(Source::Radial { c0, r0, c1, r1, stops }) => DeviceSource::Radial {
                c0: self.tp(c0.0, c0.1),
                r0: r0 * sc,
                c1: self.tp(c1.0, c1.1),
                r1: r1 * sc,
                stops: stops.clone(),
            },
            Some(Source::Conic { angle, c, stops }) => {
                let m = &self.transform;
                DeviceSource::Conic { angle: angle + m[1].atan2(m[0]), c: self.tp(c.0, c.1), stops: stops.clone() }
            }
            Some(Source::Pattern { rgba, w, h, repeat_x, repeat_y }) => DeviceSource::Pattern {
                rgba: rgba.clone(),
                w: *w,
                h: *h,
                rx: *repeat_x,
                ry: *repeat_y,
                inv: invert(self.transform),
            },
        }
    }

    /// 走蒙版的绘制：`draw_mask` 往一张透明画布上用白色画出形状
    pub(super) fn paint_with_fx(&mut self, stroke: bool, draw_mask: impl FnOnce(&mut Canvas)) {
        self.paint_with_fx_in(stroke, None, draw_mask)
    }

    /// 同上；`bounds` 给了形状的设备坐标外接框（必须保守：光栅器只会写在框外扩 2px 以内）时，
    /// 画蒙版裁剪到这个框、只在框里找包围盒 —— 1320×2868 的 UI 画布上整张扫一遍要 1ms 往上
    /// （iPhone 萌宠小岛 Time Profiler：fill 里四分之一的样本在扫蒙版）。裁剪只是兜底：
    /// 框真漏了，结果是形状被切一块，而不是缓存的蒙版里留下脏像素去污染之后的绘制
    pub(super) fn paint_with_fx_in(&mut self, stroke: bool, bounds: Option<(f32, f32, f32, f32)>, draw_mask: impl FnOnce(&mut Canvas)) {
        let _t = crate::game::slow_frame::enabled().then(SubTimer::start);
        let Ok(mut canvas) = self.canvas.lock() else { return };
        let (w, h) = (canvas.width(), canvas.height());
        let alpha = self.global_alpha;
        let op = self.fx.composite;
        let sh = self.fx.shadow;
        // 常见情形（渐变 / 图案填充、文字，source-over 这类只动形状内的模式，无阴影）：只在被写过的
        // 包围盒里取覆盖率与合成（见 fx_mask.rs 文件头）。阴影的模糊会把覆盖率摊到框外、
        // clears_outside 的模式要动框外像素，这两种仍走整画布
        let fast = !sh.visible() && !op.clears_outside();
        let full = (0, 0, w as usize, h as usize);
        let region = match bounds {
            Some(b) if fast => super::fx_mask::pixel_region(b, w, h),
            _ => full,
        };
        if region.2 <= region.0 || region.3 <= region.1 {
            return;
        }
        let mut mask = super::fx_mask::take(w, h);
        let clipped = region != full;
        if clipped {
            let (x0, y0, x1, y1) = region;
            mask.clip_rect(crate::Rect::new(x0 as f32, y0 as f32, (x1 - x0) as f32, (y1 - y0) as f32));
        }
        draw_mask(&mut mask);
        if clipped {
            mask.reset_clip();
        }
        if fast {
            if let Some(bb) = super::fx_mask::dirty_bbox(&mask, region) {
                let cov = super::fx_mask::coverage(&mask, bb);
                super::fx_mask::clear_rect(&mut mask, bb);
                super::fx_mask::give_back(mask);
                let src = self.device_source(stroke);
                composite_region(&mut canvas, &cov, bb, &src, alpha, op);
            } else {
                super::fx_mask::give_back(mask);
            }
            return;
        }
        let cov: Vec<u8> = mask.pixels().iter().map(|p| p.a).collect();
        mask.clear(Color::TRANSPARENT);
        super::fx_mask::give_back(mask);
        if sh.visible() {
            let blurred = blur_mask(&cov, w as usize, h as usize, sh.blur * self.dpr / 2.0);
            let (dx, dy) = ((sh.ox * self.dpr).round() as i64, (sh.oy * self.dpr).round() as i64);
            let shifted = shift_mask(&blurred, w as usize, h as usize, dx, dy);
            let c = [sh.color.r as f32, sh.color.g as f32, sh.color.b as f32, sh.color.a as f32];
            composite(&mut canvas, &shifted, &DeviceSource::Solid(c), alpha, op);
        }
        let src = self.device_source(stroke);
        composite(&mut canvas, &cov, &src, alpha, op);
    }
}

/// 慢帧归因（`MINI_GAME_SLOW_MS`）里的「fx 蒙版」一项
struct SubTimer(std::time::Instant);

impl SubTimer {
    fn start() -> Self {
        Self(std::time::Instant::now())
    }
}

impl Drop for SubTimer {
    fn drop(&mut self) {
        crate::game::slow_frame::note_sub(true, self.0.elapsed().as_nanos() as u64);
    }
}

/// 三遍盒式模糊（≈ 高斯，σ = blur / 2）
fn blur_mask(m: &[u8], w: usize, h: usize, sigma: f32) -> Vec<u8> {
    if sigma <= 0.0 {
        return m.to_vec();
    }
    let r = ((sigma * 1.88 - 1.0) / 2.0).round().max(1.0) as usize;
    let mut a: Vec<f32> = m.iter().map(|&v| v as f32).collect();
    let mut b = vec![0.0f32; a.len()];
    for _ in 0..3 {
        for y in 0..h {
            let row = &a[y * w..(y + 1) * w];
            let mut acc = 0.0;
            for x in 0..(r + 1).min(w) {
                acc += row[x];
            }
            for x in 0..w {
                b[y * w + x] = acc / (2 * r + 1) as f32;
                if x + r + 1 < w {
                    acc += row[x + r + 1];
                }
                if x >= r {
                    acc -= row[x - r];
                }
            }
        }
        for x in 0..w {
            let mut acc = 0.0;
            for y in 0..(r + 1).min(h) {
                acc += b[y * w + x];
            }
            for y in 0..h {
                a[y * w + x] = acc / (2 * r + 1) as f32;
                if y + r + 1 < h {
                    acc += b[(y + r + 1) * w + x];
                }
                if y >= r {
                    acc -= b[(y - r) * w + x];
                }
            }
        }
    }
    a.iter().map(|&v| v.round().clamp(0.0, 255.0) as u8).collect()
}

fn shift_mask(m: &[u8], w: usize, h: usize, dx: i64, dy: i64) -> Vec<u8> {
    let mut out = vec![0u8; m.len()];
    for y in 0..h as i64 {
        let sy = y - dy;
        if sy < 0 || sy >= h as i64 {
            continue;
        }
        for x in 0..w as i64 {
            let sx = x - dx;
            if sx >= 0 && sx < w as i64 {
                out[(y as usize) * w + x as usize] = m[sy as usize * w + sx as usize];
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(ctx: &Canvas2DContext, x: usize, y: usize) -> [u8; 4] {
        let d = ctx.get_image_data();
        let i = (y * ctx.width as usize + x) * 4;
        [d[i], d[i + 1], d[i + 2], d[i + 3]]
    }

    #[test]
    fn linear_gradient_fill_interpolates_along_the_axis() {
        let mut ctx = Canvas2DContext::new("g", 100, 10);
        ctx.fx.fill = Some(Source::Linear { p0: (0.0, 0.0), p1: (100.0, 0.0), stops: vec![(0.0, Color::rgb(255, 0, 0)), (1.0, Color::rgb(0, 0, 255))] });
        ctx.fill_rect(0.0, 0.0, 100.0, 10.0);
        let l = px(&ctx, 2, 5);
        let m = px(&ctx, 50, 5);
        let r = px(&ctx, 97, 5);
        assert!(l[0] > 240 && l[2] < 15, "{l:?}");
        assert!((m[0] as i32 - 127).abs() < 6 && (m[2] as i32 - 127).abs() < 6, "{m:?}");
        assert!(r[2] > 240 && r[0] < 15, "{r:?}");
    }

    #[test]
    fn shadow_draws_offset_blurred_copy_under_the_shape() {
        let mut ctx = Canvas2DContext::new("s", 60, 60);
        ctx.set_fill_style("#ff0000");
        ctx.fx.shadow = Shadow { blur: 0.0, color: Color::rgb(0, 0, 0), ox: 20.0, oy: 20.0 };
        ctx.fill_rect(5.0, 5.0, 20.0, 20.0);
        assert_eq!(px(&ctx, 10, 10), [255, 0, 0, 255], "形状本身");
        assert_eq!(px(&ctx, 40, 40), [0, 0, 0, 255], "偏移 (20,20) 的阴影");
        assert_eq!(px(&ctx, 55, 5)[3], 0);
    }

    #[test]
    fn composite_destination_out_erases_and_source_in_clips() {
        let mut ctx = Canvas2DContext::new("c", 40, 40);
        ctx.set_fill_style("#00ff00");
        ctx.fill_rect(0.0, 0.0, 40.0, 40.0);
        ctx.fx.composite = Composite::DestinationOut;
        ctx.set_fill_style("#000000");
        ctx.fill_rect(0.0, 0.0, 20.0, 40.0);
        assert_eq!(px(&ctx, 10, 10)[3], 0, "destination-out 挖掉左半");
        assert_eq!(px(&ctx, 30, 10), [0, 255, 0, 255]);
        ctx.fx.composite = Composite::SourceIn;
        ctx.set_fill_style("#0000ff");
        ctx.fill_rect(25.0, 0.0, 15.0, 40.0);
        assert_eq!(px(&ctx, 30, 10), [0, 0, 255, 255], "source-in：只在已有内容上");
        assert_eq!(px(&ctx, 22, 10)[3], 0, "source-in 清掉形状外的已有内容");
    }

    /// 包围盒快路与整画布合成逐字节相同，且缓存的蒙版用完是干净的（第二次画不残留第一次的形状）
    #[test]
    fn bbox_fast_path_matches_full_canvas_composite() {
        let grad = || Some(Source::Linear { p0: (0.0, 0.0), p1: (80.0, 30.0), stops: vec![(0.0, Color::rgb(255, 0, 0)), (1.0, Color::new(0, 0, 255, 128))] });
        let draw = |ctx: &mut Canvas2DContext| {
            ctx.set_fill_style("#336699");
            ctx.fill_rect(0.0, 0.0, 80.0, 60.0);
            ctx.fx.fill = grad();
            ctx.begin_path();
            ctx.arc(30.0, 25.0, 13.3, 0.0, 6.0, false);
            ctx.fill();
            ctx.fill_rect(41.5, 33.25, 50.0, 40.0);
        };
        let mut a = Canvas2DContext::new("a", 80, 60);
        draw(&mut a);
        let mut b = Canvas2DContext::new("b", 80, 60);
        draw(&mut b);
        assert_eq!(a.get_image_data(), b.get_image_data(), "复用的蒙版没清干净");
        // 参照：同一形状走整画布合成（Lighter 也只动形状内，但这里直接调整画布版本）
        let mut c = Canvas2DContext::new("c", 80, 60);
        c.set_fill_style("#336699");
        c.fill_rect(0.0, 0.0, 80.0, 60.0);
        c.fx.fill = grad();
        for shape in 0..2 {
            let src = c.device_source(false);
            let mut mask = Canvas::new(80, 60);
            mask.clear(Color::TRANSPARENT);
            let paint = crate::Paint::new().with_color(Color::WHITE).with_anti_alias(true).with_style(crate::PaintStyle::Fill);
            let path = if shape == 0 {
                c.begin_path();
                c.arc(30.0, 25.0, 13.3, 0.0, 6.0, false);
                c.build_path()
            } else {
                c.rect_path(41.5, 33.25, 50.0, 40.0)
            };
            mask.draw_path(&path, &paint);
            let cov: Vec<u8> = mask.pixels().iter().map(|p| p.a).collect();
            let canvas = c.canvas.clone();
            composite(&mut canvas.lock().unwrap(), &cov, &src, 1.0, Composite::SourceOver);
        }
        assert_eq!(a.get_image_data(), c.get_image_data(), "包围盒快路与整画布合成不一致");
    }

    /// 默认状态必须走直接光栅化：曾因 `Shadow` derive 了 Default（不透明黑）而永远绕进蒙版，
    /// 旧基线的抗锯齿边缘全差 1 级
    #[test]
    fn default_state_and_invisible_shadow_stay_on_the_direct_path() {
        assert!(Fx::default().trivial(false) && Fx::default().trivial(true));
        let mut fx = Fx::default();
        fx.shadow = Shadow { blur: 0.0, color: Color::rgb(0, 0, 0), ox: 0.0, oy: 0.0 };
        assert!(fx.trivial(false), "setShadow(0, 0, 0, '#000')：不画阴影");
        fx.shadow.blur = 4.0;
        assert!(!fx.trivial(false));
    }
}
