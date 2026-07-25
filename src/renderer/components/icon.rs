//! icon 组件 - 图标
//! 
//! 支持完整的 CSS 样式，同时保留微信默认样式作为 fallback
//! 微信官方图标类型：
//! - success: 成功（绿色对勾）
//! - success_no_circle: 成功无圆圈
//! - info: 信息（蓝色）
//! - warn: 警告（红色）
//! - waiting: 等待（蓝色）
//! - cancel: 取消（红色X）
//! - download: 下载
//! - search: 搜索
//! - clear: 清除
//! 
//! CSS 支持：
//! - width/height: 自定义尺寸（覆盖 size 属性）
//! - color: 自定义颜色（覆盖 color 属性）
//! - opacity: 透明度
//! - box-shadow: 阴影

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::{Canvas, Color, Paint, PaintStyle, Path};
use taffy::prelude::*;

pub struct IconComponent;

impl IconComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let icon_type = node.get_attr("type").unwrap_or("success");
        let icon_size = node.get_attr("size").and_then(|s| s.parse::<f32>().ok()).unwrap_or(23.0);
        let icon_color = node.get_attr("color").and_then(|c| parse_color_str(c));
        
        // 检查 CSS 是否定义了尺寸
        let has_custom_size = !matches!(ts.size.width, Dimension::Auto) || 
                              !matches!(ts.size.height, Dimension::Auto);
        
        // 只在 CSS 没有定义尺寸时使用 size 属性
        if !has_custom_size {
            let size = icon_size * sf;
            ts.size = Size { width: length(size), height: length(size) };
        }
        // 图标不参与 flex 压缩/拉伸（与 HTML 端 .wxicon{flex:none} 一致），
        // 否则同列的兄弟节点一有溢出就会把图标压扁。
        ts.flex_shrink = 0.0;
        ts.flex_grow = 0.0;
        
        // 颜色优先级：WXML color 属性 > CSS color > 默认颜色
        if let Some(color) = icon_color {
            // WXML 属性颜色优先
            ns.text_color = Some(color);
        } else if ns.text_color.is_none() {
            // 没有 CSS 颜色时使用默认颜色
            let default_color = match icon_type {
                "success" | "success_no_circle" => Color::from_hex(0x09BB07),
                "info" | "info_circle" => Color::from_hex(0x10AEFF),
                "warn" => Color::from_hex(0xF76260),
                "waiting" | "waiting_circle" => Color::from_hex(0x10AEFF),
                "cancel" | "clear" => Color::from_hex(0xF43530),
                "download" => Color::from_hex(0x09BB07),
                "search" => Color::from_hex(0xB2B2B2),
                _ => Color::from_hex(0x09BB07),
            };
            ns.text_color = Some(default_color);
        }
        
        ns.font_size = icon_size;
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "icon".into(),
            text: icon_type.into(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, _sf: f32) {
        let style = &node.style;
        
        // 绘制盒子阴影
        if let Some(shadow) = &style.box_shadow {
            draw_box_shadow(canvas, shadow, x, y, w, h, 0.0);
        }
        
        // 应用透明度
        let apply_opacity = |color: Color| -> Color {
            if style.opacity < 1.0 {
                Color::new(color.r, color.g, color.b, (color.a as f32 * style.opacity) as u8)
            } else {
                color
            }
        };
        
        let color = apply_opacity(style.text_color.unwrap_or(Color::from_hex(0x09BB07)));
        let icon_type = node.text.as_str();
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        let r = w.min(h) / 2.0;
        // 线条粗细根据图标大小调整
        let stroke_width = (r * 0.12).max(2.0);
        
        match icon_type {
            "success" => Self::draw_success(canvas, cx, cy, r, color, true, stroke_width),
            "success_no_circle" => Self::draw_success(canvas, cx, cy, r, color, false, stroke_width),
            "info" | "info_circle" => Self::draw_info(canvas, cx, cy, r, color, stroke_width),
            "warn" => Self::draw_warn(canvas, cx, cy, r, color, stroke_width),
            "waiting" | "waiting_circle" => Self::draw_waiting(canvas, cx, cy, r, color, stroke_width),
            "cancel" | "clear" => Self::draw_cancel(canvas, cx, cy, r, color, stroke_width),
            "download" => Self::draw_download(canvas, cx, cy, r, color, stroke_width),
            "search" => Self::draw_search(canvas, cx, cy, r, color, stroke_width),
            "circle" => Self::draw_circle_icon(canvas, cx, cy, r, color, stroke_width),
            "star" => Self::draw_star(canvas, cx, cy, r, color, true, stroke_width),
            "star-o" | "star_o" => Self::draw_star(canvas, cx, cy, r, color, false, stroke_width),
            "heart" => Self::draw_heart(canvas, cx, cy, r, color),
            _ => Self::draw_success(canvas, cx, cy, r, color, true, stroke_width),
        }
    }
    
    /// 绘制五角星（filled=true 实心，否则描边）
    fn draw_star(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color, filled: bool, stroke_width: f32) {
        use std::f32::consts::PI;
        let outer = r;
        let inner = r * 0.42;
        let mut path = Path::new();
        for i in 0..10 {
            let ang = -PI / 2.0 + i as f32 * PI / 5.0; // 从正上方开始
            let rad = if i % 2 == 0 { outer } else { inner };
            let px = cx + rad * ang.cos();
            let py = cy + rad * ang.sin();
            if i == 0 { path.move_to(px, py); } else { path.line_to(px, py); }
        }
        path.close();
        let style = if filled { PaintStyle::Fill } else { PaintStyle::Stroke };
        let mut paint = Paint::new().with_color(color).with_style(style).with_anti_alias(true);
        paint.stroke_width = stroke_width;
        canvas.draw_path(&path, &paint);
    }
    
    /// 绘制爱心
    fn draw_heart(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color) {
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
        // 两个圆 + 一个三角近似爱心
        let cr = r * 0.5;
        canvas.draw_circle(cx - cr * 0.6, cy - cr * 0.35, cr, &paint);
        canvas.draw_circle(cx + cr * 0.6, cy - cr * 0.35, cr, &paint);
        let mut tri = Path::new();
        tri.move_to(cx - r * 0.92, cy - r * 0.05);
        tri.line_to(cx + r * 0.92, cy - r * 0.05);
        tri.line_to(cx, cy + r * 0.9);
        tri.close();
        canvas.draw_path(&tri, &paint);
    }
    
    /// 绘制空心圆圈图标
    fn draw_circle_icon(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color, stroke_width: f32) {
        // 绘制空心圆圈
        let paint = Paint::new()
            .with_color(color)
            .with_style(PaintStyle::Stroke)
            .with_anti_alias(true);
        canvas.draw_circle(cx, cy, r - stroke_width, &paint);
    }
    
    fn draw_success(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color, with_circle: bool, stroke_width: f32) {
        if with_circle {
            // 绘制圆形背景 - 使用抗锯齿
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
            canvas.draw_circle(cx, cy, r, &paint);
            
            // 绘制白色对勾
            Self::draw_thick_check(canvas, cx, cy, r, Color::WHITE, stroke_width * 1.2);
        } else {
            // 只绘制对勾
            Self::draw_thick_check(canvas, cx, cy, r * 0.9, color, stroke_width * 1.5);
        }
    }
    
    /// 绘制粗对勾（使用填充多边形）
    fn draw_thick_check(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color, thickness: f32) {
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
        
        // 对勾的三个关键点
        let p1 = (cx - r * 0.35, cy);           // 左端点
        let p2 = (cx - r * 0.05, cy + r * 0.3); // 拐点
        let p3 = (cx + r * 0.35, cy - r * 0.25); // 右端点
        
        let half = thickness / 2.0;
        
        // 绘制第一段（左到中）
        let angle1 = ((p2.1 - p1.1) / (p2.0 - p1.0)).atan();
        let dx1 = half * angle1.sin();
        let dy1 = half * angle1.cos();
        
        let mut seg1 = Path::new();
        seg1.move_to(p1.0 - dx1, p1.1 + dy1);
        seg1.line_to(p1.0 + dx1, p1.1 - dy1);
        seg1.line_to(p2.0 + dx1, p2.1 - dy1);
        seg1.line_to(p2.0 - dx1, p2.1 + dy1);
        seg1.close();
        canvas.draw_path(&seg1, &paint);
        
        // 绘制第二段（中到右）
        let angle2 = ((p3.1 - p2.1) / (p3.0 - p2.0)).atan();
        let dx2 = half * angle2.sin();
        let dy2 = half * angle2.cos();
        
        let mut seg2 = Path::new();
        seg2.move_to(p2.0 - dx2, p2.1 + dy2);
        seg2.line_to(p2.0 + dx2, p2.1 - dy2);
        seg2.line_to(p3.0 + dx2, p3.1 - dy2);
        seg2.line_to(p3.0 - dx2, p3.1 + dy2);
        seg2.close();
        canvas.draw_path(&seg2, &paint);
        
        // 绘制拐点圆形，使连接更平滑
        canvas.draw_circle(p2.0, p2.1, half, &paint);
    }
    
    fn draw_info(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color, _stroke_width: f32) {
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
        
        // 绘制圆形背景
        canvas.draw_circle(cx, cy, r, &paint);
        
        // 绘制白色 "i"
        let white = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill).with_anti_alias(true);
        
        // 点
        canvas.draw_circle(cx, cy - r * 0.35, r * 0.14, &white);
        
        // 竖线 - 圆角矩形
        let bar_w = r * 0.18;
        let bar_h = r * 0.55;
        let bar_x = cx - bar_w / 2.0;
        let bar_y = cy - r * 0.05;
        
        let mut bar = Path::new();
        bar.add_round_rect(bar_x, bar_y, bar_w, bar_h, bar_w / 2.0);
        canvas.draw_path(&bar, &white);
    }
    
    fn draw_warn(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color, _stroke_width: f32) {
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
        
        // 绘制圆形背景
        canvas.draw_circle(cx, cy, r, &paint);
        
        // 绘制白色 "!"
        let white = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill).with_anti_alias(true);
        
        // 竖线 - 圆角矩形
        let bar_w = r * 0.18;
        let bar_h = r * 0.55;
        let bar_x = cx - bar_w / 2.0;
        let bar_y = cy - r * 0.5;
        
        let mut bar = Path::new();
        bar.add_round_rect(bar_x, bar_y, bar_w, bar_h, bar_w / 2.0);
        canvas.draw_path(&bar, &white);
        
        // 点
        canvas.draw_circle(cx, cy + r * 0.38, r * 0.14, &white);
    }
    
    fn draw_waiting(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color, stroke_width: f32) {
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
        
        // 绘制圆形背景
        canvas.draw_circle(cx, cy, r, &paint);
        
        // 绘制白色时钟指针
        let white = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill).with_anti_alias(true);
        let line_w = stroke_width * 1.2;
        
        // 垂直指针（12点方向）
        let mut v_line = Path::new();
        v_line.add_round_rect(cx - line_w / 2.0, cy - r * 0.45, line_w, r * 0.45, line_w / 2.0);
        canvas.draw_path(&v_line, &white);
        
        // 水平指针（3点方向）
        let mut h_line = Path::new();
        h_line.add_round_rect(cx, cy - line_w / 2.0, r * 0.32, line_w, line_w / 2.0);
        canvas.draw_path(&h_line, &white);
        
        // 中心圆点
        canvas.draw_circle(cx, cy, line_w * 0.8, &white);
    }
    
    fn draw_cancel(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color, stroke_width: f32) {
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
        
        // 绘制圆形背景
        canvas.draw_circle(cx, cy, r, &paint);
        
        // 绘制白色 X
        Self::draw_thick_x(canvas, cx, cy, r * 0.35, Color::WHITE, stroke_width * 1.3);
    }
    
    /// 绘制粗 X
    fn draw_thick_x(canvas: &mut Canvas, cx: f32, cy: f32, offset: f32, color: Color, thickness: f32) {
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
        let half = thickness / 2.0;
        
        // 左上到右下
        let mut line1 = Path::new();
        line1.move_to(cx - offset - half, cy - offset);
        line1.line_to(cx - offset, cy - offset - half);
        line1.line_to(cx + offset + half, cy + offset);
        line1.line_to(cx + offset, cy + offset + half);
        line1.close();
        canvas.draw_path(&line1, &paint);
        
        // 右上到左下
        let mut line2 = Path::new();
        line2.move_to(cx + offset, cy - offset - half);
        line2.line_to(cx + offset + half, cy - offset);
        line2.line_to(cx - offset, cy + offset + half);
        line2.line_to(cx - offset - half, cy + offset);
        line2.close();
        canvas.draw_path(&line2, &paint);
    }
    
    fn draw_download(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color, stroke_width: f32) {
        let stroke_paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
        
        // 外圆
        canvas.draw_circle(cx, cy, r, &stroke_paint);
        
        // 内圆（挖空）
        let inner_r = r - stroke_width * 1.2;
        let bg_paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill).with_anti_alias(true);
        canvas.draw_circle(cx, cy, inner_r, &bg_paint);
        
        // 绘制下载箭头
        let arrow_w = stroke_width * 1.2;
        
        // 垂直线
        let mut v_line = Path::new();
        v_line.add_round_rect(cx - arrow_w / 2.0, cy - r * 0.4, arrow_w, r * 0.5, arrow_w / 2.0);
        canvas.draw_path(&v_line, &stroke_paint);
        
        // 箭头三角形
        let mut arrow = Path::new();
        arrow.move_to(cx, cy + r * 0.35);
        arrow.line_to(cx - r * 0.28, cy);
        arrow.line_to(cx + r * 0.28, cy);
        arrow.close();
        canvas.draw_path(&arrow, &stroke_paint);
        
        // 底部横线
        let mut h_line = Path::new();
        h_line.add_round_rect(cx - r * 0.35, cy + r * 0.45, r * 0.7, arrow_w, arrow_w / 2.0);
        canvas.draw_path(&h_line, &stroke_paint);
    }
    
    fn draw_search(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color, stroke_width: f32) {
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
        
        // 放大镜参数
        let lens_r = r * 0.45;
        let lens_cx = cx - r * 0.12;
        let lens_cy = cy - r * 0.12;
        let ring_width = stroke_width * 1.5;
        
        // 绘制放大镜圆环（外圆 - 内圆）
        canvas.draw_circle(lens_cx, lens_cy, lens_r, &paint);
        
        let bg_paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill).with_anti_alias(true);
        canvas.draw_circle(lens_cx, lens_cy, lens_r - ring_width, &bg_paint);
        
        // 绘制手柄
        let handle_len = r * 0.45;
        let handle_w = ring_width;
        let angle = std::f32::consts::PI / 4.0; // 45度
        
        let start_x = lens_cx + lens_r * 0.7;
        let start_y = lens_cy + lens_r * 0.7;
        
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let half_w = handle_w / 2.0;
        
        let mut handle = Path::new();
        handle.move_to(start_x - half_w * cos_a, start_y + half_w * sin_a);
        handle.line_to(start_x + half_w * cos_a, start_y - half_w * sin_a);
        handle.line_to(start_x + handle_len * sin_a + half_w * cos_a, start_y + handle_len * cos_a - half_w * sin_a);
        handle.line_to(start_x + handle_len * sin_a - half_w * cos_a, start_y + handle_len * cos_a + half_w * sin_a);
        handle.close();
        canvas.draw_path(&handle, &paint);
        
        // 手柄末端圆角
        canvas.draw_circle(
            start_x + handle_len * sin_a, 
            start_y + handle_len * cos_a, 
            half_w,
            &paint
        );
    }
}
