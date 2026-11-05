//! 行内混排：`<span>`（Skyline，官方 component/span：内联文本和 image 的混排）与 rich-text 的段落。
//!
//! 引擎没有真正的行内格式化上下文，做法与 `<text>` 嵌套 `<text>` 一样（text.rs `build_inline_runs`）：
//! 容器是「横排 + 换行 + 基线对齐」的弹性盒，内容切成一个个 run —— 文字按可换行单元（汉字逐字、西文整词）
//! 一个 run，图片整张一个 run。于是文字可以接在图片后面、到行尾折行，各段保留各自的样式（class / style）。
//! 比 `<text>` 多出来的：
//! - 子节点可以是图片（`image` / `cover-image`），也可以是 rich-text 的行内标签（`rt-b`、`rt-span` …，
//!   rich_text/tree.rs），而且可以**多层**嵌套（`<b><i>x</i></b>`：每层按自己的样式、祖先链匹配选择器）；
//! - 硬换行（`<br>`、`white-space:pre` 里的 `\n`）真的换行：一个宽 100%、高 0 的 run 把后面的内容挤到下一行，
//!   连着的第二个换行占一整行高（空行）。
//!
//! 近似（写明，不假装）：行尾的空格不会被吃掉；`vertical-align`（sub / sup）不偏移；
//! 带底色 / 内边距的一段整段一个 run（不能在中间折行，同 inline-block）；run 上不带事件（同 text 嵌套），
//! 图片 run 保留自己的事件。子节点里有别的组件（navigator、view …）时整个 span 退回普通容器
//! （横排换行，`native_placeholder::ua_declarations`），每个子节点一个盒子。
use super::base::*;
use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::parser::wxss::ElementDesc;
use taffy::prelude::*;

/// 与 text.rs 同一把尺子（系统字体；`font-family` 指定的字族另取）
static MEASURE: once_cell::sync::Lazy<Option<std::sync::Arc<crate::text::TextRenderer>>> =
    once_cell::sync::Lazy::new(crate::text::shared_fonts);

/// rich-text 行内标签（合成节点的标签名带 `rt-` 前缀，见 rich_text/tree.rs）与 text：可以拆进 run 的元素
pub fn is_inline_tag(tag: &str) -> bool {
    tag == "text" || (tag.starts_with("rt-") && super::rich_text::tree::is_inline_html(&tag[3..]))
}

fn is_image(tag: &str) -> bool {
    matches!(tag, "image" | "cover-image")
}

/// 这棵子树能不能整个拆成 run
fn flowable(node: &WxmlNode) -> bool {
    node.children.iter().all(|c| match c.node_type {
        WxmlNodeType::Element => is_image(&c.tag_name) || (is_inline_tag(&c.tag_name) && flowable(c)),
        _ => true,
    })
}

/// `<span>` 的建树。拆不成 run（有别的组件）时按普通容器建，孩子交给渲染器照常建（layout.rs `span_as_container`）
pub fn build_span(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
    if !flowable(node) {
        return super::ViewComponent::build(node, ctx);
    }
    let (mut ts, mut ns) = build_base_style(node, ctx);
    let mut runs = Vec::new();
    let mut ancestors = ctx.ancestors.clone();
    ancestors.push(desc_of(node, ctx.stylesheet));
    let mut at_line_start = true;
    collect(node, &ns, &ancestors, ctx, &mut runs, &mut at_line_start);
    if runs.is_empty() {
        return None;
    }
    if ts.display != Display::None {
        ts.display = Display::Flex;
    }
    ts.flex_direction = FlexDirection::Row;
    ts.flex_wrap = if matches!(ns.white_space, WhiteSpace::NoWrap | WhiteSpace::Pre) { FlexWrap::NoWrap } else { FlexWrap::Wrap };
    ts.align_items = Some(AlignItems::BASELINE);
    super::text_attrs::resolve_clamp(node, &mut ns, ctx.inherited.line_clamp);
    clamp_container(&mut ts, &mut ns, ctx.scale_factor);
    let ids: Vec<NodeId> = runs.iter().map(|r: &RenderNode| r.taffy_node).collect();
    let tn = ctx.taffy.new_with_children(ts, &ids).ok()?;
    Some(RenderNode {
        tag: node.tag_name.clone(),
        text: String::new(),
        attrs: node.attributes.clone(),
        taffy_node: tn,
        style: ns,
        children: runs,
        events: extract_events(node),
    })
}

/// 混排容器的行数限制：最多 N 行高，其余裁掉（run 各是一个盒子，最后一行补不了省略号 —— 近似）
pub fn clamp_container(ts: &mut Style, ns: &mut NodeStyle, sf: f32) {
    if ns.line_clamp == 0 || ns.clamp_overflow == ClampOverflow::Visible {
        return;
    }
    let font_px = ns.font_size * sf;
    let lh = ns.line_height.map(|l| l * sf).unwrap_or_else(|| natural_line_height_px_for("中", font_px)).max(font_px);
    ts.max_size.height = length(lh * ns.line_clamp as f32);
    ts.overflow.x = taffy::style::Overflow::Hidden;
    ts.overflow.y = taffy::style::Overflow::Hidden;
    ns.overflow = Overflow::Hidden;
}

/// 祖先链上的一层。属性表只有 `[attr]` 选择器用得到，没有这类规则就不带（同 layout.rs，省一次深拷贝）
fn desc_of(n: &WxmlNode, sheet: &crate::parser::wxss::StyleSheet) -> ElementDesc {
    static EMPTY: std::sync::OnceLock<std::collections::HashMap<String, String>> = std::sync::OnceLock::new();
    let classes = get_classes(n);
    let attrs = if sheet.has_attr_selectors() { &n.attributes } else { EMPTY.get_or_init(Default::default) };
    ElementDesc::new(&n.tag_name, n.get_attr("id"), &classes, attrs)
}

/// 子节点的建树上下文：祖先链换成 `ancestors`，继承的文字样式来自 `outer`
fn sub_ctx<'b>(ctx: &'b mut ComponentContext, ancestors: &[ElementDesc], outer: &NodeStyle, idx: usize, count: usize) -> ComponentContext<'b> {
    ComponentContext {
        scale_factor: ctx.scale_factor,
        screen_width: ctx.screen_width,
        screen_height: ctx.screen_height,
        stylesheet: ctx.stylesheet,
        taffy: &mut *ctx.taffy,
        ancestors: ancestors.to_vec(),
        inherited: InheritedText {
            font_size: outer.font_size,
            color: outer.text_color,
            weight: outer.font_weight,
            align: outer.text_align,
            line_height: outer.line_height,
            letter_spacing: outer.letter_spacing,
            font_family: outer.font_family.clone(),
            line_clamp: 0,
            white_space: None,
        },
        sibling_index: idx,
        sibling_count: count,
        has_positioned_ancestor: ctx.has_positioned_ancestor,
    }
}

fn collect(parent: &WxmlNode, outer: &NodeStyle, ancestors: &[ElementDesc], ctx: &mut ComponentContext, runs: &mut Vec<RenderNode>, at_line_start: &mut bool) {
    let n = parent.children.len();
    for (i, c) in parent.children.iter().enumerate() {
        match c.node_type {
            WxmlNodeType::Element if is_image(&c.tag_name) => {
                let built = super::ImageComponent::build(c, &mut sub_ctx(ctx, ancestors, outer, i, n));
                if let Some(mut img) = built {
                    img.tag = "image".into();
                    if let Ok(mut st) = ctx.taffy.style(img.taffy_node).cloned() {
                        st.flex_shrink = 0.0;
                        ctx.taffy.set_style(img.taffy_node, st).ok();
                    }
                    runs.push(img);
                    *at_line_start = false;
                }
            }
            WxmlNodeType::Element => {
                let (_, cns) = build_base_style(c, &mut sub_ctx(ctx, ancestors, outer, i, n));
                if c.children.iter().any(|g| g.node_type == WxmlNodeType::Element) {
                    let mut chain = ancestors.to_vec();
                    chain.push(desc_of(c, ctx.stylesheet));
                    collect(c, &cns, &chain, ctx, runs, at_line_start);
                    continue;
                }
                // `<text>` 的内容照 text 组件的规矩（去首尾空白、decode / space）；rich-text 的行内标签原样（空白已在合成时处理）
                let raw = if c.tag_name == "text" {
                    super::text_attrs::apply_content_attrs(c, std::borrow::Cow::Owned(get_text_content(c))).into_owned()
                } else {
                    c.children.iter().map(|g| g.text_content.as_str()).collect::<String>()
                };
                push_text(&raw, &cns, true, ctx, runs, at_line_start);
            }
            WxmlNodeType::Text => push_text(&c.text_content, outer, false, ctx, runs, at_line_start),
            _ => {}
        }
    }
}

/// 一段同样式的文字切成 run（`element`：来自子元素，带底色 / 内边距 / 边框时整段一个 run）
fn push_text(text: &str, st: &NodeStyle, element: bool, ctx: &mut ComponentContext, runs: &mut Vec<RenderNode>, at_line_start: &mut bool) {
    if text.is_empty() || (text.trim().is_empty() && !text.contains(' ') && !text.contains('\n')) {
        return;
    }
    let sf = ctx.scale_factor;
    let family = crate::text_family::renderer_for_family(st.font_family.as_deref());
    let measure = family.as_deref().or(MEASURE.as_deref());
    let font_px = st.font_size * sf;
    let ls = st.letter_spacing * sf;
    let bold = matches!(st.font_weight, FontWeight::Bold | FontWeight::W600 | FontWeight::W700 | FontWeight::W800 | FontWeight::W900)
        && measure.map(|tr| tr.has_bold_face()).unwrap_or(false);
    let decorated = element
        && (st.background_color.is_some()
            || st.padding_left > 0.0
            || st.padding_right > 0.0
            || st.padding_top > 0.0
            || st.padding_bottom > 0.0
            || st.border_width > 0.0);
    let (pad_w, pad_h) = if decorated {
        ((st.padding_left + st.padding_right) * sf, (st.padding_top + st.padding_bottom) * sf)
    } else {
        (0.0, 0.0)
    };
    let units = if decorated && !text.contains('\n') { vec![text.to_string()] } else { super::rich_text::split_wrappable(text) };
    let line_h_of = |u: &str| st.line_height.map(|lh| lh * sf).unwrap_or_else(|| natural_line_height_px_for(u, font_px)).max(font_px);
    for unit in units {
        if unit.is_empty() {
            continue;
        }
        let cts = if unit == "\n" {
            // 硬换行：整行宽、高 0 的盒子把后面的内容挤到下一行；行首的换行（空行）占一行高
            let h = if *at_line_start { line_h_of("中") } else { 0.0 };
            *at_line_start = true;
            Style { size: Size { width: percent(1.0_f32), height: length(h) }, flex_shrink: 0.0, ..Default::default() }
        } else {
            *at_line_start = false;
            let w = measure
                .map(|tr| tr.measure_text_weighted(&unit, font_px, ls, bold))
                .unwrap_or_else(|| unit.chars().map(|c| if c.is_ascii() { font_px * 0.62 } else { font_px }).sum())
                .ceil()
                + pad_w;
            Style { size: Size { width: length(w), height: length(line_h_of(&unit) + pad_h) }, flex_shrink: 0.0, ..Default::default() }
        };
        let Ok(tn) = ctx.taffy.new_leaf(cts) else { continue };
        let brk = unit == "\n";
        runs.push(RenderNode {
            tag: "text".into(),
            text: if brk { String::new() } else { unit },
            attrs: std::collections::HashMap::new(),
            taffy_node: tn,
            // 换行盒什么都不画（带底色的一段里的换行不能铺一整行底色）
            style: if brk { NodeStyle { opacity: 1.0, ..Default::default() } } else { st.clone() },
            children: vec![],
            events: vec![],
        });
    }
}
