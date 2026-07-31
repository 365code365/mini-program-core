//! 标签映射与开标签生成：wx 组件 → HTML 元素 + data-* 事件桩
//!
//! `transpile_new` 的一片。**纯搬迁**：从 805 行按职责切开，一行逻辑没改。
use super::emit::{escape_attr, is_truthy};
use super::*;

/// 标签映射：返回 (html标签, 附加内联样式, 是否自闭合)
pub(super) fn map_tag(node: &WxmlNode) -> (&'static str, String, bool) {
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
pub(super) fn build_open_tag(node: &WxmlNode) -> (String, &'static str, bool) {
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
    // 排序遍历：HashMap 的随机顺序会让导出的 HTML 每次编译都不一样（见 attrs_sorted）
    for (k, v) in node.attrs_sorted() {
        let ev = k.strip_prefix("bind").or_else(|| k.strip_prefix("catch"));
        if let Some(ev) = ev {
            if !ev.is_empty() {
                attrs.push_str(&format!(" data-{}=\"{}\"", ev, escape_attr(v)));
            }
        }
    }
    for (k, v) in node.attrs_sorted() {
        if let Some(dk) = k.strip_prefix("data-") {
            attrs.push_str(&format!(" data-ds-{}=\"{}\"", dk, escape_attr(v)));
        }
    }

    (format!("<{}{}>", tag, attrs), tag, void)
}
