//! 画笔模块

use crate::Color;

/// 画笔样式
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaintStyle {
    Fill,
    Stroke,
    FillAndStroke,
}

impl Default for PaintStyle {
    fn default() -> Self {
        Self::Fill
    }
}

/// 线帽样式
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StrokeCap {
    Butt,
    Round,
    Square,
}

/// 线连接样式
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StrokeJoin {
    Miter,
    Round,
    Bevel,
}

/// 画笔
#[derive(Debug, Clone)]
pub struct Paint {
    pub color: Color,
    pub style: PaintStyle,
    pub stroke_width: f32,
    pub stroke_cap: StrokeCap,
    pub stroke_join: StrokeJoin,
    pub anti_alias: bool,
}

impl Default for Paint {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            style: PaintStyle::Fill,
            stroke_width: 1.0,
            stroke_cap: StrokeCap::Butt,
            stroke_join: StrokeJoin::Miter,
            anti_alias: true,
        }
    }
}

impl Paint {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_style(mut self, style: PaintStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    pub fn with_anti_alias(mut self, aa: bool) -> Self {
        self.anti_alias = aa;
        self
    }
}
