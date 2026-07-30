//! slider 组件 - 滑动选择器
//! 
//! 支持完整的 CSS 样式，同时保留微信默认样式作为 fallback
//! 属性：
//! - min: 最小值，默认 0
//! - max: 最大值，默认 100
//! - step: 步长，默认 1
//! - value: 当前值
//! - activeColor: 已选择的颜色
//! - backgroundColor: 背景条颜色
//! - block-size: 滑块大小，默认 28
//! - block-color: 滑块颜色
//! - show-value: 是否显示当前值
//! 
//! CSS 支持：
//! - width/height: 自定义尺寸
//! - background-color: 自定义轨道背景色（覆盖 backgroundColor 属性）
//! - color: 自定义已选择颜色（覆盖 activeColor 属性）
//! - border-radius: 自定义轨道圆角
//! - opacity: 透明度
//! - box-shadow: 阴影

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path};
use taffy::prelude::*;

pub struct SliderComponent;

impl SliderComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let value = node.get_attr("value").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
        let min = node.get_attr("min").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
        let max = node.get_attr("max").and_then(|s| s.parse::<f32>().ok()).unwrap_or(100.0);
        let block_size = node.get_attr("block-size").and_then(|s| s.parse::<f32>().ok()).unwrap_or(28.0);
        let show_value = node.get_attr("show-value").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        
        // 检查 CSS 是否定义了颜色
        let has_custom_bg = ns.background_color.is_some();
        let has_custom_active = ns.text_color.is_some();
        let has_custom_block = ns.border_color.is_some();
        let has_custom_height = !dim_is_auto(ts.size.height);
        
        // 只在 CSS 没有定义时使用属性值或默认值
        if !has_custom_bg {
            let bg_color = node.get_attr("backgroundColor").or(node.get_attr("background-color"))
                .and_then(|c| parse_color_str(c)).unwrap_or(Color::from_hex(0xE9E9E9));
            ns.background_color = Some(bg_color);
        }
        
        if !has_custom_active {
            let active_color = node.get_attr("activeColor").or(node.get_attr("active-color"))
                .and_then(|c| parse_color_str(c)).unwrap_or(Color::from_hex(0x1AAD19));
            ns.text_color = Some(active_color);
        }
        
        if !has_custom_block {
            let block_color = node.get_attr("block-color").and_then(|c| parse_color_str(c))
                .unwrap_or(Color::WHITE);
            ns.border_color = Some(block_color);
        }
        
        // 滑块高度
        if !has_custom_height {
            let height = block_size * sf;
            ts.size.height = length(height);
        }
        
        // 默认宽度 100%
        if dim_is_auto(ts.size.width) {
            ts.size.width = percent(1.0_f32);
        }
        
        ts.flex_direction = FlexDirection::Row;
        if ts.align_items.is_none() {
            ts.align_items = Some(AlignItems::CENTER);
        }
        
        ns.custom_data = ((value - min) / (max - min)).clamp(0.0, 1.0);
        ns.border_width = block_size;
        ns.font_size = value;
        
        let text = if show_value { format!("{}", value as i32) } else { String::new() };
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "slider".into(),
            text,
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        sf: f32
    ) {
        let style = &node.style;
        let progress = style.custom_data;
        let block_size = style.border_width * sf;
        let show_value = !node.text.is_empty();
        
        // 绘制盒子阴影
        if let Some(shadow) = &style.box_shadow {
            draw_box_shadow(canvas, shadow, x, y, w, h, style.border_radius);
        }
        
        // 应用透明度
        let apply_opacity = |color: Color| -> Color {
            if style.opacity < 1.0 {
                Color::new(color.r, color.g, color.b, (color.a as f32 * style.opacity) as u8)
            } else {
                color
            }
        };
        
        // 计算轨道区域 - 预留更多空间给数值显示
        let value_width = if show_value { 50.0 * sf } else { 0.0 };
        let track_width = w - value_width - block_size;
        let track_height = 4.0 * sf;
        let track_x = x + block_size / 2.0;
        let track_y = y + (h - track_height) / 2.0;
        
        // 获取轨道圆角
        let track_radius = if style.border_radius > 0.0 {
            style.border_radius.min(track_height / 2.0)
        } else {
            track_height / 2.0
        };
        
        // 绘制背景轨道 - 使用抗锯齿
        if let Some(bg) = style.background_color {
            let paint = Paint::new().with_color(apply_opacity(bg)).with_style(PaintStyle::Fill).with_anti_alias(true);
            let mut path = Path::new();
            path.add_round_rect(track_x, track_y, track_width, track_height, track_radius);
            canvas.draw_path(&path, &paint);
        }
        
        // 绘制已选择部分 - 使用抗锯齿
        let active_width = track_width * progress;
        if active_width > 0.0 {
            if let Some(active) = style.text_color {
                let paint = Paint::new().with_color(apply_opacity(active)).with_style(PaintStyle::Fill).with_anti_alias(true);
                let mut path = Path::new();
                path.add_round_rect(track_x, track_y, active_width, track_height, track_radius);
                canvas.draw_path(&path, &paint);
            }
        }
        
        // 绘制滑块
        let knob_x = track_x + active_width;
        let knob_y = y + h / 2.0;
        let knob_radius = block_size / 2.0;
        
        // 滑块阴影 - 使用抗锯齿
        let shadow_paint = Paint::new()
            .with_color(Color::new(0, 0, 0, 40))
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        canvas.draw_circle(knob_x, knob_y + 2.0 * sf, knob_radius, &shadow_paint);
        
        // 滑块本体 - 使用抗锯齿
        let block_color = style.border_color.unwrap_or(Color::WHITE);
        let knob_paint = Paint::new().with_color(apply_opacity(block_color)).with_style(PaintStyle::Fill).with_anti_alias(true);
        canvas.draw_circle(knob_x, knob_y, knob_radius, &knob_paint);
        
        // 滑块边框 - 使用填充方式绘制圆环
        let border_width = 1.0 * sf;
        let border_paint = Paint::new()
            .with_color(Color::from_hex(0xE9E9E9))
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        canvas.draw_circle(knob_x, knob_y, knob_radius, &border_paint);
        
        let inner_paint = Paint::new().with_color(apply_opacity(block_color)).with_style(PaintStyle::Fill).with_anti_alias(true);
        canvas.draw_circle(knob_x, knob_y, knob_radius - border_width, &inner_paint);
        
        // 绘制数值
        if show_value {
            if let Some(tr) = text_renderer {
                let font_size = 14.0 * sf;
                let text_width = tr.measure_text(&node.text, font_size);
                // 右对齐数值，确保不超出容器
                let text_x = x + w - text_width - 4.0 * sf;
                let text_y = y + (h + font_size) / 2.0 - 2.0 * sf;
                let paint = Paint::new().with_color(Color::from_hex(0x888888)).with_style(PaintStyle::Fill);
                tr.draw_text(canvas, &node.text, text_x, text_y, font_size, &paint);
            }
        }
    }
}
