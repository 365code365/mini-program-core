//! 文字绘制与渐变构造
//!
//! `Canvas2DContext` 的一片，由 `canvas2d/mod.rs` 组合。**纯搬迁**：从 1132 行的
//! components/canvas.rs 按职责切开，一行逻辑没改。
use super::*;

impl Canvas2DContext {
    /// 填充文本
    pub fn fill_text(&mut self, text: &str, x: f32, y: f32) {
        let tr = match CANVAS_FONT.as_ref() { Some(t) => t, None => return };
        let size = self.font_size * self.avg_scale();
        // 测量宽度以支持 textAlign
        let tw = tr.measure_text(text, size);
        let mut ax = x;
        match self.text_align {
            TextAlign::Center => ax = x - tw / self.avg_scale() / 2.0,
            TextAlign::Right => ax = x - tw / self.avg_scale(),
            _ => {}
        }
        // baseline 调整（近似）
        let ay = match self.text_baseline {
            TextBaseline::Top | TextBaseline::Hanging => y + self.font_size * 0.8,
            TextBaseline::Middle => y + self.font_size * 0.35,
            TextBaseline::Bottom => y,
            _ => y, // alphabetic/ideographic
        };
        let (px, py) = self.tp(ax, ay);
        let color = self.apply_alpha(self.fill_style);
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
            tr.draw_text(&mut canvas, text, px, py, size, &paint);
        }
    }

    /// 描边文本（本实现以填充近似）
    pub fn stroke_text(&mut self, text: &str, x: f32, y: f32) {
        // 保存 fill，用 stroke 颜色画文本
        let saved = self.fill_style;
        self.fill_style = self.stroke_style;
        self.fill_text(text, x, y);
        self.fill_style = saved;
    }

    /// 创建线性渐变
    pub fn create_linear_gradient(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> LinearGradient {
        LinearGradient::new(x0, y0, x1, y1)
    }

    /// 创建径向渐变
    pub fn create_radial_gradient(&self, x0: f32, y0: f32, r0: f32, x1: f32, y1: f32, r1: f32) -> RadialGradient {
        RadialGradient::new(x0, y0, r0, x1, y1, r1)
    }

    // ========== 辅助方法 ==========
}
