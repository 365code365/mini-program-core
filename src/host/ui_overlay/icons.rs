//! 提示框里的图标：成功（不带圈的对勾）、失败（圈里一个感叹号）、加载中的转圈、自定义图片。
//!
//! 形状逐条取自 WeUI v2.4.4（微信 8.0 的设计语言）：
//! - 成功 `weui-icon-success-no-circle`、失败 `weui-icon-warn`：24×24 的 SVG 遮罩，
//!   颜色 rgba(255,255,255,0.9)。从前这里画的是**绿色圆圈加对勾**（旧版 WeUI 的 success 图标），
//!   与微信的白色对勾不是一个东西；失败图标从前画成一个空心圆环。
//! - 转圈 `weui-primary-loading`（`.setCircleLoading`）：外径 40、环宽 4、颜色 #EDEDED；
//!   左半环的不透明度从上 8% 处的 1 渐到下 95% 处的 0.3，右半环从 0 渐到 0.3，
//!   顶上一个右边圆头的小点当「头」；整体 `1s steps(60)` 顺时针转。
//!   从前是 12 个灰点，而且时钟传的是「这一帧刚开始的时刻」，转角恒为 0 —— 根本不转。
//!
//! 对勾与感叹号的覆盖率按 4×4 超采样算一次，按（种类, 像素边长）缓存；转圈每一步的
//! 转角都不同，逐帧按距离解析算（只有 40pt 见方，算一遍不到 0.1ms）。
use super::draw::blend;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum Glyph {
    Success,
    Warn,
}

thread_local! {
    static MASKS: RefCell<HashMap<(Glyph, u32), Arc<Vec<u8>>>> = RefCell::new(HashMap::new());
}

/// WeUI `weui-icon-success-no-circle` 的轮廓（24×24 视口）：
/// `M8.657 18.435L3 12.778l1.414-1.414 4.95 4.95L20.678 5l1.414 1.414-12.02 12.021a1 1 0 01-1.415 0z`
/// 末尾那段半径 1 的小圆弧（对勾底部的圆角）展平成三个点。
const SUCCESS: [(f32, f32); 10] = [
    (8.657, 18.435),
    (3.0, 12.778),
    (4.414, 11.364),
    (9.364, 16.314),
    (20.678, 5.0),
    (22.092, 6.414),
    (10.072, 18.435),
    (9.747, 18.652),
    (9.365, 18.728),
    (8.982, 18.652),
];

/// 点是否在多边形里（偶奇规则）
fn in_poly(poly: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// WeUI `weui-icon-warn`：半径 10 的实心圆，挖掉一根上宽下窄的竖条和一个圆点
fn in_warn(x: f32, y: f32) -> bool {
    let (dx, dy) = (x - 12.0, y - 12.0);
    if dx * dx + dy * dy > 100.0 {
        return false;
    }
    // 竖条：y 6.136 处宽 11.237..12.762，y 13.732 处宽 11.347..12.652
    if (6.136..=13.732).contains(&y) {
        let t = (y - 6.136) / (13.732 - 6.136);
        let (l, r) = (11.237 + 0.11 * t, 12.762 - 0.11 * t);
        if x >= l && x <= r {
            return false;
        }
    }
    let (px, py) = (x - 11.996, y - 16.221);
    px * px + py * py > 0.882 * 0.882
}

fn mask_of(glyph: Glyph, px: u32) -> Arc<Vec<u8>> {
    if let Some(m) = MASKS.with(|c| c.borrow().get(&(glyph, px)).cloned()) {
        return m;
    }
    const SS: u32 = 4;
    let unit = 24.0 / px as f32;
    let mut out = vec![0u8; (px * px) as usize];
    for y in 0..px {
        for x in 0..px {
            let mut hits = 0u32;
            for sy in 0..SS {
                for sx in 0..SS {
                    let ux = (x as f32 + (sx as f32 + 0.5) / SS as f32) * unit;
                    let uy = (y as f32 + (sy as f32 + 0.5) / SS as f32) * unit;
                    let inside = match glyph {
                        Glyph::Success => in_poly(&SUCCESS, ux, uy),
                        Glyph::Warn => in_warn(ux, uy),
                    };
                    hits += inside as u32;
                }
            }
            out[(y * px + x) as usize] = (hits * 255 / (SS * SS)) as u8;
        }
    }
    let m = Arc::new(out);
    MASKS.with(|c| c.borrow_mut().insert((glyph, px), m.clone()));
    m
}

/// 把对勾 / 感叹号画在 (x, y) 起、边长 `size` 的方块里（物理像素）
pub(super) fn draw_glyph(buf: &mut [u32], w: u32, h: u32, glyph: Glyph, (x, y, size): (f32, f32, f32), rgb: u32) {
    let px = size.round().max(1.0) as u32;
    let mask = mask_of(glyph, px);
    let (ox, oy) = (x.round() as i32, y.round() as i32);
    for my in 0..px as i32 {
        let ty = oy + my;
        if ty < 0 || ty >= h as i32 {
            continue;
        }
        for mx in 0..px as i32 {
            let tx = ox + mx;
            if tx < 0 || tx >= w as i32 {
                continue;
            }
            let a = mask[(my as u32 * px + mx as u32) as usize];
            if a > 0 {
                blend(&mut buf[(ty as u32 * w + tx as u32) as usize], rgb, a as f32 / 255.0);
            }
        }
    }
}

/// 转圈一圈 60 步（`animation: circleLoading 1s steps(60, end) infinite`）
pub const SPINNER_STEPS: u32 = 60;

/// 这一刻转到第几步（0..60）
pub fn spinner_step(secs: f32) -> u32 {
    ((secs.max(0.0).fract() * SPINNER_STEPS as f32) as u32).min(SPINNER_STEPS - 1)
}

/// 转圈：中心 (cx, cy)、外径 `size`、环宽 `ring`（物理像素），按 `step` 定转角
pub(super) fn draw_spinner(buf: &mut [u32], w: u32, h: u32, (cx, cy): (f32, f32), size: f32, ring: f32, step: u32, rgb: u32) {
    let r_out = size / 2.0;
    let r_in = r_out - ring;
    let angle = step as f32 / SPINNER_STEPS as f32 * std::f32::consts::TAU;
    let (sin, cos) = angle.sin_cos();
    // 不透明度遮罩：`linear-gradient(180deg, a 8%, b 95%)`，相对于元素自身（未旋转）的高度
    let grad = |t: f32, from: f32, to: f32| from + (to - from) * ((t - 0.08) / 0.87).clamp(0.0, 1.0);
    // 头上那个点：左半是方块、右半是半圆（`border-*-right-radius`），宽高都是环宽
    let dot_c = (0.0, -r_out + ring / 2.0);
    let half = ring / 2.0;
    let (x0, y0) = (((cx - r_out - 1.0).floor() as i32).max(0), ((cy - r_out - 1.0).floor() as i32).max(0));
    let (x1, y1) = (((cx + r_out + 1.0).ceil() as i32).min(w as i32), ((cy + r_out + 1.0).ceil() as i32).min(h as i32));
    for py in y0..y1 {
        for px in x0..x1 {
            let (dx, dy) = (px as f32 + 0.5 - cx, py as f32 + 0.5 - cy);
            // 转回元素自身的坐标系（CSS rotate 在 y 朝下的屏幕上是顺时针）
            let (ux, uy) = (cos * dx + sin * dy, -sin * dx + cos * dy);
            let d = (ux * ux + uy * uy).sqrt();
            let ring_cov = (r_out - d + 0.5).clamp(0.0, 1.0) * (d - r_in + 0.5).clamp(0.0, 1.0);
            let mut a = 0.0;
            if ring_cov > 0.0 {
                let t = (uy + r_out) / size;
                let left = (0.5 - ux).clamp(0.0, 1.0);
                a = ring_cov * (left * grad(t, 1.0, 0.3) + (1.0 - left) * grad(t, 0.0, 0.3));
            }
            // 点 = 左半方块 ∪ 圆；取两者有符号距离的较小值
            let (qx, qy) = (ux - dot_c.0, uy - dot_c.1);
            let sd_disc = (qx * qx + qy * qy).sqrt() - half;
            let (bx, by) = ((qx + half / 2.0).abs() - half / 2.0, qy.abs() - half);
            let sd_box = (bx.max(0.0).powi(2) + by.max(0.0).powi(2)).sqrt() + bx.max(by).min(0.0);
            let dot = (0.5 - sd_disc.min(sd_box)).clamp(0.0, 1.0);
            let a = a.max(dot);
            if a > 0.0 {
                blend(&mut buf[(py as u32 * w + px as u32) as usize], rgb, a);
            }
        }
    }
}

/// 自定义图标（`image`）：按 aspectFit 双线性缩放进 (x, y, size) 的方块，按 alpha 混合
pub(super) fn draw_image(buf: &mut [u32], w: u32, h: u32, src: &str, (x, y, size): (f32, f32, f32)) -> bool {
    let Some((data, iw, ih)) = crate::renderer::components::image_rgba(src) else { return false };
    if iw == 0 || ih == 0 || data.len() < (iw * ih * 4) as usize {
        return false;
    }
    let scale = (size / iw as f32).min(size / ih as f32);
    let (dw, dh) = (iw as f32 * scale, ih as f32 * scale);
    let (ox, oy) = (x + (size - dw) / 2.0, y + (size - dh) / 2.0);
    let texel = |sx: i32, sy: i32, c: usize| -> f32 {
        let (sx, sy) = (sx.clamp(0, iw as i32 - 1) as u32, sy.clamp(0, ih as i32 - 1) as u32);
        data[((sy * iw + sx) * 4) as usize + c] as f32
    };
    for py in (oy.floor() as i32).max(0)..((oy + dh).ceil() as i32).min(h as i32) {
        for px in (ox.floor() as i32).max(0)..((ox + dw).ceil() as i32).min(w as i32) {
            let (fx, fy) = ((px as f32 + 0.5 - ox) / scale - 0.5, (py as f32 + 0.5 - oy) / scale - 0.5);
            let (sx, sy) = (fx.floor() as i32, fy.floor() as i32);
            let (tx, ty) = (fx - sx as f32, fy - sy as f32);
            // 按预乘 alpha 插值再除回去：透明像素的 RGB（常是黑）不会在边缘晕出黑边
            let pm = |sx: i32, sy: i32, c: usize| {
                if c == 3 { texel(sx, sy, 3) } else { texel(sx, sy, c) * texel(sx, sy, 3) / 255.0 }
            };
            let sample = |c: usize| {
                let top = pm(sx, sy, c) * (1.0 - tx) + pm(sx + 1, sy, c) * tx;
                let bot = pm(sx, sy + 1, c) * (1.0 - tx) + pm(sx + 1, sy + 1, c) * tx;
                top * (1.0 - ty) + bot * ty
            };
            let a = sample(3) / 255.0;
            if a <= 0.0 {
                continue;
            }
            let un = |v: f32| ((v / a).round() as u32).min(255);
            let rgb = (un(sample(0)) << 16) | (un(sample(1)) << 8) | un(sample(2));
            blend(&mut buf[(py as u32 * w + px as u32) as usize], rgb, a);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 对勾在中线偏下_感叹号中间是空的() {
        let m = mask_of(Glyph::Success, 48);
        // 对勾底部拐点（9.36, 17.6）附近有墨水，正中心偏上没有
        assert!(m[(35 * 48 + 19) as usize] > 128);
        assert_eq!(m[(10 * 48 + 24) as usize], 0);
        let w = mask_of(Glyph::Warn, 48);
        // 圆的边上是实的，竖条中间被挖空
        assert!(w[(24 * 48 + 6) as usize] > 200);
        assert_eq!(w[(18 * 48 + 24) as usize], 0);
    }

    #[test]
    fn 转圈按步走_一秒一圈() {
        assert_eq!(spinner_step(0.0), 0);
        assert_eq!(spinner_step(0.5), 30);
        assert_eq!(spinner_step(1.0), 0);
        assert_eq!(spinner_step(1.999), 59);
    }

    #[test]
    fn 转圈转起来了_不同步的像素不一样() {
        let (w, h) = (100u32, 100u32);
        let draw = |step| {
            let mut buf = vec![0x4C4C4Cu32; (w * h) as usize];
            draw_spinner(&mut buf, w, h, (50.0, 50.0), 80.0, 8.0, step, 0xEDEDED);
            buf
        };
        let (a, b) = (draw(0), draw(15));
        assert_ne!(a, b, "转角变了画面必须变");
        // 第 0 步：头在正上方，最亮；正右方（右半环、靠上）几乎透明
        let top = a[(14 * w + 50) as usize] & 0xFF;
        let right = a[(50 * w + 86) as usize] & 0xFF;
        assert!(top > 0xE0, "top = {top:#x}");
        assert!(right < 0x90, "right = {right:#x}");
        // 转过 1/4 圈：头到了正右方
        let right_after = b[(50 * w + 86) as usize] & 0xFF;
        assert!(right_after > 0xE0, "right_after = {right_after:#x}");
    }
}
