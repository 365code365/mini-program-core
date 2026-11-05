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
        if !self.fx.trivial(false) {
            let text = text.to_string();
            return self.paint_with_fx(false, |mask| {
                let paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill);
                tr.draw_text(mask, &text, px, py, size, &paint);
            });
        }
        let color = self.apply_alpha(self.fill_style);
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
            tr.draw_text(&mut canvas, text, px, py, size, &paint);
        }
    }

    /// `fillText(text, x, y, maxWidth)`：自然宽度不超过 maxWidth（或没给）时与 [`Self::fill_text`] 逐字节相同；
    /// 超了按 HTML 横向压缩字形（字高不变，离屏光栅再贴回，text_raster.rs）。maxWidth ≤ 0 / NaN 什么也不画（标准）
    pub fn fill_text_max(&mut self, text: &str, x: f32, y: f32, max_width: Option<f32>) {
        match self.text_job_width(text, max_width) {
            None => {}
            Some((w, dw)) if dw < w && self.fx.trivial(false) => self.text_offscreen(text, x, y, w, dw, None),
            Some(_) => self.fill_text(text, x, y),
        }
    }

    /// `strokeText`：真描轮廓（宽 lineWidth 的环，text_raster.rs 的距离变换），maxWidth 同 fillText。
    /// 描边色是渐变 / 图案、或有阴影 / 合成模式时退回旧的「用描边色填充」近似
    pub fn stroke_text_max(&mut self, text: &str, x: f32, y: f32, max_width: Option<f32>) {
        let Some((w, dw)) = self.text_job_width(text, max_width) else { return };
        if self.fx.trivial(true) {
            self.text_offscreen(text, x, y, w, dw, Some(self.line_width));
        } else {
            self.stroke_text(text, x, y);
        }
    }

    /// (自然宽度, 要画成的宽度)，用户坐标。maxWidth 非法（≤0 / NaN）返回 None = 不画
    fn text_job_width(&self, text: &str, max_width: Option<f32>) -> Option<(f32, f32)> {
        let w = match CANVAS_FONT.as_ref() {
            Some(tr) => tr.measure_text(text, self.font_size * self.avg_scale()) / self.avg_scale().max(1e-6),
            None => return None,
        };
        match max_width {
            Some(mw) if !(mw > 0.0) => None,
            Some(mw) if mw.is_finite() && w > mw => Some((w, mw)),
            _ => Some((w, w)),
        }
    }

    /// 压缩 / 描边：按 fill_text 同一套对齐与基线（旧接口的近似基线）定左端与基线，交给离屏光栅
    fn text_offscreen(&mut self, text: &str, x: f32, y: f32, w: f32, dw: f32, stroke: Option<f32>) {
        let Some(tr) = CANVAS_FONT.as_ref() else { return };
        let ax = match self.text_align {
            TextAlign::Center => x - dw / 2.0,
            TextAlign::Right => x - dw,
            _ => x,
        };
        let ay = match self.text_baseline {
            TextBaseline::Top | TextBaseline::Hanging => y + self.font_size * 0.8,
            TextBaseline::Middle => y + self.font_size * 0.35,
            _ => y,
        };
        let job = super::text_raster::TextJob { tr, text, size: self.font_size, bold: false, x: ax, y: ay, width: w, draw_width: dw, stroke };
        let base = if stroke.is_some() { self.stroke_style } else { self.fill_style };
        self.draw_text_offscreen(job, base);
    }

    /// 描边文本（本实现以填充近似）
    pub fn stroke_text(&mut self, text: &str, x: f32, y: f32) {
        // 保存 fill，用 stroke 颜色画文本
        let saved = (self.fill_style, self.fx.fill.clone());
        self.fill_style = self.stroke_style;
        self.fx.fill = self.fx.stroke.clone();
        self.fill_text(text, x, y);
        self.fill_style = saved.0;
        self.fx.fill = saved.1;
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
