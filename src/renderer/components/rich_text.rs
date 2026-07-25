//! RichText 富文本组件
//!
//! 把 `nodes`（节点数组 JSON，或 HTML 字符串）解析成一串带样式的文本片段，
//! 构建为「行内换行」的子文本节点（各自携带颜色/字重），从而与 HTML 端一致地
//! 渲染出如「红色」这类局部着色，而不是把原始 JSON 当纯文本堆出来。

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color};
use std::collections::HashMap;
use taffy::prelude::*;

pub struct RichTextComponent;

/// 一个带样式的文本片段
struct Segment {
    text: String,
    color: Option<Color>,
    bold: bool,
    font_size: f32, // 逻辑像素
}

impl RichTextComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;

        let nodes_str = node.get_attr("nodes").unwrap_or("");
        let segments = parse_segments(nodes_str, ns.font_size, ns.text_color, ns.font_weight);

        // 无法解析或为空：退回纯文本叶子，至少显示可读文本而非原始 JSON。
        if segments.is_empty() {
            let text = strip_html_tags(nodes_str);
            let tn = ctx.taffy.new_leaf(ts).unwrap();
            return Some(RenderNode {
                tag: "rich-text".into(), text, attrs, taffy_node: tn,
                style: ns, children: vec![], events,
            });
        }

        // 富文本容器：行内排列 + 允许换行，模拟 inline 文本流
        ts.flex_direction = FlexDirection::Row;
        ts.flex_wrap = FlexWrap::Wrap;
        ts.align_items = Some(AlignItems::Baseline);

        let measure_ls = 0.0;
        let mut children: Vec<RenderNode> = Vec::new();
        for seg in &segments {
            // 按标点/空格/换行切成更细的可换行单元，避免整段片段无法折行
            for unit in split_wrappable(&seg.text) {
                if unit.is_empty() { continue; }
                let font_px = seg.font_size * sf;
                let line_h = natural_line_height_px_for(&unit, font_px);
                let w = intrinsic_text_width(&unit, font_px, measure_ls) + 1.0;
                let cts = Style {
                    size: Size { width: length(w), height: length(line_h) },
                    ..Default::default()
                };
                let ctn = ctx.taffy.new_leaf(cts).unwrap();
                let mut cstyle = NodeStyle {
                    font_size: seg.font_size,
                    text_color: seg.color.or(ns.text_color),
                    opacity: 1.0,
                    ..Default::default()
                };
                if seg.bold { cstyle.font_weight = FontWeight::Bold; }
                children.push(RenderNode {
                    tag: "text".into(),
                    text: unit,
                    attrs: HashMap::new(),
                    taffy_node: ctn,
                    style: cstyle,
                    children: vec![],
                    events: vec![],
                });
            }
        }

        let child_ids: Vec<NodeId> = children.iter().map(|c| c.taffy_node).collect();
        let tn = ctx.taffy.new_with_children(ts, &child_ids).unwrap();

        Some(RenderNode {
            tag: "rich-text".into(),
            text: String::new(),
            attrs,
            taffy_node: tn,
            style: ns,
            children,
            events,
        })
    }

    pub fn draw(node: &RenderNode, canvas: &mut Canvas, text_renderer: Option<&TextRenderer>, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        // 容器只画背景/边框；文本片段作为子节点单独绘制。
        draw_background(canvas, &node.style, x, y, w, h);

        // 纯文本退化叶子（无子节点）：直接画文本
        if node.children.is_empty() && !node.text.is_empty() {
            if let Some(tr) = text_renderer {
                let font_size = node.style.font_size * sf;
                let pl = node.style.padding_left * sf;
                let pt = node.style.padding_top * sf;
                let color = node.style.text_color.unwrap_or(Color::BLACK);
                let paint = crate::Paint::new().with_color(color);
                tr.draw_text(canvas, &node.text, x + pl.max(4.0 * sf), y + pt.max(4.0 * sf) + font_size, font_size, &paint);
            }
        }
    }
}

/// 把 nodes 解析为带样式的文本片段。支持：
/// - 节点数组 JSON（模板插值会把 " 变成 '，先还原再解析）
/// - HTML 字符串（去标签，取纯文本）
fn parse_segments(nodes_str: &str, base_font: f32, base_color: Option<Color>, base_weight: FontWeight) -> Vec<Segment> {
    let trimmed = nodes_str.trim();
    if trimmed.is_empty() { return vec![]; }
    if trimmed.starts_with('[') || trimmed.starts_with('{') {
        let json = trimmed.replace('\'', "\"");
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) {
            let mut segs = Vec::new();
            let base_bold = matches!(base_weight, FontWeight::Bold | FontWeight::W600 | FontWeight::W700 | FontWeight::W800 | FontWeight::W900);
            walk_rich(&value, base_color, base_bold, base_font, &mut segs);
            return segs;
        }
    }
    // HTML 字符串：退化为单一片段的纯文本
    let text = strip_html_tags(nodes_str);
    if text.is_empty() { vec![] } else { vec![Segment { text, color: base_color, bold: false, font_size: base_font }] }
}

/// 递归遍历富文本节点，继承并叠加样式，收集文本片段。
fn walk_rich(value: &serde_json::Value, color: Option<Color>, bold: bool, font: f32, out: &mut Vec<Segment>) {
    match value {
        serde_json::Value::Array(arr) => {
            for v in arr { walk_rich(v, color, bold, font, out); }
        }
        serde_json::Value::String(s) => {
            if !s.is_empty() { out.push(Segment { text: s.clone(), color, bold, font_size: font }); }
        }
        serde_json::Value::Object(obj) => {
            // 文本节点 { type:"text", text:"..." }
            if let Some(t) = obj.get("text").and_then(|v| v.as_str()) {
                if !t.is_empty() { out.push(Segment { text: t.to_string(), color, bold, font_size: font }); }
            }
            // 元素节点：叠加 name（b/strong→加粗）与 attrs.style（color/font-size）
            let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let mut seg_color = color;
            let mut seg_bold = bold || matches!(name, "b" | "strong");
            let mut seg_font = font;
            if let Some(style) = obj.get("attrs").and_then(|a| a.get("style")).and_then(|s| s.as_str()) {
                for decl in style.split(';') {
                    let mut kv = decl.splitn(2, ':');
                    let key = kv.next().unwrap_or("").trim().to_ascii_lowercase();
                    let val = kv.next().unwrap_or("").trim();
                    match key.as_str() {
                        "color" => { if let Some(c) = parse_color_str(val) { seg_color = Some(c); } }
                        "font-weight" => { if val == "bold" || val == "700" || val == "600" { seg_bold = true; } }
                        "font-size" => {
                            let n: String = val.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
                            if let Ok(px) = n.parse::<f32>() { if px > 0.0 { seg_font = px; } }
                        }
                        _ => {}
                    }
                }
            }
            if let Some(children) = obj.get("children") {
                walk_rich(children, seg_color, seg_bold, seg_font, out);
            }
        }
        _ => {}
    }
}

/// 把一段文本切成可独立换行的单元：CJK 每字一单元，连续 ASCII 词一单元，
/// 保留末尾空格并按显式换行断开。
fn split_wrappable(text: &str) -> Vec<String> {
    let mut units: Vec<String> = Vec::new();
    let mut ascii = String::new();
    let flush = |ascii: &mut String, units: &mut Vec<String>| {
        if !ascii.is_empty() { units.push(std::mem::take(ascii)); }
    };
    for ch in text.chars() {
        if ch == '\n' {
            flush(&mut ascii, &mut units);
            units.push("\n".into());
        } else if ch.is_ascii() && !ch.is_whitespace() {
            ascii.push(ch);
        } else if ch == ' ' {
            ascii.push(ch);
            flush(&mut ascii, &mut units);
        } else {
            // CJK 或其它：单字成单元
            flush(&mut ascii, &mut units);
            units.push(ch.to_string());
        }
    }
    flush(&mut ascii, &mut units);
    units
}

/// 去除 HTML 标签，保留纯文本，并还原常见实体。
fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    result
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_string()
}
