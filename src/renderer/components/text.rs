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
        let use_measure = matches!(ts.size.width, Dimension::Auto)
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
        if matches!(ts.size.width, Dimension::Auto) {
            ts.size.width = length(content_w);
        }
        let avail_w = if let Dimension::Length(px) = ts.size.width {
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
