//! `<text>` 的内容属性与行数限制（官方 component/text.html）：
//!
//! - `decode`（默认 false）：解码 `&nbsp; &lt; &gt; &amp; &apos; &ensp; &emsp;`（官方 tip 列的就这七个；
//!   `&quot;`、数字实体不在表里，照原样显示 —— 待验）；
//! - `space`（ensp / emsp / nbsp）：连续空格按这种空格显示（不合并）。`ensp` = 半个中文字宽（U+2002）、
//!   `emsp` = 一个中文字宽（U+2003）、`nbsp` = 字体的空格宽（U+00A0）；
//! - Skyline 的 `max-lines` + `overflow`（clip / fade / ellipsis / visible，默认 visible）；
//! - CSS 的 `display:-webkit-box; -webkit-line-clamp:N`（写在 text 自己或它的父级上），最后一行末尾省略号。
//!
//! 都是**先判再分配**：text 是每个节点都走的热路径，没写这些属性的文字一次分配都不多。
use super::base::*;
use crate::parser::wxml::WxmlNode;
use std::borrow::Cow;

/// 布尔属性：写了且不是 `false` / `0`（`decode` 光写属性名也算 true，与微信一致）
fn attr_on(node: &WxmlNode, k: &str) -> bool {
    match node.get_attr(k) {
        Some(v) => !matches!(v.trim(), "false" | "0"),
        None => false,
    }
}

/// `space` 属性对应的空格字符
fn space_char(node: &WxmlNode) -> Option<char> {
    match node.get_attr("space")?.trim() {
        "ensp" => Some('\u{2002}'),
        "emsp" => Some('\u{2003}'),
        "nbsp" => Some('\u{00A0}'),
        _ => None,
    }
}

/// 按节点自己的 `decode` / `space` 改写文字。两个都没写（绝大多数）原样借出去，不分配
pub fn apply_content_attrs<'a>(node: &WxmlNode, text: Cow<'a, str>) -> Cow<'a, str> {
    if node.attributes.is_empty() {
        return text;
    }
    let decode = attr_on(node, "decode") && text.contains('&');
    let space = space_char(node).filter(|_| text.contains(' '));
    if !decode && space.is_none() {
        return text;
    }
    let mut s = text.into_owned();
    // 先换空格再解码：`&nbsp;` 解出来的不间断空格不该再被 space 改写
    if let Some(c) = space {
        s = s.replace(' ', c.encode_utf8(&mut [0u8; 4]));
    }
    if decode {
        s = decode_entities(&s).into_owned();
    }
    Cow::Owned(s)
}

/// 官方列出的七个实体
pub fn decode_entities(s: &str) -> Cow<'_, str> {
    if !s.contains('&') {
        return Cow::Borrowed(s);
    }
    const TABLE: [(&str, char); 7] = [
        ("&nbsp;", '\u{00A0}'),
        ("&lt;", '<'),
        ("&gt;", '>'),
        ("&amp;", '&'),
        ("&apos;", '\''),
        ("&ensp;", '\u{2002}'),
        ("&emsp;", '\u{2003}'),
    ];
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        let tail = &rest[p..];
        match TABLE.iter().find(|(e, _)| tail.starts_with(e)) {
            Some((e, c)) => {
                out.push(*c);
                rest = &tail[e.len()..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
}

/// 把这段文字最终的行数限制写进 `ns.line_clamp` / `ns.clamp_overflow`：
/// Skyline `max-lines` 属性优先；其次自己的 `-webkit-line-clamp`（要 `display:-webkit-box`）；
/// 再次父级 -webkit-box 交下来的行数。都没有就清零（`line_clamp` 在非 -webkit-box 上不生效，同浏览器）
pub fn resolve_clamp(node: &WxmlNode, ns: &mut NodeStyle, inherited: u16) {
    if let Some(n) = node.get_attr("max-lines").and_then(|v| v.trim().parse::<f32>().ok()).filter(|n| *n >= 1.0) {
        ns.line_clamp = n.min(u16::MAX as f32) as u16;
        // 官方默认 visible（文本不截断）
        ns.clamp_overflow = match node.get_attr("overflow").map(str::trim) {
            Some("clip") => ClampOverflow::Clip,
            Some("fade") => ClampOverflow::Fade,
            Some("ellipsis") => ClampOverflow::Ellipsis,
            _ => ClampOverflow::Visible,
        };
        return;
    }
    ns.clamp_overflow = ClampOverflow::Ellipsis;
    if !(ns.webkit_box && ns.line_clamp > 0) {
        ns.line_clamp = inherited;
    }
}

/// 父级 `-webkit-box` 夹了行数的**裸文本**（`<view class="title">{{t}}</view>`，最常见的多行省略写法）：
/// 建成走 taffy 文本度量的叶子 —— 按容器宽折行、最多 N 行高（`TextMeasure::max_lines`）。
///
/// 别的裸文本仍是「单行内容宽的定宽叶子」（layout.rs 的老路径：它们不按容器宽折行，是现状），
/// 只有用到行数限制的才换成这条，没写 line-clamp 的页面一个像素都不变
pub fn clamped_bare_leaf(taffy: &mut Tree, text: &str, st: &NodeStyle, sf: f32) -> Option<taffy::NodeId> {
    let family = crate::text_family::renderer_for_family(st.font_family.as_deref());
    let shared = crate::text::shared_fonts();
    let tr = family.as_deref().or(shared.as_deref());
    let font_px = st.font_size * sf;
    let ls = st.letter_spacing * sf;
    let natural = tr.map(|t| t.natural_line_height_for(text, font_px)).unwrap_or(font_px * crate::text::NORMAL_LINE_HEIGHT_FACTOR);
    let line_h = st.line_height.map(|l| l * sf).unwrap_or(natural).max(font_px);
    let max_line_width = text
        .split('\n')
        .map(|l| tr.map(|t| t.measure_text_weighted(l, font_px, ls, false)).unwrap_or(l.chars().count() as f32 * font_px))
        .fold(0.0f32, f32::max)
        .ceil();
    let tm = TextMeasure {
        text: text.to_string(),
        font_px,
        letter_spacing_px: ls,
        line_height_px: line_h,
        pad_l: 0.0,
        pad_r: 0.0,
        pad_t: 0.0,
        pad_b: 0.0,
        nowrap: matches!(st.white_space, WhiteSpace::NoWrap | WhiteSpace::Pre),
        bold: false,
        min_lines: text.matches('\n').count() + 1,
        max_line_width,
        min_unit_width: min_unit_width(text, tr, font_px, ls, false),
        font_family: st.font_family.clone(),
        max_lines: st.line_clamp as usize,
    };
    let style = taffy::style::Style { flex_shrink: 1.0, ..Default::default() };
    taffy.new_leaf_with_context(style, tm).ok()
}

/// 父级交给直接子文本的行数（`InheritedText::line_clamp`）
pub fn clamp_for_children(ns: &NodeStyle) -> u16 {
    if ns.webkit_box { ns.line_clamp } else { 0 }
}

/// `overflow="fade"` 的最后一行：盒子右缘往左一个「…」宽的范围里逐字变淡（Skyline 是 Flutter 式渲染，
/// Flutter 的 fade 就是在这段宽度上铺一条横向渐变）。逐字设透明度是近似：一个字内部不渐变
#[allow(clippy::too_many_arguments)]
pub fn draw_faded_line(
    canvas: &mut crate::Canvas,
    tr: &crate::text::TextRenderer,
    line: &str,
    x: f32,
    y: f32,
    size: f32,
    ls: f32,
    bold: bool,
    paint: &crate::Paint,
    box_right: f32,
) {
    let fade_w = tr.measure_text_weighted("\u{2026}", size, ls, bold).max(1.0);
    let fade_start = box_right - fade_w;
    let mut cx = x;
    let mut buf = [0u8; 4];
    for ch in line.chars() {
        let w = tr.measure_char_weighted(ch, size, bold) + ls;
        let mid = cx + w / 2.0;
        let k = if mid <= fade_start { 1.0 } else { ((box_right - mid) / fade_w).clamp(0.0, 1.0) };
        if k > 0.0 {
            let mut p = paint.clone();
            p.color.a = (p.color.a as f32 * k).round() as u8;
            tr.draw_text_weighted(canvas, ch.encode_utf8(&mut buf), cx, y, size, ls, bold, &p);
        }
        cx += w;
    }
}

/// 第 N 行（后面还有字）画在哪：`x` 是内容区左边，`lx` 是按 text-align 对齐后的行首
pub struct ClampTail {
    pub x: f32,
    pub lx: f32,
    pub y: f32,
    pub size: f32,
    pub max_width: f32,
    pub ls: f32,
    pub bold: bool,
}

/// 行数限制的最后一行：按 overflow 收尾（text.rs 的多行绘制在第 N 行、后面还有字时调）
pub fn draw_clamp_tail(
    canvas: &mut crate::Canvas,
    tr: &crate::text::TextRenderer,
    line: &[char],
    mode: ClampOverflow,
    t: &ClampTail,
    paint: &crate::Paint,
) {
    let s: String = line.iter().collect();
    match mode {
        ClampOverflow::Ellipsis => {
            // 同 Chrome 的 line-clamp：这一行按原样断好，省略号放在行尾，放不下就从行尾往回让出位置
            // （不把下一行的字拉上来）。点用 "..."：与本引擎单行 text-overflow 的省略号同一个写法
            let dots_w = tr.measure_text_weighted("...", t.size, t.ls, t.bold);
            let mut end = line.len();
            let mut w: f32 = line.iter().map(|c| tr.measure_char_weighted(*c, t.size, t.bold) + t.ls).sum();
            while end > 0 && w + dots_w > t.max_width + LAYOUT_ROUNDING_SLACK_PX {
                end -= 1;
                w -= tr.measure_char_weighted(line[end], t.size, t.bold) + t.ls;
            }
            let mut shown: String = line[..end].iter().collect::<String>().trim_end().to_string();
            shown.push_str("...");
            tr.draw_text_weighted(canvas, &shown, t.x, t.y, t.size, t.ls, t.bold, paint);
        }
        ClampOverflow::Fade => draw_faded_line(canvas, tr, &s, t.lx, t.y, t.size, t.ls, t.bold, paint, t.x + t.max_width),
        _ => tr.draw_text_weighted(canvas, &s, t.lx, t.y, t.size, t.ls, t.bold, paint),
    }
}

/// 行数按限制截一下（0 = 不限）
pub fn clamp_lines(lines: usize, max_lines: u16) -> usize {
    if max_lines == 0 { lines } else { lines.min(max_lines as usize).max(1) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 解码官方七个实体_其余原样() {
        assert_eq!(decode_entities("a&nbsp;b&lt;c&gt;&amp;&apos;&ensp;&emsp;&quot;&x"), "a\u{a0}b<c>&'\u{2002}\u{2003}&quot;&x");
        assert!(matches!(decode_entities("plain"), Cow::Borrowed(_)));
    }
}
