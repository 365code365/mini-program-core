//! COLR 的绘制器：实现 ttf-parser 的 `colr::Painter`，把一个彩色字形画进 RGBA（预乘、f32）缓冲。
//!
//! 语义按 ttf-parser 的遍历（它自己的 examples/font2svg.rs 也是这么用的）：
//! - `outline_glyph`：按**此刻**的变换取轮廓、存下来；
//! - `paint`：用存下的轮廓 ∩ 当前裁剪去填，颜色 / 渐变在**此刻**的变换下求值（每个像素中心逆变换回 paint 空间算 t）；
//! - `push_clip` / `push_clip_box`：与栈顶裁剪求交；`push_layer` 开一块透明图层，`pop_layer` 按合成模式叠回下一层。
//!
//! 渐变：线性按规范用 p2 把 p0→p1 投影到「垂直于 p0p2」的方向；径向是双圆锥（取 r(t) ≥ 0 的较大根）；
//! 扫描角按 180° 一个单位（与 ttf-parser 的 `new_rotate` 同一口径），逆时针为正（字体空间 y 向上）。
//! 颜色线在预乘空间插值；Pad / Repeat / Reflect 都相对第一个到最后一个色标。
//! 合成模式：Porter-Duff 全套 + Plus / Screen / Multiply；其余可分离 / 不可分离混合模式按 SourceOver 处理
//! （Noto Color Emoji 里没用到，写明取舍，不假装支持）。
use super::raster::{coverage, Affine, Contours, Flatten};
use ttf_parser::colr::{ClipBox, ColorStop, CompositeMode, GradientExtend, Paint, Painter};
use ttf_parser::{Face, GlyphId, RgbaColor, Transform};

type Px = [f32; 4];

pub struct ColrPainter<'f, 'a> {
    face: &'f Face<'a>,
    w: usize,
    h: usize,
    cur: Affine,
    transforms: Vec<Affine>,
    path: Contours,
    clips: Vec<Vec<f32>>,
    layers: Vec<(Vec<Px>, CompositeMode)>,
}

impl<'f, 'a> ColrPainter<'f, 'a> {
    pub fn new(face: &'f Face<'a>, w: usize, h: usize, base: Affine) -> Self {
        Self { face, w, h, cur: base, transforms: Vec::new(), path: Vec::new(), clips: Vec::new(), layers: vec![(vec![[0.0; 4]; w * h], CompositeMode::SourceOver)] }
    }

    /// 画完之后的结果：去预乘的 RGBA8
    pub fn finish(mut self) -> Vec<u8> {
        while self.layers.len() > 1 {
            self.pop_layer();
        }
        let mut out = Vec::with_capacity(self.w * self.h * 4);
        for p in &self.layers[0].0 {
            let a = p[3].clamp(0.0, 1.0);
            let un = |c: f32| if a > 0.0 { (c / a * 255.0).round().clamp(0.0, 255.0) as u8 } else { 0 };
            out.extend_from_slice(&[un(p[0]), un(p[1]), un(p[2]), (a * 255.0).round() as u8]);
        }
        out
    }

    fn mask(&self, contours: &Contours) -> Vec<f32> {
        let mut m = coverage(contours, self.w, self.h);
        if let Some(top) = self.clips.last() {
            for (v, c) in m.iter_mut().zip(top) {
                *v *= c;
            }
        }
        m
    }

    fn push_mask(&mut self, contours: Contours) {
        let m = self.mask(&contours);
        self.clips.push(m);
    }

    /// 某个像素中心（像素坐标）对应的颜色（预乘）；None = 这个像素不画（径向渐变无解的区域）
    /// 渐变的色标（按偏移排序、预乘）：每次 paint 取一次，别每个像素都解一遍颜色线
    fn stops(&self, paint: &Paint<'a>) -> Vec<(f32, Px)> {
        let coords = self.face.variation_coordinates();
        let it: Box<dyn Iterator<Item = ColorStop>> = match paint {
            Paint::Solid(_) => return Vec::new(),
            Paint::LinearGradient(g) => Box::new(g.stops(0, coords)),
            Paint::RadialGradient(g) => Box::new(g.stops(0, coords)),
            Paint::SweepGradient(g) => Box::new(g.stops(0, coords)),
        };
        let mut v: Vec<(f32, Px)> = it.map(|c| (c.stop_offset, premul(c.color))).collect();
        v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        v
    }

    fn shade(paint: &Paint<'a>, stops: &[(f32, Px)], inv: Option<&Affine>, px: f32, py: f32) -> Option<Px> {
        match paint {
            Paint::Solid(c) => Some(premul(*c)),
            Paint::LinearGradient(g) => {
                let (x, y) = inv?.apply(px, py);
                // p3 = p0 + (p1 - p0) 在「垂直于 p0p2」方向上的投影
                let (nx, ny) = (-(g.y2 - g.y0), g.x2 - g.x0);
                let (vx, vy) = (g.x1 - g.x0, g.y1 - g.y0);
                let nn = nx * nx + ny * ny;
                let (dx, dy) = if nn > 1e-9 { let k = (vx * nx + vy * ny) / nn; (nx * k, ny * k) } else { (vx, vy) };
                let dd = dx * dx + dy * dy;
                if dd < 1e-9 {
                    return None;
                }
                let t = ((x - g.x0) * dx + (y - g.y0) * dy) / dd;
                Some(color_at(stops, t, g.extend))
            }
            Paint::RadialGradient(g) => {
                let (x, y) = inv?.apply(px, py);
                let t = conical_t(x, y, g.x0, g.y0, g.r0, g.x1, g.y1, g.r1)?;
                Some(color_at(stops, t, g.extend))
            }
            Paint::SweepGradient(g) => {
                let (x, y) = inv?.apply(px, py);
                let mut ang = (y - g.center_y).atan2(x - g.center_x).to_degrees();
                if ang < 0.0 {
                    ang += 360.0;
                }
                let (s, e) = (g.start_angle * 180.0, g.end_angle * 180.0);
                if (e - s).abs() < 1e-6 {
                    return None;
                }
                Some(color_at(stops, (ang - s) / (e - s), g.extend))
            }
        }
    }
}

fn premul(c: RgbaColor) -> Px {
    let a = c.alpha as f32 / 255.0;
    [c.red as f32 / 255.0 * a, c.green as f32 / 255.0 * a, c.blue as f32 / 255.0 * a, a]
}

/// 双圆锥渐变：|p - c(t)| = r(t)，c(t) = c0 + t·(c1 - c0)，r(t) = r0 + t·(r1 - r0)，取 r(t) ≥ 0 的最大 t
#[allow(clippy::too_many_arguments)]
fn conical_t(x: f32, y: f32, x0: f32, y0: f32, r0: f32, x1: f32, y1: f32, r1: f32) -> Option<f32> {
    let (cdx, cdy, dr) = (x1 - x0, y1 - y0, r1 - r0);
    let (pdx, pdy) = (x - x0, y - y0);
    let a = cdx * cdx + cdy * cdy - dr * dr;
    let b = pdx * cdx + pdy * cdy + r0 * dr;
    let c = pdx * pdx + pdy * pdy - r0 * r0;
    let ok = |t: f32| r0 + t * dr >= 0.0;
    if a.abs() < 1e-6 {
        if b.abs() < 1e-9 {
            return None;
        }
        let t = c / (2.0 * b);
        return ok(t).then_some(t);
    }
    let disc = b * b - a * c;
    if disc < 0.0 {
        return None;
    }
    let sq = disc.sqrt();
    let (t1, t2) = ((b + sq) / a, (b - sq) / a);
    let (hi, lo) = if t1 > t2 { (t1, t2) } else { (t2, t1) };
    if ok(hi) {
        Some(hi)
    } else if ok(lo) {
        Some(lo)
    } else {
        None
    }
}

/// 颜色线取值（预乘插值）；Repeat / Reflect 相对第一个到最后一个色标的区间
fn color_at(stops: &[(f32, Px)], t: f32, ext: GradientExtend) -> Px {
    let Some(first) = stops.first() else { return [0.0; 4] };
    let last = stops[stops.len() - 1];
    let (lo, hi) = (first.0, last.0);
    let span = hi - lo;
    let t = if span <= 1e-6 {
        t
    } else {
        match ext {
            GradientExtend::Pad => t,
            GradientExtend::Repeat => lo + (t - lo).rem_euclid(span),
            GradientExtend::Reflect => {
                let u = (t - lo).rem_euclid(2.0 * span);
                lo + if u > span { 2.0 * span - u } else { u }
            }
        }
    };
    if t <= lo {
        return first.1;
    }
    if t >= hi {
        return last.1;
    }
    for w in stops.windows(2) {
        let (a, b) = (w[0], w[1]);
        if t >= a.0 && t <= b.0 {
            let k = if b.0 - a.0 > 1e-9 { (t - a.0) / (b.0 - a.0) } else { 1.0 };
            return [0, 1, 2, 3].map(|i| a.1[i] + (b.1[i] - a.1[i]) * k);
        }
    }
    last.1
}

/// 按合成模式把 src 叠到 dst（都是预乘）
fn composite(mode: CompositeMode, s: Px, d: Px) -> Px {
    use CompositeMode::*;
    let (sa, da) = (s[3], d[3]);
    let pd = |fs: f32, fd: f32| [0, 1, 2, 3].map(|i| s[i] * fs + d[i] * fd);
    match mode {
        Clear => [0.0; 4],
        Source => s,
        Destination => d,
        DestinationOver => pd(1.0 - da, 1.0),
        SourceIn => pd(da, 0.0),
        DestinationIn => pd(0.0, sa),
        SourceOut => pd(1.0 - da, 0.0),
        DestinationOut => pd(0.0, 1.0 - sa),
        SourceAtop => pd(da, 1.0 - sa),
        DestinationAtop => pd(1.0 - da, sa),
        Xor => pd(1.0 - da, 1.0 - sa),
        Plus => [0, 1, 2, 3].map(|i| (s[i] + d[i]).min(1.0)),
        Screen => [0, 1, 2, 3].map(|i| s[i] + d[i] - s[i] * d[i]),
        Multiply => {
            let c = |i: usize| s[i] * d[i] + s[i] * (1.0 - da) + d[i] * (1.0 - sa);
            [c(0), c(1), c(2), sa + da - sa * da]
        }
        // 其余混合模式：按 SourceOver（见文件头的取舍）
        _ => pd(1.0, 1.0 - sa),
    }
}

impl<'f, 'a> Painter<'a> for ColrPainter<'f, 'a> {
    fn outline_glyph(&mut self, glyph_id: GlyphId) {
        let mut b = Flatten::new(self.cur);
        self.face.outline_glyph(glyph_id, &mut b);
        self.path = b.finish();
    }

    fn paint(&mut self, paint: Paint<'a>) {
        let path = std::mem::take(&mut self.path);
        let m = self.mask(&path);
        self.path = path;
        let inv = self.cur.invert();
        let stops = self.stops(&paint);
        let w = self.w;
        let mut buf = std::mem::take(&mut self.layers.last_mut().expect("底层图层").0);
        for (i, &cov) in m.iter().enumerate() {
            if cov <= 0.0 {
                continue;
            }
            let (px, py) = ((i % w) as f32 + 0.5, (i / w) as f32 + 0.5);
            if let Some(c) = Self::shade(&paint, &stops, inv.as_ref(), px, py) {
                let s = c.map(|v| v * cov);
                let d = buf[i];
                buf[i] = [0, 1, 2, 3].map(|k| s[k] + d[k] * (1.0 - s[3]));
            }
        }
        self.layers.last_mut().expect("底层图层").0 = buf;
    }

    fn push_clip(&mut self) {
        let path = self.path.clone();
        self.push_mask(path);
    }

    fn push_clip_box(&mut self, b: ClipBox) {
        let m = self.cur;
        let poly = vec![m.apply(b.x_min, b.y_min), m.apply(b.x_max, b.y_min), m.apply(b.x_max, b.y_max), m.apply(b.x_min, b.y_max)];
        self.push_mask(vec![poly]);
    }

    fn pop_clip(&mut self) {
        self.clips.pop();
    }

    fn push_layer(&mut self, mode: CompositeMode) {
        self.layers.push((vec![[0.0; 4]; self.w * self.h], mode));
    }

    fn pop_layer(&mut self) {
        if self.layers.len() < 2 {
            return;
        }
        let (src, mode) = self.layers.pop().expect("上面判过");
        let dst = &mut self.layers.last_mut().expect("上面判过").0;
        for (d, s) in dst.iter_mut().zip(src) {
            *d = composite(mode, s, *d);
        }
    }

    fn push_transform(&mut self, t: Transform) {
        self.transforms.push(self.cur);
        self.cur = self.cur.then_inner(&Affine { a: t.a, b: t.b, c: t.c, d: t.d, e: t.e, f: t.f });
    }

    fn pop_transform(&mut self) {
        if let Some(t) = self.transforms.pop() {
            self.cur = t;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conical_两个同心圆退化成普通径向() {
        // 圆心不动、半径 0 → 10：到圆心距离 5 的点 t = 0.5
        let t = conical_t(5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 10.0).unwrap();
        assert!((t - 0.5).abs() < 1e-5, "{t}");
    }

    #[test]
    fn 颜色线_pad_repeat_reflect() {
        let s = [(0.0, [0.0, 0.0, 0.0, 1.0]), (1.0, [1.0, 1.0, 1.0, 1.0])];
        assert_eq!(color_at(&s, 1.5, GradientExtend::Pad)[0], 1.0);
        assert!((color_at(&s, 1.25, GradientExtend::Repeat)[0] - 0.25).abs() < 1e-5);
        assert!((color_at(&s, 1.25, GradientExtend::Reflect)[0] - 0.75).abs() < 1e-5);
    }

    #[test]
    fn 合成_source_over_与_dest_out() {
        let s = [0.5, 0.0, 0.0, 0.5];
        let d = [0.0, 0.0, 1.0, 1.0];
        assert_eq!(composite(CompositeMode::SourceOver, s, d), [0.5, 0.0, 0.5, 1.0]);
        assert_eq!(composite(CompositeMode::DestinationOut, s, d), [0.0, 0.0, 0.5, 0.5]);
    }
}
