//! 预览层上的按钮与文字：右上角关闭、底部「当前 / 总数」、视频暂停时中间的播放键。
//! 几何（逻辑像素）绘制与命中共用一份。
use super::super::draw::{draw_text_direct, fill_rrect_aa};
use crate::text::TextRenderer;
use crate::Color;

/// 关闭键：直径与离右边、离状态栏的距离
const CLOSE_D: f32 = 32.0;
const CLOSE_RIGHT: f32 = 16.0;
const CLOSE_GAP: f32 = 8.0;
/// 命中区比画出来的圈大一圈（手指按不准）
const CLOSE_HIT: f32 = 22.0;
/// 播放键直径
const PLAY_D: f32 = 64.0;

/// 关闭键的圆心（逻辑像素）。`top` 是状态栏高
pub(super) fn close_center(vw: f32, top: f32) -> (f32, f32) {
    (vw - CLOSE_RIGHT - CLOSE_D / 2.0, top + CLOSE_GAP + CLOSE_D / 2.0)
}

pub(super) fn hits_close(vw: f32, top: f32, x: f32, y: f32) -> bool {
    let (cx, cy) = close_center(vw, top);
    (x - cx).abs() <= CLOSE_HIT && (y - cy).abs() <= CLOSE_HIT
}

/// 抗锯齿线段（端点圆头），`half` 是半线宽（设备像素）
fn line(buf: &mut [u32], w: u32, h: u32, (ax, ay): (f32, f32), (bx, by): (f32, f32), half: f32, rgb: u32, alpha: f32) {
    let (x0, x1) = ((ax.min(bx) - half - 1.0).max(0.0) as i32, ((ax.max(bx) + half + 1.0) as i32).min(w as i32 - 1));
    let (y0, y1) = ((ay.min(by) - half - 1.0).max(0.0) as i32, ((ay.max(by) + half + 1.0) as i32).min(h as i32 - 1));
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = (dx * dx + dy * dy).max(1e-6);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let t = (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0);
            let (qx, qy) = (ax + dx * t - px, ay + dy * t - py);
            let cov = (half + 0.5 - (qx * qx + qy * qy).sqrt()).clamp(0.0, 1.0);
            super::super::draw::blend(&mut buf[(y as u32 * w + x as u32) as usize], rgb, cov * alpha);
        }
    }
}

/// 朝右的实心三角（播放键），`(cx, cy)` 是外接圆心、`r` 是外接圆半径（设备像素）
fn triangle(buf: &mut [u32], w: u32, h: u32, (cx, cy): (f32, f32), r: f32, rgb: u32, alpha: f32) {
    let p = [(cx + r, cy), (cx - r * 0.5, cy - r * 0.866), (cx - r * 0.5, cy + r * 0.866)];
    let (x0, x1) = (((cx - r) as i32 - 1).max(0), ((cx + r) as i32 + 1).min(w as i32 - 1));
    let (y0, y1) = (((cy - r) as i32 - 1).max(0), ((cy + r) as i32 + 1).min(h as i32 - 1));
    for y in y0..=y1 {
        for x in x0..=x1 {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            // 到三条边（朝内为正）的有符号距离，取最小的当覆盖率
            let mut d = f32::MAX;
            for i in 0..3 {
                let (a, b) = (p[i], p[(i + 1) % 3]);
                let (ex, ey) = (b.0 - a.0, b.1 - a.1);
                let n = (ex * ex + ey * ey).sqrt().max(1e-6);
                d = d.min(((px - a.0) * ey - (py - a.1) * ex) / n);
            }
            let cov = (d + 0.5).clamp(0.0, 1.0);
            if cov > 0.0 {
                super::super::draw::blend(&mut buf[(y as u32 * w + x as u32) as usize], rgb, cov * alpha);
            }
        }
    }
}

/// 画按钮与文字。`alpha` 跟着黑底一起淡入淡出
#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    buf: &mut [u32],
    (w, h, sf): (u32, u32, f32),
    text: Option<&TextRenderer>,
    (top, bottom): (f32, f32),
    (index, total): (usize, usize),
    paused_video: bool,
    alpha: f32,
) {
    if alpha <= 0.01 {
        return;
    }
    let vw = w as f32 / sf;
    // 右上角关闭：半透明深色圆底 + 白色 ×（深色底压在浅色照片上也看得清）
    let (cx, cy) = close_center(vw, top);
    let r = CLOSE_D / 2.0 * sf;
    fill_rrect_aa(buf, w, h, (cx * sf - r, cy * sf - r, r * 2.0, r * 2.0), r, 0x000000, 0.45 * alpha, [true; 4]);
    let arm = 6.0 * sf;
    let c = (cx * sf, cy * sf);
    line(buf, w, h, (c.0 - arm, c.1 - arm), (c.0 + arm, c.1 + arm), 0.9 * sf, 0xFFFFFF, alpha);
    line(buf, w, h, (c.0 - arm, c.1 + arm), (c.0 + arm, c.1 - arm), 0.9 * sf, 0xFFFFFF, alpha);
    if paused_video {
        let r = PLAY_D / 2.0 * sf;
        let (px, py) = (w as f32 / 2.0, h as f32 / 2.0);
        fill_rrect_aa(buf, w, h, (px - r, py - r, r * 2.0, r * 2.0), r, 0x000000, 0.45 * alpha, [true; 4]);
        triangle(buf, w, h, (px + r * 0.08, py), r * 0.42, 0xFFFFFF, alpha);
    }
    if total > 1 {
        if let Some(tr) = text {
            let label = format!("{} / {}", index + 1, total);
            let size = 14.0 * sf;
            let x = ((w as f32 - tr.measure_text(&label, size)) / 2.0) as i32;
            let a = (alpha * 255.0) as u8;
            let y = h as f32 - (48.0 + bottom) * sf;
            draw_text_direct(buf, w, h, tr, &label, x, y as i32, size, Color::new(255, 255, 255, a));
        }
    }
}
