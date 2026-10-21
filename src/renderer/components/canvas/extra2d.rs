//! Canvas 2D 新接口补的几条（旧 `wx.createCanvasContext` 没有，`type="2d"` 要）：
//! `setTransform` / `transform`、`ellipse` / `arcTo`、`putImageData` 与像素读回、按真实字体量宽、导出 PNG / JPEG。
//!
//! 坐标约定见 `prelude/canvas2d.js`：开发者在 `canvas.width × canvas.height` 的后备像素上画，
//! 原生缓冲是「CSS 尺寸 × dpr」。`pre` 是 JS 侧算好的「CSS 尺寸 / canvas 尺寸」，像素读写与
//! `setTransform` 都用它在两套像素之间换算。

use super::*;

impl Canvas2DContext {
    /// `setTransform(a, b, c, d, e, f)`：相对「canvas 像素空间」的绝对矩阵（不是累乘）
    pub fn set_transform_pre(&mut self, m: [f32; 6], pre: (f32, f32)) {
        self.transform = [self.dpr, 0.0, 0.0, self.dpr, 0.0, 0.0];
        self.mul([pre.0, 0.0, 0.0, pre.1, 0.0, 0.0]);
        self.mul(m);
    }

    /// `transform(a, b, c, d, e, f)`：右乘
    pub fn transform_by(&mut self, m: [f32; 6]) {
        self.mul(m);
    }

    /// 当前路径的最后一个点（用户坐标）
    fn last_point(&self) -> Option<(f32, f32)> {
        self.current_path.iter().rev().find_map(|c| match *c {
            PathCommand::MoveTo(x, y) | PathCommand::LineTo(x, y) => Some((x, y)),
            PathCommand::QuadraticCurveTo(_, _, x, y) => Some((x, y)),
            PathCommand::BezierCurveTo(_, _, _, _, x, y) => Some((x, y)),
            PathCommand::Arc(cx, cy, r, _, e, _) => Some((cx + r * e.cos(), cy + r * e.sin())),
            PathCommand::Rect(x, y, ..) => Some((x, y)),
            PathCommand::ClosePath => None,
        })
    }

    /// `ellipse`：按 64 段/整圈拆成折线（用户坐标，画的时候过矩阵）
    #[allow(clippy::too_many_arguments)]
    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, start: f32, end: f32, ccw: bool) {
        let tau = std::f32::consts::TAU;
        let mut sweep = end - start;
        if ccw {
            if sweep > 0.0 { sweep -= tau * (sweep / tau).ceil().max(1.0) }
            sweep = sweep.max(-tau);
        } else {
            if sweep < 0.0 { sweep += tau * (-sweep / tau).ceil().max(1.0) }
            sweep = sweep.min(tau);
        }
        let n = ((sweep.abs() / tau) * 64.0).ceil().max(1.0) as usize;
        let (sr, cr) = rot.sin_cos();
        let pt = |a: f32| {
            let (x, y) = (rx * a.cos(), ry * a.sin());
            (cx + x * cr - y * sr, cy + x * sr + y * cr)
        };
        let p0 = pt(start);
        if self.last_point().is_some() {
            self.current_path.push(PathCommand::LineTo(p0.0, p0.1));
        } else {
            self.current_path.push(PathCommand::MoveTo(p0.0, p0.1));
        }
        for i in 1..=n {
            let p = pt(start + sweep * i as f32 / n as f32);
            self.current_path.push(PathCommand::LineTo(p.0, p.1));
        }
    }

    /// `arcTo(x1, y1, x2, y2, r)`：从当前点连到切点，再画一段与两条边相切的圆弧
    pub fn arc_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, r: f32) {
        let Some((x0, y0)) = self.last_point() else {
            self.current_path.push(PathCommand::MoveTo(x1, y1));
            return;
        };
        let (v1, v2) = ((x0 - x1, y0 - y1), (x2 - x1, y2 - y1));
        let (l1, l2) = ((v1.0 * v1.0 + v1.1 * v1.1).sqrt(), (v2.0 * v2.0 + v2.1 * v2.1).sqrt());
        let cross = v1.0 * v2.1 - v1.1 * v2.0;
        if r <= 0.0 || l1 < 1e-6 || l2 < 1e-6 || cross.abs() < 1e-6 {
            self.current_path.push(PathCommand::LineTo(x1, y1));
            return;
        }
        let (u1, u2) = ((v1.0 / l1, v1.1 / l1), (v2.0 / l2, v2.1 / l2));
        let cos = (u1.0 * u2.0 + u1.1 * u2.1).clamp(-1.0, 1.0);
        let half = cos.acos() / 2.0;
        let d = r / half.tan();
        let t1 = (x1 + u1.0 * d, y1 + u1.1 * d);
        let t2 = (x1 + u2.0 * d, y1 + u2.1 * d);
        // 圆心在角平分线上，距角点 r / sin(half)
        let bis = (u1.0 + u2.0, u1.1 + u2.1);
        let bl = (bis.0 * bis.0 + bis.1 * bis.1).sqrt().max(1e-6);
        let k = r / half.sin();
        let c = (x1 + bis.0 / bl * k, y1 + bis.1 / bl * k);
        let a0 = (t1.1 - c.1).atan2(t1.0 - c.0);
        let a1 = (t2.1 - c.1).atan2(t2.0 - c.0);
        self.current_path.push(PathCommand::LineTo(t1.0, t1.1));
        self.current_path.push(PathCommand::Arc(c.0, c.1, r, a0, a1, cross > 0.0));
    }

    /// `putImageData`：`rgba` 是 `w × h` 的 canvas 像素，左上角放在 canvas 像素 (dx, dy)；
    /// 直接覆盖（不混合、不过矩阵 —— HTML 语义）
    pub fn put_image_data(&mut self, rgba: &[u8], w: u32, h: u32, dx: f32, dy: f32, pre: (f32, f32)) {
        let Ok(mut canvas) = self.canvas.lock() else { return };
        let (sx, sy) = (pre.0 * self.dpr, pre.1 * self.dpr);
        let (cw, ch) = (canvas.width() as i32, canvas.height() as i32);
        let x0 = (dx * sx).round() as i32;
        let y0 = (dy * sy).round() as i32;
        let (tw, th) = ((w as f32 * sx).round() as i32, (h as f32 * sy).round() as i32);
        for ty in 0..th {
            let py = y0 + ty;
            if py < 0 || py >= ch {
                continue;
            }
            let srow = ((ty as f32 / sy) as u32).min(h.saturating_sub(1));
            for tx in 0..tw {
                let px = x0 + tx;
                if px < 0 || px >= cw {
                    continue;
                }
                let scol = ((tx as f32 / sx) as u32).min(w.saturating_sub(1));
                let i = ((srow * w + scol) * 4) as usize;
                if i + 3 < rgba.len() {
                    canvas.set_pixel_direct(px, py, Color::new(rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]));
                }
            }
        }
    }

    /// 读回 canvas 像素区域 `(x, y, w, h)`（RGBA、未预乘、按 canvas 像素采样）
    pub fn read_rgba(&self, x: f32, y: f32, w: u32, h: u32, pre: (f32, f32)) -> Vec<u8> {
        let mut out = vec![0u8; (w as usize) * (h as usize) * 4];
        let Ok(canvas) = self.canvas.lock() else { return out };
        let (sx, sy) = (pre.0 * self.dpr, pre.1 * self.dpr);
        let (cw, ch) = (canvas.width() as i32, canvas.height() as i32);
        let pixels = canvas.pixels();
        for j in 0..h {
            let py = ((y + j as f32 + 0.5) * sy).floor() as i32;
            for i in 0..w {
                let px = ((x + i as f32 + 0.5) * sx).floor() as i32;
                if px < 0 || py < 0 || px >= cw || py >= ch {
                    continue;
                }
                let c = pixels[(py * cw + px) as usize];
                let o = ((j * w + i) * 4) as usize;
                out[o..o + 4].copy_from_slice(&[c.r, c.g, c.b, c.a]);
            }
        }
        out
    }

    /// 整张后备缓冲编码成 PNG / JPEG（`toDataURL` / `canvasToTempFilePath`）。
    /// `region`：canvas 像素的 (x, y, w, h)，`dest`：输出尺寸（0 = 与区域同大）
    pub fn encode(&self, jpeg: bool, quality: f32, region: Option<(f32, f32, u32, u32)>, dest: (u32, u32), pre: (f32, f32)) -> Result<Vec<u8>, String> {
        let (cw, ch) = {
            let c = self.canvas.lock().map_err(|e| e.to_string())?;
            (c.width(), c.height())
        };
        let (x, y, w, h) = region.unwrap_or((0.0, 0.0, (cw as f32 / (pre.0 * self.dpr)).round() as u32, (ch as f32 / (pre.1 * self.dpr)).round() as u32));
        let w = w.max(1);
        let h = h.max(1);
        let raw = self.read_rgba(x, y, w, h, pre);
        let mut img = image::RgbaImage::from_raw(w, h, raw).ok_or("bad image")?;
        if dest.0 > 0 && dest.1 > 0 && (dest.0, dest.1) != (w, h) {
            img = image::imageops::resize(&img, dest.0, dest.1, image::imageops::FilterType::Triangle);
        }
        let mut out = std::io::Cursor::new(Vec::new());
        if jpeg {
            let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
            let q = (quality.clamp(0.0, 1.0) * 100.0).round().max(1.0) as u8;
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, q).encode_image(&rgb).map_err(|e| e.to_string())?;
        } else {
            img.write_to(&mut out, image::ImageFormat::Png).map_err(|e| e.to_string())?;
        }
        Ok(out.into_inner())
    }
}

/// 按画布字体量一段文字的宽（逻辑 px）：`measureText` 用，与 fillText 的排字同一份字体
pub fn measure_text(font_size: f32, text: &str) -> f32 {
    CANVAS_FONT.as_ref().map(|tr| tr.measure_text(text, font_size)).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_then_read_round_trips_and_set_transform_is_absolute() {
        let mut ctx = Canvas2DContext::new_with_dpr("p", 10, 10, 2.0);
        let data: Vec<u8> = (0..4).flat_map(|i| [i * 60, 10, 20, 255]).collect();
        ctx.put_image_data(&data, 2, 2, 3.0, 4.0, (1.0, 1.0));
        let back = ctx.read_rgba(3.0, 4.0, 2, 2, (1.0, 1.0));
        assert_eq!(back, data);
        ctx.translate(5.0, 5.0);
        ctx.set_transform_pre([1.0, 0.0, 0.0, 1.0, 1.0, 0.0], (1.0, 1.0));
        assert_eq!(ctx.tp(0.0, 0.0), (2.0, 0.0), "setTransform 不累乘，且叠在 dpr 之上");
    }

    #[test]
    fn arc_to_rounds_a_corner() {
        let mut ctx = Canvas2DContext::new("a", 100, 100);
        ctx.set_fill_style("#000000");
        ctx.begin_path();
        ctx.move_to(10.0, 10.0);
        ctx.arc_to(90.0, 10.0, 90.0, 90.0, 20.0);
        ctx.line_to(90.0, 90.0);
        ctx.line_to(10.0, 90.0);
        ctx.close_path();
        ctx.fill();
        let px = |x: usize, y: usize| ctx.get_image_data()[(y * 100 + x) * 4 + 3];
        assert_eq!(px(88, 12), 0, "角被倒圆了");
        assert_eq!(px(50, 50), 255);
    }
}
