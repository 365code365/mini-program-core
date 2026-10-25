//! 键盘顶起画面时的 CPU 合成（`FrameLayers::content_shift > 0`）。
//!
//! 输出的第 `y`（≥ 导航栏行数）行取「没顶起时」第 `y + shift` 行的样子：页面、tabBar、fixed 层一起往上走，
//! 导航栏留在原处（之后由 `paint_top_band` 盖上）。顶出画面底边的那几行在键盘后面，填页面底色。
//! tabBar 区里画布没盖到的像素写 0 —— 与 `compose.wgsl` 的 `base` 同一套（GPU 那边是缓冲初值 0）。
//!
//! 为什么单独一个函数而不是在 `present_to_buffer` 里加偏移：那条路的每一行是 GPU / CPU 逐字节对齐的基线，
//! shift = 0 时必须一个字节都不变；顶起只在键盘弹着时出现，单独写清楚更好查
use crate::Canvas;

pub(super) struct Layers<'a> {
    pub canvas: &'a Canvas,
    pub fixed_canvas: Option<&'a Canvas>,
    pub tabbar_canvas: Option<&'a Canvas>,
    pub scroll_offset: i32,
    pub has_tabbar: bool,
    pub tabbar_physical_height: u32,
    pub fixed_rows: Option<(u32, u32)>,
    pub page_bg: u32,
    pub top_px: u32,
}

#[inline]
fn rgb(c: &crate::Color) -> u32 {
    ((c.r as u32) << 16) | ((c.g as u32) << 8) | (c.b as u32)
}

pub(super) fn present_shifted(buffer: &mut [u32], bw: u32, bh: u32, l: &Layers<'_>, rows: (u32, u32), shift: u32) {
    let content_h = bh - if l.has_tabbar { l.tabbar_physical_height } else { 0 };
    let top = l.top_px.min(content_h);
    let (pw, ph) = (l.canvas.width(), l.canvas.height() as i32);
    let copy_w = bw.min(pw) as usize;
    let pixels = l.canvas.pixels();
    for y in top.max(rows.0)..bh.min(rows.1) {
        let v = y + shift;
        let row = &mut buffer[(y * bw) as usize..((y + 1) * bw) as usize];
        if v >= bh {
            row.fill(l.page_bg);
            continue;
        }
        if v < content_h {
            let sy = (v - top) as i32 + l.scroll_offset;
            if sy >= 0 && sy < ph {
                let src = &pixels[sy as usize * pw as usize..sy as usize * pw as usize + copy_w];
                for (d, c) in row[..copy_w].iter_mut().zip(src) {
                    *d = rgb(c);
                }
                row[copy_w..].fill(l.page_bg);
            } else {
                row.fill(l.page_bg);
            }
        } else {
            row.fill(0);
            if let Some(t) = l.tabbar_canvas {
                let ty = v - content_h;
                if ty < l.tabbar_physical_height.min(t.height()) {
                    let tw = (bw as usize).min(t.width() as usize);
                    let src = &t.pixels()[ty as usize * t.width() as usize..ty as usize * t.width() as usize + tw];
                    for (d, c) in row[..tw].iter_mut().zip(src) {
                        *d = rgb(c);
                    }
                }
            }
        }
        // fixed 层：视口坐标第 fy 行（没顶起时落在输出的 fy + top 行）
        if let (Some(f), Some((r0, r1))) = (l.fixed_canvas, l.fixed_rows) {
            let fy = v - top;
            let y_end = bh.saturating_sub(top).min(f.height()).min(r1 + 1);
            if fy >= r0 && fy < y_end {
                let fw = (bw as usize).min(f.width() as usize);
                let src = &f.pixels()[fy as usize * f.width() as usize..fy as usize * f.width() as usize + fw];
                for (d, c) in row[..fw].iter_mut().zip(src) {
                    if c.a == 0 {
                        continue;
                    }
                    if c.a == 255 {
                        *d = rgb(c);
                    } else {
                        let (a, inv) = (c.a as u32, 255 - c.a as u32);
                        let p = *d;
                        let r = (c.r as u32 * a + ((p >> 16) & 0xFF) * inv) / 255;
                        let g = (c.g as u32 * a + ((p >> 8) & 0xFF) * inv) / 255;
                        let b = (c.b as u32 * a + (p & 0xFF) * inv) / 255;
                        *d = (r << 16) | (g << 8) | b;
                    }
                }
            }
        }
    }
}
