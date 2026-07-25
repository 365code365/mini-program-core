//! switch 组件 - 开关选择器
//! 
//! 支持完整的 CSS 样式，同时保留微信默认样式作为 fallback
//! 属性：
//! - checked: 是否选中
//! - disabled: 是否禁用
//! - type: switch(默认) / checkbox
//! - color: 选中时的颜色
//! 
//! CSS 支持：
//! - width/height: 自定义尺寸
//! - background-color: 自定义轨道背景色
//! - border-radius: 自定义圆角
//! - opacity: 透明度
//! - box-shadow: 阴影

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::{Canvas, Color, Paint, PaintStyle, Path};
use taffy::prelude::*;

/// 微信 switch 默认轨道尺寸（weui：52x32，1px 边框，滑块 30）
pub const SWITCH_WIDTH: f32 = 52.0;
pub const SWITCH_HEIGHT: f32 = 32.0;
/// 关态轨道底色 / 边框色
const TRACK_OFF: u32 = 0xDFDFDF;
/// 关态内部填充（weui 用一个略小的白色胶囊盖住轨道，开启时缩到 0）
const TRACK_INNER: u32 = 0xFDFDFD;
/// switch 默认开启色（微信文档默认值）
const DEFAULT_ON: u32 = 0x04BE02;

pub struct SwitchComponent;

impl SwitchComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let checked = node.get_attr("checked").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let switch_type = node.get_attr("type").unwrap_or("switch");
        
        // 检查 CSS 是否定义了尺寸和颜色
        let has_custom_size = !matches!(ts.size.width, Dimension::Auto) || 
                              !matches!(ts.size.height, Dimension::Auto);
        let has_custom_radius = ns.border_radius > 0.0;
        
        // 微信（weui）官方 switch 度量：轨道 52x32、1px 边框、滑块直径 30。
        // 之前用的 51x31 + 直径 27 让滑块在轨道里"缩了一圈"，缺少微信那种
        // 滑块几乎填满轨道的贴合感。
        let (default_width, default_height) = if switch_type == "checkbox" {
            (24.0, 24.0)
        } else {
            (SWITCH_WIDTH, SWITCH_HEIGHT)
        };
        
        if !has_custom_size {
            ts.size = Size { width: length(default_width * sf), height: length(default_height * sf) };
        }
        // 开关不参与 flex 伸缩（对齐 HTML .wx-switch{flex:none}），否则会被兄弟节点压扁
        ts.flex_shrink = 0.0;
        ts.flex_grow = 0.0;
        
        // 只在 CSS 没有定义时使用默认圆角
        if !has_custom_radius {
            ns.border_radius = if switch_type == "checkbox" { 
                4.0 * sf 
            } else { 
                default_height * sf / 2.0 
            };
        }
        
        // custom_data 存「开启进度」：0=关、1=开，中间值用于点击后的滑动过渡。
        // 轨道/滑块颜色都在绘制期按进度求值，这样运行时切换也能拿到正确配色
        // （此前一旦交互层改写 background_color，关态就变成纯白而不是微信的浅灰）。
        ns.custom_data = if checked { 1.0 } else { 0.0 };
        
        let text = switch_type.to_string();
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "switch".into(),
            text,
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        let style = &node.style;
        let switch_type = node.text.as_str();
        
        // 绘制盒子阴影
        if let Some(shadow) = &style.box_shadow {
            draw_box_shadow(canvas, shadow, x, y, w, h, style.border_radius);
        }
        
        if switch_type == "checkbox" {
            Self::draw_checkbox(canvas, style, style.custom_data > 0.5, x, y, w, h, sf);
        } else {
            let progress = style.custom_data.clamp(0.0, 1.0);
            let on_color = node.attrs.get("color").and_then(|c| parse_color_str(c))
                .unwrap_or(Color::from_hex(DEFAULT_ON));
            let disabled = node.attrs.get("disabled")
                .map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
            Self::draw_switch(canvas, style, progress, on_color, disabled, x, y, w, h, sf);
        }
    }
    
    /// 按微信（weui）结构绘制开关：
    /// 轨道（关态浅灰边框 + 内部白色胶囊，开态整体填充主色）+ 带投影的白色滑块。
    /// `progress` 为 0..1 的开启进度，点击后由交互层按缓动推进，形成滑动手感。
    fn draw_switch(
        canvas: &mut Canvas,
        style: &NodeStyle,
        progress: f32,
        on_color: Color,
        disabled: bool,
        x: f32, y: f32, w: f32, h: f32,
        sf: f32,
    ) {
        let base_opacity = style.opacity * if disabled { 0.45 } else { 1.0 };
        let apply_opacity = |color: Color| -> Color {
            if base_opacity < 1.0 {
                Color::new(color.r, color.g, color.b, (color.a as f32 * base_opacity) as u8)
            } else {
                color
            }
        };
        let lerp = |a: u8, b: u8| -> u8 {
            (a as f32 + (b as f32 - a as f32) * progress).round().clamp(0.0, 255.0) as u8
        };
        
        let radius = if style.border_radius > 0.0 { style.border_radius.min(h / 2.0) } else { h / 2.0 };
        let fill_round = |canvas: &mut Canvas, rx: f32, ry: f32, rw: f32, rh: f32, r: f32, color: Color| {
            if rw <= 0.0 || rh <= 0.0 { return; }
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
            let mut path = Path::new();
            path.add_round_rect(rx, ry, rw, rh, r.min(rw / 2.0).min(rh / 2.0));
            canvas.draw_path(&path, &paint);
        };
        
        // 轨道：CSS 指定过背景就尊重 CSS，否则关态浅灰 → 开态主色渐变
        let off_track = Color::from_hex(TRACK_OFF);
        let track = style.background_color.unwrap_or_else(|| {
            Color::new(lerp(off_track.r, on_color.r), lerp(off_track.g, on_color.g), lerp(off_track.b, on_color.b), 255)
        });
        fill_round(canvas, x, y, w, h, radius, apply_opacity(track));
        
        // 关态内部白色胶囊（weui 的 ::before，开启时 scale 到 0）
        if style.background_color.is_none() {
            let shrink = (1.0 - progress).clamp(0.0, 1.0);
            if shrink > 0.01 {
                let border = 1.0 * sf;
                let (iw, ih) = ((w - border * 2.0) * shrink, (h - border * 2.0) * shrink);
                let ix = x + (w - iw) / 2.0;
                let iy = y + (h - ih) / 2.0;
                fill_round(canvas, ix, iy, iw, ih, ih / 2.0, apply_opacity(Color::from_hex(TRACK_INNER)));
            }
        }
        
        // 滑块：直径 = 轨道高 - 2*边框，左右各留 1px（微信是几乎填满轨道的）
        let inset = 1.0 * sf;
        let knob_radius = (h - inset * 2.0) / 2.0;
        let knob_y = y + h / 2.0;
        let left_x = x + inset + knob_radius;
        let right_x = x + w - inset - knob_radius;
        let knob_x = left_x + (right_x - left_x) * progress;
        
        // 投影：0 1px 3px rgba(0,0,0,.4) 的近似（三层递减不透明度的圆）
        for (i, alpha) in [(3.0, 26u8), (2.0, 34), (1.0, 44)] {
            let paint = Paint::new()
                .with_color(Color::new(0, 0, 0, (alpha as f32 * base_opacity) as u8))
                .with_style(PaintStyle::Fill)
                .with_anti_alias(true);
            canvas.draw_circle(knob_x, knob_y + 1.0 * sf, knob_radius + i * sf * 0.5, &paint);
        }
        
        let knob_paint = Paint::new().with_color(apply_opacity(Color::WHITE)).with_style(PaintStyle::Fill).with_anti_alias(true);
        canvas.draw_circle(knob_x, knob_y, knob_radius, &knob_paint);
    }
    
    fn draw_checkbox(canvas: &mut Canvas, style: &NodeStyle, checked: bool, x: f32, y: f32, w: f32, h: f32, _sf: f32) {
        // 应用透明度
        let apply_opacity = |color: Color| -> Color {
            if style.opacity < 1.0 {
                Color::new(color.r, color.g, color.b, (color.a as f32 * style.opacity) as u8)
            } else {
                color
            }
        };
        
        // 获取圆角值
        let radius_tl = style.border_radius_tl.unwrap_or(style.border_radius);
        let radius_tr = style.border_radius_tr.unwrap_or(style.border_radius);
        let radius_br = style.border_radius_br.unwrap_or(style.border_radius);
        let radius_bl = style.border_radius_bl.unwrap_or(style.border_radius);
        let uniform_radius = radius_tl == radius_tr && radius_tr == radius_br && radius_br == radius_bl;
        let radius = radius_tl;
        
        // 绘制背景 - 使用抗锯齿
        if let Some(bg) = style.background_color {
            let paint = Paint::new().with_color(apply_opacity(bg)).with_style(PaintStyle::Fill).with_anti_alias(true);
            let mut path = Path::new();
            if uniform_radius {
                path.add_round_rect(x, y, w, h, radius);
            } else {
                path.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
            }
            canvas.draw_path(&path, &paint);
        }
        
        // 绘制边框
        if !checked {
            let border_width = 2.0;
            let border_paint = Paint::new()
                .with_color(Color::from_hex(0xD1D1D1))
                .with_style(PaintStyle::Fill)
                .with_anti_alias(true);
            let mut border = Path::new();
            if uniform_radius {
                border.add_round_rect(x, y, w, h, radius);
            } else {
                border.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
            }
            canvas.draw_path(&border, &border_paint);
            
            // 内部白色
            let inner_paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill).with_anti_alias(true);
            let mut inner = Path::new();
            let inner_radius = |r: f32| (r - border_width).max(0.0);
            if uniform_radius {
                inner.add_round_rect(
                    x + border_width, 
                    y + border_width, 
                    w - border_width * 2.0, 
                    h - border_width * 2.0, 
                    inner_radius(radius)
                );
            } else {
                inner.add_round_rect_varying(
                    x + border_width, 
                    y + border_width, 
                    w - border_width * 2.0, 
                    h - border_width * 2.0, 
                    inner_radius(radius_tl), inner_radius(radius_tr),
                    inner_radius(radius_br), inner_radius(radius_bl)
                );
            }
            canvas.draw_path(&inner, &inner_paint);
        }
        
        // 绘制对勾 - 使用粗线条
        if checked {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let thickness = w * 0.12;
            
            let p1 = (cx - w * 0.25, cy);
            let p2 = (cx - w * 0.05, cy + h * 0.2);
            let p3 = (cx + w * 0.25, cy - h * 0.2);
            
            let paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill).with_anti_alias(true);
            let half = thickness / 2.0;
            
            // 第一段
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
            
            // 第二段
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
            
            // 拐点圆形
            canvas.draw_circle(p2.0, p2.1, half, &paint);
        }
    }
}
