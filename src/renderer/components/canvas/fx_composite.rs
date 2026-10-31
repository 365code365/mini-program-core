//! fx 蒙版路径的逐像素合成：「源 × 覆盖率 × globalAlpha」按 `globalCompositeOperation` 叠到画布上
//! （预乘空间里算 Porter-Duff，从 fx2d.rs 搬来，逐像素算法一字未改）。
//!
//! 为什么另外做了两件事（都不改任何像素的结果）：
//! - **少取样**：纯色处处同色；竖直的线性渐变（`createLinearGradient(0, y0, 0, y1)`，UI 面板最常见）
//!   一行同色，水平的一列同色 —— 渐变参数 t 里另一个方向的项是「有限数 × 0.0」，恒为 ±0，不改变 t。
//!   这几种按行 / 按列取一次样，不再每个像素算一遍渐变。
//! - **记住上一个像素**：覆盖率、源色、目标色都与上一个像素相同（面板内部压在纯色底上），
//!   结果必然相同，直接复用。合成是这三者（加上不变的 alpha 与模式）的纯函数。
//!
//! 实测（iPhone 16 Pro Max，萌宠小岛 1320×2868 的 UI 画布）：一次渐变面板 fill 3.5ms，
//! 几乎全在逐像素的浮点合成与渐变插值里。

use super::fx2d::{Composite, DeviceSource};
use crate::{Canvas, Color};

/// 源色随位置怎么变
#[derive(Clone, Copy, PartialEq)]
enum Varies {
    Never,
    /// 只随 y 变（一行同色）
    Rows,
    /// 只随 x 变（一列同色）
    Cols,
    Both,
}

fn varies(src: &DeviceSource) -> Varies {
    match src {
        DeviceSource::Solid(_) => Varies::Never,
        DeviceSource::Linear { p0, d, .. } if d.0 == 0.0 && p0.0.is_finite() => Varies::Rows,
        DeviceSource::Linear { p0, d, .. } if d.1 == 0.0 && p0.1.is_finite() => Varies::Cols,
        _ => Varies::Both,
    }
}

/// 整画布合成（`cov` 与画布同尺寸）
pub(super) fn composite(canvas: &mut Canvas, cov: &[u8], src: &DeviceSource, alpha: f32, op: Composite) {
    let (w, h) = (canvas.width() as usize, canvas.height() as usize);
    composite_region(canvas, cov, (0, 0, w, h), src, alpha, op);
}

/// 只合成 `[x0, x1) × [y0, y1)` 这一块；`cov` 按框宽排
pub(super) fn composite_region(canvas: &mut Canvas, cov: &[u8], (x0, y0, x1, _y1): (usize, usize, usize, usize), src: &DeviceSource, alpha: f32, op: Composite) {
    let w = canvas.width() as usize;
    let bw = x1 - x0;
    if bw == 0 {
        return;
    }
    let clears = op.clears_outside();
    let how = varies(src);
    let konst = (how == Varies::Never).then(|| src.sample(0.5, 0.5));
    let cols: Vec<[f32; 4]> = if how == Varies::Cols {
        (x0..x1).map(|xi| src.sample(xi as f32 + 0.5, y0 as f32 + 0.5)).collect()
    } else {
        Vec::new()
    };
    let (mut row_y, mut row_s) = (usize::MAX, [0.0f32; 4]);
    let mut memo: Option<(u8, [f32; 4], Color, Color)> = None;
    let pixels = canvas.pixels_mut();
    for (ci, &c) in cov.iter().enumerate() {
        if c == 0 && !clears {
            continue;
        }
        let (xi, yi) = (x0 + ci % bw, y0 + ci / bw);
        let s = if c == 0 {
            [0.0; 4]
        } else {
            match how {
                Varies::Never => konst.unwrap_or([0.0; 4]),
                Varies::Rows => {
                    if yi != row_y {
                        row_y = yi;
                        row_s = src.sample(x0 as f32 + 0.5, yi as f32 + 0.5);
                    }
                    row_s
                }
                Varies::Cols => cols[xi - x0],
                Varies::Both => src.sample(xi as f32 + 0.5, yi as f32 + 0.5),
            }
        };
        let px = &mut pixels[yi * w + xi];
        let dst = *px;
        if let Some((mc, ms, md, mo)) = memo {
            if mc == c && ms == s && md == dst {
                *px = mo;
                continue;
            }
        }
        let out = composite_px(c, s, dst, alpha, op);
        memo = Some((c, s, dst, out));
        *px = out;
    }
}

/// 一个像素：覆盖率 `c`、源色 `s`（RGBA 0~255，未预乘）、目标 `px`
fn composite_px(c: u8, s: [f32; 4], px: Color, alpha: f32, op: Composite) -> Color {
    let sa = s[3] / 255.0 * (c as f32 / 255.0) * alpha;
    let (sr, sg, sb) = (s[0] / 255.0 * sa, s[1] / 255.0 * sa, s[2] / 255.0 * sa);
    let da = px.a as f32 / 255.0;
    let (dr, dg, db) = (px.r as f32 / 255.0 * da, px.g as f32 / 255.0 * da, px.b as f32 / 255.0 * da);
    // Porter-Duff 的两个系数：结果 = 源 × fa + 目标 × fb
    let pd = |fa: f32, fb: f32| ([sr * fa + dr * fb, sg * fa + dg * fb, sb * fa + db * fb], sa * fa + da * fb);
    let blend = |f: fn(f32, f32) -> f32| {
        // 可分离混合：结果 = (1-da)·s + (1-sa)·d + sa·da·B(s, d)（未预乘的 s/d 代入 B）
        let un = |v: f32, a: f32| if a > 0.0 { v / a } else { 0.0 };
        let ch = |s: f32, d: f32| (1.0 - da) * s + (1.0 - sa) * d + sa * da * f(un(s, sa), un(d, da));
        ([ch(sr, dr), ch(sg, dg), ch(sb, db)], sa + da - sa * da)
    };
    let (rgb, a) = match op {
        Composite::SourceOver => pd(1.0, 1.0 - sa),
        Composite::SourceIn => pd(da, 0.0),
        Composite::SourceOut => pd(1.0 - da, 0.0),
        Composite::SourceAtop => pd(da, 1.0 - sa),
        Composite::DestinationOver => pd(1.0 - da, 1.0),
        Composite::DestinationIn => pd(0.0, sa),
        Composite::DestinationOut => pd(0.0, 1.0 - sa),
        Composite::DestinationAtop => pd(1.0 - da, sa),
        Composite::Lighter => ([(sr + dr).min(1.0), (sg + dg).min(1.0), (sb + db).min(1.0)], (sa + da).min(1.0)),
        Composite::Copy => pd(1.0, 0.0),
        Composite::Xor => pd(1.0 - da, 1.0 - sa),
        Composite::Multiply => blend(|s, d| s * d),
        Composite::Screen => blend(|s, d| s + d - s * d),
        Composite::Darken => blend(f32::min),
        Composite::Lighten => blend(f32::max),
        Composite::Difference => blend(|s, d| (s - d).abs()),
    };
    let a = a.clamp(0.0, 1.0);
    let unp = |v: f32| if a > 0.0 { (v / a * 255.0).round().clamp(0.0, 255.0) as u8 } else { 0 };
    Color::new(unp(rgb[0]), unp(rgb[1]), unp(rgb[2]), (a * 255.0).round() as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 按行 / 按列取样 + 记住上一个像素，与逐像素老算法逐字节相同（竖直、水平、斜向渐变，各种合成模式，带透明底）
    #[test]
    fn 少取样与复用和逐像素逐字节相同() {
        let stops = vec![(0.0, Color::new(255, 0, 0, 255)), (0.4, Color::new(0, 200, 30, 90)), (1.0, Color::new(0, 0, 255, 255))];
        let srcs = [
            DeviceSource::Linear { p0: (3.3, 2.0), d: (0.0, 37.5), len2: 37.5 * 37.5, stops: stops.clone() },
            DeviceSource::Linear { p0: (-4.0, 9.0), d: (51.0, 0.0), len2: 51.0 * 51.0, stops: stops.clone() },
            DeviceSource::Linear { p0: (0.0, 0.0), d: (30.0, 20.0), len2: 1300.0, stops: stops.clone() },
            DeviceSource::Solid([10.0, 200.0, 99.5, 180.0]),
        ];
        let ops = [Composite::SourceOver, Composite::SourceAtop, Composite::DestinationOut, Composite::Lighter, Composite::Multiply, Composite::Xor];
        let (w, h) = (48usize, 40usize);
        let base = |canvas: &mut Canvas| {
            for (i, p) in canvas.pixels_mut().iter_mut().enumerate() {
                let (x, y) = (i % w, i / w);
                // 左半纯色底（让复用命中）、右半逐像素变化、底下一条透明
                *p = if y > 34 { Color::TRANSPARENT } else if x < 24 { Color::new(40, 50, 60, 255) } else { Color::new((x * 5) as u8, (y * 6) as u8, 77, (128 + x) as u8) };
            }
        };
        let (x0, y0, x1, y1) = (2usize, 1usize, 46usize, 39usize);
        let cov: Vec<u8> = (0..(x1 - x0) * (y1 - y0)).map(|i| [255u8, 255, 255, 0, 128, 255, 7][i % 7]).collect();
        for src in &srcs {
            for &op in &ops {
                for alpha in [1.0f32, 0.6] {
                    let mut a = Canvas::new(w as u32, h as u32);
                    base(&mut a);
                    let mut b = Canvas::new(w as u32, h as u32);
                    base(&mut b);
                    composite_region(&mut a, &cov, (x0, y0, x1, y1), src, alpha, op);
                    // 参照：逐像素、每个都重新取样与合成
                    let bp = b.pixels_mut();
                    for (ci, &c) in cov.iter().enumerate() {
                        if c == 0 && !op.clears_outside() {
                            continue;
                        }
                        let (xi, yi) = (x0 + ci % (x1 - x0), y0 + ci / (x1 - x0));
                        let s = if c == 0 { [0.0; 4] } else { src.sample(xi as f32 + 0.5, yi as f32 + 0.5) };
                        bp[yi * w + xi] = composite_px(c, s, bp[yi * w + xi], alpha, op);
                    }
                    assert!(a.pixels() == b.pixels(), "{op:?} alpha={alpha}");
                }
            }
        }
    }
}
