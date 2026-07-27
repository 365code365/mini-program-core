//! text 组件 - 文本显示

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle};
use taffy::prelude::*;

/// 与渲染器同款的系统字体（用于 build 阶段按真实字形宽度测量文本盒子宽度，
/// 避免用粗糙估算导致盒子偏窄、二次布局误判换行）。
static TEXT_MEASURE_FONT: once_cell::sync::Lazy<Option<std::sync::Arc<TextRenderer>>> =
    once_cell::sync::Lazy::new(crate::text::shared_fonts);

pub struct TextComponent;

impl TextComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        
        let text_content = get_text_content(node);
        if text_content.is_empty() { return None; }

        // `<text>` 里嵌套 `<text>`：每一段用**自己**的样式。
        //
        // 这是小程序里很常见的写法 —— 价格 `<text class="amt">¥<text class="num">99</text></text>`、
        // 必填项 `<text class="label">挂牌名 <text class="req">*</text></text>`。
        // 从前的做法是把整棵子树的文字**拍平成一个字符串**，用最外层的样式画一遍：
        // 优惠券页的「¥10」于是整个是 32rpx，而 `.c-num` 明明写了 64rpx（浏览器里 ¥ 小、数字大）。
        //
        // 处理方式与 `rich-text` 一致：容器改成「行内排列 + 允许换行 + 基线对齐」，
        // 把混排内容切成一个个 run（裸文字按最外层样式，嵌套 `<text>` 按它自己的 CSS），
        // 每个 run 再按可换行单元细分，这样既保留各自样式又不会整段无法折行。
        if let Some(runs) = Self::build_inline_runs(node, ctx, &ns) {
            ts.flex_direction = FlexDirection::Row;
            ts.flex_wrap = FlexWrap::Wrap;
            ts.align_items = Some(AlignItems::BASELINE);
            let child_ids: Vec<NodeId> = runs.iter().map(|c| c.taffy_node).collect();
            let tn = ctx.taffy.new_with_children(ts, &child_ids).unwrap();
            return Some(RenderNode {
                tag: "text".into(),
                // 文本交给 run 子节点画，容器自己不再画一遍
                text: String::new(),
                attrs,
                taffy_node: tn,
                style: ns,
                children: runs,
                events,
            });
        }
        
        let sf = ctx.scale_factor;
        let font_size = ns.font_size * sf;
        // 无显式 line-height 时，用字体自然行高（≈浏览器 line-height:normal），
        // 而非写死 1.5，避免与 HTML 逐行累积垂直漂移。
        // 度量必须用**这段文字实际会用的字体**：字体不同，字形宽度与自然行高都不同，
        // 用默认字体量、用宋体画的话盒子宽度与文字对不上（换行位置也会错）。
        let family_font = crate::text_family::renderer_for_family(ns.font_family.as_deref());
        let measure_font_arc = family_font.clone().or_else(|| TEXT_MEASURE_FONT.clone());
        let natural_lh = measure_font_arc
            .as_ref()
            .map(|tr| tr.natural_line_height_for(&text_content, font_size))
            .unwrap_or(font_size * crate::text::NORMAL_LINE_HEIGHT_FACTOR);
        let line_height = ns.line_height.map(|lh| lh * sf).unwrap_or(natural_lh);
        // 确保 line_height 至少等于字号
        let actual_line_height = line_height.max(font_size);
        
        // 计算文本行数（考虑换行符）
        let newline_count = text_content.matches('\n').count();
        let min_lines = (newline_count + 1).max(1);
        
        // 单行最大宽度：优先用真实字体度量（与绘制/二次布局一致），无字体时回退估算。
        let letter_spacing = ns.letter_spacing * sf;
        let measure_font = measure_font_arc.as_deref();
        // 字重影响字形宽度：布局度量必须与绘制所用字面一致
        let is_bold_weight = matches!(
            ns.font_weight,
            FontWeight::Bold | FontWeight::W600 | FontWeight::W700 | FontWeight::W800 | FontWeight::W900
        );
        let measure_bold = is_bold_weight && measure_font.map(|tr| tr.has_bold_face()).unwrap_or(false);
        let mut max_line_width: f32 = 0.0;
        for line in text_content.split('\n') {
            let line_width = if let Some(tr) = measure_font {
                tr.measure_text_weighted(line, font_size, letter_spacing, measure_bold)
            } else {
                line.chars().map(|c| if c.is_ascii() { font_size * 0.62 } else { font_size }).sum()
            };
            max_line_width = max_line_width.max(line_width);
        }
        // 向上取整到整像素即可，不再额外加宽。
        //
        // 这里原来无条件 `+= 4px`，名义上是"吸收亚像素误差防误换行"，代价是**每一个**
        // 靠内容定宽的文本盒都比浏览器宽 4px：徽标/标签/胶囊会明显臃肿（分类页 badge
        // 实测 23.5px vs Chrome 18.9px），居中文本的可用宽也整体偏大。
        // 正确做法是在「是否换行」的比较里留亚像素容差（见 WRAP_TOLERANCE_PX），
        // 而不是把盒子本身撑大。
        max_line_width = max_line_width.ceil();
        
        // 设置 flex-shrink 允许收缩
        ts.flex_shrink = 1.0;
        
        // 文本自身内边距：清空 taffy padding，改为把 padding 计入盒子尺寸，
        // 绘制时再把文字画进内容区（修复带 padding 的 <text>，如标签/徽章，文字溢出背景）
        let pad_t = ns.padding_top * sf;
        let pad_b = ns.padding_bottom * sf;
        let pad_l = ns.padding_left * sf;
        let pad_r = ns.padding_right * sf;
        ts.padding = Rect::zero();
        
        let content_w = max_line_width + pad_l + pad_r;
        let nowrap = matches!(ns.white_space, WhiteSpace::NoWrap | WhiteSpace::Pre);

        // display:block 或居中/右对齐且宽度未显式指定：交给 taffy 文本度量。
        // 度量提供 min-content / max-content，使文本在定宽父级按容器换行、在收缩父级
        // 撑到内容宽——两种 CSS 语义都成立，无需 percent+min-width 的 hack（会破坏定宽换行）。
        let use_measure = dim_is_auto(ts.size.width)
            && (ns.is_block || matches!(ns.text_align, TextAlign::Center | TextAlign::Right));
        if use_measure {
            let min_unit_width = min_unit_width(&text_content, measure_font, font_size, letter_spacing, measure_bold);
            let tm = TextMeasure {
                text: text_content.clone(),
                font_px: font_size,
                letter_spacing_px: letter_spacing,
                line_height_px: actual_line_height,
                pad_l, pad_r, pad_t, pad_b,
                nowrap,
                bold: measure_bold,
                min_lines,
                max_line_width,
                min_unit_width,
                font_family: ns.font_family.clone(),
            };
            let tn = ctx.taffy.new_leaf_with_context(ts, tm).unwrap();
            return Some(RenderNode {
                tag: "text".into(),
                text: text_content,
                attrs,
                taffy_node: tn,
                style: ns,
                children: vec![],
                events,
            });
        }

        // 否则（inline 文本或显式宽度）：用内容宽 + 按内容宽估算换行行数设定显式盒高。
        if dim_is_auto(ts.size.width) {
            ts.size.width = length(content_w);
            // 靠内容定宽的行内文本**不参与收缩**。
            //
            // taffy 0.12 开始严格执行 flex 项的自动最小宽度（`min-width:auto` = min-content），
            // 于是「一行里几段文本加起来略微超过容器宽」时，收缩量会落到这些定宽文本上，
            // 把它们压窄、绘制期折行、还溢出容器（弹窗优惠券行里的「立即领取」就是这样）。
            //
            // 浏览器里这些文本本来就能压到 min-content（中文一个字），但我们把它们建成了
            // **定宽叶子**（min-content == max-content == 整行宽），所以「谁该被压」的分配
            // 从一开始就和浏览器不同。两条路可选：让文本真正可压（需要文字度量与 Chrome
            // 完全对齐，否则 1px 的度量误差就会让标题突然折行 —— 实测首页差异 4.1%→13.0%），
            // 或者让它们不参与收缩、宁可溢出。这里选后者：溢出只影响这一处，
            // 而误折行会让整页排版跳掉。
            ts.flex_shrink = 0.0;
        }
        let avail_w = if let Some(px) = dim_length(ts.size.width) {
            (px - pad_l - pad_r).max(1.0)
        } else {
            f32::MAX
        };
        let wrap_lines = if !nowrap && avail_w + WRAP_TOLERANCE_PX < max_line_width {
            (max_line_width / avail_w).ceil() as usize
        } else {
            1
        };
        let total_lines = min_lines.max(wrap_lines);
        let text_height = actual_line_height * total_lines as f32;
        ts.size.height = length(text_height + pad_t + pad_b);
        ts.min_size.height = length(text_height + pad_t + pad_b);
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "text".into(),
            text: text_content,
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    /// 把 `<text>` 的混排内容切成行内 run。
    ///
    /// 返回 `None` 表示「不需要按 run 处理」——没有元素子节点，或者子元素里有
    /// 非 `<text>` 的东西（`<image>` 之类）。后者退回原来的拍平逻辑：本引擎没有真正的
    /// 行内流，硬拆反而会把图文混排拆成上下两行，不如保持现状。
    fn build_inline_runs(
        node: &WxmlNode,
        ctx: &mut ComponentContext,
        outer: &NodeStyle,
    ) -> Option<Vec<RenderNode>> {
        use crate::parser::wxml::WxmlNodeType;
        let mut has_element = false;
        for c in &node.children {
            match c.node_type {
                WxmlNodeType::Element => {
                    if c.tag_name != "text" {
                        return None; // 图文混排等复杂情况不接
                    }
                    has_element = true;
                }
                _ => {}
            }
        }
        if !has_element {
            return None;
        }

        // 嵌套 `<text>` 的 CSS 解析要带上「自己作为父级」的祖先链，
        // 否则 `.label .req{…}` 这种后代选择器匹配不上。
        let classes = get_classes(node);
        let mut child_ancestors = ctx.ancestors.clone();
        child_ancestors.push(crate::parser::wxss::ElementDesc::new(
            &node.tag_name,
            node.get_attr("id"),
            &classes,
            &node.attributes,
        ));

        let sf = ctx.scale_factor;
        let mut runs: Vec<RenderNode> = Vec::new();
        let child_count = node.children.len();
        for (idx, child) in node.children.iter().enumerate() {
            // 每一段的样式：裸文字用最外层的，嵌套 <text> 用它自己解析出来的
            let (seg_text, seg_style) = match child.node_type {
                WxmlNodeType::Element => {
                    let mut sub_ctx = ComponentContext {
                        scale_factor: ctx.scale_factor,
                        screen_width: ctx.screen_width,
                        screen_height: ctx.screen_height,
                        stylesheet: ctx.stylesheet,
                        taffy: ctx.taffy,
                        ancestors: child_ancestors.clone(),
                        inherited: InheritedText {
                            font_size: outer.font_size,
                            color: outer.text_color,
                            weight: outer.font_weight,
                            align: outer.text_align,
                            line_height: outer.line_height,
                            letter_spacing: outer.letter_spacing,
                            font_family: outer.font_family.clone(),
                        },
                        sibling_index: idx,
                        sibling_count: child_count,
                        has_positioned_ancestor: ctx.has_positioned_ancestor,
                    };
                    let (_, cns) = build_base_style(child, &mut sub_ctx);
                    (get_text_content(child), cns)
                }
                _ => (child.text_content.clone(), outer.clone()),
            };
            if seg_text.trim().is_empty() && !seg_text.contains(' ') {
                continue;
            }
            let family = crate::text_family::renderer_for_family(seg_style.font_family.as_deref());
            let font_px = seg_style.font_size * sf;
            let ls = seg_style.letter_spacing * sf;
            let bold = matches!(
                seg_style.font_weight,
                FontWeight::Bold | FontWeight::W600 | FontWeight::W700 | FontWeight::W800 | FontWeight::W900
            ) && family
                .as_deref()
                .or(TEXT_MEASURE_FONT.as_deref())
                .map(|tr| tr.has_bold_face())
                .unwrap_or(false);
            // run 带上来源节点的 class：排查和回归用例都要靠它定位到具体这一段
            let seg_attrs: std::collections::HashMap<String, String> = match child.node_type {
                WxmlNodeType::Element => child
                    .get_attr("class")
                    .map(|c| {
                        let mut m = std::collections::HashMap::new();
                        m.insert("class".to_string(), c.to_string());
                        m
                    })
                    .unwrap_or_default(),
                _ => std::collections::HashMap::new(),
            };
            for unit in crate::renderer::components::rich_text::split_wrappable(&seg_text) {
                if unit.is_empty() {
                    continue;
                }
                let line_h = seg_style
                    .line_height
                    .map(|lh| lh * sf)
                    .unwrap_or_else(|| natural_line_height_px_for(&unit, font_px))
                    .max(font_px);
                let measure = family.as_deref().or(TEXT_MEASURE_FONT.as_deref());
                let w = measure
                    .map(|tr| tr.measure_text_weighted(&unit, font_px, ls, bold))
                    .unwrap_or_else(|| {
                        unit.chars()
                            .map(|c| if c.is_ascii() { font_px * 0.62 } else { font_px })
                            .sum()
                    })
                    .ceil();
                let cts = Style {
                    size: Size { width: length(w), height: length(line_h) },
                    flex_shrink: 0.0,
                    ..Default::default()
                };
                let ctn = ctx.taffy.new_leaf(cts).ok()?;
                runs.push(RenderNode {
                    tag: "text".into(),
                    text: unit,
                    attrs: seg_attrs.clone(),
                    taffy_node: ctn,
                    style: seg_style.clone(),
                    children: vec![],
                    events: vec![],
                });
            }
        }
        if runs.is_empty() {
            None
        } else {
            Some(runs)
        }
    }

    pub fn draw(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        sf: f32
    ) {
        let color = node.style.text_color.unwrap_or(Color::BLACK);
        let size = node.style.font_size * sf;
        // `font-family` 指定的字族优先（度量端用的是同一个，见 build / measure_text_node）
        let family = crate::text_family::renderer_for_family(node.style.font_family.as_deref());
        let text_renderer = family.as_deref().or(text_renderer);
        let natural_lh = text_renderer
            .map(|tr| tr.natural_line_height_for(&node.text, size))
            .unwrap_or(size * crate::text::NORMAL_LINE_HEIGHT_FACTOR);
        let line_height = node.style.line_height.map(|lh| lh * sf).unwrap_or(natural_lh);
        let letter_spacing = node.style.letter_spacing * sf;
        
        // 绘制文本自身的背景（<text> 也可有 background-color / 圆角），覆盖整个盒子
        draw_background(canvas, &node.style, x, y, w, h);
        
        // 内容区 = 盒子扣除自身 padding，文字在内容区内定位
        let pl = node.style.padding_left * sf;
        let pr = node.style.padding_right * sf;
        let pt = node.style.padding_top * sf;
        let pb = node.style.padding_bottom * sf;
        let cx = x + pl;
        let cy = y + pt;
        let cw = (w - pl - pr).max(0.0);
        let chh = (h - pt - pb).max(0.0);
        
        if let Some(tr) = text_renderer {
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
            
            // 处理 white-space: nowrap 和 text-overflow: ellipsis
            let should_wrap = !matches!(node.style.white_space, WhiteSpace::NoWrap | WhiteSpace::Pre);
            let use_ellipsis = matches!(node.style.text_overflow, TextOverflow::Ellipsis);
            
            // 粗体：优先用字体自带的粗体字面（与浏览器一致）；字体没有粗体字面时，
            // 才退回「轻微偏移二次描绘」的 faux-bold 近似。
            let is_bold = matches!(
                node.style.font_weight,
                FontWeight::Bold | FontWeight::W600 | FontWeight::W700 | FontWeight::W800 | FontWeight::W900
            );
            let real_bold = is_bold && tr.has_bold_face();
            let faux_bold_dx = if is_bold && !real_bold { 0.7 * sf } else { 0.0 };
            
            let text = &node.text;
            let text_w = tr.measure_text_weighted(text, size, letter_spacing, real_bold);
            // 是否需要多行：含换行符，或允许换行且超出内容宽度
            let multiline = text.contains('\n') || (should_wrap && cw > 0.0 && text_w > cw);
            
            if multiline {
                // 多行：顶部对齐换行绘制（在内容区内）
                draw_text_wrapped_advanced(
                    canvas, tr, text, cx, cy + size, size, cw, chh,
                    line_height, letter_spacing, real_bold, &node.style, &paint
                );
                if faux_bold_dx > 0.0 {
                    draw_text_wrapped_advanced(
                        canvas, tr, text, cx + faux_bold_dx, cy + size, size, cw, chh,
                        line_height, letter_spacing, real_bold, &node.style, &paint
                    );
                }
            } else {
                // 单行：在内容区内垂直居中（基线 ≈ 内容区中心 + 0.34*字号）
                let baseline = cy + chh / 2.0 + size * 0.34;
                // 水平对齐：text-align center/right 时在内容区内做水平偏移
                let align_dx = if cw > text_w {
                    match node.style.text_align {
                        TextAlign::Center => (cw - text_w) / 2.0,
                        TextAlign::Right => cw - text_w,
                        _ => 0.0,
                    }
                } else { 0.0 };
                let ax = cx + align_dx;
                if cw > 0.0 && use_ellipsis && text_w > cw {
                    draw_text_with_ellipsis(canvas, tr, text, cx, baseline, size, cw, letter_spacing, real_bold, &paint);
                    if faux_bold_dx > 0.0 {
                        draw_text_with_ellipsis(canvas, tr, text, cx + faux_bold_dx, baseline, size, cw, letter_spacing, real_bold, &paint);
                    }
                } else {
                    tr.draw_text_weighted(canvas, text, ax, baseline, size, letter_spacing, real_bold, &paint);
                    if faux_bold_dx > 0.0 {
                        tr.draw_text_weighted(canvas, text, ax + faux_bold_dx, baseline, size, letter_spacing, real_bold, &paint);
                    }
                }
            }
            
            // 绘制文本装饰
            if node.style.text_decoration != TextDecoration::None {
                draw_text_decoration(canvas, &node.style, x, y, w, size, line_height, &paint);
            }
        }
    }
}

/// 计算某一行按 text-align 对齐后的起始 x
fn aligned_line_x(tr: &TextRenderer, line: &str, x: f32, max_width: f32, size: f32, ls: f32, bold: bool, align: TextAlign) -> f32 {
    match align {
        TextAlign::Center | TextAlign::Right => {
            let lw = tr.measure_text_weighted(line, size, ls, bold);
            if max_width > lw {
                if align == TextAlign::Center { x + (max_width - lw) / 2.0 } else { x + (max_width - lw) }
            } else {
                x
            }
        }
        _ => x,
    }
}

/// 高级换行绘制（支持 line-height, letter-spacing, 换行符）
fn draw_text_wrapped_advanced(
    canvas: &mut Canvas,
    tr: &TextRenderer,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    max_width: f32,
    max_height: f32,
    line_height: f32,
    letter_spacing: f32,
    bold: bool,
    style: &NodeStyle,
    paint: &Paint,
) {
    if max_width <= 0.0 {
        tr.draw_text_weighted(canvas, text, x, y, size, letter_spacing, bold, paint);
        return;
    }
    
    // 确保 line_height 至少等于 font_size（默认已是字体自然行高）
    let actual_line_height = line_height.max(size);
    
    let mut current_y = y;
    let use_ellipsis = matches!(style.text_overflow, TextOverflow::Ellipsis);
    
    // 先按换行符分割
    let paragraphs: Vec<&str> = text.split('\n').collect();
    
    for (para_idx, paragraph) in paragraphs.iter().enumerate() {
        // 空段落也要换行
        if paragraph.is_empty() {
            current_y += actual_line_height;
            continue;
        }
        
        let chars: Vec<char> = paragraph.chars().collect();
        // 断行交给共享算法：西文整词不可断、行尾空白不占位（与度量、行数统计同一套规则）
        let measure = |s: &[char]| -> f32 {
            s.iter().map(|c| tr.measure_char_weighted(*c, size, bold) + letter_spacing).sum()
        };
        let lines = super::base::wrap_paragraph_lines(&chars, max_width, measure);

        for (idx, (ls_i, le_i)) in lines.iter().enumerate() {
            let has_more = idx + 1 < lines.len();
            // 仅当设置了 text-overflow:ellipsis 且下一行超出容器高度时，用省略号截断并停止；
            // 否则正常继续换行（宁可纵向溢出也不横向溢出/挤成一行）。
            if has_more && use_ellipsis && max_height > 0.0
                && current_y + actual_line_height > (y - size) + max_height
            {
                let rest: String = chars[*ls_i..].iter().collect();
                draw_text_with_ellipsis(canvas, tr, &rest, x, current_y, size, max_width, letter_spacing, bold, paint);
                return;
            }
            let line: String = chars[*ls_i..*le_i].iter().collect();
            let lx = aligned_line_x(tr, &line, x, max_width, size, letter_spacing, bold, style.text_align);
            tr.draw_text_weighted(canvas, &line, lx, current_y, size, letter_spacing, bold, paint);
            if has_more {
                current_y += actual_line_height;
            }
        }
        
        // 段落之间换行
        if para_idx < paragraphs.len() - 1 {
            current_y += actual_line_height;
        }
    }
}

/// 绘制带省略号的文本
fn draw_text_with_ellipsis(
    canvas: &mut Canvas,
    tr: &TextRenderer,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    max_width: f32,
    letter_spacing: f32,
    bold: bool,
    paint: &Paint,
) {
    let ellipsis = "...";
    let ellipsis_width = tr.measure_text_weighted(ellipsis, size, letter_spacing, bold);
    
    let text_width = tr.measure_text_weighted(text, size, letter_spacing, bold);
    if text_width <= max_width {
        tr.draw_text_weighted(canvas, text, x, y, size, letter_spacing, bold, paint);
        return;
    }
    
    // 找到合适的截断点
    let chars: Vec<char> = text.chars().collect();
    let mut current_width = 0.0;
    let mut truncate_at = chars.len();
    
    for (i, ch) in chars.iter().enumerate() {
        let char_width = tr.measure_char_weighted(*ch, size, bold) + letter_spacing;
        if current_width + char_width + ellipsis_width > max_width {
            truncate_at = i;
            break;
        }
        current_width += char_width;
    }
    
    let truncated: String = chars[..truncate_at].iter().collect();
    let display_text = format!("{}{}", truncated, ellipsis);
    tr.draw_text_weighted(canvas, &display_text, x, y, size, letter_spacing, bold, paint);
}

/// 绘制文本装饰（下划线、删除线等）
fn draw_text_decoration(
    canvas: &mut Canvas,
    style: &NodeStyle,
    x: f32,
    y: f32,
    w: f32,
    size: f32,
    _line_height: f32,
    paint: &Paint,
) {
    let line_y = match style.text_decoration {
        TextDecoration::Underline => y + size + 2.0,
        TextDecoration::LineThrough => y + size * 0.6,
        TextDecoration::Overline => y,
        TextDecoration::None => return,
    };
    
    // 绘制装饰线
    let mut line_paint = paint.clone();
    line_paint.style = PaintStyle::Stroke;
    
    use crate::path::Path;
    let mut path = Path::new();
    path.move_to(x, line_y);
    path.line_to(x + w, line_y);
    canvas.draw_path(&path, &line_paint);
}
