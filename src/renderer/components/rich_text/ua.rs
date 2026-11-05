//! rich-text 里 HTML 标签的默认样式（WebKit `html.css` 的取值；微信 rich-text 在 WebView 下就是这套）。
//!
//! 合成节点的标签名是 `rt-<html 标签>`（tree.rs），`build_base_style` 在页面 CSS 之前落这里的声明 ——
//! 页面里给 rich-text 节点写的 class / style 照样能盖掉它们（与浏览器里 UA 样式的优先级一致）。
//!
//! `em` 不是本引擎通用样式的单位语义（通用路径里 1em 恒 = 16px），这里专门换算：`font-size` 按继承来的字号，
//! 其余按元素自己的字号（所以每个标签的 `font-size` 写在第一条）。
//!
//! 近似（写明）：
//! - 外边距不折叠（弹性盒没有 margin collapsing）：不是第一个孩子的块**不落 margin-top**，相邻块之间的距离
//!   取前一块的 margin-bottom（p→p、h→p 与浏览器的「取大者」相同；p→h2 这种后者更大时会窄一点）；
//! - `font-style: italic`、`vertical-align: sub/super` 引擎不支持（i / em 不斜、sub / sup 只缩小不偏移）；
//! - hr 的 `border: 1px inset` 画成上深下浅两条 1px 线；表格按弹性盒排（tr 横排、单元格等分），不支持 colspan / rowspan
//!   （Skyline 的 rich-text 本来也「不支持 td/tr 等表格布局」，官方 rich-text 页）。
use std::borrow::Cow;

pub fn declarations(tag: &str) -> &'static [(&'static str, &'static str)] {
    let Some(html) = tag.strip_prefix("rt-") else { return &[] };
    match html {
        "h1" => &[("font-size", "2em"), ("font-weight", "bold"), ("margin-top", "0.67em"), ("margin-bottom", "0.67em")],
        "h2" => &[("font-size", "1.5em"), ("font-weight", "bold"), ("margin-top", "0.83em"), ("margin-bottom", "0.83em")],
        "h3" => &[("font-size", "1.17em"), ("font-weight", "bold"), ("margin-top", "1em"), ("margin-bottom", "1em")],
        "h4" => &[("font-weight", "bold"), ("margin-top", "1.33em"), ("margin-bottom", "1.33em")],
        "h5" => &[("font-size", "0.83em"), ("font-weight", "bold"), ("margin-top", "1.67em"), ("margin-bottom", "1.67em")],
        "h6" => &[("font-size", "0.67em"), ("font-weight", "bold"), ("margin-top", "2.33em"), ("margin-bottom", "2.33em")],
        "p" | "dl" => &[("margin-top", "1em"), ("margin-bottom", "1em")],
        "blockquote" => &[("margin-top", "1em"), ("margin-bottom", "1em"), ("margin-left", "40px"), ("margin-right", "40px")],
        "figure" => &[("margin-top", "1em"), ("margin-bottom", "1em"), ("margin-left", "40px"), ("margin-right", "40px")],
        "ul" | "ol" => &[("margin-top", "1em"), ("margin-bottom", "1em"), ("padding-left", "40px")],
        // 嵌套的列表没有上下外边距（`ul ul, ol ul … { margin-block: 0 }`）
        "ul-nested" | "ol-nested" => &[("padding-left", "40px")],
        "dd" => &[("margin-left", "40px")],
        // 项目符号挂在 li 左边的内边距里（list-style-position: outside）
        "li" => &[("position", "relative")],
        "marker" => &[("position", "absolute"), ("right", "100%"), ("top", "0"), ("padding-right", "0.5em"), ("white-space", "nowrap")],
        "pre" => &[("font-size", "0.8125em"), ("font-family", "monospace"), ("white-space", "pre"), ("margin-top", "1em"), ("margin-bottom", "1em")],
        "hr" => &[
            ("height", "0"),
            ("margin-top", "0.5em"),
            ("margin-bottom", "0.5em"),
            ("border-top", "1px solid #9a9a9a"),
            ("border-bottom", "1px solid #eeeeee"),
        ],
        "center" | "caption" => &[("text-align", "center")],
        "th" => &[("font-weight", "bold"), ("text-align", "center"), ("flex", "1"), ("padding", "1px")],
        "td" => &[("flex", "1"), ("padding", "1px")],
        "tr" => &[("flex-direction", "row")],
        "b" | "strong" => &[("font-weight", "bold")],
        "u" | "ins" => &[("text-decoration", "underline")],
        "s" | "del" | "strike" => &[("text-decoration", "line-through")],
        "small" | "sub" | "sup" => &[("font-size", "0.83em")],
        "big" => &[("font-size", "1.2em")],
        "code" | "tt" | "kbd" | "samp" => &[("font-size", "0.8125em"), ("font-family", "monospace")],
        "mark" => &[("background-color", "#ffff00"), ("color", "#000000")],
        _ => &[],
    }
}

/// 落一条之前的取值换算：`em` → px。`inherited_px` 是父级字号（给 font-size 用），`own_px` 是元素当前字号
pub fn resolve<'a>(name: &str, value: &'a str, inherited_px: f32, own_px: f32) -> Cow<'a, str> {
    let Some(n) = value.strip_suffix("em").and_then(|n| n.trim().parse::<f32>().ok()) else { return Cow::Borrowed(value) };
    let base = if name == "font-size" { inherited_px } else { own_px };
    Cow::Owned(format!("{}px", n * base))
}

/// 不是父级第一个孩子的块不落 margin-top（见文件头「外边距不折叠」）
pub fn skip(name: &str, sibling_index: usize) -> bool {
    name == "margin-top" && sibling_index > 0
}
