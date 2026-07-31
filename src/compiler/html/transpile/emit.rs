//! 节点输出：紧凑与美化两种形态，以及文本/属性转义
//!
//! `transpile_new` 的一片。**纯搬迁**：从 805 行按职责切开，一行逻辑没改。
use super::inner::{container_suffix_html, custom_inner_html, has_custom_inner};
use super::tags::build_open_tag;
use super::*;

/// WXML(+data) → HTML 片段（不含 <html>/<body> 包裹）
pub fn wxml_to_html(nodes: &[WxmlNode], data: &JsonValue) -> String {
    let rendered = TemplateEngine::render(nodes, data);
    let mut out = String::new();
    for n in &rendered {
        emit_node(n, &mut out);
    }
    out
}

pub(super) fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
pub(super) fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;")
}

/// 布尔属性判定：`checked`、`checked="true"`、`checked="{{true}}"`(渲染为 "true") 等均视为真
pub(super) fn is_truthy(v: Option<&str>) -> bool {
    match v {
        None => false,
        Some(s) => {
            let t = s.trim();
            t.is_empty() || t == "true" || t == "1"
        }
    }
}

/// 紧凑输出（用于 devserver 实时渲染）
fn emit_node(node: &WxmlNode, out: &mut String) {
    if node.node_type == WxmlNodeType::Text {
        out.push_str(&escape_text(&node.text_content));
        return;
    }
    if node.node_type != WxmlNodeType::Element { return; }
    let (open, tag, void) = build_open_tag(node);
    out.push_str(&open);
    if void { return; }
    if has_custom_inner(&node.tag_name) {
        out.push_str(&custom_inner_html(node));
        out.push_str(&format!("</{}>", tag));
        return;
    }
    if node.tag_name == "textarea" {
        if let Some(v) = node.get_attr("value") { out.push_str(&escape_text(v)); }
    }
    for c in &node.children { emit_node(c, out); }
    if let Some(suffix) = container_suffix_html(node) { out.push_str(&suffix); }
    out.push_str(&format!("</{}>", tag));
}

/// WXML(+data) → 带缩进的可读 HTML（用于导出 HTML 工程源码）
pub fn wxml_to_html_pretty(nodes: &[WxmlNode], data: &JsonValue) -> String {
    let rendered = TemplateEngine::render(nodes, data);
    let mut out = String::new();
    for n in &rendered {
        emit_pretty(n, 1, &mut out);
    }
    out
}

fn emit_pretty(node: &WxmlNode, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    if node.node_type == WxmlNodeType::Text {
        let t = node.text_content.trim();
        if !t.is_empty() { out.push_str(&format!("{}{}\n", indent, escape_text(t))); }
        return;
    }
    if node.node_type != WxmlNodeType::Element { return; }
    let (open, tag, void) = build_open_tag(node);
    if void {
        out.push_str(&format!("{}{}\n", indent, open));
        return;
    }
    // switch/progress/rich-text：内部结构由编译器生成，单行输出
    if has_custom_inner(&node.tag_name) {
        out.push_str(&format!("{}{}{}</{}>\n", indent, open, custom_inner_html(node), tag));
        return;
    }
    // `<text>` 一律单行输出（哪怕里面嵌了 `<text>`）。
    //
    // `.wx-text` 带 `white-space: pre-line` —— 那是为了保留 WXML 里写在文本里的换行。
    // 代价是：一旦漂亮打印在 `<text>` 内部插入缩进换行，浏览器就会把它当**真实换行**渲染。
    // 优惠券的 `<text class="c-amount">¥<text class="c-num">10</text></text>` 因此在
    // Chrome 里被拆成两行（¥ 一行、10 一行），而小程序里本该是一行。
    // 这条 bug 还会污染双端对比的基线 —— 参照物自己先错了。
    if node.tag_name == "text" {
        let mut inner = String::new();
        for c in &node.children {
            emit_node(c, &mut inner);
        }
        out.push_str(&format!("{}{}{}</{}>\n", indent, open, inner, tag));
        return;
    }
    // 无子元素（或只有文本）：单行输出更紧凑可读
    let only_text = node.children.iter().all(|c| c.node_type == WxmlNodeType::Text);
    if only_text {
        let mut inner = String::new();
        if node.tag_name == "textarea" {
            if let Some(v) = node.get_attr("value") { inner.push_str(&escape_text(v)); }
        }
        for c in &node.children {
            if c.node_type == WxmlNodeType::Text { inner.push_str(&escape_text(c.text_content.trim())); }
        }
        out.push_str(&format!("{}{}{}</{}>\n", indent, open, inner, tag));
        return;
    }
    out.push_str(&format!("{}{}\n", indent, open));
    for c in &node.children { emit_pretty(c, depth + 1, out); }
    if let Some(suffix) = container_suffix_html(node) {
        out.push_str(&format!("{}  {}\n", indent, suffix));
    }
    out.push_str(&format!("{}</{}>\n", indent, tag));
}
