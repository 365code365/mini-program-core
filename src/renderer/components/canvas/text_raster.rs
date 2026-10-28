//! 小游戏 fillText / strokeText 的「先离屏光栅、再按变换贴回」路径（text_font.rs 调）。
//!
//! 为什么要它（GT-F3 与 Chrome 对比时发现的三处差别，tools/game-compare.py 的 g-text 格 12 / 13 / 15）：
//! - **旋转 / 斜切**：字形光栅器只会横着画，从前只把起点按矩阵变换了，字本身还是水平的；
//! - **maxWidth**：浏览器是横向压缩字形（字高不变），从前是整体缩字号；
//! - **strokeText**：浏览器描轮廓，从前用描边色填充。
//! 做法：在设备像素分辨率上把这段字画进一张透明的覆盖率图（描边时由覆盖率图算出轮廓环），
//! 再当成一张图经 `draw_image_rgba`（image2d.rs，仿射反算 + 双线性）按当前矩阵贴回 —— 旋转、横向压缩都是这一步的矩阵。
//! 只有平移 + 等比缩放、不压缩、不描边的普通情况不走这里（直接画，像素与从前相同）。

use super::Canvas2DContext;
use crate::renderer::components::canvas::image2d::ImageRects;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle};

/// 离屏图四周留的边（逻辑像素）：描边与反走样不被裁掉
const PAD: f32 = 2.0;
/// 描边的超采样倍数
const SS: u32 = 4;

pub struct TextJob<'a> {
    pub tr: &'a TextRenderer,
    pub text: &'a str,
    pub size: f32,
    pub bold: bool,
    /// 文字左端、基线（用户坐标，已按 textAlign / textBaseline 调好）
    pub x: f32,
    pub y: f32,
    /// 自然宽度与要画成的宽度（maxWidth 压缩时后者更小）
    pub width: f32,
    pub draw_width: f32,
    /// Some(线宽) = 描边
    pub stroke: Option<f32>,
}

impl Canvas2DContext {
    /// 当前矩阵有旋转 / 斜切
    pub(super) fn has_rotation(&self) -> bool {
        self.transform[1] != 0.0 || self.transform[2] != 0.0
    }

    pub(super) fn draw_text_offscreen(&mut self, job: TextJob, color: Color) {
        let s = self.avg_scale().max(0.01);
        let (asc, desc) = job.tr.line_metrics(job.size);
        let (bw, bh) = (((job.width + PAD * 2.0) * s).ceil() as u32, ((asc + desc + PAD * 2.0) * s).ceil() as u32);
        if bw == 0 || bh == 0 || bw > 8192 || bh > 8192 {
            return;
        }
        // 描边在 4 倍分辨率上做再面积平均回来：1px 的线落在 3px 宽的笔画上，在原分辨率上膨胀 / 腐蚀只剩半透明的斑点
        let ss = if job.stroke.is_some() { SS } else { 1 };
        let (ow, oh) = (bw * ss, bh * ss);
        let mut off = Canvas::new(ow, oh);
        off.clear(Color::TRANSPARENT);
        let paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill);
        let k = s * ss as f32;
        job.tr.draw_text_weighted(&mut off, job.text, PAD * k, (PAD + asc) * k, job.size * k, 0.0, job.bold, &paint);
        let mut cov: Vec<u8> = off.pixels().iter().map(|p| p.a).collect();
        if let Some(lw) = job.stroke {
            cov = box_down(&outline(&cov, ow as usize, oh as usize, lw * k / 2.0), ow as usize, oh as usize, ss as usize);
        }
        let rgba: Vec<u8> = cov.iter().flat_map(|&a| [color.r, color.g, color.b, ((a as u32 * color.a as u32 + 127) / 255) as u8]).collect();
        let k = job.draw_width / job.width.max(1e-6);
        let r = ImageRects {
            sx: 0.0,
            sy: 0.0,
            sw: bw as f32,
            sh: bh as f32,
            dx: job.x - PAD * k,
            dy: job.y - asc - PAD,
            dw: bw as f32 / s * k,
            dh: bh as f32 / s,
        };
        self.draw_image_rgba(&rgba, bw, bh, r, true, None);
    }
}

/// `n × n` 面积平均（宽高都是 n 的整数倍）
fn box_down(src: &[u8], w: usize, h: usize, n: usize) -> Vec<u8> {
    let (dw, dh) = (w / n, h / n);
    let mut out = vec![0u8; dw * dh];
    for y in 0..dh {
        for x in 0..dw {
            let mut acc = 0u32;
            for j in 0..n {
                let row = &src[(y * n + j) * w + x * n..(y * n + j) * w + x * n + n];
                acc += row.iter().map(|&v| v as u32).sum::<u32>();
            }
            out[y * dw + x] = ((acc + (n * n / 2) as u32) / (n * n) as u32) as u8;
        }
    }
    out
}

/// 覆盖率图 → 宽 2r 的轮廓环（以字形边缘为中线）：外扩 r 的膨胀减去内缩 r 的腐蚀。
/// 邻点权重 `clamp(r − d + 1, 0, 1)`：r < 1 的细线（默认 lineWidth 1）也有一圈半透明的环，而不是整圈消失
fn outline(cov: &[u8], w: usize, h: usize, r: f32) -> Vec<u8> {
    let k = r.ceil().max(1.0) as i32;
    let mut taps = Vec::new();
    for dy in -k..=k {
        for dx in -k..=k {
            let d = ((dx * dx + dy * dy) as f32).sqrt();
            let wgt = (r - d + 1.0).clamp(0.0, 1.0);
            if wgt > 0.0 && (dx, dy) != (0, 0) {
                taps.push((dx, dy, wgt));
            }
        }
    }
    let at = |x: i32, y: i32| if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 { 0.0 } else { cov[y as usize * w + x as usize] as f32 / 255.0 };
    let mut out = vec![0u8; cov.len()];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let c = at(x, y);
            let (mut dil, mut ero_gap) = (c, 1.0 - c);
            for &(dx, dy, wgt) in &taps {
                let n = at(x + dx, y + dy);
                dil = dil.max(n * wgt);
                ero_gap = ero_gap.max((1.0 - n) * wgt);
            }
            let ring = (dil - (1.0 - ero_gap)).clamp(0.0, 1.0);
            out[y as usize * w + x as usize] = (ring * 255.0).round() as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn 轮廓环_实心块只剩边() {
        // 10×10 里中间 6×6 实心
        let (w, h) = (10usize, 10usize);
        let cov: Vec<u8> = (0..w * h).map(|i| if (2..8).contains(&(i % w)) && (2..8).contains(&(i / w)) { 255 } else { 0 }).collect();
        let o = super::outline(&cov, w, h, 0.5);
        assert_eq!(o[5 * w + 5], 0, "块中心没有描边");
        assert!(o[5 * w + 2] > 100, "块左边缘有描边：{}", o[5 * w + 2]);
        assert!(o[5 * w + 1] > 0, "边缘外侧一像素也有半透明：{}", o[5 * w + 1]);
        assert_eq!(o[5 * w + 0], 0, "离远了没有");
    }
}

