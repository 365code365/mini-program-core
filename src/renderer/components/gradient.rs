//! `background-image: linear-gradient(...)` 的绘制。
//!
//! 从 `base.rs` 拆出，并补了两条快路径。原实现对每个像素都要：算圆角覆盖率、
//! 把点投影到渐变轴、在色标里线性查找插值、再走一次带边界与裁剪判断的 `set_pixel`。
//! tea-app 的卡片上铺着整片渐变蒙层（`.tea-wash` / `.tea-fade` / `.hero-wash-fade`），
//! 一屏就有几十万像素走这条路，占掉滑动帧的一大块。
//!
//! 而实际用到的渐变几乎全是轴对齐的（`to bottom` / `to top` / `to right` / `to left`）：
//! - 垂直：同一行颜色相同 → 每行只算一次色，整行一次填充
//! - 水平：所有行颜色序列相同 → 只算一行，其余行整行复用
//!
//! 只有圆角波及的那几行退回逐像素（保证圆角抗锯齿不变），因此输出与逐像素实现一致。

use super::base::LinearGradientBg;
use crate::{Canvas, Color};

/// 绘制线性渐变背景（圆角边缘抗锯齿）。
pub fn draw_linear_gradient(
    canvas: &mut Canvas,
    grad: &LinearGradientBg,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radii: [f32; 4],
    opacity: f32,
) {
    if w <= 0.0 || h <= 0.0 || grad.stops.is_empty() {
        return;
    }
    let stops = &grad.stops;
    // 渐变方向单位向量：0deg 向上=(0,-1)，顺时针
    let a = grad.angle_deg.to_radians();
    let (dx, dy) = (a.sin(), -a.cos());
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let full = (w * dx).abs() + (h * dy).abs(); // 渐变线总长
    let [tl, tr, br, bl] = radii;

    let color_at = |t: f32| -> Color { sample_stops(stops, t) };

    // 圆角覆盖率（与图片圆角一致）
    let corner_cover = |px: f32, py: f32| -> f32 {
        let lx = px - x;
        let ly = py - y;
        let calc = |ccx: f32, ccy: f32, r: f32| -> f32 {
            if r <= 0.0 {
                return 1.0;
            }
            let d = ((lx - ccx) * (lx - ccx) + (ly - ccy) * (ly - ccy)).sqrt();
            (r + 0.5 - d).clamp(0.0, 1.0)
        };
        if lx < tl && ly < tl {
            calc(tl, tl, tl)
        } else if lx > w - tr && ly < tr {
            calc(w - tr, tr, tr)
        } else if lx > w - br && ly > h - br {
            calc(w - br, h - br, br)
        } else if lx < bl && ly > h - bl {
            calc(bl, h - bl, bl)
        } else {
            1.0
        }
    };

    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = (x + w).ceil() as i32;
    let y1 = (y + h).ceil() as i32;

    // 圆角只影响顶部/底部各一段行；这两段之外的行没有圆角衰减，可以整行处理
    let top_band = y + tl.max(tr);
    let bottom_band = y + h - bl.max(br);
    let plain_row = |py: i32| -> bool {
        let fy = py as f32 + 0.5;
        fy >= top_band && fy <= bottom_band
    };

    const AXIS_EPS: f32 = 1e-3;
    let vertical = dx.abs() < AXIS_EPS;
    let horizontal = dy.abs() < AXIS_EPS;

    // 水平渐变：一行的颜色序列对所有行都一样，只算一次
    let mut row_cache: Option<Vec<Color>> = None;
    if horizontal && x1 > x0 {
        let mut row = Vec::with_capacity((x1 - x0) as usize);
        for px in x0..x1 {
            let fx = px as f32 + 0.5;
            let proj = (fx - cx) * dx;
            let t = if full > 0.0 { proj / full + 0.5 } else { 0.5 };
            let mut c = color_at(t);
            c.a = (c.a as f32 * opacity) as u8;
            row.push(c);
        }
        row_cache = Some(row);
    }

    for py in y0..y1 {
        let fy = py as f32 + 0.5;
        if plain_row(py) {
            if vertical {
                let proj = (fy - cy) * dy;
                let t = if full > 0.0 { proj / full + 0.5 } else { 0.5 };
                let mut c = color_at(t);
                c.a = (c.a as f32 * opacity) as u8;
                canvas.fill_span(x0, x1, py, c);
                continue;
            }
            if let Some(row) = &row_cache {
                canvas.blend_pixels(x0, py, row, row.len());
                continue;
            }
        }
        for px in x0..x1 {
            let fx = px as f32 + 0.5;
            let cover = corner_cover(fx, fy);
            if cover <= 0.0 {
                continue;
            }
            let proj = (fx - cx) * dx + (fy - cy) * dy;
            let t = if full > 0.0 { proj / full + 0.5 } else { 0.5 };
            let mut c = color_at(t);
            c.a = (c.a as f32 * opacity * cover) as u8;
            canvas.set_pixel(px, py, c);
        }
    }
}

/// 在色标序列上按位置取色（含 alpha 插值）。
fn sample_stops(stops: &[(f32, Color)], t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    if t <= stops[0].0 {
        return stops[0].1;
    }
    let last = stops.len() - 1;
    if t >= stops[last].0 {
        return stops[last].1;
    }
    for i in 0..last {
        let (t0, c0) = stops[i];
        let (t1, c1) = stops[i + 1];
        if t >= t0 && t <= t1 {
            let r = if (t1 - t0).abs() < 1e-6 { 0.0 } else { (t - t0) / (t1 - t0) };
            return Color::new(
                (c0.r as f32 + (c1.r as f32 - c0.r as f32) * r) as u8,
                (c0.g as f32 + (c1.g as f32 - c0.g as f32) * r) as u8,
                (c0.b as f32 + (c1.b as f32 - c0.b as f32) * r) as u8,
                (c0.a as f32 + (c1.a as f32 - c0.a as f32) * r) as u8,
            );
        }
    }
    stops[last].1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grad(angle: f32) -> LinearGradientBg {
        LinearGradientBg {
            angle_deg: angle,
            stops: vec![
                (0.0, Color::new(249, 245, 238, 140)),
                (1.0, Color::new(249, 245, 238, 0)),
            ],
        }
    }

    /// 快路径必须与逐像素路径逐像素一致：用一个非轴对齐的角度（走慢路径）
    /// 和轴对齐角度分别画，再各自与「参考实现」比对。
    /// 这里的参考是「同一函数在 89.9deg（几乎水平但不触发快路径）」的连续性，
    /// 因此改为直接比对轴对齐快路径与手写逐像素结果。
    #[test]
    fn vertical_fast_path_matches_per_pixel() {
        let g = grad(180.0);
        let (w, h) = (40.0f32, 20.0f32);
        let mut fast = Canvas::new(40, 20);
        fast.clear(Color::new(0, 0, 0, 255));
        draw_linear_gradient(&mut fast, &g, 0.0, 0.0, w, h, [0.0; 4], 1.0);

        // 手写逐像素参考
        let mut slow = Canvas::new(40, 20);
        slow.clear(Color::new(0, 0, 0, 255));
        for py in 0..20i32 {
            let fy = py as f32 + 0.5;
            let t = (fy - h / 2.0) / h + 0.5;
            let mut c = sample_stops(&g.stops, t);
            c.a = c.a;
            for px in 0..40i32 {
                slow.set_pixel(px, py, c);
            }
        }
        assert_eq!(fast.pixels(), slow.pixels(), "垂直渐变快路径与逐像素结果应完全一致");
    }

    /// 水平渐变：行内颜色随 x 变化，各行相同
    #[test]
    fn horizontal_gradient_varies_along_x_only() {
        let g = grad(90.0);
        let mut c = Canvas::new(40, 20);
        c.clear(Color::new(0, 0, 0, 255));
        draw_linear_gradient(&mut c, &g, 0.0, 0.0, 40.0, 20.0, [0.0; 4], 1.0);
        let px = c.pixels();
        let at = |x: usize, y: usize| px[y * 40 + x];
        assert_eq!(at(3, 2), at(3, 17), "同一列在不同行应同色");
        assert_ne!(at(3, 2), at(30, 2), "同一行在不同列应不同色");
    }
}
