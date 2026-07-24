//! 小程序源码 → HTML/CSS 编译（transpile）
//!
//! 由于 WXSS 基本是标准 CSS、WXML 与 HTML 结构一一对应，本模块把小程序页面
//! 编译成浏览器可原生渲染的 HTML + CSS：
//! - WXML(+data) → HTML：`TemplateEngine` 先展开 `wx:for/wx:if/{{}}`，再做标签映射
//! - WXSS → CSS：仅需把 `rpx` 换算为 `px`（1rpx = 0.5px @375 宽），其余原样保留
//!
//! 用途：① 浏览器端原生渲染预览（极快、真实 DOM 可交互）；② 静态导出 HTML 工程；
//! ③ 作为后续「编译到各端原生源码」的中间层。

use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::parser::TemplateEngine;
use serde_json::Value as JsonValue;

/// 把字符串中的 `<number>rpx` 换算为 `px`（1rpx = 0.5px，对应 375 逻辑宽），其余不变。
pub fn convert_rpx(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < n {
        // 尝试匹配 数字 + "rpx"
        let start = i;
        let mut j = i;
        if j < n && (chars[j] == '-' || chars[j] == '+') { j += 1; }
        let mut has_digit = false;
        while j < n && (chars[j].is_ascii_digit() || chars[j] == '.') {
            if chars[j].is_ascii_digit() { has_digit = true; }
            j += 1;
        }
        if has_digit && j + 3 <= n && chars[j] == 'r' && chars[j + 1] == 'p' && chars[j + 2] == 'x' {
            let num: String = chars[start..j].iter().collect();
            if let Ok(v) = num.parse::<f64>() {
                // 去掉多余小数
                let px = v * 0.5;
                if (px.fract()).abs() < 1e-6 {
                    out.push_str(&format!("{}px", px as i64));
                } else {
                    out.push_str(&format!("{:.3}px", px));
                }
                i = j + 3;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// WXSS → CSS
pub fn wxss_to_css(src: &str) -> String {
    convert_rpx(src)
}

/// WXML(+data) → HTML 片段（不含 <html>/<body> 包裹）
pub fn wxml_to_html(nodes: &[WxmlNode], data: &JsonValue) -> String {
    let rendered = TemplateEngine::render(nodes, data);
    let mut out = String::new();
    for n in &rendered {
        emit_node(n, &mut out);
    }
    out
}

fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;")
}

/// 标签映射：返回 (html标签, 附加内联样式, 是否自闭合)
fn map_tag(node: &WxmlNode) -> (&'static str, String, bool) {
    match node.tag_name.as_str() {
        "view" | "block" | "cover-view" => ("div", String::new(), false),
        "scroll-view" => {
            let sx = node.get_attr("scroll-x").map(|v| v == "true").unwrap_or(false);
            let style = if sx { "overflow-x:auto;overflow-y:hidden" } else { "overflow-y:auto" };
            ("div", style.to_string(), false)
        }
        "text" => ("span", String::new(), false),
        "image" => {
            let mode = node.get_attr("mode").unwrap_or("scaleToFill");
            let fit = match mode {
                "aspectFit" => "object-fit:contain",
                "aspectFill" => "object-fit:cover",
                "widthFix" => "height:auto",
                _ => "object-fit:fill",
            };
            ("img", fit.to_string(), true)
        }
        "button" => ("button", String::new(), false),
        "input" => ("input", String::new(), true),
        "textarea" => ("textarea", String::new(), false),
        "navigator" => ("a", String::new(), false),
        "swiper" => ("div", "overflow:hidden;position:relative".to_string(), false),
        "swiper-item" => ("div", String::new(), false),
        "icon" => ("i", String::new(), false),
        _ => ("div", String::new(), false),
    }
}

fn emit_node(node: &WxmlNode, out: &mut String) {
    if node.node_type == WxmlNodeType::Text {
        out.push_str(&escape_text(&node.text_content));
        return;
    }
    if node.node_type != WxmlNodeType::Element {
        return;
    }
    let (tag, extra_style, void) = map_tag(node);
    let mut attrs = String::new();

    // class：icon 追加内置图标类
    let mut class = node.get_attr("class").unwrap_or("").to_string();
    if node.tag_name == "icon" {
        if let Some(t) = node.get_attr("type") {
            if !class.is_empty() { class.push(' '); }
            class.push_str(&format!("wxicon wxicon-{}", t));
        }
    }
    if !class.is_empty() {
        attrs.push_str(&format!(" class=\"{}\"", escape_attr(&class)));
    }
    if let Some(id) = node.get_attr("id") {
        attrs.push_str(&format!(" id=\"{}\"", escape_attr(id)));
    }

    // style（rpx→px）+ 附加样式
    let mut style = node.get_attr("style").map(convert_rpx).unwrap_or_default();
    if !extra_style.is_empty() {
        if !style.is_empty() && !style.trim_end().ends_with(';') { style.push(';'); }
        style.push_str(&extra_style);
    }
    if !style.is_empty() {
        attrs.push_str(&format!(" style=\"{}\"", escape_attr(&style)));
    }

    // 元素特有属性
    match node.tag_name.as_str() {
        "image" => {
            if let Some(s) = node.get_attr("src") {
                attrs.push_str(&format!(" src=\"{}\"", escape_attr(s)));
            }
        }
        "input" => {
            let ty = if node.get_attr("password").map(|v| v == "true").unwrap_or(false) {
                "password".to_string()
            } else {
                match node.get_attr("type").unwrap_or("text") {
                    "number" | "digit" | "idcard" => "number".to_string(),
                    other => other.to_string(),
                }
            };
            attrs.push_str(&format!(" type=\"{}\"", ty));
            if let Some(v) = node.get_attr("value") { attrs.push_str(&format!(" value=\"{}\"", escape_attr(v))); }
            if let Some(p) = node.get_attr("placeholder") { attrs.push_str(&format!(" placeholder=\"{}\"", escape_attr(p))); }
        }
        "navigator" => {
            if let Some(u) = node.get_attr("url") { attrs.push_str(&format!(" href=\"#{}\"", escape_attr(u))); }
        }
        _ => {}
    }

    // 事件：bindX / catchX → data-<event>；dataset → data-ds-*
    for (k, v) in &node.attributes {
        let ev = k.strip_prefix("bind").or_else(|| k.strip_prefix("catch"));
        if let Some(ev) = ev {
            if !ev.is_empty() {
                attrs.push_str(&format!(" data-{}=\"{}\"", ev, escape_attr(v)));
            }
        }
    }
    for (k, v) in &node.attributes {
        if let Some(dk) = k.strip_prefix("data-") {
            attrs.push_str(&format!(" data-ds-{}=\"{}\"", dk, escape_attr(v)));
        }
    }

    out.push_str(&format!("<{}{}>", tag, attrs));
    if void {
        return;
    }
    // textarea 的初始值放内容
    if node.tag_name == "textarea" {
        if let Some(v) = node.get_attr("value") { out.push_str(&escape_text(v)); }
    }
    for c in &node.children {
        emit_node(c, out);
    }
    out.push_str(&format!("</{}>", tag));
}

/// 基础样式：CSS reset + 常见默认（贴近小程序默认盒模型）+ 内置 icon 图标。
pub fn base_css() -> &'static str {
    r#"
*{box-sizing:border-box;margin:0;padding:0;-webkit-tap-highlight-color:transparent;}
view,scroll-view,swiper,swiper-item,cover-view{display:flex;flex-direction:column;}
image{display:block;}
button{border:0;background:none;font:inherit;color:inherit;cursor:pointer;display:block;width:100%;}
input,textarea{border:0;outline:none;background:none;font:inherit;color:inherit;width:100%;}
a{color:inherit;text-decoration:none;}
body{font-size:16px;color:#333;font-family:-apple-system,system-ui,"PingFang SC","Hiragino Sans GB",sans-serif;background:#f5f6f8;}
/* 内置矢量图标（对齐 <icon type> 常用类型）*/
.wxicon{display:inline-flex;align-items:center;justify-content:center;width:1.2em;height:1.2em;border-radius:50%;color:#fff;font-style:normal;font-size:.7em;line-height:1;}
.wxicon-success{background:#09bb07;}.wxicon-success::before{content:"\2713";}
.wxicon-info{background:#10aeff;}.wxicon-info::before{content:"i";font-weight:bold;}
.wxicon-warn{background:#f76260;}.wxicon-warn::before{content:"!";font-weight:bold;}
.wxicon-waiting{background:#10aeff;}.wxicon-waiting::before{content:"\2026";}
.wxicon-cancel{background:#f43530;}.wxicon-cancel::before{content:"\2715";}
.wxicon-download{background:#09bb07;}.wxicon-download::before{content:"\2193";}
.wxicon-search{background:#b2b2b2;}.wxicon-search::before{content:"\1F50D";}
.wxicon-clear{background:#b2b2b2;}.wxicon-clear::before{content:"\2715";}
"#
}

/// 生成一个自包含的静态 HTML 文档（用于静态导出）。
pub fn make_html_doc(title: &str, css: &str, body_html: &str, width_px: u32) -> String {
    format!(
        "<!doctype html>\n<html lang=\"zh\"><head><meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<title>{title}</title>\n<style>\n{base}\n#app{{width:{w}px;min-height:100vh;margin:0 auto;background:#f5f6f8;position:relative;overflow:hidden;}}\n{css}\n</style></head>\n\
<body><div id=\"app\">{body}</div></body></html>",
        title = escape_text(title),
        base = base_css(),
        w = width_px,
        css = css,
        body = body_html,
    )
}
