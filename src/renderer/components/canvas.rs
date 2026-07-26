//! Canvas 组件 - 微信小程序 Canvas 2D API 实现

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Canvas 组件
pub struct CanvasComponent;

/// Canvas 2D 上下文 - 实现微信小程序 Canvas 2D API
#[derive(Clone)]
pub struct Canvas2DContext {
    /// canvas-id
    pub canvas_id: String,
    /// 画布宽度
    pub width: u32,
    /// 画布高度
    pub height: u32,
    /// 内部画布
    canvas: Arc<Mutex<Canvas>>,
    /// 当前填充颜色
    fill_style: Color,
    /// 当前描边颜色
    stroke_style: Color,
    /// 线宽
    line_width: f32,
    /// 字体大小
    font_size: f32,
    /// 文本对齐
    text_align: TextAlign,
    /// 文本基线
    text_baseline: TextBaseline,
    /// 全局透明度
    global_alpha: f32,
    /// 线帽 / 线连接
    line_cap: crate::paint::StrokeCap,
    line_join: crate::paint::StrokeJoin,
    /// 设备像素比：后备缓冲按物理像素分配，绘制指令（小程序 API 用逻辑 px）
    /// 由基础矩阵预乘这个比例 —— 这样 2x 屏上的 canvas 内容是原生分辨率而不是放大的马赛克。
    dpr: f32,
    /// 当前 2D 仿射变换矩阵 [a, b, c, d, e, f]（列优先：x'=a*x+c*y+e, y'=b*x+d*y+f）
    transform: [f32; 6],
    /// 当前路径
    current_path: Vec<PathCommand>,
    /// 状态栈
    state_stack: Vec<ContextState>,
}

/// Canvas 文本渲染用的全局字体（懒加载系统字体）
static CANVAS_FONT: once_cell::sync::Lazy<Option<std::sync::Arc<crate::text::TextRenderer>>> =
    once_cell::sync::Lazy::new(crate::text::shared_fonts);


/// 文本基线
#[derive(Clone, Copy, Default)]
pub enum TextBaseline {
    Top,
    Hanging,
    #[default]
    Middle,
    Alphabetic,
    Ideographic,
    Bottom,
}

/// 路径命令
#[derive(Clone)]
enum PathCommand {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    Arc(f32, f32, f32, f32, f32, bool),
    QuadraticCurveTo(f32, f32, f32, f32),
    BezierCurveTo(f32, f32, f32, f32, f32, f32),
    Rect(f32, f32, f32, f32),
    ClosePath,
}

/// 上下文状态（用于 save/restore）
#[derive(Clone)]
struct ContextState {
    fill_style: Color,
    stroke_style: Color,
    line_width: f32,
    font_size: f32,
    text_align: TextAlign,
    text_baseline: TextBaseline,
    global_alpha: f32,
    transform: [f32; 6],
}

impl Canvas2DContext {
    /// 单位矩阵
    const IDENTITY: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

    /// 用当前矩阵变换一个点
    fn tp(&self, x: f32, y: f32) -> (f32, f32) {
        let m = &self.transform;
        (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
    }

    /// 当前矩阵的平均缩放（用于圆半径、线宽的近似缩放）
    fn avg_scale(&self) -> f32 {
        let m = &self.transform;
        let sx = (m[0] * m[0] + m[1] * m[1]).sqrt();
        let sy = (m[2] * m[2] + m[3] * m[3]).sqrt();
        ((sx + sy) / 2.0).max(0.0001)
    }

    /// 右乘一个矩阵 other（self = self * other）
    fn mul(&mut self, o: [f32; 6]) {
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
    fn rect_path(&self, x: f32, y: f32, w: f32, h: f32) -> Path {
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
    
    /// 开始新路径
    pub fn begin_path(&mut self) {
        self.current_path.clear();
    }
    
    /// 关闭路径
    pub fn close_path(&mut self) {
        self.current_path.push(PathCommand::ClosePath);
    }
    
    /// 移动到指定点
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.current_path.push(PathCommand::MoveTo(x, y));
    }
    
    /// 绘制直线到指定点
    pub fn line_to(&mut self, x: f32, y: f32) {
        self.current_path.push(PathCommand::LineTo(x, y));
    }
    
    /// 绘制圆弧
    pub fn arc(&mut self, x: f32, y: f32, radius: f32, start_angle: f32, end_angle: f32, counter_clockwise: bool) {
        self.current_path.push(PathCommand::Arc(x, y, radius, start_angle, end_angle, counter_clockwise));
    }
    
    /// 绘制二次贝塞尔曲线
    pub fn quadratic_curve_to(&mut self, cpx: f32, cpy: f32, x: f32, y: f32) {
        self.current_path.push(PathCommand::QuadraticCurveTo(cpx, cpy, x, y));
    }
    
    /// 绘制三次贝塞尔曲线
    pub fn bezier_curve_to(&mut self, cp1x: f32, cp1y: f32, cp2x: f32, cp2y: f32, x: f32, y: f32) {
        self.current_path.push(PathCommand::BezierCurveTo(cp1x, cp1y, cp2x, cp2y, x, y));
    }
    
    /// 添加矩形路径
    pub fn rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.current_path.push(PathCommand::Rect(x, y, width, height));
    }

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
    
    /// 构建 Path 对象（所有坐标经当前仿射矩阵变换）
    fn build_path(&self) -> Path {
        let mut path = Path::new();
        for cmd in &self.current_path {
            match cmd {
                PathCommand::MoveTo(x, y) => { let (px, py) = self.tp(*x, *y); path.move_to(px, py); }
                PathCommand::LineTo(x, y) => { let (px, py) = self.tp(*x, *y); path.line_to(px, py); }
                PathCommand::Arc(x, y, r, start, end, ccw) => {
                    // 以变换后圆心 + 平均缩放半径近似（不支持椭圆化的斜切）
                    let (cx, cy) = self.tp(*x, *y);
                    path.arc(cx, cy, *r * self.avg_scale(), *start, *end, *ccw);
                }
                PathCommand::QuadraticCurveTo(cpx, cpy, x, y) => {
                    let (c0, c1) = self.tp(*cpx, *cpy);
                    let (px, py) = self.tp(*x, *y);
                    path.quad_to(c0, c1, px, py);
                }
                PathCommand::BezierCurveTo(cp1x, cp1y, cp2x, cp2y, x, y) => {
                    let (a0, a1) = self.tp(*cp1x, *cp1y);
                    let (b0, b1) = self.tp(*cp2x, *cp2y);
                    let (px, py) = self.tp(*x, *y);
                    path.cubic_to(a0, a1, b0, b1, px, py);
                }
                PathCommand::Rect(x, y, w, h) => {
                    let p0 = self.tp(*x, *y);
                    let p1 = self.tp(*x + *w, *y);
                    let p2 = self.tp(*x + *w, *y + *h);
                    let p3 = self.tp(*x, *y + *h);
                    path.move_to(p0.0, p0.1);
                    path.line_to(p1.0, p1.1);
                    path.line_to(p2.0, p2.1);
                    path.line_to(p3.0, p3.1);
                    path.close();
                }
                PathCommand::ClosePath => { path.close(); }
            }
        }
        path
    }

    // ========== 圆形绘制 ==========
    
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
    
    /// 创建线性渐变
    pub fn create_linear_gradient(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> LinearGradient {
        LinearGradient::new(x0, y0, x1, y1)
    }
    
    /// 创建径向渐变
    pub fn create_radial_gradient(&self, x0: f32, y0: f32, r0: f32, x1: f32, y1: f32, r1: f32) -> RadialGradient {
        RadialGradient::new(x0, y0, r0, x1, y1, r1)
    }

    // ========== 辅助方法 ==========
    
    /// 应用全局透明度
    fn apply_alpha(&self, color: Color) -> Color {
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


/// 线性渐变
#[derive(Clone)]
pub struct LinearGradient {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    stops: Vec<(f32, Color)>,
}

impl LinearGradient {
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self { x0, y0, x1, y1, stops: Vec::new() }
    }
    
    /// 添加颜色停止点
    pub fn add_color_stop(&mut self, offset: f32, color: &str) {
        if let Some(c) = parse_color_str(color) {
            self.stops.push((offset.clamp(0.0, 1.0), c));
            self.stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        }
    }
    
    /// 获取指定位置的颜色
    pub fn get_color_at(&self, x: f32, y: f32) -> Color {
        if self.stops.is_empty() {
            return Color::TRANSPARENT;
        }
        if self.stops.len() == 1 {
            return self.stops[0].1;
        }
        
        // 计算点在渐变线上的投影位置
        let dx = self.x1 - self.x0;
        let dy = self.y1 - self.y0;
        let len_sq = dx * dx + dy * dy;
        if len_sq == 0.0 {
            return self.stops[0].1;
        }
        
        let t = ((x - self.x0) * dx + (y - self.y0) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);
        
        self.interpolate_color(t)
    }
    
    fn interpolate_color(&self, t: f32) -> Color {
        if t <= self.stops[0].0 {
            return self.stops[0].1;
        }
        if t >= self.stops.last().unwrap().0 {
            return self.stops.last().unwrap().1;
        }
        
        for i in 0..self.stops.len() - 1 {
            let (t0, c0) = &self.stops[i];
            let (t1, c1) = &self.stops[i + 1];
            if t >= *t0 && t <= *t1 {
                let ratio = (t - t0) / (t1 - t0);
                return Color::new(
                    (c0.r as f32 + (c1.r as f32 - c0.r as f32) * ratio) as u8,
                    (c0.g as f32 + (c1.g as f32 - c0.g as f32) * ratio) as u8,
                    (c0.b as f32 + (c1.b as f32 - c0.b as f32) * ratio) as u8,
                    (c0.a as f32 + (c1.a as f32 - c0.a as f32) * ratio) as u8,
                );
            }
        }
        self.stops[0].1
    }
}


/// 径向渐变
#[derive(Clone)]
pub struct RadialGradient {
    x0: f32,
    y0: f32,
    r0: f32,
    x1: f32,
    y1: f32,
    r1: f32,
    stops: Vec<(f32, Color)>,
}

impl RadialGradient {
    pub fn new(x0: f32, y0: f32, r0: f32, x1: f32, y1: f32, r1: f32) -> Self {
        Self { x0, y0, r0, x1, y1, r1, stops: Vec::new() }
    }
    
    /// 添加颜色停止点
    pub fn add_color_stop(&mut self, offset: f32, color: &str) {
        if let Some(c) = parse_color_str(color) {
            self.stops.push((offset.clamp(0.0, 1.0), c));
            self.stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        }
    }
    
    /// 获取指定位置的颜色
    pub fn get_color_at(&self, x: f32, y: f32) -> Color {
        if self.stops.is_empty() {
            return Color::TRANSPARENT;
        }
        
        // 简化实现：使用到中心点的距离
        let dx = x - self.x1;
        let dy = y - self.y1;
        let dist = (dx * dx + dy * dy).sqrt();
        let t = if self.r1 > self.r0 {
            ((dist - self.r0) / (self.r1 - self.r0)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        
        self.interpolate_color(t)
    }
    
    fn interpolate_color(&self, t: f32) -> Color {
        if self.stops.is_empty() {
            return Color::TRANSPARENT;
        }
        if self.stops.len() == 1 {
            return self.stops[0].1;
        }
        if t <= self.stops[0].0 {
            return self.stops[0].1;
        }
        if t >= self.stops.last().unwrap().0 {
            return self.stops.last().unwrap().1;
        }
        
        for i in 0..self.stops.len() - 1 {
            let (t0, c0) = &self.stops[i];
            let (t1, c1) = &self.stops[i + 1];
            if t >= *t0 && t <= *t1 {
                let ratio = (t - t0) / (t1 - t0);
                return Color::new(
                    (c0.r as f32 + (c1.r as f32 - c0.r as f32) * ratio) as u8,
                    (c0.g as f32 + (c1.g as f32 - c0.g as f32) * ratio) as u8,
                    (c0.b as f32 + (c1.b as f32 - c0.b as f32) * ratio) as u8,
                    (c0.a as f32 + (c1.a as f32 - c0.a as f32) * ratio) as u8,
                );
            }
        }
        self.stops[0].1
    }
}


/// Canvas 上下文管理器 - 全局管理所有 canvas 实例
pub struct CanvasContextManager {
    contexts: HashMap<String, Canvas2DContext>,
    /// 每个 canvas 最近一次收到的指令流。元素真实尺寸与设备像素比只有布局后才知道，
    /// 而 `ctx.draw()` 一般发生在 onLoad —— 后备缓冲重建后要靠它把内容重画一遍。
    last_commands: HashMap<String, String>,
}

impl CanvasContextManager {
    pub fn new() -> Self {
        Self { contexts: HashMap::new(), last_commands: HashMap::new() }
    }

    /// 按元素的逻辑尺寸与设备像素比对齐后备缓冲；尺寸/DPR 变了就重建并重放指令。
    ///
    /// 没有这一步时上下文一律是写死的 400x300 逻辑缓冲，又被 1:1 拷进 2 倍分辨率的
    /// 页面画布 —— 内容只落在元素左上角的四分之一里（看起来就是「canvas 画的不居中」）。
    pub fn sync_backing(&mut self, canvas_id: &str, logical_w: u32, logical_h: u32, dpr: f32) {
        if logical_w == 0 || logical_h == 0 {
            return;
        }
        let needs_rebuild = match self.contexts.get(canvas_id) {
            Some(ctx) => {
                ctx.logical_size() != (logical_w, logical_h)
                    || (ctx.device_pixel_ratio() - dpr).abs() > 0.01
            }
            None => true,
        };
        if !needs_rebuild {
            return;
        }
        self.contexts.insert(
            canvas_id.to_string(),
            Canvas2DContext::new_with_dpr(canvas_id, logical_w, logical_h, dpr),
        );
        if let Some(commands) = self.last_commands.get(canvas_id).cloned() {
            self.execute_commands(canvas_id, &commands);
        }
    }
    
    /// 获取或创建 canvas 上下文
    pub fn get_context(&mut self, canvas_id: &str, width: u32, height: u32) -> &mut Canvas2DContext {
        self.contexts.entry(canvas_id.to_string())
            .or_insert_with(|| Canvas2DContext::new(canvas_id, width, height))
    }
    
    /// 获取已存在的上下文
    pub fn get_existing_context(&self, canvas_id: &str) -> Option<&Canvas2DContext> {
        self.contexts.get(canvas_id)
    }
    
    /// 移除上下文
    pub fn remove_context(&mut self, canvas_id: &str) {
        self.contexts.remove(canvas_id);
    }
    
    /// 清除所有上下文
    pub fn clear(&mut self) {
        self.contexts.clear();
    }
    
    /// 执行绘制命令
    pub fn execute_commands(&mut self, canvas_id: &str, commands_json: &str) {
        // 记住指令流：后备缓冲按元素真实尺寸重建后要重放一遍（见 `sync_backing`）
        if self.last_commands.get(canvas_id).map(|s| s.as_str()) != Some(commands_json) {
            self.last_commands.insert(canvas_id.to_string(), commands_json.to_string());
        }
        // 解析命令
        let commands: Vec<serde_json::Value> = serde_json::from_str(commands_json).unwrap_or_default();
        
        // 获取或创建上下文（元素尺寸未知时先给个较大的默认值，随后由 sync_backing 校正）
        let ctx = self.contexts.entry(canvas_id.to_string())
            .or_insert_with(|| Canvas2DContext::new(canvas_id, 400, 300));
        
        // 执行每个命令
        for cmd in commands {
            let cmd_type = cmd.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match cmd_type {
                "setFillStyle" => {
                    if let Some(color) = cmd.get("color").and_then(|v| v.as_str()) {
                        ctx.set_fill_style(color);
                    }
                }
                "setStrokeStyle" => {
                    if let Some(color) = cmd.get("color").and_then(|v| v.as_str()) {
                        ctx.set_stroke_style(color);
                    }
                }
                "setLineWidth" => {
                    if let Some(width) = cmd.get("width").and_then(|v| v.as_f64()) {
                        ctx.set_line_width(width as f32);
                    }
                }
                "fillRect" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let w = cmd.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let h = cmd.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.fill_rect(x, y, w, h);
                }
                "strokeRect" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let w = cmd.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let h = cmd.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.stroke_rect(x, y, w, h);
                }
                "clearRect" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let w = cmd.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let h = cmd.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.clear_rect(x, y, w, h);
                }
                "beginPath" => ctx.begin_path(),
                "closePath" => ctx.close_path(),
                "moveTo" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.move_to(x, y);
                }
                "lineTo" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.line_to(x, y);
                }
                "arc" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let r = cmd.get("r").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let s = cmd.get("sAngle").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let e = cmd.get("eAngle").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let cc = cmd.get("counterclockwise").and_then(|v| v.as_bool()).unwrap_or(false);
                    ctx.arc(x, y, r, s, e, cc);
                }
                "fill" => ctx.fill(),
                "stroke" => ctx.stroke(),
                "save" => ctx.save(),
                "restore" => ctx.restore(),
                "translate" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.translate(x, y);
                }
                "rotate" => {
                    let a = cmd.get("angle").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.rotate(a);
                }
                "scale" => {
                    let sx = cmd.get("scaleX").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                    let sy = cmd.get("scaleY").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                    ctx.scale(sx, sy);
                }
                "setFontSize" => {
                    if let Some(s) = cmd.get("size").and_then(|v| v.as_f64()) { ctx.font_size = s as f32; }
                }
                "setTextAlign" => {
                    if let Some(a) = cmd.get("align").and_then(|v| v.as_str()) { ctx.set_text_align(a); }
                }
                "setTextBaseline" => {
                    if let Some(b) = cmd.get("baseline").and_then(|v| v.as_str()) { ctx.set_text_baseline(b); }
                }
                "setGlobalAlpha" => {
                    if let Some(a) = cmd.get("alpha").and_then(|v| v.as_f64()) { ctx.set_global_alpha(a as f32); }
                }
                "setLineCap" => {
                    if let Some(c) = cmd.get("cap").and_then(|v| v.as_str()) { ctx.set_line_cap(c); }
                }
                "setLineJoin" => {
                    if let Some(j) = cmd.get("join").and_then(|v| v.as_str()) { ctx.set_line_join(j); }
                }
                "rect" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let w = cmd.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let h = cmd.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.rect(x, y, w, h);
                }
                "quadraticCurveTo" => {
                    let cpx = cmd.get("cpx").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let cpy = cmd.get("cpy").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.quadratic_curve_to(cpx, cpy, x, y);
                }
                "bezierCurveTo" => {
                    let a = |k: &str| cmd.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.bezier_curve_to(a("cp1x"), a("cp1y"), a("cp2x"), a("cp2y"), a("x"), a("y"));
                }
                "fillText" => {
                    let text = cmd.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.fill_text(text, x, y);
                }
                "strokeText" => {
                    let text = cmd.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.stroke_text(text, x, y);
                }
                "drawImage" => {
                    let src = cmd.get("src").and_then(|v| v.as_str()).unwrap_or("");
                    let dx = cmd.get("dx").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let dy = cmd.get("dy").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let dw = cmd.get("dWidth").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
                    let dh = cmd.get("dHeight").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
                    ctx.draw_image(src, dx, dy, dw, dh);
                }
                _ => {}
            }
        }
    }
}

impl Default for CanvasContextManager {
    fn default() -> Self {
        Self::new()
    }
}

// ========== Canvas 组件实现 ==========

use once_cell::sync::Lazy;

/// 全局 Canvas 上下文管理器
pub static CANVAS_MANAGER: Lazy<Mutex<CanvasContextManager>> = Lazy::new(|| {
    Mutex::new(CanvasContextManager::new())
});

/// 预创建指定尺寸的 canvas 上下文（用于静态渲染场景先建好画布再绘制）。
pub fn ensure_canvas_context(canvas_id: &str, width: u32, height: u32) {
    if let Ok(mut manager) = CANVAS_MANAGER.lock() {
        manager.get_context(canvas_id, width, height);
    }
}

/// 执行 Canvas 绘制命令（供外部调用）
pub fn execute_canvas_draw(canvas_id: &str, commands_json: &str) {
    println!("[Canvas] execute_canvas_draw: {} commands for '{}'", 
        commands_json.len(), canvas_id);
    if let Ok(mut manager) = CANVAS_MANAGER.lock() {
        manager.execute_commands(canvas_id, commands_json);
    }
}

impl CanvasComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        
        // 设置默认背景色为白色
        if ns.background_color.is_none() {
            ns.background_color = Some(Color::WHITE);
        }
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "canvas".into(),
            text: String::new(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    /// 绘制 canvas 组件
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        // 绘制背景
        draw_background(canvas, &node.style, x, y, w, h);
        
        // 获取 canvas-id
        let canvas_id = node.attrs.get("canvas-id").cloned().unwrap_or_default();
        
        // 从全局管理器获取绘制内容并复制到主 canvas
        if !canvas_id.is_empty() {
            if let Ok(mut manager) = CANVAS_MANAGER.lock() {
                // 先让后备缓冲与元素实际尺寸/设备像素比一致（必要时重建并重放指令），
                // 之后就能 1:1 拷贝：canvas 内容是原生分辨率，不做任何放大。
                let dpr = if sf.is_finite() && sf > 0.0 { sf } else { 1.0 };
                manager.sync_backing(
                    &canvas_id,
                    (w / dpr).round().max(0.0) as u32,
                    (h / dpr).round().max(0.0) as u32,
                    dpr,
                );
                if let Some(ctx) = manager.get_existing_context(&canvas_id) {
                    if let Ok(src_canvas) = ctx.get_canvas().lock() {
                        let src_w = src_canvas.width() as usize;
                        let src_h = src_canvas.height() as usize;
                        let src_pixels = src_canvas.pixels();
                        let dst_x = x.round() as i32;
                        let dst_y = y.round() as i32;
                        let copy_w = (w.round().max(0.0) as usize).min(src_w);
                        let copy_h = (h.round().max(0.0) as usize).min(src_h);

                        for sy in 0..copy_h {
                            let row = sy * src_w;
                            for sx in 0..copy_w {
                                let color = src_pixels[row + sx];
                                // set_pixel 走 alpha 混合：半透明绘制能正确压在元素背景上
                                if color.a > 0 {
                                    canvas.set_pixel(dst_x + sx as i32, dst_y + sy as i32, color);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // 绘制边框
        if node.style.border_width > 0.0 {
            let border_color = node.style.border_color.unwrap_or(Color::from_hex(0xE5E5E5));
            let paint = Paint::new()
                .with_color(border_color)
                .with_style(PaintStyle::Stroke)
                .with_stroke_width(node.style.border_width);
            let rect = GeoRect::new(x, y, w, h);
            canvas.draw_rect(&rect, &paint);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_fill_rect_produces_pixels() {
        let mut ctx = Canvas2DContext::new("t", 100, 100);
        ctx.set_fill_style("#ff0000");
        ctx.fill_rect(10.0, 10.0, 50.0, 50.0);
        let data = ctx.get_image_data();
        // 应存在红色像素
        let mut red = 0;
        for px in data.chunks(4) {
            if px[0] > 200 && px[1] < 60 && px[2] < 60 && px[3] > 200 { red += 1; }
        }
        assert!(red > 1000, "红色填充像素过少: {}", red);
    }

    #[test]
    fn test_canvas_transform_rotate_scale() {
        // 变换矩阵：translate + rotate + scale 组合后点变换正确
        let mut ctx = Canvas2DContext::new("t", 200, 200);
        ctx.translate(100.0, 100.0);
        ctx.rotate(std::f32::consts::FRAC_PI_2); // 90°
        let (x, y) = ctx.tp(10.0, 0.0);
        // (10,0) 绕原点转 90° -> (0,10)，再平移 (100,100) -> (100,110)
        assert!((x - 100.0).abs() < 0.5 && (y - 110.0).abs() < 0.5, "变换结果 =({},{})", x, y);
    }

    #[test]
    fn test_canvas_path_fill() {
        let mut ctx = Canvas2DContext::new("t", 100, 100);
        ctx.set_fill_style("#00ff00");
        ctx.begin_path();
        ctx.move_to(10.0, 10.0);
        ctx.line_to(90.0, 10.0);
        ctx.line_to(50.0, 90.0);
        ctx.close_path();
        ctx.fill();
        let data = ctx.get_image_data();
        let mut green = 0;
        for px in data.chunks(4) {
            if px[1] > 200 && px[0] < 60 && px[3] > 200 { green += 1; }
        }
        assert!(green > 500, "三角形填充像素过少: {}", green);
    }

    #[test]
    fn test_canvas_execute_commands_json() {
        let mut mgr = CanvasContextManager::new();
        let cmds = r##"[
            {"type":"setFillStyle","color":"#0000ff"},
            {"type":"fillRect","x":0,"y":0,"width":40,"height":40},
            {"type":"save"},{"type":"translate","x":50,"y":50},{"type":"rotate","angle":0.5},
            {"type":"setFillStyle","color":"#ff0000"},
            {"type":"fillRect","x":-10,"y":-10,"width":20,"height":20},
            {"type":"restore"}
        ]"##;
        mgr.execute_commands("c", cmds);
        let ctx = mgr.get_existing_context("c").unwrap();
        let data = ctx.get_image_data();
        assert!(data.iter().any(|&b| b > 0), "命令执行后画布应有内容");
    }
}
