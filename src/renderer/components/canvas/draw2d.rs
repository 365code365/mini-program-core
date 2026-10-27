//! 基本图形与图片：矩形/圆/线/路径的填充描边、drawImage
//!
//! `Canvas2DContext` 的一片，由 `canvas2d/mod.rs` 组合。**纯搬迁**：从 1132 行的
//! components/canvas.rs 按职责切开，一行逻辑没改。
use super::*;

impl Canvas2DContext {
    /// 清除矩形区域（设置为透明）
    pub fn clear_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        // 坐标同样要过当前矩阵（含 dpr 预缩放），否则 2x 屏上只清掉左上角四分之一
        let (tx0, ty0) = self.tp(x, y);
        let (tx1, ty1) = self.tp(x + width, y + height);
        let (x, y) = (tx0.min(tx1), ty0.min(ty1));
        let (width, height) = ((tx1 - tx0).abs(), (ty1 - ty0).abs());
        if let Ok(mut canvas) = self.canvas.lock() {
            let x0 = x.max(0.0) as i32;
            let y0 = y.max(0.0) as i32;
            let x1 = (x + width).min(canvas.width() as f32) as i32;
            let y1 = (y + height).min(canvas.height() as f32) as i32;
            // 直接设置像素为透明：按行切片 fill，与逐像素 set_pixel_direct 结果相同（同一组像素、同一个值），
            // 从前逐像素调用在全屏清屏上要 0.24ms（桌面）/ 2ms（一加 PLY110）
            if x1 > x0 && y1 > y0 {
                let w = canvas.width() as usize;
                let px = canvas.pixels_mut();
                for py in y0 as usize..y1 as usize {
                    px[py * w + x0 as usize..py * w + x1 as usize].fill(Color::TRANSPARENT);
                }
            }
        }
    }

    /// 构建一个经当前矩阵变换的矩形路径（支持旋转/缩放）
    pub(super) fn rect_path(&self, x: f32, y: f32, w: f32, h: f32) -> Path {
        let (p0, p1, p2, p3) = (
            self.tp(x, y), self.tp(x + w, y), self.tp(x + w, y + h), self.tp(x, y + h),
        );
        let mut path = Path::new();
        path.move_to(p0.0, p0.1);
        path.line_to(p1.0, p1.1);
        path.line_to(p2.0, p2.1);
        path.line_to(p3.0, p3.1);
        path.close();
        path
    }

    /// 填充矩形
    pub fn fill_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        if self.fill_rect_fast(x, y, width, height) {
            return;
        }
        let path = self.rect_path(x, y, width, height);
        if !self.fx.trivial(false) {
            return self.fx_path(&path, None);
        }
        let color = self.apply_alpha(self.fill_style);
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
            canvas.draw_path(&path, &paint);
        }
    }

    /// 描边矩形
    pub fn stroke_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let path = self.dashed(self.rect_path(x, y, width, height));
        let color = self.apply_alpha(self.stroke_style);
        let lw = self.line_width * self.avg_scale();
        if !self.fx.trivial(true) {
            return self.fx_path(&path, Some(lw));
        }
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Stroke)
                .with_stroke_width(lw).with_anti_alias(true);
            canvas.draw_path(&path, &paint);
        }
    }

    // ========== 路径绑制 ==========

    /// 绘制填充圆
    pub fn fill_circle(&mut self, x: f32, y: f32, radius: f32) {
        let (cx, cy) = self.tp(x, y);
        let r = radius * self.avg_scale();
        let color = self.apply_alpha(self.fill_style);
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
            canvas.draw_circle(cx, cy, r, &paint);
        }
    }

    /// 绘制描边圆
    pub fn stroke_circle(&mut self, x: f32, y: f32, radius: f32) {
        let (cx, cy) = self.tp(x, y);
        let r = radius * self.avg_scale();
        let lw = self.line_width * self.avg_scale();
        let color = self.apply_alpha(self.stroke_style);
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Stroke)
                .with_stroke_width(lw).with_anti_alias(true);
            canvas.draw_circle(cx, cy, r, &paint);
        }
    }

    // ========== 线条绘制 ==========

    /// 绘制线条（从当前点到指定点）
    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        let (a0, a1) = self.tp(x1, y1);
        let (b0, b1) = self.tp(x2, y2);
        let lw = self.line_width * self.avg_scale();
        let color = self.apply_alpha(self.stroke_style);
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new().with_color(color).with_stroke_width(lw).with_anti_alias(true);
            canvas.draw_line(a0, a1, b0, b1, &paint);
        }
    }

    // ========== 变换 ==========

    /// 填充当前路径
    pub fn fill(&mut self) {
        let path = self.build_path();
        if !self.fx.trivial(false) {
            return self.fx_path(&path, None);
        }
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new()
                .with_color(self.apply_alpha(self.fill_style))
                .with_style(PaintStyle::Fill)
                .with_anti_alias(true);
            canvas.draw_path(&path, &paint);
        }
    }

    /// 描边当前路径（有虚线时先切段，见 clipdash.rs）
    pub fn stroke(&mut self) {
        let path = self.dashed(self.build_path());
        let lw = self.line_width * self.avg_scale();
        if !self.fx.trivial(true) {
            return self.fx_path(&path, Some(lw));
        }
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new()
                .with_color(self.apply_alpha(self.stroke_style))
                .with_style(PaintStyle::Stroke)
                .with_stroke_width(lw)
                .with_anti_alias(true);
            canvas.draw_path(&path, &paint);
        }
    }

    /// 走 fx 蒙版画一条路径：`stroke` 给线宽时是描边，否则填充
    fn fx_path(&mut self, path: &Path, stroke: Option<f32>) {
        let (cap, join) = (self.line_cap, self.line_join);
        self.paint_with_fx(stroke.is_some(), |mask| {
            let mut paint = Paint::new().with_color(Color::WHITE).with_anti_alias(true);
            paint = match stroke {
                Some(lw) => paint.with_style(PaintStyle::Stroke).with_stroke_width(lw),
                None => paint.with_style(PaintStyle::Fill),
            };
            paint.stroke_cap = cap;
            paint.stroke_join = join;
            mask.draw_path(path, &paint);
        });
    }

    /// 绘制图片（支持目标位置与目标尺寸；旋转/斜切按轴对齐近似）
    pub fn draw_image(&mut self, src: &str, dx: f32, dy: f32, dw: f32, dh: f32) {
        // 加载图片
        // 包内路径 / `wxfile://` 走与 `<image>` 同一套解析（G-16：从前按进程工作目录读，宿主里一张都读不到）
        let bytes = crate::assets::resolve(src).and_then(|p| std::fs::read(p).ok())
            .or_else(|| std::fs::read(src).ok())
            .or_else(|| std::fs::read(src.trim_start_matches('/')).ok());
        let bytes = match bytes { Some(b) => b, None => return };
        let img = match image::load_from_memory(&bytes) { Ok(i) => i, Err(_) => return };
        use image::GenericImageView;
        let (iw, ih) = img.dimensions();
        let rgba = img.to_rgba8().into_raw();
        let (tw, th) = (dw * self.avg_scale(), dh * self.avg_scale());
        let (px, py) = self.tp(dx, dy);
        if let Ok(mut canvas) = self.canvas.lock() {
            canvas.draw_image(&rgba, iw, ih, px, py, tw, th, "scaleToFill", 0.0);
        }
    }
}

impl Canvas2DContext {
    /// fillRect 的快路：轴对齐、四边落在整像素上、不透明纯色、普通合成。这种矩形的抗锯齿覆盖率在每个像素上不是 0 就是 1，
    /// 通用路径光栅（fill_path）算出来就是把这些像素写成这个颜色 —— 这里直接写，结果逐字节相同（`快路与通用路径逐字节相同`守着）。
    /// 游戏每帧整屏清底、画方块精灵占位都是这种；一加 PLY110 上整屏 fillRect 走通用路径要 3~4ms（simpleperf：fill_path 占游戏线程 36%）。
    /// 小程序的 canvas 也走这里：结果相同，只是更快。不满足条件（旋转、半像素边、半透明、渐变 / 阴影 / 合成模式、有裁剪）返回 false
    fn fill_rect_fast(&mut self, x: f32, y: f32, w: f32, h: f32) -> bool {
        let m = self.transform;
        if m[1] != 0.0 || m[2] != 0.0 || !self.fx.trivial(false) {
            return false;
        }
        let color = self.apply_alpha(self.fill_style);
        if color.a != 255 {
            return false;
        }
        let (ax, ay) = self.tp(x, y);
        let (bx, by) = self.tp(x + w, y + h);
        let (x0, x1, y0, y1) = (ax.min(bx), ax.max(bx), ay.min(by), ay.max(by));
        let int = |v: f32| v.is_finite() && (v - v.round()).abs() < 1e-4;
        if !(int(x0) && int(x1) && int(y0) && int(y1)) {
            return false;
        }
        let Ok(mut canvas) = self.canvas.lock() else { return false };
        // 画布自己的裁剪矩形（clip_rect）：通用路径会遵守它，快路不去复刻，有就走原路
        if canvas.has_clip_rect() {
            return false;
        }
        let (cw, ch) = (canvas.width() as i64, canvas.height() as i64);
        let cx0 = (x0.round() as i64).clamp(0, cw) as usize;
        let cx1 = (x1.round() as i64).clamp(0, cw) as usize;
        let cy0 = (y0.round() as i64).clamp(0, ch) as usize;
        let cy1 = (y1.round() as i64).clamp(0, ch) as usize;
        if cx1 > cx0 && cy1 > cy0 {
            let stride = cw as usize;
            let px = canvas.pixels_mut();
            for py in cy0..cy1 {
                px[py * stride + cx0..py * stride + cx1].fill(color);
            }
        }
        true
    }
}

#[cfg(test)]
mod fast_tests {
    use super::*;

    /// 快路与通用路径（fill_path）逐字节相同：随机整像素矩形、各种 dpr / 平移 / 缩放、越界、负宽高
    #[test]
    fn 快路与通用路径逐字节相同() {
        let mut seed = 12345u32;
        let mut r = |n: u32| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 8) % n
        };
        let mut fast_hits = 0;
        for case in 0..400 {
            let dpr = [1.0f32, 2.0, 3.0, 2.75][case % 4];
            let mut a = Canvas2DContext::new_with_dpr("a", 64, 48, dpr);
            let mut b = Canvas2DContext::new_with_dpr("b", 64, 48, dpr);
            let (tx, ty) = (r(7) as f32, r(5) as f32);
            for g in [&mut a, &mut b] {
                g.set_fill_style("#336699");
                g.fill_rect(0.0, 0.0, 64.0, 48.0);
                if case % 4 == 1 {
                    g.translate(tx, ty);
                }
                if case % 5 == 2 {
                    g.scale(2.0, 3.0);
                }
                g.set_fill_style(["#ff0000", "#00ff00", "#123456"][case % 3]);
            }
            let (x, y) = (r(80) as f32 - 10.0, r(60) as f32 - 10.0);
            let (w, h) = (r(50) as f32 - 5.0, r(40) as f32 - 5.0);
            if a.fill_rect_fast(x, y, w, h) {
                fast_hits += 1;
            } else {
                a.fill_rect(x, y, w, h);
            }
            // b：强制通用路径
            let path = b.rect_path(x, y, w, h);
            let color = b.apply_alpha(b.fill_style);
            b.canvas.lock().unwrap().draw_path(&path, &Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true));
            let pa = a.canvas.lock().unwrap().pixels().to_vec();
            let pb = b.canvas.lock().unwrap().pixels().to_vec();
            assert!(pa == pb, "第 {case} 例不一致：rect ({x},{y},{w},{h}) dpr {dpr}");
        }
        assert!(fast_hits > 100, "快路几乎没走到，测试没测到东西：{fast_hits}");
    }
}
