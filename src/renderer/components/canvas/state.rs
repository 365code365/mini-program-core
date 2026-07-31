//! 上下文状态：样式设置、变换矩阵、save/restore 栈、取像素
//!
//! `Canvas2DContext` 的一片，由 `canvas2d/mod.rs` 组合。**纯搬迁**：从 1132 行的
//! components/canvas.rs 按职责切开，一行逻辑没改。
use super::*;

impl Canvas2DContext {
    pub fn new(canvas_id: &str, width: u32, height: u32) -> Self {
        Self::new_with_dpr(canvas_id, width, height, 1.0)
    }

    /// 按设备像素比创建：`width`/`height` 是逻辑尺寸，后备缓冲按 `dpr` 放大，
    /// 基础变换矩阵同步预乘 `dpr`，所以调用方仍然用逻辑坐标下指令。
    pub fn new_with_dpr(canvas_id: &str, width: u32, height: u32, dpr: f32) -> Self {
        let dpr = if dpr.is_finite() && dpr > 0.0 { dpr } else { 1.0 };
        let canvas = Canvas::new(
            ((width as f32 * dpr).round() as u32).max(1),
            ((height as f32 * dpr).round() as u32).max(1),
        );
        Self {
            canvas_id: canvas_id.to_string(),
            width,
            height,
            dpr,
            canvas: Arc::new(Mutex::new(canvas)),
            fill_style: Color::BLACK,
            stroke_style: Color::BLACK,
            line_width: 1.0,
            font_size: 10.0,
            text_align: TextAlign::Left,
            text_baseline: TextBaseline::default(),
            global_alpha: 1.0,
            line_cap: crate::paint::StrokeCap::Butt,
            line_join: crate::paint::StrokeJoin::Miter,
            transform: [dpr, 0.0, 0.0, dpr, 0.0, 0.0],
            current_path: Vec::new(),
            state_stack: Vec::new(),
        }
    }

    /// 逻辑尺寸与设备像素比（供宿主判断后备缓冲是否需要按元素实际尺寸重建）
    pub fn logical_size(&self) -> (u32, u32) { (self.width, self.height) }

    pub fn device_pixel_ratio(&self) -> f32 { self.dpr }

    // ========== 状态管理 ==========

    /// 保存当前状态
    pub fn save(&mut self) {
        self.state_stack.push(ContextState {
            fill_style: self.fill_style,
            stroke_style: self.stroke_style,
            line_width: self.line_width,
            font_size: self.font_size,
            text_align: self.text_align,
            text_baseline: self.text_baseline,
            global_alpha: self.global_alpha,
            transform: self.transform,
        });
    }

    /// 恢复上一次保存的状态
    pub fn restore(&mut self) {
        if let Some(state) = self.state_stack.pop() {
            self.fill_style = state.fill_style;
            self.stroke_style = state.stroke_style;
            self.line_width = state.line_width;
            self.font_size = state.font_size;
            self.text_align = state.text_align;
            self.text_baseline = state.text_baseline;
            self.global_alpha = state.global_alpha;
            self.transform = state.transform;
        }
    }

    // ========== 样式设置 ==========

    /// 设置填充颜色
    pub fn set_fill_style(&mut self, color: &str) {
        if let Some(c) = parse_color_str(color) {
            self.fill_style = c;
        }
    }

    /// 设置描边颜色
    pub fn set_stroke_style(&mut self, color: &str) {
        if let Some(c) = parse_color_str(color) {
            self.stroke_style = c;
        }
    }

    /// 设置线宽
    pub fn set_line_width(&mut self, width: f32) {
        self.line_width = width;
    }

    /// 设置全局透明度
    pub fn set_global_alpha(&mut self, alpha: f32) {
        self.global_alpha = alpha.clamp(0.0, 1.0);
    }

    /// 设置字体
    pub fn set_font(&mut self, font: &str) {
        // 解析字体字符串，如 "16px sans-serif"
        for part in font.split_whitespace() {
            if part.ends_with("px") {
                if let Ok(size) = part.trim_end_matches("px").parse::<f32>() {
                    self.font_size = size;
                }
            }
        }
    }

    /// 设置文本对齐
    pub fn set_text_align(&mut self, align: &str) {
        self.text_align = match align {
            "center" => TextAlign::Center,
            "right" | "end" => TextAlign::Right,
            _ => TextAlign::Left,
        };
    }

    /// 设置文本基线
    pub fn set_text_baseline(&mut self, baseline: &str) {
        self.text_baseline = match baseline {
            "top" => TextBaseline::Top,
            "hanging" => TextBaseline::Hanging,
            "middle" => TextBaseline::Middle,
            "alphabetic" => TextBaseline::Alphabetic,
            "ideographic" => TextBaseline::Ideographic,
            "bottom" => TextBaseline::Bottom,
            _ => TextBaseline::Middle,
        };
    }

    // ========== 矩形绑制 ==========

    /// 设置线帽
    pub fn set_line_cap(&mut self, cap: &str) {
        self.line_cap = match cap {
            "round" => crate::paint::StrokeCap::Round,
            "square" => crate::paint::StrokeCap::Square,
            _ => crate::paint::StrokeCap::Butt,
        };
    }

    /// 设置线连接
    pub fn set_line_join(&mut self, join: &str) {
        self.line_join = match join {
            "round" => crate::paint::StrokeJoin::Round,
            "bevel" => crate::paint::StrokeJoin::Bevel,
            _ => crate::paint::StrokeJoin::Miter,
        };
    }

    // ========== 渐变 ==========

    /// 平移（累积到当前变换矩阵）
    pub fn translate(&mut self, x: f32, y: f32) {
        self.mul([1.0, 0.0, 0.0, 1.0, x, y]);
    }

    /// 旋转（弧度，累积到当前变换矩阵）
    pub fn rotate(&mut self, angle: f32) {
        let (s, c) = angle.sin_cos();
        self.mul([c, s, -s, c, 0.0, 0.0]);
    }

    /// 缩放（累积到当前变换矩阵）
    pub fn scale(&mut self, sx: f32, sy: f32) {
        self.mul([sx, 0.0, 0.0, sy, 0.0, 0.0]);
    }

    /// 用当前矩阵变换一个点
    pub(super) fn tp(&self, x: f32, y: f32) -> (f32, f32) {
        let m = &self.transform;
        (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
    }

    /// 当前矩阵的平均缩放（用于圆半径、线宽的近似缩放）
    pub(super) fn avg_scale(&self) -> f32 {
        let m = &self.transform;
        let sx = (m[0] * m[0] + m[1] * m[1]).sqrt();
        let sy = (m[2] * m[2] + m[3] * m[3]).sqrt();
        ((sx + sy) / 2.0).max(0.0001)
    }

    /// 右乘一个矩阵 other（self = self * other）
    pub(super) fn mul(&mut self, o: [f32; 6]) {
        let m = self.transform;
        self.transform = [
            m[0] * o[0] + m[2] * o[1],
            m[1] * o[0] + m[3] * o[1],
            m[0] * o[2] + m[2] * o[3],
            m[1] * o[2] + m[3] * o[3],
            m[0] * o[4] + m[2] * o[5] + m[4],
            m[1] * o[4] + m[3] * o[5] + m[5],
        ];
    }

    /// 应用全局透明度
    pub(super) fn apply_alpha(&self, color: Color) -> Color {
        if self.global_alpha >= 1.0 {
            color
        } else {
            Color::new(color.r, color.g, color.b, (color.a as f32 * self.global_alpha) as u8)
        }
    }

    /// 获取画布像素数据
    pub fn get_image_data(&self) -> Vec<u8> {
        if let Ok(canvas) = self.canvas.lock() {
            canvas.to_rgba()
        } else {
            Vec::new()
        }
    }

    /// 获取内部 Canvas 引用（用于渲染）
    pub fn get_canvas(&self) -> Arc<Mutex<Canvas>> {
        self.canvas.clone()
    }
}
