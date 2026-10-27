//! `drawImage` 的 HTML 语义（小游戏的立即模式用，game-doc/02 §5.2、03 §2.2）：源是已经解码好的 RGBA（不是路径）。
//!
//! 与 `draw2d::draw_image`（小程序 `<canvas>` 旧接口那条：按路径每次解码、只认平移 + 缩放）不同：
//! - 3 / 5 / 9 参数：源矩形 (sx, sy, sw, sh) → 目标矩形 (dx, dy, dw, dh)，负宽高先规范化（HTML：不翻转）；
//! - 完整仿射变换（旋转、斜切）：按设备像素中心反算回源图坐标，像素中心落在目标四边形内才画；
//! - `imageSmoothingEnabled`：开 = 双线性（预乘 alpha 里插值，透明边不发黑），关 = 最近邻（像素风游戏要这个）；
//!   采样夹在源矩形以内（Chrome 的做法：图集里相邻的精灵不会渗进来）；
//! - `globalAlpha` 乘在源 alpha 上；合成模式 / 阴影不是默认值时走 fx 蒙版那条路（`draw_image_fx`，最近邻）。
//!
//! 已知与浏览器的差别（记在 game-doc/验收/G4.md）：旋转后的边缘没有抗锯齿；大幅缩小（< 0.5）不做 mip，
//! 双线性只看 4 个邻域（浏览器会多级降采样）。
//!
//! 快路：变换只有平移、1:1、目标落在整像素上 —— 精灵图最常见的情况 —— 整行拷贝 / 混合，不逐像素反算。
//! 与通用路径逐字节相同（`image2d_tests::快路与逐像素反算逐字节相同`）。

use super::{fx2d, Canvas2DContext};
use crate::{Canvas, Color};
use std::sync::Arc;

/// 一次 drawImage 的源与目标（都已规范化成正宽高）
#[derive(Clone, Copy, Debug)]
pub struct ImageRects {
    pub sx: f32,
    pub sy: f32,
    pub sw: f32,
    pub sh: f32,
    pub dx: f32,
    pub dy: f32,
    pub dw: f32,
    pub dh: f32,
}

impl ImageRects {
    /// 负宽高规范化（HTML：矩形取绝对值、起点移到较小的一端，不翻转）；源矩形与图片求交并按比例缩目标矩形。
    /// 源或目标退化成 0 面积返回 None（什么也不画）
    pub fn normalize(self, iw: u32, ih: u32) -> Option<Self> {
        let norm = |p: f32, s: f32| if s < 0.0 { (p + s, -s) } else { (p, s) };
        let (mut sx, mut sw) = norm(self.sx, self.sw);
        let (mut sy, mut sh) = norm(self.sy, self.sh);
        let (mut dx, mut dw) = norm(self.dx, self.dw);
        let (mut dy, mut dh) = norm(self.dy, self.dh);
        if sw <= 0.0 || sh <= 0.0 || dw <= 0.0 || dh <= 0.0 {
            return None;
        }
        // 源矩形超出图片的部分裁掉，目标矩形按同比例收缩（规范 drawImage 第 5 步）
        let (kx, ky) = (dw / sw, dh / sh);
        let x0 = sx.max(0.0);
        let y0 = sy.max(0.0);
        let x1 = (sx + sw).min(iw as f32);
        let y1 = (sy + sh).min(ih as f32);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        dx += (x0 - sx) * kx;
        dy += (y0 - sy) * ky;
        dw = (x1 - x0) * kx;
        dh = (y1 - y0) * ky;
        (sx, sy, sw, sh) = (x0, y0, x1 - x0, y1 - y0);
        Some(Self { sx, sy, sw, sh, dx, dy, dw, dh })
    }
}

/// 每行的「非透明段」表：不透明段整段拷贝、半透明段逐像素混合、全透明的不碰。
/// 精灵图（圆形 / 角色）大部分是整片不透明或全透明，一加 PLY110 上 500 个 32×32 精灵逐像素混合占游戏线程 1/3；
/// 按段拷贝与逐像素 `put` 逐字节相同（`image2d_tests::分段拷贝与逐像素逐字节相同`）。图解码之后算一次，随图缓存
pub struct RowSpans {
    /// 第 y 行的段在 `spans[offs[y]..offs[y + 1]]`
    offs: Vec<u32>,
    spans: Vec<(u32, u32, bool)>,
}

impl RowSpans {
    pub fn build(rgba: &[u8], w: u32, h: u32) -> Self {
        let mut offs = Vec::with_capacity(h as usize + 1);
        let mut spans = Vec::new();
        for y in 0..h as usize {
            offs.push(spans.len() as u32);
            let row = &rgba[y * w as usize * 4..(y + 1) * w as usize * 4];
            let mut x = 0usize;
            while x < w as usize {
                let a = row[x * 4 + 3];
                if a == 0 {
                    x += 1;
                    continue;
                }
                let opaque = a == 255;
                let start = x;
                while x < w as usize && row[x * 4 + 3] != 0 && (row[x * 4 + 3] == 255) == opaque {
                    x += 1;
                }
                spans.push((start as u32, (x - start) as u32, opaque));
            }
        }
        offs.push(spans.len() as u32);
        Self { offs, spans }
    }

    fn row(&self, y: usize) -> &[(u32, u32, bool)] {
        &self.spans[self.offs[y] as usize..self.offs[y + 1] as usize]
    }
}

fn invert(m: [f32; 6]) -> Option<[f32; 6]> {
    let det = m[0] * m[3] - m[1] * m[2];
    if det.abs() < 1e-12 || !det.is_finite() {
        return None;
    }
    let id = 1.0 / det;
    Some([m[3] * id, -m[1] * id, -m[2] * id, m[0] * id, (m[2] * m[5] - m[3] * m[4]) * id, (m[1] * m[4] - m[0] * m[5]) * id])
}

impl Canvas2DContext {
    /// `drawImage`：`rgba` 是 `iw × ih` 的非预乘 RGBA8，`r` 已规范化（[`ImageRects::normalize`]），`smooth` = imageSmoothingEnabled
    /// `spans`：这张图的分段表（有的话 1:1 快路按段拷贝；画布当源时没有）
    pub fn draw_image_rgba(&mut self, rgba: &[u8], iw: u32, ih: u32, r: ImageRects, smooth: bool, spans: Option<&RowSpans>) {
        if iw == 0 || ih == 0 || rgba.len() < (iw as usize * ih as usize * 4) || self.global_alpha <= 0.0 {
            return;
        }
        // 只看合成模式与阴影：fillStyle 是渐变 / 图案跟 drawImage 无关（从前按 trivial(false) 判，
        // 设过图案填充之后每次 drawImage 都走整画布蒙版，一次 0.26ms）
        if !self.fx.trivial_for_image() {
            return self.draw_image_fx(rgba, iw, ih, r);
        }
        let m = self.transform;
        let alpha = self.global_alpha.clamp(0.0, 1.0);
        let Ok(mut canvas) = self.canvas.lock() else { return };
        if let Some((ox, oy)) = unit_translation(m, r) {
            match spans.filter(|_| alpha >= 1.0) {
                Some(sp) => blit_spans(&mut canvas, rgba, iw, r, ox, oy, sp),
                None => blit_1to1(&mut canvas, rgba, iw, r, ox, oy, alpha),
            }
        } else {
            sample_affine(&mut canvas, rgba, iw, m, r, smooth, alpha);
        }
    }

    /// 合成模式 / 阴影不是默认值：把图当成「不重复的图案」填目标矩形，走 fx 蒙版（`paint_with_fx`）。
    /// 要拷一份源像素给图案（这条路本来就要整画布大小的蒙版，不在热路径上）。
    /// 变换临时换成「源图像素坐标 → 设备」，让图案的反算落到源图上
    fn draw_image_fx(&mut self, rgba: &[u8], iw: u32, ih: u32, r: ImageRects) {
        let saved = (self.transform, self.fx.fill.clone());
        let (kx, ky) = (r.dw / r.sw, r.dh / r.sh);
        self.mul([kx, 0.0, 0.0, ky, r.dx - r.sx * kx, r.dy - r.sy * ky]);
        self.fx.fill = Some(fx2d::Source::Pattern { rgba: Arc::new(rgba.to_vec()), w: iw, h: ih, repeat_x: false, repeat_y: false });
        let path = self.rect_path(r.sx, r.sy, r.sw, r.sh);
        self.paint_with_fx(false, |mask| {
            let paint = crate::Paint::new().with_color(Color::WHITE).with_anti_alias(true).with_style(crate::PaintStyle::Fill);
            mask.draw_path(&path, &paint);
        });
        (self.transform, self.fx.fill) = saved;
    }
}

/// 变换只有平移、1:1、源与目标同尺寸且都在整像素上：返回目标在设备上的整数左上角
fn unit_translation(m: [f32; 6], r: ImageRects) -> Option<(i32, i32)> {
    let int = |v: f32| v.is_finite() && (v - v.round()).abs() < 1e-4;
    if m[0] != 1.0 || m[1] != 0.0 || m[2] != 0.0 || m[3] != 1.0 {
        return None;
    }
    if !(int(r.sx) && int(r.sy) && int(r.sw) && int(r.sh) && (r.dw - r.sw).abs() < 1e-4 && (r.dh - r.sh).abs() < 1e-4) {
        return None;
    }
    let (x, y) = (r.dx + m[4], r.dy + m[5]);
    (int(x) && int(y)).then(|| (x.round() as i32, y.round() as i32))
}

/// 写一个像素：源非预乘 RGBA、乘上 globalAlpha 之后按 source-over 混合（与 `set_pixel` 同一套规则）
#[inline]
fn put(dst: &mut Color, s: [u8; 4], alpha: f32) {
    let a = if alpha >= 1.0 { s[3] } else { (s[3] as f32 * alpha).round() as u8 };
    if a == 255 {
        *dst = Color::new(s[0], s[1], s[2], 255);
    } else if a > 0 {
        *dst = Color::new(s[0], s[1], s[2], a).blend(dst);
    }
}

fn blit_1to1(canvas: &mut Canvas, rgba: &[u8], iw: u32, r: ImageRects, ox: i32, oy: i32, alpha: f32) {
    let (bx0, bx1, by0, by1) = canvas.draw_bounds();
    let (sx, sy, w, h) = (r.sx.round() as i32, r.sy.round() as i32, r.sw.round() as i32, r.sh.round() as i32);
    let (x0, x1) = (ox.max(bx0), (ox + w).min(bx1));
    let (y0, y1) = (oy.max(by0), (oy + h).min(by1));
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let cw = canvas.width() as usize;
    let px = canvas.pixels_mut();
    for y in y0..y1 {
        let src = ((sy + y - oy) as usize * iw as usize + (sx + x0 - ox) as usize) * 4;
        let row = &rgba[src..src + (x1 - x0) as usize * 4];
        let dst = &mut px[y as usize * cw + x0 as usize..y as usize * cw + x1 as usize];
        for (d, s) in dst.iter_mut().zip(row.chunks_exact(4)) {
            put(d, [s[0], s[1], s[2], s[3]], alpha);
        }
    }
}

/// 1:1 快路的分段版（globalAlpha = 1）：不透明段整段拷贝，半透明段逐像素，透明段跳过
fn blit_spans(canvas: &mut Canvas, rgba: &[u8], iw: u32, r: ImageRects, ox: i32, oy: i32, sp: &RowSpans) {
    let (bx0, bx1, by0, by1) = canvas.draw_bounds();
    let (sx, sy, w, h) = (r.sx.round() as i32, r.sy.round() as i32, r.sw.round() as i32, r.sh.round() as i32);
    let (x0, x1) = (ox.max(bx0), (ox + w).min(bx1));
    let (y0, y1) = (oy.max(by0), (oy + h).min(by1));
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    // 目标 [x0, x1) 对应源列 [u0, u1)
    let (u0, u1) = ((sx + x0 - ox) as u32, (sx + x1 - ox) as u32);
    let cw = canvas.width() as usize;
    let px = canvas.pixels_mut();
    for y in y0..y1 {
        let v = (sy + y - oy) as usize;
        let src_row = &rgba[v * iw as usize * 4..(v + 1) * iw as usize * 4];
        let dst_row = &mut px[y as usize * cw..(y as usize + 1) * cw];
        for &(s0, len, opaque) in sp.row(v) {
            let (a, b) = (s0.max(u0), (s0 + len).min(u1));
            if b <= a {
                continue;
            }
            let d0 = (a as i32 - u0 as i32 + x0) as usize;
            let dst = &mut dst_row[d0..d0 + (b - a) as usize];
            let src = &src_row[a as usize * 4..b as usize * 4];
            if opaque {
                crate::canvas::color_bytes_mut(dst).copy_from_slice(src);
            } else {
                for (d, s) in dst.iter_mut().zip(src.chunks_exact(4)) {
                    put(d, [s[0], s[1], s[2], s[3]], 1.0);
                }
            }
        }
    }
}

/// 通用路径：设备像素中心反算到用户坐标，再到源图坐标
fn sample_affine(canvas: &mut Canvas, rgba: &[u8], iw: u32, m: [f32; 6], r: ImageRects, smooth: bool, alpha: f32) {
    let Some(inv) = invert(m) else { return };
    let tp = |x: f32, y: f32| (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]);
    let corners = [tp(r.dx, r.dy), tp(r.dx + r.dw, r.dy), tp(r.dx, r.dy + r.dh), tp(r.dx + r.dw, r.dy + r.dh)];
    let (mut lx, mut ly, mut hx, mut hy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for (x, y) in corners {
        (lx, ly, hx, hy) = (lx.min(x), ly.min(y), hx.max(x), hy.max(y));
    }
    if !(lx.is_finite() && ly.is_finite() && hx.is_finite() && hy.is_finite()) {
        return;
    }
    let (bx0, bx1, by0, by1) = canvas.draw_bounds();
    let x0 = (lx.floor() as i32).max(bx0);
    let x1 = (hx.ceil() as i32).min(bx1);
    let y0 = (ly.floor() as i32).max(by0);
    let y1 = (hy.ceil() as i32).min(by1);
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    // 用户坐标 → 源图坐标：u = sx + (ux - dx) * ku
    let (ku, kv) = (r.sw / r.dw, r.sh / r.dh);
    // 源矩形覆盖的像素范围（采样夹在这里面）
    let (su0, sv0) = (r.sx.floor() as i32, r.sy.floor() as i32);
    let (su1, sv1) = ((r.sx + r.sw).ceil() as i32 - 1, (r.sy + r.sh).ceil() as i32 - 1);
    let texel = |x: i32, y: i32| -> [f32; 4] {
        let i = (y.clamp(sv0, sv1) as usize * iw as usize + x.clamp(su0, su1) as usize) * 4;
        let a = rgba[i + 3] as f32;
        // 预乘之后再插值：透明像素的 RGB 不参与（否则精灵的透明边会混出黑边）
        [rgba[i] as f32 * a / 255.0, rgba[i + 1] as f32 * a / 255.0, rgba[i + 2] as f32 * a / 255.0, a]
    };
    let cw = canvas.width() as usize;
    let px = canvas.pixels_mut();
    for y in y0..y1 {
        let cy = y as f32 + 0.5;
        for x in x0..x1 {
            let cx = x as f32 + 0.5;
            let ux = inv[0] * cx + inv[2] * cy + inv[4];
            let uy = inv[1] * cx + inv[3] * cy + inv[5];
            let (tx, ty) = (ux - r.dx, uy - r.dy);
            if tx < 0.0 || ty < 0.0 || tx >= r.dw || ty >= r.dh {
                continue;
            }
            let (u, v) = (r.sx + tx * ku, r.sy + ty * kv);
            let s = if smooth {
                let (fu, fv) = (u - 0.5, v - 0.5);
                let (iu, iv) = (fu.floor() as i32, fv.floor() as i32);
                let (wu, wv) = (fu - iu as f32, fv - iv as f32);
                let (a, b, c, d) = (texel(iu, iv), texel(iu + 1, iv), texel(iu, iv + 1), texel(iu + 1, iv + 1));
                let mut o = [0f32; 4];
                for k in 0..4 {
                    o[k] = (a[k] * (1.0 - wu) + b[k] * wu) * (1.0 - wv) + (c[k] * (1.0 - wu) + d[k] * wu) * wv;
                }
                o
            } else {
                texel(u.floor() as i32, v.floor() as i32)
            };
            if s[3] <= 0.0 {
                continue;
            }
            // 去预乘回直通 alpha
            let k = 255.0 / s[3];
            let c = [(s[0] * k).round().clamp(0.0, 255.0) as u8, (s[1] * k).round().clamp(0.0, 255.0) as u8, (s[2] * k).round().clamp(0.0, 255.0) as u8, s[3].round().clamp(0.0, 255.0) as u8];
            put(&mut px[y as usize * cw + x as usize], c, alpha);
        }
    }
}

#[cfg(test)]
mod image2d_tests {
    use super::*;

    fn img(w: u32, h: u32) -> Arc<Vec<u8>> {
        Arc::new((0..w * h).flat_map(|i| [(i * 37 % 256) as u8, (i * 11 % 256) as u8, (i * 5 % 256) as u8, [255, 128, 0, 200][(i % 4) as usize]]).collect())
    }

    #[test]
    fn 快路与逐像素反算逐字节相同() {
        let (iw, ih) = (24u32, 18u32);
        let src = img(iw, ih);
        for (case, &(sx, sy, w, h, dx, dy, alpha, smooth)) in [
            (0.0, 0.0, 24.0, 18.0, 3.0, 4.0, 1.0, true),
            (5.0, 2.0, 10.0, 9.0, -4.0, 30.0, 0.5, false),
            (0.0, 0.0, 24.0, 18.0, 50.0, 40.0, 1.0, true),
        ].iter().enumerate() {
            let r = ImageRects { sx, sy, sw: w, sh: h, dx, dy, dw: w, dh: h }.normalize(iw, ih).unwrap();
            let mut a = Canvas::new(64, 48);
            let mut b = Canvas::new(64, 48);
            a.clear(Color::new(10, 20, 30, 255));
            b.clear(Color::new(10, 20, 30, 255));
            let (ox, oy) = unit_translation([1.0, 0.0, 0.0, 1.0, 0.0, 0.0], r).expect("应当走快路");
            blit_1to1(&mut a, &src, iw, r, ox, oy, alpha);
            sample_affine(&mut b, &src, iw, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], r, smooth, alpha);
            assert!(a.pixels() == b.pixels(), "第 {case} 例不一致");
        }
    }

    #[test]
    fn 分段拷贝与逐像素逐字节相同() {
        // 随机 alpha（含 0 / 255 / 中间值的连续段），随机位置（含越出画布）、随机源矩形
        let (iw, ih) = (37u32, 23u32);
        let mut seed = 7u32;
        let mut rnd = move |n: u32| { seed = seed.wrapping_mul(1103515245).wrapping_add(12345); (seed >> 8) % n };
        let rgba: Vec<u8> = (0..iw * ih).flat_map(|i| {
            let a = [0u8, 0, 255, 255, 255, 128, 7][((i / 3) % 7) as usize];
            [(i * 13 % 256) as u8, (i * 7 % 256) as u8, (i * 3 % 256) as u8, a]
        }).collect();
        let sp = RowSpans::build(&rgba, iw, ih);
        for case in 0..300 {
            let (sx, sy) = (rnd(10) as f32, rnd(8) as f32);
            let (w, h) = ((rnd(iw - sx as u32) + 1) as f32, (rnd(ih - sy as u32) + 1) as f32);
            let (dx, dy) = (rnd(80) as f32 - 20.0, rnd(60) as f32 - 15.0);
            let r = ImageRects { sx, sy, sw: w, sh: h, dx, dy, dw: w, dh: h }.normalize(iw, ih).unwrap();
            let (ox, oy) = unit_translation([1.0, 0.0, 0.0, 1.0, 0.0, 0.0], r).unwrap();
            let mut a = Canvas::new(64, 48);
            let mut b = Canvas::new(64, 48);
            a.clear(Color::new(40, 50, 60, [255, 100][case % 2]));
            b.clear(Color::new(40, 50, 60, [255, 100][case % 2]));
            blit_spans(&mut a, &rgba, iw, r, ox, oy, &sp);
            blit_1to1(&mut b, &rgba, iw, r, ox, oy, 1.0);
            assert!(a.pixels() == b.pixels(), "第 {case} 例不一致");
        }
    }

    #[test]
    fn 负宽高规范化不翻转_源越界按比例缩() {
        let r = ImageRects { sx: 10.0, sy: 0.0, sw: -10.0, sh: 10.0, dx: 20.0, dy: 0.0, dw: -20.0, dh: 20.0 }.normalize(100, 100).unwrap();
        assert_eq!((r.sx, r.sw, r.dx, r.dw), (0.0, 10.0, 0.0, 20.0));
        let r = ImageRects { sx: -5.0, sy: 0.0, sw: 10.0, sh: 10.0, dx: 0.0, dy: 0.0, dw: 20.0, dh: 20.0 }.normalize(100, 100).unwrap();
        assert_eq!((r.sx, r.sw, r.dx, r.dw), (0.0, 5.0, 10.0, 10.0), "源左边越出 5 像素：目标左边让出 10");
        assert!(ImageRects { sx: 0.0, sy: 0.0, sw: 0.0, sh: 10.0, dx: 0.0, dy: 0.0, dw: 10.0, dh: 10.0 }.normalize(10, 10).is_none());
    }

    #[test]
    fn 最近邻放大每个源像素成块_透明边不发黑() {
        // 2×1：左不透明红、右全透明（RGB 是黑）
        let src = Arc::new(vec![255, 0, 0, 255, 0, 0, 0, 0]);
        let r = ImageRects { sx: 0.0, sy: 0.0, sw: 2.0, sh: 1.0, dx: 0.0, dy: 0.0, dw: 8.0, dh: 4.0 };
        let mut c = Canvas::new(8, 4);
        c.clear(Color::WHITE);
        sample_affine(&mut c, &src, 2, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], r, false, 1.0);
        assert_eq!(c.get_pixel(3, 2), Color::new(255, 0, 0, 255));
        assert_eq!(c.get_pixel(4, 2), Color::WHITE, "最近邻：右半是透明");
        let mut d = Canvas::new(8, 4);
        d.clear(Color::WHITE);
        sample_affine(&mut d, &src, 2, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], r, true, 1.0);
        let mid = d.get_pixel(4, 2);
        assert!(mid.r == 255 && mid.g > 0 && mid.g == mid.b, "双线性过渡是红→白，不混黑：{mid:?}");
    }

    #[test]
    fn 旋转九十度() {
        // 4×2 的图转 90°（绕原点）后画在 x∈[-2,0)，平移回来
        let src = img(4, 2);
        let m = [0.0, 1.0, -1.0, 0.0, 2.0, 0.0]; // x' = -y + 2, y' = x
        let r = ImageRects { sx: 0.0, sy: 0.0, sw: 4.0, sh: 2.0, dx: 0.0, dy: 0.0, dw: 4.0, dh: 2.0 };
        let mut c = Canvas::new(4, 4);
        c.clear(Color::TRANSPARENT);
        sample_affine(&mut c, &src, 4, m, r, false, 1.0);
        // 源 (u=3, v=0) → 设备 (x = 2 - 0.5 → 1, y = 3)
        let s = &src[(0 * 4 + 3) * 4..(0 * 4 + 3) * 4 + 4];
        assert_eq!(c.get_pixel(1, 3), Color::new(s[0], s[1], s[2], s[3]));
    }
}
