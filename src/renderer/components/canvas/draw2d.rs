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
            
            // 直接设置像素为透明
            for py in y0..y1 {
                for px in x0..x1 {
                    canvas.set_pixel_direct(px, py, Color::TRANSPARENT);
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
        let path = self.rect_path(x, y, width, height);
        let color = self.apply_alpha(self.fill_style);
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
            canvas.draw_path(&path, &paint);
        }
    }

    /// 描边矩形
    pub fn stroke_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let path = self.rect_path(x, y, width, height);
        let color = self.apply_alpha(self.stroke_style);
        let lw = self.line_width * self.avg_scale();
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
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new()
                .with_color(self.apply_alpha(self.fill_style))
                .with_style(PaintStyle::Fill)
                .with_anti_alias(true);
            canvas.draw_path(&path, &paint);
        }
    }

    /// 描边当前路径
    pub fn stroke(&mut self) {
        let path = self.build_path();
        let lw = self.line_width * self.avg_scale();
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new()
                .with_color(self.apply_alpha(self.stroke_style))
                .with_style(PaintStyle::Stroke)
                .with_stroke_width(lw)
                .with_anti_alias(true);
            canvas.draw_path(&path, &paint);
        }
    }

    /// 绘制图片（支持目标位置与目标尺寸；旋转/斜切按轴对齐近似）
    pub fn draw_image(&mut self, src: &str, dx: f32, dy: f32, dw: f32, dh: f32) {
        // 加载图片
        let bytes = std::fs::read(src).ok()
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
