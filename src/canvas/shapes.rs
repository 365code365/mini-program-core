//! 基本图形：矩形 / 圆角矩形 / 圆 / 线段的填充与描边
//!
//! `Canvas` 的一片，由 `canvas/mod.rs` 组合。**纯搬迁**：从 1250 行的 canvas.rs 按职责
//! 切开，一行逻辑没改；判据是 65 张画廊图与 21 页整帧快照逐字节不变。
use super::*;

impl Canvas {
    /// 绘制矩形
    pub fn draw_rect(&mut self, rect: &Rect, paint: &Paint) {
        match paint.style {
            PaintStyle::Fill => self.fill_rect(rect, &paint.color),
            PaintStyle::Stroke => self.stroke_rect(rect, paint),
            PaintStyle::FillAndStroke => {
                self.fill_rect(rect, &paint.color);
                self.stroke_rect(rect, paint);
            }
        }
    }

    pub(super) fn fill_rect(&mut self, rect: &Rect, color: &Color) {
        let tx = self.translation.0;
        let ty = self.translation.1;
        
        let mut x0 = (rect.x + tx).max(0.0) as i32;
        let mut y0 = (rect.y + ty).max(0.0) as i32;
        let mut x1 = (rect.right() + tx).min(self.width as f32) as i32;
        let mut y1 = (rect.bottom() + ty).min(self.height as f32) as i32;

        // 先把裁剪区并进矩形范围，这样逐像素时不必再判裁剪
        if let Some(clip) = &self.clip_rect {
            x0 = x0.max(clip.x as i32);
            y0 = y0.max(clip.y as i32);
            x1 = x1.min(clip.right() as i32);
            y1 = y1.min(clip.bottom() as i32);
        }
        if x1 <= x0 || y1 <= y0 {
            return;
        }

        // 不透明实色：整行 memset，跳过逐像素的越界/裁剪判断与混色。
        // 卡片背景、色块这类大面积填充占了绘制耗时的大头，行填充比逐像素快一个量级。
        if color.a == 255 {
            let width = self.width as usize;
            for y in y0..y1 {
                let start = y as usize * width + x0 as usize;
                let end = y as usize * width + x1 as usize;
                self.pixels[start..end].fill(*color);
            }
            return;
        }

        // 半透明实色：alpha 是常量，把系数提到循环外，内层只做整数乘加。
        // 全屏遮罩（rgba(0,0,0,.5) 铺满视口）是最典型的场景，一帧要混上百万像素。
        let alpha = color.a as u32;
        let inv = 255 - alpha;
        let (sr, sg, sb) = (
            color.r as u32 * alpha,
            color.g as u32 * alpha,
            color.b as u32 * alpha,
        );
        let width = self.width as usize;
        for y in y0..y1 {
            let row = y as usize * width;
            for px in &mut self.pixels[row + x0 as usize..row + x1 as usize] {
                if px.a == 255 {
                    px.r = ((sr + px.r as u32 * inv) / 255) as u8;
                    px.g = ((sg + px.g as u32 * inv) / 255) as u8;
                    px.b = ((sb + px.b as u32 * inv) / 255) as u8;
                } else {
                    *px = color.blend(px);
                }
            }
        }
    }

    /// 快速填充圆角矩形：实心内部用整块矩形填充，仅四个圆角做抗锯齿。
    /// 相比通用 4x 超采样扫描线填充（遍历整块面积）快一个数量级，
    /// 是圆角背景/卡片大量出现时的关键性能优化。
    pub fn fill_round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
        if w <= 0.0 || h <= 0.0 { return; }
        let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
        if r < 0.6 {
            self.fill_rect(&Rect::new(x, y, w, h), &color);
            return;
        }
        // 内部实心（中间整宽条 + 上下去角条），避开四角
        let strips = [
            Rect::new(x, y + r, w, h - 2.0 * r),
            Rect::new(x + r, y, w - 2.0 * r, r),
            Rect::new(x + r, y + h - r, w - 2.0 * r, r),
        ];
        for s in &strips {
            self.fill_rect(s, &color);
        }
        // 四角抗锯齿。每个像素只能混合一次：角区按 floor/ceil 取整后会和矩形条、
        // 以及上下（左右）相对的角区各重叠一行/一列 —— 不透明色画两遍看不出来，
        // 半透明底色（`rgba(…,.12)` 的胶囊按钮）就会在拼接处多出一条更深的线。
        // 所以角区只管矩形条之外的那一角：边界与 `fill_rect` 的取整完全一致，
        // 矩形条与四个角区正好不重不漏。
        let (tx, ty) = self.translation;
        let (left_edge, right_edge) = ((x + tx + r).floor() as i32, (x + tx + w - r).floor() as i32);
        let (top_edge, bottom_edge) = ((y + ty + r).floor() as i32, (y + ty + h - r).floor() as i32);
        let corners = [
            (x + r, y + r, x, y, false, false),                         // 左上：角区 [x, x+r]
            (x + w - r, y + r, x + w - r, y, true, false),              // 右上
            (x + w - r, y + h - r, x + w - r, y + h - r, true, true),   // 右下
            (x + r, y + h - r, x, y + h - r, false, true),              // 左下
        ];
        for &(cx, cy, bx, by, right, bottom) in &corners {
            let x0 = (bx + tx).floor() as i32;
            let y0 = (by + ty).floor() as i32;
            let x1 = (bx + tx + r).ceil() as i32;
            let y1 = (by + ty + r).ceil() as i32;
            let (ccx, ccy) = (cx + tx, cy + ty);
            for py in y0..y1 {
                if (bottom && py < bottom_edge) || (!bottom && py >= top_edge) {
                    continue;
                }
                for px in x0..x1 {
                    if (right && px < right_edge) || (!right && px >= left_edge) {
                        continue;
                    }
                    let dx = px as f32 + 0.5 - ccx;
                    let dy = py as f32 + 0.5 - ccy;
                    let d = (dx * dx + dy * dy).sqrt();
                    let cov = (r + 0.5 - d).clamp(0.0, 1.0);
                    if cov > 0.0 {
                        self.set_pixel_aa(px, py, color, cov);
                    }
                }
            }
        }
    }

    pub(super) fn stroke_rect(&mut self, rect: &Rect, paint: &Paint) {
        let w = paint.stroke_width;
        // 上边
        self.fill_rect(&Rect::new(rect.x, rect.y, rect.width, w), &paint.color);
        // 下边
        self.fill_rect(&Rect::new(rect.x, rect.bottom() - w, rect.width, w), &paint.color);
        // 左边
        self.fill_rect(&Rect::new(rect.x, rect.y, w, rect.height), &paint.color);
        // 右边
        self.fill_rect(&Rect::new(rect.right() - w, rect.y, w, rect.height), &paint.color);
    }

    /// 绘制圆形
    pub fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, paint: &Paint) {
        match paint.style {
            PaintStyle::Fill => self.fill_circle(cx, cy, radius, paint),
            PaintStyle::Stroke => self.stroke_circle(cx, cy, radius, paint),
            PaintStyle::FillAndStroke => {
                self.fill_circle(cx, cy, radius, paint);
                self.stroke_circle(cx, cy, radius, paint);
            }
        }
    }

    pub(super) fn fill_circle(&mut self, cx: f32, cy: f32, radius: f32, paint: &Paint) {
        let cx = cx + self.translation.0;
        let cy = cy + self.translation.1;

        let r2 = radius * radius;
        let x0 = (cx - radius - 1.0).max(0.0) as i32;
        let y0 = (cy - radius - 1.0).max(0.0) as i32;
        let x1 = (cx + radius + 1.0).min(self.width as f32) as i32;
        let y1 = (cy + radius + 1.0).min(self.height as f32) as i32;

        for y in y0..y1 {
            for x in x0..x1 {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let d2 = dx * dx + dy * dy;

                if paint.anti_alias {
                    // 覆盖率用有符号距离的线性斜坡：对圆这种曲率半径远大于像素的边界，
                    // 「面积占比」在法线方向本来就是线性的，所以这个近似是连续且一阶精确的。
                    // 换成 4x4 子采样反而把它量化成 1/16 档，实测与 Chrome 的差异变大。
                    let d = d2.sqrt();
                    if d <= radius + 0.5 {
                        let coverage = (radius + 0.5 - d).min(1.0);
                        self.set_pixel_aa(x, y, paint.color, coverage);
                    }
                } else if d2 <= r2 {
                    self.set_pixel(x, y, paint.color);
                }
            }
        }
    }

    pub(super) fn stroke_circle(&mut self, cx: f32, cy: f32, radius: f32, paint: &Paint) {
        let cx = cx + self.translation.0;
        let cy = cy + self.translation.1;

        let inner = radius - paint.stroke_width / 2.0;
        let outer = radius + paint.stroke_width / 2.0;

        let x0 = (cx - outer - 1.0).max(0.0) as i32;
        let y0 = (cy - outer - 1.0).max(0.0) as i32;
        let x1 = (cx + outer + 1.0).min(self.width as f32) as i32;
        let y1 = (cy + outer + 1.0).min(self.height as f32) as i32;

        for y in y0..y1 {
            for x in x0..x1 {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let d = (dx * dx + dy * dy).sqrt();

                if d >= inner && d <= outer {
                    if paint.anti_alias {
                        // 内外两侧各半像素的线性过渡（与 fill_circle 同一套近似）
                        let coverage = if d < inner + 0.5 {
                            d - inner + 0.5
                        } else if d > outer - 0.5 {
                            outer - d + 0.5
                        } else {
                            1.0
                        };
                        self.set_pixel_aa(x, y, paint.color, coverage.min(1.0));
                    } else {
                        self.set_pixel(x, y, paint.color);
                    }
                }
            }
        }
    }

    /// 绘制线段
    pub fn draw_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, paint: &Paint) {
        let x0 = x0 + self.translation.0;
        let y0 = y0 + self.translation.1;
        let x1 = x1 + self.translation.0;
        let y1 = y1 + self.translation.1;

        if paint.anti_alias {
            self.draw_line_aa(x0, y0, x1, y1, paint);
        } else {
            self.draw_line_bresenham(x0 as i32, y0 as i32, x1 as i32, y1 as i32, paint);
        }
    }

    /// Bresenham 直线算法
    pub(super) fn draw_line_bresenham(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, paint: &Paint) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.set_pixel(x0, y0, paint.color);
            if x0 == x1 && y0 == y1 { break; }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// 抗锯齿直线 (Wu's algorithm)
    pub(super) fn draw_line_aa(&mut self, mut x0: f32, mut y0: f32, mut x1: f32, mut y1: f32, paint: &Paint) {
        let steep = (y1 - y0).abs() > (x1 - x0).abs();
        if steep {
            std::mem::swap(&mut x0, &mut y0);
            std::mem::swap(&mut x1, &mut y1);
        }
        if x0 > x1 {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }

        let dx = x1 - x0;
        let dy = y1 - y0;
        let gradient = if dx == 0.0 { 1.0 } else { dy / dx };

        // 起点
        let xend = x0.round();
        let yend = y0 + gradient * (xend - x0);
        let xpxl1 = xend as i32;
        let mut intery = yend + gradient;

        // 终点
        let xend = x1.round();
        let xpxl2 = xend as i32;

        for x in xpxl1..=xpxl2 {
            let y = intery.floor() as i32;
            let frac = intery - intery.floor();

            if steep {
                self.set_pixel_aa(y, x, paint.color, 1.0 - frac);
                self.set_pixel_aa(y + 1, x, paint.color, frac);
            } else {
                self.set_pixel_aa(x, y, paint.color, 1.0 - frac);
                self.set_pixel_aa(x, y + 1, paint.color, frac);
            }
            intery += gradient;
        }
    }
}
