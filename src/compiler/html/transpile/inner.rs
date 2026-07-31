//! 需要内部结构的组件：slider 轨道、icon、rich-text、容器后缀
//!
//! `transpile_new` 的一片。**纯搬迁**：从 805 行按职责切开，一行逻辑没改。
use super::emit::{escape_attr, escape_text, is_truthy};
use super::*;

/// 判断某标签是否需要在 emit 时生成自定义内部结构（switch/progress/rich-text/icon）
pub(super) fn has_custom_inner(tag: &str) -> bool {
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

/// 容器类标签在子节点之后追加的静态结构。
///
/// swiper 的指示点原来只由 runtime.js 在水合时创建，于是「无 JS 的静态首屏」
/// （也包括双端对比截图）里没有圆点，而原生端一直画着 —— 两端凭空差一排点。
pub(super) fn container_suffix_html(node: &WxmlNode) -> Option<String> {
    if node.tag_name != "swiper" || !is_truthy(node.get_attr("indicator-dots")) {
        return None;
    }
    let total = node
        .children
        .iter()
        .filter(|c| c.node_type == WxmlNodeType::Element)
        .count();
    if total < 2 {
        return None;
    }
    let active = node
        .get_attr("current")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0)
        .min(total - 1);
    let color = node.get_attr("indicator-color").unwrap_or("");
    let active_color = node.get_attr("indicator-active-color").unwrap_or("");
    let mut dots = String::from("<div class=\"wx-swiper-dots\">");
    for i in 0..total {
        let on = i == active;
        let style = if on && !active_color.is_empty() {
            format!(" style=\"background:{}\"", escape_attr(active_color))
        } else if !on && !color.is_empty() {
            format!(" style=\"background:{}\"", escape_attr(color))
        } else {
            String::new()
        };
        dots.push_str(&format!(
            "<i class=\"wx-swiper-dot{}\"{}></i>",
            if on { " active" } else { "" },
            style
        ));
    }
    dots.push_str("</div>");
    Some(dots)
}

/// `<icon>` 的矢量图形（内联 SVG）。
///
/// 图形数据与原生渲染器共用 `icon_data` 那张 WeUI 字形表 —— 以前两边各写一套
/// 近似图形（这边圆 + 白描边、那边圆 + 填充多边形），对勾比例/叉号粗细都对不上，
/// 而且都不像微信。共用一张表之后，HTML 参考图和原生渲染在几何上同源。
pub(super) fn icon_svg(icon_type: &str) -> String {
    crate::renderer::components::icon_data::icon_svg_markup(icon_type)
}

/// 生成 switch/progress/rich-text 的内部 HTML
pub(super) fn custom_inner_html(node: &WxmlNode) -> String {
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
