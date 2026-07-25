//! 变换合成：把离屏画布按 CSS `transform` 仿射贴回目标画布。
//!
//! 为什么要离屏：扫描线渲染器的绘制 API 都是「轴对齐矩形 + 绝对坐标」，
//! 旋转/缩放没法直接下发给每个子节点。做法是把带 transform 的子树先画到一张
//! 透明临时画布，再按矩阵逆变换逐像素双线性采样贴回 —— 这样 `transform` 对
//! 任意子树（含文字、图片、圆角、阴影）都成立，语义与浏览器一致。

use super::components::Transform;
use crate::{Canvas, Color};

/// transform 是否只是平移（可以直接用偏移绘制，不必离屏）
pub fn is_translate_only(t: &Transform) -> bool {
    (t.scale_x - 1.0).abs() < 1e-3
        && (t.scale_y - 1.0).abs() < 1e-3
        && t.rotate.abs() < 1e-3
        && t.skew_x.abs() < 1e-3
        && t.skew_y.abs() < 1e-3
}

/// transform 是否等价于「什么都不做」
pub fn is_identity(t: &Transform) -> bool {
    is_translate_only(t) && t.translate_x.abs() < 1e-3 && t.translate_y.abs() < 1e-3
}

/// 2x2 线性部分：rotate ∘ scale ∘ skew（与 CSS 常见书写顺序一致的近似）
fn linear_matrix(t: &Transform) -> (f32, f32, f32, f32) {
    let rad = t.rotate.to_radians();
    let (sin, cos) = (rad.sin(), rad.cos());
    let (sx, sy) = (t.scale_x, t.scale_y);
    let (kx, ky) = (t.skew_x.to_radians().tan(), t.skew_y.to_radians().tan());

    // skew 矩阵 [[1, kx], [ky, 1]]，再左乘 scale，再左乘 rotate
    let (a1, b1, c1, d1) = (sx, sx * kx, sy * ky, sy);
    (
        cos * a1 - sin * c1,
        cos * b1 - sin * d1,
        sin * a1 + cos * c1,
        sin * b1 + cos * d1,
    )
}

/// 计算带 transform 后子树在目标画布上的包围盒（用于确定采样范围）
fn transformed_bounds(
    src_w: f32,
    src_h: f32,
    src_pos: (f32, f32),
    center: (f32, f32),
    t: &Transform,
) -> (f32, f32, f32, f32) {
    let (a, b, c, d) = linear_matrix(t);
    let corners = [
        (src_pos.0, src_pos.1),
        (src_pos.0 + src_w, src_pos.1),
        (src_pos.0, src_pos.1 + src_h),
        (src_pos.0 + src_w, src_pos.1 + src_h),
    ];
    let (mut min_x, mut min_y) = (f32::MAX, f32::MAX);
    let (mut max_x, mut max_y) = (f32::MIN, f32::MIN);
    for (px, py) in corners {
        let (dx, dy) = (px - center.0, py - center.1);
        let tx = center.0 + a * dx + b * dy + t.translate_x;
        let ty = center.1 + c * dx + d * dy + t.translate_y;
        min_x = min_x.min(tx);
        min_y = min_y.min(ty);
        max_x = max_x.max(tx);
        max_y = max_y.max(ty);
    }
    (min_x, min_y, max_x, max_y)
}

/// 把 `src` 按 transform 合成到 `dst`。
///
/// - `src_pos`：`src` 左上角在目标画布中的（未变换）位置
/// - `center`：变换原点（CSS `transform-origin` 默认元素中心）
/// - `opacity`：整体透明度（0..1），与像素自身 alpha 相乘
pub fn blit_transformed(
    dst: &mut Canvas,
    src: &Canvas,
    src_pos: (f32, f32),
    center: (f32, f32),
    t: &Transform,
    opacity: f32,
) {
    let (a, b, c, d) = linear_matrix(t);
    let det = a * d - b * c;
    if det.abs() < 1e-6 {
        return; // 退化矩阵（scale 0）：整体不可见
    }
    // 逆矩阵
    let (ia, ib, ic, id) = (d / det, -b / det, -c / det, a / det);

    let (src_w, src_h) = (src.width() as f32, src.height() as f32);
    let (min_x, min_y, max_x, max_y) = transformed_bounds(src_w, src_h, src_pos, center, t);

    let x0 = min_x.floor().max(0.0) as i32;
    let y0 = min_y.floor().max(0.0) as i32;
    let x1 = (max_x.ceil() as i32).min(dst.width() as i32);
    let y1 = (max_y.ceil() as i32).min(dst.height() as i32);
    if x1 <= x0 || y1 <= y0 {
        return;
    }

    let src_pixels = src.pixels().to_vec();
    let sw = src.width() as i32;
    let sh = src.height() as i32;

    for py in y0..y1 {
        for px in x0..x1 {
            // 目标像素中心 → 逆变换 → 源画布坐标
            let dx = px as f32 + 0.5 - center.0 - t.translate_x;
            let dy = py as f32 + 0.5 - center.1 - t.translate_y;
            let ux = center.0 + ia * dx + ib * dy - src_pos.0;
            let uy = center.1 + ic * dx + id * dy - src_pos.1;
            if ux < -0.5 || uy < -0.5 || ux > src_w + 0.5 || uy > src_h + 0.5 {
                continue;
            }
            if let Some(color) = sample_bilinear(&src_pixels, sw, sh, ux - 0.5, uy - 0.5) {
                let alpha = (color.a as f32 * opacity).round().clamp(0.0, 255.0) as u8;
                if alpha > 0 {
                    dst.set_pixel(px, py, Color::new(color.r, color.g, color.b, alpha));
                }
            }
        }
    }
}

/// 双线性采样（按 alpha 加权，避免透明边缘把黑色带进来）
fn sample_bilinear(pixels: &[Color], w: i32, h: i32, fx: f32, fy: f32) -> Option<Color> {
    if w <= 0 || h <= 0 {
        return None;
    }
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;

    let at = |x: i32, y: i32| -> Color {
        let x = x.clamp(0, w - 1);
        let y = y.clamp(0, h - 1);
        pixels[(y * w + x) as usize]
    };
    let p00 = at(x0, y0);
    let p10 = at(x0 + 1, y0);
    let p01 = at(x0, y0 + 1);
    let p11 = at(x0 + 1, y0 + 1);

    let w00 = (1.0 - tx) * (1.0 - ty);
    let w10 = tx * (1.0 - ty);
    let w01 = (1.0 - tx) * ty;
    let w11 = tx * ty;

    let a = p00.a as f32 * w00 + p10.a as f32 * w10 + p01.a as f32 * w01 + p11.a as f32 * w11;
    if a < 0.5 {
        return None;
    }
    // 颜色按 alpha 加权平均：透明像素不该把它的 RGB 混进来
    let wa = |p: &Color, wt: f32| p.a as f32 * wt;
    let (s00, s10, s01, s11) = (wa(&p00, w00), wa(&p10, w10), wa(&p01, w01), wa(&p11, w11));
    let sum = s00 + s10 + s01 + s11;
    if sum < 1e-3 {
        return None;
    }
    let r = (p00.r as f32 * s00 + p10.r as f32 * s10 + p01.r as f32 * s01 + p11.r as f32 * s11) / sum;
    let g = (p00.g as f32 * s00 + p10.g as f32 * s10 + p01.g as f32 * s01 + p11.g as f32 * s11) / sum;
    let bl = (p00.b as f32 * s00 + p10.b as f32 * s10 + p01.b as f32 * s01 + p11.b as f32 * s11) / sum;
    Some(Color::new(
        r.round().clamp(0.0, 255.0) as u8,
        g.round().clamp(0.0, 255.0) as u8,
        bl.round().clamp(0.0, 255.0) as u8,
        a.round().clamp(0.0, 255.0) as u8,
    ))
}

/// 离屏画布需要的边距（px）：缩放放大与旋转都会让内容溢出原盒子。
pub fn transform_padding(w: f32, h: f32, t: &Transform) -> f32 {
    let long = w.max(h);
    let scale_pad = (t.scale_x.abs().max(t.scale_y.abs()) - 1.0).max(0.0) * long / 2.0;
    let rotate_pad = if t.rotate.abs() > 1e-3 || t.skew_x.abs() > 1e-3 || t.skew_y.abs() > 1e-3 {
        ((w * w + h * h).sqrt() - w.min(h)) / 2.0
    } else {
        0.0
    };
    (scale_pad.max(rotate_pad) + 2.0).ceil()
}
