//! Picker 选择器组件

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Rect as GeoRect};
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

/// Picker 状态管理器
pub struct PickerStateManager {
    states: HashMap<String, PickerState>,
}

#[derive(Clone)]
pub struct PickerState {
    pub value: usize,
    pub range: Vec<String>,
    pub mode: PickerMode,
    pub visible: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PickerMode {
    Selector,    // 普通选择器
    MultiSelector, // 多列选择器
    Time,        // 时间选择器
    Date,        // 日期选择器
    Region,      // 省市区选择器
}

impl Default for PickerMode {
    fn default() -> Self {
        PickerMode::Selector
    }
}

impl PickerStateManager {
    pub fn new() -> Self {
        Self { states: HashMap::new() }
    }
    
    pub fn get_or_create(&mut self, id: &str, range: Vec<String>, mode: PickerMode) -> &mut PickerState {
        self.states.entry(id.to_string()).or_insert_with(|| PickerState {
            value: 0,
            range,
            mode,
            visible: false,
        })
    }
    
    pub fn get(&self, id: &str) -> Option<&PickerState> {
        self.states.get(id)
    }
    
    pub fn set_value(&mut self, id: &str, value: usize) {
        if let Some(state) = self.states.get_mut(id) {
            if value < state.range.len() {
                state.value = value;
            }
        }
    }
    
    pub fn show(&mut self, id: &str) {
        if let Some(state) = self.states.get_mut(id) {
            state.visible = true;
        }
    }
    
    pub fn hide(&mut self, id: &str) {
        if let Some(state) = self.states.get_mut(id) {
            state.visible = false;
        }
    }
}

/// 全局 Picker 状态管理器
pub static PICKER_MANAGER: Lazy<Mutex<PickerStateManager>> = Lazy::new(|| {
    Mutex::new(PickerStateManager::new())
});

/// Picker 组件
pub struct PickerComponent;

impl PickerComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let mut attrs = node.attributes.clone();
        // 有元素子节点时标记：绘制期只作容器（绘制路径的浅拷贝不带 children）
        if node.children.iter().any(|c| c.node_type == crate::parser::wxml::WxmlNodeType::Element) {
            attrs.insert("__has_trigger".to_string(), "1".to_string());
        }
        
        // 解析 mode
        let mode = match node.get_attr("mode").unwrap_or("selector") {
            "multiSelector" => PickerMode::MultiSelector,
            "time" => PickerMode::Time,
            "date" => PickerMode::Date,
            "region" => PickerMode::Region,
            _ => PickerMode::Selector,
        };
        
        // 解析 range（选项列表）
        let range_str = node.get_attr("range").unwrap_or("[]");
        let range: Vec<String> = serde_json::from_str(range_str)
            .unwrap_or_else(|_| vec![]);
        
        // 获取当前值
        let value = node.get_attr("value")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0usize);
        
        // 获取显示文本
        let display_text = if value < range.len() {
            range[value].clone()
        } else {
            node.get_attr("placeholder").unwrap_or("请选择").to_string()
        };
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "picker".into(),
            text: display_text,
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, text_renderer: Option<&TextRenderer>, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        // 绘制背景
        draw_background(canvas, &node.style, x, y, w, h);

        // 有子节点（开发者自定义触发视图，如“当前选择：xxx”）时，picker 只作容器，
        // 由子节点渲染内容，不再合成占位文本/箭头（与微信及 HTML 端一致）。
        if node.attrs.get("__has_trigger").is_some() {
            // 开发者自定义了触发视图（子节点），picker 只作容器
            return;
        }

        let font_size = node.style.font_size * sf;
        let text_color = node.style.text_color.unwrap_or(Color::BLACK);
        let padding = 12.0 * sf;
        
        // 绘制当前选中值
        if let Some(tr) = text_renderer {
            let text_y = y + (h + font_size) / 2.0 - 2.0 * sf;
            let paint = Paint::new().with_color(text_color);
            tr.draw_text(canvas, &node.text, x + padding, text_y, font_size, &paint);
        }
        
        // 绘制下拉箭头
        let arrow_size = 8.0 * sf;
        let arrow_x = x + w - padding - arrow_size;
        let arrow_y = y + (h - arrow_size) / 2.0;
        
        let arrow_color = Color::from_hex(0x999999);
        let paint = Paint::new()
            .with_color(arrow_color)
            .with_style(PaintStyle::Stroke)
            .with_stroke_width(2.0 * sf)
            .with_anti_alias(true);
        
        // 绘制向下箭头 V
        canvas.draw_line(
            arrow_x, arrow_y,
            arrow_x + arrow_size / 2.0, arrow_y + arrow_size,
            &paint
        );
        canvas.draw_line(
            arrow_x + arrow_size / 2.0, arrow_y + arrow_size,
            arrow_x + arrow_size, arrow_y,
            &paint
        );
        
        // 绘制边框
        if node.style.border_width > 0.0 {
            let border_color = node.style.border_color.unwrap_or(Color::from_hex(0xE5E5E5));
            let paint = Paint::new()
                .with_color(border_color)
                .with_style(PaintStyle::Stroke)
                .with_stroke_width(node.style.border_width);
            canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
        }
    }
    
    /// 显示选择器
    pub fn show(picker_id: &str) {
        if let Ok(mut manager) = PICKER_MANAGER.lock() {
            manager.show(picker_id);
        }
    }
    
    /// 隐藏选择器
    pub fn hide(picker_id: &str) {
        if let Ok(mut manager) = PICKER_MANAGER.lock() {
            manager.hide(picker_id);
        }
    }
    
    /// 设置选中值
    pub fn set_value(picker_id: &str, value: usize) {
        if let Ok(mut manager) = PICKER_MANAGER.lock() {
            manager.set_value(picker_id, value);
        }
    }
}

/// PickerView 嵌入式选择器组件
pub struct PickerViewComponent;

impl PickerViewComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (ts, ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "picker-view".into(),
            text: String::new(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, _text_renderer: Option<&TextRenderer>, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        // 绘制背景
        draw_background(canvas, &node.style, x, y, w, h);
        
        // 绘制选择器指示线
        let indicator_color = Color::from_hex(0xE5E5E5);
        let paint = Paint::new()
            .with_color(indicator_color)
            .with_style(PaintStyle::Stroke)
            .with_stroke_width(1.0 * sf);
        
        let center_y = y + h / 2.0;
        let item_height = 36.0 * sf;
        
        // 上边线
        canvas.draw_line(x, center_y - item_height / 2.0, x + w, center_y - item_height / 2.0, &paint);
        // 下边线
        canvas.draw_line(x, center_y + item_height / 2.0, x + w, center_y + item_height / 2.0, &paint);
    }
}

/// PickerViewColumn 选择器列组件
pub struct PickerViewColumnComponent;

impl PickerViewColumnComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (ts, ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "picker-view-column".into(),
            text: String::new(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
}
