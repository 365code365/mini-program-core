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

/// 小程序内置元素标签名（这些在 HTML 里被映射成 div/span/img 等，需把选择器改写为 .wx-<tag>）
const WX_TAGS: &[&str] = &[
    "view", "scroll-view", "swiper", "swiper-item", "movable-view", "movable-area",
    "cover-view", "cover-image", "text", "rich-text", "image", "icon", "progress",
    "button", "checkbox", "checkbox-group", "radio", "radio-group", "switch", "slider",
    "input", "textarea", "picker", "picker-view", "picker-view-column", "form", "label",
    "navigator", "video", "audio", "camera", "live-player", "live-pusher", "canvas",
    "map", "web-view", "ad", "block",
];

/// WXSS → CSS：
/// ① rpx → px；② 元素类型选择器改写为 `.wx-<tag>`（因为 HTML 里 view→div、text→span 等，
///    否则 `.foo text{...}` 这类后代标签选择器全部失效，样式大面积丢失）。
pub fn wxss_to_css(src: &str) -> String {
    rewrite_tag_selectors(&convert_rpx(src))
}

/// 判断某标识符是否为需要改写的小程序标签
fn is_wx_tag(ident: &str) -> bool {
    WX_TAGS.contains(&ident)
}

/// 改写单个选择器串里的元素类型选择器：`.a text` → `.a .wx-text`，`view>text` → `.wx-view>.wx-text`
fn rewrite_selector_tokens(sel: &str) -> String {
    let chars: Vec<char> = sel.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(sel.len() + 8);
    let mut i = 0;
    let mut attr_depth = 0i32; // 处于 [...] 内不改写
    while i < n {
        let c = chars[i];
        if c == '[' { attr_depth += 1; out.push(c); i += 1; continue; }
        if c == ']' { if attr_depth > 0 { attr_depth -= 1; } out.push(c); i += 1; continue; }
        if attr_depth > 0 { out.push(c); i += 1; continue; }

        if c.is_ascii_alphabetic() {
            // 上一个有效字符（决定是否处于「类型选择器」位置）
            let prev = out.chars().last();
            let type_pos = match prev {
                None => true,
                Some(p) => matches!(p, ' ' | '\t' | '\n' | '\r' | '>' | '+' | '~' | ',' | '('),
            };
            let start = i;
            while i < n && (chars[i].is_ascii_alphanumeric() || chars[i] == '-') { i += 1; }
            let ident: String = chars[start..i].iter().collect();
            if type_pos && ident == "page" {
                // WXSS 根选择器 page → HTML 根容器 #app
                out.push_str("#app");
            } else if type_pos && is_wx_tag(&ident) {
                out.push_str(".wx-");
                out.push_str(&ident);
            } else {
                out.push_str(&ident);
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// 扫描整段 CSS，只在「选择器位置」改写标签选择器；声明块、@media 条件、@keyframes 内容保持原样。
fn rewrite_tag_selectors(css: &str) -> String {
    let mut out = String::with_capacity(css.len() + 64);
    let mut sel = String::new();
    let mut in_block = false;
    let mut depth = 0i32;
    for c in css.chars() {
        if in_block {
            out.push(c);
            if c == '{' { depth += 1; }
            else if c == '}' { depth -= 1; if depth == 0 { in_block = false; } }
            continue;
        }
        // 选择器 / at-rule 前导 区域
        match c {
            '{' => {
                let trimmed = sel.trim_start();
                let low = trimmed.to_ascii_lowercase();
                if low.starts_with("@media") || low.starts_with("@supports") || low.starts_with("@container") {
                    // 分组 at-rule：条件原样输出，内部仍是规则（继续在选择器模式）
                    out.push_str(&sel);
                    out.push('{');
                } else if trimmed.starts_with('@') {
                    // @keyframes / @font-face / @page 等：内容原样，进入块模式
                    out.push_str(&sel);
                    out.push('{');
                    in_block = true;
                    depth = 1;
                } else {
                    out.push_str(&rewrite_selector_tokens(&sel));
                    out.push('{');
                    in_block = true;
                    depth = 1;
                }
                sel.clear();
            }
            '}' => {
                // 关闭分组 at-rule（如 @media）
                out.push_str(&sel);
                sel.clear();
                out.push('}');
            }
            ';' => {
                // 顶层 @import / @charset 等以分号结束的语句
                out.push_str(&sel);
                sel.clear();
                out.push(';');
            }
            _ => sel.push(c),
        }
    }
    out.push_str(&sel);
    out
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

/// 布尔属性判定：`checked`、`checked="true"`、`checked="{{true}}"`(渲染为 "true") 等均视为真
fn is_truthy(v: Option<&str>) -> bool {
    match v {
        None => false,
        Some(s) => {
            let t = s.trim();
            t.is_empty() || t == "true" || t == "1"
        }
    }
}

/// 判断某标签是否需要在 emit 时生成自定义内部结构（switch/progress/rich-text/icon）
fn has_custom_inner(tag: &str) -> bool {
    matches!(tag, "switch" | "progress" | "rich-text" | "icon" | "slider")
}

/// `<slider>` 的内部结构：range input（已滑过部分着色）+ 可选数值标签。
///
/// 原生渲染器与微信都会把「已滑过的轨道」画成激活色，并在 show-value 时显示数值；
/// 纯 `<input type=range>` 做不到这两点，故在编译期用渐变背景与数值标签补齐。
fn slider_inner_html(node: &WxmlNode) -> String {
    let num = |name: &str, fallback: f32| -> f32 {
        node.get_attr(name).and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(fallback)
    };
    let min = num("min", 0.0);
    let max = num("max", 100.0);
    let step = node.get_attr("step").unwrap_or("1").to_string();
    let value = num("value", min);
    let span = if (max - min).abs() < f32::EPSILON { 1.0 } else { max - min };
    let percent = (((value - min) / span) * 100.0).clamp(0.0, 100.0);
    let active = node.get_attr("activeColor")
        .or_else(|| node.get_attr("active-color"))
        .unwrap_or("#09bb07");
    let background = node.get_attr("backgroundColor")
        .or_else(|| node.get_attr("background-color"))
        .unwrap_or("#e5e5e5");
    let block_color = node.get_attr("block-color").unwrap_or("#ffffff");
    let disabled = if is_truthy(node.get_attr("disabled")) { " disabled" } else { "" };
    let mut html = format!(
        "<input class=\"wx-slider\" type=\"range\" min=\"{min}\" max=\"{max}\" step=\"{step}\" value=\"{value}\"{disabled} \
style=\"background:linear-gradient(to right,{active} 0%,{active} {percent}%,{background} {percent}%,{background} 100%);--wx-block:{block_color}\">",
        min = min, max = max, step = escape_attr(&step), value = value,
        disabled = disabled, active = active, background = background,
        percent = percent, block_color = block_color,
    );
    if is_truthy(node.get_attr("show-value")) || is_truthy(node.get_attr("show-info")) {
        let text = if (value - value.round()).abs() < 1e-6 {
            format!("{}", value.round() as i64)
        } else {
            format!("{value}")
        };
        html.push_str(&format!("<span class=\"wx-slider-value\">{text}</span>"));
    }
    html
}

/// `<icon>` 的矢量图形（内联 SVG）。
///
/// 之前用文字字形（✓ / i / ! / … / ✕）近似，`waiting` 这类实际是时钟的图标会明显走形，
/// 与原生渲染器自绘的矢量图标不一致。这里改为与原生同构的 SVG：圆底用 currentColor，
/// 内部标记用白色，尺寸由外层 width/height 控制。
fn icon_svg(icon_type: &str) -> String {
    // 统一 24x24 视图盒，圆心 (12,12) 半径 12
    let circle = "<circle cx=\"12\" cy=\"12\" r=\"12\" fill=\"currentColor\"/>";
    let body = match icon_type {
        "success" => format!(
            "{circle}<path d=\"M5.8 12.4 10 16.4 18.2 7.6\" fill=\"none\" stroke=\"#fff\" stroke-width=\"2.6\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>"
        ),
        "success_no_circle" => "<path d=\"M3.5 12.5 9 18 20.5 5.5\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"3\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>".to_string(),
        "info" | "info_circle" => format!(
            "{circle}<circle cx=\"12\" cy=\"7.6\" r=\"1.7\" fill=\"#fff\"/><rect x=\"10.9\" y=\"10.8\" width=\"2.2\" height=\"7\" rx=\"1.1\" fill=\"#fff\"/>"
        ),
        "warn" => format!(
            "{circle}<rect x=\"10.9\" y=\"5.4\" width=\"2.2\" height=\"7.2\" rx=\"1.1\" fill=\"#fff\"/><circle cx=\"12\" cy=\"16.6\" r=\"1.7\" fill=\"#fff\"/>"
        ),
        // waiting：微信为时钟表盘（时针+分针）
        "waiting" | "waiting_circle" => format!(
            "{circle}<rect x=\"10.9\" y=\"6.2\" width=\"2.2\" height=\"6.6\" rx=\"1.1\" fill=\"#fff\"/><rect x=\"12\" y=\"10.9\" width=\"5.2\" height=\"2.2\" rx=\"1.1\" fill=\"#fff\"/><circle cx=\"12\" cy=\"12\" r=\"1.6\" fill=\"#fff\"/>"
        ),
        "info_no_circle" => "<circle cx=\"12\" cy=\"5.6\" r=\"2\" fill=\"currentColor\"/><rect x=\"10.4\" y=\"9.8\" width=\"3.2\" height=\"9.4\" rx=\"1.6\" fill=\"currentColor\"/>".to_string(),
        "warn_no_circle" => "<rect x=\"10.4\" y=\"3\" width=\"3.2\" height=\"11.2\" rx=\"1.6\" fill=\"currentColor\"/><circle cx=\"12\" cy=\"18.8\" r=\"2\" fill=\"currentColor\"/>".to_string(),
        "waiting_no_circle" | "clock" => "<circle cx=\"12\" cy=\"12\" r=\"10.4\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2.2\"/><path d=\"M12 5.6V12h5.2\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2.2\" stroke-linecap=\"round\"/>".to_string(),
        // 纯叉号（无圆底），与原生 draw_thick_x 对应
        "close" | "cancel_no_circle" => "<path d=\"M5 5 19 19M19 5 5 19\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2.6\" stroke-linecap=\"round\"/>".to_string(),
        "cancel" | "clear" => format!(
            "{circle}<path d=\"M7.6 7.6 16.4 16.4M16.4 7.6 7.6 16.4\" fill=\"none\" stroke=\"#fff\" stroke-width=\"2.6\" stroke-linecap=\"round\"/>"
        ),
        "download" => "<circle cx=\"12\" cy=\"12\" r=\"10.8\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2.2\"/><path d=\"M12 6v7\" stroke=\"currentColor\" stroke-width=\"2.2\" stroke-linecap=\"round\"/><path d=\"M8 12.4 12 16.6 16 12.4Z\" fill=\"currentColor\"/><rect x=\"7\" y=\"17.4\" width=\"10\" height=\"2.2\" rx=\"1.1\" fill=\"currentColor\"/>".to_string(),
        "search" => "<circle cx=\"10.4\" cy=\"10.4\" r=\"6.4\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2.4\"/><path d=\"M15.2 15.2 21 21\" stroke=\"currentColor\" stroke-width=\"2.4\" stroke-linecap=\"round\"/>".to_string(),
        "circle" => "<circle cx=\"12\" cy=\"12\" r=\"10.8\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2.2\"/>".to_string(),
        "star" => "<path d=\"M12 1.6 15.2 8.6 22.8 9.5 17.2 14.6 18.7 22 12 18.3 5.3 22 6.8 14.6 1.2 9.5 8.8 8.6Z\" fill=\"currentColor\"/>".to_string(),
        "star-o" | "star_o" => "<path d=\"M12 1.6 15.2 8.6 22.8 9.5 17.2 14.6 18.7 22 12 18.3 5.3 22 6.8 14.6 1.2 9.5 8.8 8.6Z\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linejoin=\"round\"/>".to_string(),
        "heart" => "<path d=\"M12 21C6 16.5 2.6 13.4 2.6 9.6 2.6 6.5 5 4.2 8 4.2c1.8 0 3.2.9 4 2.2.8-1.3 2.2-2.2 4-2.2 3 0 5.4 2.3 5.4 5.4 0 3.8-3.4 6.9-9.4 11.4Z\" fill=\"currentColor\"/>".to_string(),
        _ => format!(
            "{circle}<path d=\"M5.8 12.4 10 16.4 18.2 7.6\" fill=\"none\" stroke=\"#fff\" stroke-width=\"2.6\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>"
        ),
    };
    format!("<svg viewBox=\"0 0 24 24\" aria-hidden=\"true\">{body}</svg>")
}

/// 生成 switch/progress/rich-text 的内部 HTML
fn custom_inner_html(node: &WxmlNode) -> String {
    match node.tag_name.as_str() {
        "switch" => "<span class=\"wx-switch-knob\"></span>".to_string(),
        "progress" => {
            let percent = node.get_attr("percent")
                .and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(0.0)
                .clamp(0.0, 100.0);
            let color = node.get_attr("activeColor")
                .or_else(|| node.get_attr("active-color"))
                .or_else(|| node.get_attr("color"))
                .unwrap_or("#09bb07");
            let mut s = format!(
                "<div class=\"wx-progress-outer\"><div class=\"wx-progress-inner\" style=\"width:{}%;background:{}\"></div></div>",
                percent, color
            );
            if is_truthy(node.get_attr("show-info")) {
                s.push_str(&format!("<span class=\"wx-progress-info\">{}%</span>", percent as i32));
            }
            s
        }
        "rich-text" => rich_text_inner(node),
        "icon" => icon_svg(node.get_attr("type").unwrap_or("success")),
        "slider" => slider_inner_html(node),
        _ => String::new(),
    }
}

/// rich-text：nodes 可为 HTML 字符串或节点数组（模板插值会把 " 转成 '，此处还原后按 JSON 解析）
fn rich_text_inner(node: &WxmlNode) -> String {
    let nodes = match node.get_attr("nodes") {
        Some(n) => n.trim().to_string(),
        None => return String::new(),
    };
    if nodes.starts_with('[') || nodes.starts_with('{') {
        if let Ok(v) = serde_json::from_str::<JsonValue>(&nodes.replace('\'', "\"")) {
            return render_rich_nodes(&v);
        }
    }
    // 否则视为 HTML 字符串，原样输出
    nodes
}

fn render_rich_nodes(v: &JsonValue) -> String {
    match v {
        JsonValue::Array(a) => a.iter().map(render_rich_nodes).collect(),
        JsonValue::String(s) => escape_text(s),
        JsonValue::Object(o) => {
            // 文本节点：{ type:"text", text:"..." }
            if o.get("type").and_then(|t| t.as_str()) == Some("text") {
                return escape_text(o.get("text").and_then(|t| t.as_str()).unwrap_or(""));
            }
            // 元素节点：{ name, attrs:{...}, children:[...] }
            let name = o.get("name").and_then(|n| n.as_str()).unwrap_or("div");
            let mut attrs = String::new();
            if let Some(JsonValue::Object(at)) = o.get("attrs") {
                for (k, val) in at {
                    if let Some(sv) = val.as_str() {
                        attrs.push_str(&format!(" {}=\"{}\"", k, escape_attr(sv)));
                    }
                }
            }
            let children = o.get("children").map(render_rich_nodes).unwrap_or_default();
            format!("<{n}{a}>{c}</{n}>", n = name, a = attrs, c = children)
        }
        _ => String::new(),
    }
}

/// 标签映射：返回 (html标签, 附加内联样式, 是否自闭合)
fn map_tag(node: &WxmlNode) -> (&'static str, String, bool) {
    match node.tag_name.as_str() {
        "view" | "block" | "cover-view" => ("div", String::new(), false),
        "scroll-view" => {
            let sx = is_truthy(node.get_attr("scroll-x"));
            // 横向滚动：改为 flex 行排列 + 不换行；纵向滚动：保持列排列
            let style = if sx {
                "display:flex;flex-direction:row;flex-wrap:nowrap;overflow-x:auto;overflow-y:hidden"
            } else {
                "overflow-y:auto"
            };
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
        // 轮播：布局与指示点由 base.css / runtime.js 处理
        "swiper" => ("div", String::new(), false),
        "swiper-item" => ("div", String::new(), false),
        "icon" => ("i", String::new(), false),
        // 表单控件：映射为原生元素（可交互 + 微信风格样式见 base.css）
        "checkbox" => ("input", String::new(), true),
        "radio" => ("input", String::new(), true),
        // slider 编译为容器：内部是 range input + 可选数值（见 slider_inner_html）
        "slider" => ("div", String::new(), false),
        // switch / progress / rich-text 的内部结构在 emit_* 中生成
        "switch" => ("div", String::new(), false),
        "progress" => ("div", String::new(), false),
        "rich-text" => ("div", String::new(), false),
        // 媒体
        "video" => ("video", String::new(), false),
        "audio" => ("audio", String::new(), false),
        "canvas" => ("canvas", String::new(), false),
        _ => ("div", String::new(), false),
    }
}

/// 构建元素的开标签字符串 `<tag ...>`，返回 (开标签, html标签名, 是否自闭合)
fn build_open_tag(node: &WxmlNode) -> (String, &'static str, bool) {
    let (tag, extra_style, void) = map_tag(node);
    let mut attrs = String::new();

    // class：始终带上 wx-<原标签> 基类（用于还原小程序默认盒模型：view=flex列 等），
    // 再拼用户 class，icon 追加内置图标类。
    let mut class = if node.tag_name == "slider" {
        "wx-slider-wrap".to_string()
    } else {
        format!("wx-{}", node.tag_name)
    };
    if let Some(c) = node.get_attr("class") {
        if !c.is_empty() { class.push(' '); class.push_str(c); }
    }
    if node.tag_name == "icon" {
        if let Some(t) = node.get_attr("type") {
            class.push_str(&format!(" wxicon wxicon-{}", t));
        }
    }
    // switch 初始开启状态用类名表示（knob 位置由 CSS 控制）
    if node.tag_name == "switch" && is_truthy(node.get_attr("disabled")) {
        class.push_str(" wx-switch-disabled");
    }
    if node.tag_name == "switch" && is_truthy(node.get_attr("checked")) {
        class.push_str(" wx-switch-on");
    }
    // button 的 type/size/disabled → 修饰类（还原微信默认按钮外观）
    if node.tag_name == "button" {
        match node.get_attr("type") {
            Some("primary") => class.push_str(" wx-button-primary"),
            Some("warn") => class.push_str(" wx-button-warn"),
            Some("default") | None => class.push_str(" wx-button-default"),
            _ => class.push_str(" wx-button-default"),
        }
        if node.get_attr("size") == Some("mini") { class.push_str(" wx-button-mini"); }
        if is_truthy(node.get_attr("plain")) { class.push_str(" wx-button-plain"); }
        if is_truthy(node.get_attr("disabled")) { class.push_str(" wx-button-disabled"); }
    }
    attrs.push_str(&format!(" class=\"{}\"", escape_attr(&class)));
    if let Some(id) = node.get_attr("id") {
        attrs.push_str(&format!(" id=\"{}\"", escape_attr(id)));
    }

    // style（rpx→px）+ 附加样式
    let mut style = node.get_attr("style").map(convert_rpx).unwrap_or_default();
    if !extra_style.is_empty() {
        if !style.is_empty() && !style.trim_end().ends_with(';') { style.push(';'); }
        style.push_str(&extra_style);
    }
    // icon 的 size/color 映射为内联样式：size 决定图标直径（与原生一致），
    // color 作为 CSS color 供内联 SVG 的 currentColor 使用。
    if node.tag_name == "icon" {
        let mut push_style = |s: &str| {
            if !style.is_empty() && !style.trim_end().ends_with(';') { style.push(';'); }
            style.push_str(s);
        };
        if let Some(sz) = node.get_attr("size") {
            let px: f32 = sz.trim().trim_end_matches("px").parse().unwrap_or(23.0);
            push_style(&format!("width:{px}px;height:{px}px"));
        }
        if let Some(col) = node.get_attr("color") {
            push_style(&format!("color:{}", col));
        }
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
            // value 优先 value，其次 model:value（双向绑定初始值）
            if let Some(v) = node.get_attr("value").or_else(|| node.get_attr("model:value")) {
                attrs.push_str(&format!(" value=\"{}\"", escape_attr(v)));
            }
            if let Some(p) = node.get_attr("placeholder") { attrs.push_str(&format!(" placeholder=\"{}\"", escape_attr(p))); }
        }
        "navigator" => {
            if let Some(u) = node.get_attr("url") { attrs.push_str(&format!(" href=\"#{}\"", escape_attr(u))); }
        }
        "checkbox" | "radio" => {
            let ty = if node.tag_name == "checkbox" { "checkbox" } else { "radio" };
            attrs.push_str(&format!(" type=\"{}\"", ty));
            if let Some(v) = node.get_attr("value") { attrs.push_str(&format!(" value=\"{}\"", escape_attr(v))); }
            if is_truthy(node.get_attr("checked")) { attrs.push_str(" checked"); }
            if is_truthy(node.get_attr("disabled")) { attrs.push_str(" disabled"); }
        }

        "swiper" => {
            // 轮播配置透传给 runtime.js（自动播放 / 间隔 / 循环 / 指示点 / 纵向）
            attrs.push_str(&format!(" data-autoplay=\"{}\"", is_truthy(node.get_attr("autoplay"))));
            attrs.push_str(&format!(" data-interval=\"{}\"", node.get_attr("interval").unwrap_or("5000")));
            attrs.push_str(&format!(" data-circular=\"{}\"", is_truthy(node.get_attr("circular"))));
            attrs.push_str(&format!(" data-dots=\"{}\"", is_truthy(node.get_attr("indicator-dots"))));
            if is_truthy(node.get_attr("vertical")) { attrs.push_str(" data-vertical=\"true\""); }
        }
        "video" => {
            if let Some(s) = node.get_attr("src") { attrs.push_str(&format!(" src=\"{}\"", escape_attr(s))); }
            if let Some(p) = node.get_attr("poster") { attrs.push_str(&format!(" poster=\"{}\"", escape_attr(p))); }
            if node.get_attr("controls").map(|v| v != "false").unwrap_or(true) { attrs.push_str(" controls"); }
            if is_truthy(node.get_attr("autoplay")) { attrs.push_str(" autoplay"); }
            if is_truthy(node.get_attr("loop")) { attrs.push_str(" loop"); }
            if is_truthy(node.get_attr("muted")) { attrs.push_str(" muted"); }
            attrs.push_str(" playsinline");
        }
        "audio" => {
            if let Some(s) = node.get_attr("src") { attrs.push_str(&format!(" src=\"{}\"", escape_attr(s))); }
            if node.get_attr("controls").map(|v| v != "false").unwrap_or(true) { attrs.push_str(" controls"); }
        }
        "canvas" => {
            // canvas-id / type 透传，供 runtime 定位并创建 2D 上下文
            if let Some(cid) = node.get_attr("canvas-id") {
                attrs.push_str(&format!(" data-canvas-id=\"{}\"", escape_attr(cid)));
            }
            if let Some(t) = node.get_attr("type") {
                attrs.push_str(&format!(" data-type=\"{}\"", escape_attr(t)));
            }
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

    (format!("<{}{}>", tag, attrs), tag, void)
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
    out.push_str(&format!("{}</{}>\n", indent, tag));
}

/// 基础样式：CSS reset + 常见默认（贴近小程序默认盒模型）+ 内置 icon 图标。
pub fn base_css() -> &'static str {
    r#"
*{box-sizing:border-box;margin:0;padding:0;-webkit-tap-highlight-color:transparent;}
@keyframes wxspin{to{transform:rotate(360deg);}}
/* 还原小程序默认盒模型：view 等默认 flex 纵向排列（与引擎一致，可收缩以适应宽度）*/
.wx-view,.wx-cover-view,.wx-navigator,.wx-checkbox-group,.wx-radio-group,.wx-form,.wx-movable-view,.wx-movable-area{display:flex;flex-direction:column;}
/* scroll-view：块级可滚动容器——子元素保持自身高度、超出则滚动，而非像 flex 那样被压缩 */
.wx-scroll-view{display:block;}
.wx-scroll-view>*{flex-shrink:0;}
/* text：保留数据里的换行（小程序 <text> 中 \n 就是换行），但仍折叠连续空格与缩进 */
.wx-text{display:inline;white-space:pre-line;}
.wx-image{display:block;}
.wx-input,.wx-textarea{border:0;outline:none;background:none;font:inherit;color:inherit;width:100%;}
.wx-navigator{text-decoration:none;color:inherit;}
img{display:block;}
body{font-size:16px;color:#333;font-family:-apple-system,system-ui,"PingFang SC","Hiragino Sans GB",sans-serif;background:#f5f6f8;}

/* ── 交互反馈：带事件的元素显示手型光标 + 按压态 ── */
[data-tap],[data-longpress],[data-longtap],.wx-button,.wx-navigator,.wx-checkbox,.wx-radio,.wx-switch,.wx-slider,.wx-picker{cursor:pointer;}
[data-tap]:active{opacity:.6;}

/* ── button：还原微信默认按钮盒模型（页面样式只需覆盖颜色等即可保持一致） ── */
.wx-button{position:relative;box-sizing:border-box;display:flex;align-items:center;justify-content:center;min-height:46px;padding:0 14px;font-size:17px;line-height:1.35;text-align:center;border:0;border-radius:5px;background:#f7f7f7;color:#000;cursor:pointer;transition:opacity .12s ease;overflow:hidden;}
.wx-button:active{opacity:.85;}
.wx-button-primary{background:#07c160;color:#fff;}
.wx-button-warn{background:#fa5151;color:#fff;}
.wx-button-plain{background:transparent;border:1px solid currentColor;}
.wx-button-mini{display:inline-flex;width:auto;min-height:30px;padding:0 14px;font-size:13px;border-radius:4px;}
.wx-button-disabled{opacity:.5;pointer-events:none;}

/* ── swiper 轮播：横向 scroll-snap，一屏一页 ── */
.wx-swiper{display:flex;flex-direction:row;flex-wrap:nowrap;overflow-x:auto;overflow-y:hidden;height:150px;scroll-snap-type:x mandatory;scroll-behavior:smooth;-webkit-overflow-scrolling:touch;scrollbar-width:none;}
.wx-swiper::-webkit-scrollbar{display:none;width:0;height:0;}
.wx-swiper[data-vertical="true"]{flex-direction:column;overflow-x:hidden;overflow-y:auto;scroll-snap-type:y mandatory;}
.wx-swiper-item{flex:0 0 100%;width:100%;min-width:100%;height:100%;scroll-snap-align:start;display:flex;flex-direction:column;}
.wx-swiper[data-vertical="true"] .wx-swiper-item{flex:0 0 100%;height:100%;}
/* 无 JS（静态首屏/截图）时只显示第一屏，避免各 item 堆叠；JS 接管后加 .wx-swiper-live 恢复多屏滚动 */
.wx-swiper:not(.wx-swiper-live)>.wx-swiper-item:not(:first-child){display:none;}
.wx-swiper-wrap{position:relative;}
.wx-swiper-dots{position:absolute;left:0;right:0;bottom:8px;display:flex;flex-direction:row;justify-content:center;gap:6px;pointer-events:none;}
.wx-swiper-dot{width:7px;height:7px;border-radius:50%;background:rgba(0,0,0,.3);transition:background .2s;}
.wx-swiper-dot.active{background:#fff;box-shadow:0 0 2px rgba(0,0,0,.4);}

/* ── checkbox / radio：自绘微信风格（圆角方框 / 圆形，选中填充微信绿 + 白勾）── */
.wx-checkbox,.wx-radio{-webkit-appearance:none;appearance:none;width:22px;height:22px;margin:0;flex:none;cursor:pointer;box-sizing:border-box;border:1px solid #cfcfcf;background:#fff;transition:background .15s,border-color .15s;vertical-align:middle;}
.wx-checkbox{border-radius:4px;}
.wx-radio{border-radius:50%;}
.wx-checkbox:checked{background:#07c160 url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'%3E%3Cpath d='M3.5 8.5 6.5 11.5 12.5 5' fill='none' stroke='white' stroke-width='2.2' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E") center/15px no-repeat;border-color:#07c160;}
.wx-radio:checked{border-color:#07c160;background:#07c160;box-shadow:inset 0 0 0 3px #fff;}
.wx-checkbox:disabled,.wx-radio:disabled{background:#e6e6e6;border-color:#dcdcdc;}

/* ── switch 开关：还原微信(weui)度量——轨道 52x32、1px 边框、滑块直径 30 ──
   关态是「浅灰边框 + 内部白色胶囊」，开态胶囊缩到 0 露出主色，滑块带投影并缓动位移 */
.wx-switch{flex:none;box-sizing:border-box;width:52px;height:32px;border:1px solid #dfdfdf;border-radius:16px;background:#dfdfdf;position:relative;transition:background-color .2s,border-color .2s;cursor:pointer;display:inline-block;vertical-align:middle;}
.wx-switch::before{content:"";position:absolute;top:0;left:0;width:50px;height:30px;border-radius:15px;background:#fdfdfd;transition:transform .35s cubic-bezier(.45,1,.4,1);}
.wx-switch.wx-switch-on{background:#04be02;border-color:#04be02;}
.wx-switch.wx-switch-on::before{transform:scale(0);}
.wx-switch-knob{position:absolute;top:0;left:0;width:30px;height:30px;border-radius:50%;background:#fff;box-shadow:0 1px 3px rgba(0,0,0,.4);transition:transform .35s cubic-bezier(.4,.4,.25,1.35);}
.wx-switch.wx-switch-on .wx-switch-knob{transform:translateX(20px);}
.wx-switch-disabled{opacity:.45;pointer-events:none;}

/* ── slider 滑块（wrapper + 轨道 + 数值） ── */
.wx-slider-wrap{display:flex;flex-direction:row;align-items:center;}
.wx-slider-value{margin-left:12px;font-size:14px;color:#999;min-width:2.2em;text-align:center;}
.wx-slider{-webkit-appearance:none;appearance:none;width:100%;height:4px;border-radius:2px;background:#e5e5e5;accent-color:#09bb07;cursor:pointer;}
.wx-slider::-webkit-slider-thumb{-webkit-appearance:none;appearance:none;width:22px;height:22px;border-radius:50%;background:#fff;box-shadow:0 1px 4px rgba(0,0,0,.3);}

/* ── progress 进度条 ── */
.wx-progress{display:flex;flex-direction:row;align-items:center;}
.wx-progress-outer{flex:1;height:6px;border-radius:3px;background:#ebebeb;overflow:hidden;}
.wx-progress-inner{height:100%;background:#09bb07;border-radius:3px;transition:width .3s;}
.wx-progress-info{margin-left:8px;font-size:12px;color:#666;min-width:2.5em;text-align:right;}

/* ── tabBar 底部导航（固定，居中对齐 375 宽）── */
.wx-tabbar{position:fixed;left:50%;transform:translateX(-50%);bottom:0;width:375px;max-width:100%;height:50px;display:flex;flex-direction:row;border-top:1px solid #ededed;z-index:500;box-shadow:0 -1px 6px rgba(0,0,0,.04);}
.wx-tabbar-item{flex:1;display:flex;flex-direction:column;align-items:center;justify-content:center;text-decoration:none;padding-top:4px;}
.wx-tabbar-item svg{width:24px;height:24px;display:block;}
.wx-tabbar-text{font-size:11px;line-height:1;margin-top:3px;}

/* ── 媒体 ── */
.wx-video{display:block;width:100%;height:225px;background:#000;}
.wx-canvas{display:block;}
.wx-audio{display:block;width:100%;}

/* 内置矢量图标：内联 SVG（见 icon_svg），圆底取 currentColor，尺寸由 size 属性决定，
   与原生渲染器自绘的矢量图标一一对应（含 waiting 时钟表盘）。*/
.wxicon{display:inline-flex;align-items:center;justify-content:center;width:23px;height:23px;flex:none;font-style:normal;line-height:0;vertical-align:middle;}
.wxicon>svg{width:100%;height:100%;display:block;}
.wxicon-success{color:#09bb07;}
.wxicon-success_no_circle{color:#09bb07;}
.wxicon-info,.wxicon-info_circle{color:#10aeff;}
.wxicon-warn{color:#f76260;}
.wxicon-waiting,.wxicon-waiting_circle{color:#10aeff;}
.wxicon-cancel{color:#f43530;}
.wxicon-download{color:#09bb07;}
.wxicon-search{color:#b2b2b2;}
.wxicon-clear{color:#f43530;}
.wxicon-circle{color:#ccc;}
.wxicon-close,.wxicon-cancel_no_circle,.wxicon-info_no_circle,.wxicon-warn_no_circle,.wxicon-waiting_no_circle,.wxicon-clock{color:currentColor;}
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
