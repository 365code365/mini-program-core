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

// ── `Canvas2DContext` 的方法按职责分片（都是 `impl Canvas2DContext`）──
/// 上下文状态、变换、样式
mod state;
/// 基本图形与图片
mod draw2d;
/// 路径构建
mod path2d;
/// 文字与渐变
mod text2d;

/// 渐变对象（线性 / 径向）
mod gradient;
/// 画布上下文管理器与命令回放
mod manager;
pub use gradient::{LinearGradient, RadialGradient};
pub use manager::{ensure_canvas_context, execute_canvas_draw, CanvasContextManager, CANVAS_MANAGER};

impl CanvasComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        // 替换元素：作者写死的尺寸就是它的最小尺寸，别被兄弟压没。
        // 必须在合成默认尺寸**之前**调用 —— CSS 语义里「指定尺寸」只算作者写的那个，
        // 引擎给 textarea 之类补的默认高度不算，那种情况仍应允许被父级压缩。
        super::pin_replaced_min_size(&mut ts);
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
