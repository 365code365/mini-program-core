//! CheckboxGroup 和 RadioGroup 组件

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::Canvas;

/// CheckboxGroup 组件 - 复选框组
pub struct CheckboxGroupComponent;

impl CheckboxGroupComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (ts, ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "checkbox-group".into(),
            text: String::new(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, _sf: f32) {
        // checkbox-group 本身不渲染，只作为容器
        // 背景绘制（如果有）
        draw_background(canvas, &node.style, x, y, w, h);
    }
}

/// RadioGroup 组件 - 单选框组
pub struct RadioGroupComponent;

impl RadioGroupComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (ts, ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "radio-group".into(),
            text: String::new(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, _sf: f32) {
        // radio-group 本身不渲染，只作为容器
        // 背景绘制（如果有）
        draw_background(canvas, &node.style, x, y, w, h);
    }
}
