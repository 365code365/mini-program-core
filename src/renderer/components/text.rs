//! text 组件 - 文本显示

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle};
use taffy::prelude::*;

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
        let line_height = ns.line_height.map(|lh| lh * sf).unwrap_or(font_size * 1.5);
        // 确保 line_height 至少等于 font_size
        let actual_line_height = line_height.max(font_size * 1.2);
        
        // 计算文本行数（考虑换行符）
        let newline_count = text_content.matches('\n').count();
        let min_lines = (newline_count + 1).max(1);
        
        // 估算文本宽度（单行最大宽度）
        // 注意：ASCII 字符系数取略大于平均值，并额外加余量，避免估算宽度
        // 略小于实际测量宽度而触发非预期换行（例如 "Canvas" 恰好溢出 0.1px）
        let mut max_line_width: f32 = 0.0;
        for line in text_content.split('\n') {
            let line_width: f32 = line.chars().map(|c| {
                if c.is_ascii() {
                    font_size * 0.62
                } else {
                    font_size
                }
            }).sum();
            max_line_width = max_line_width.max(line_width);
        }
        // 额外余量，吸收估算与实际测量之间的误差
        max_line_width += 4.0 * sf;
        
        // 设置 flex-shrink 允许收缩
        ts.flex_shrink = 1.0;
        
        // 设置高度为文本行数 * 行高（而不是 min_size，确保 taffy 正确计算父容器高度）
        // 同时设置 min_size 作为保底
        let text_height = actual_line_height * min_lines as f32;
        ts.size.height = length(text_height);
        ts.min_size.height = length(text_height);
        
        // 如果 CSS 没有设置宽度，根据 display 属性决定宽度
        // display: block 时使用 100%，否则使用估算的文本宽度
        if matches!(ts.size.width, Dimension::Auto) {
            if ns.is_block {
                ts.size.width = Dimension::Percent(1.0);
            } else {
                // 使用估算的文本宽度
                ts.size.width = length(max_line_width);
            }
        }
        
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
        let line_height = node.style.line_height.map(|lh| lh * sf).unwrap_or(size * 1.5);
        let letter_spacing = node.style.letter_spacing * sf;
        
        // 绘制文本自身的背景（<text> 也可有 background-color / 圆角）
        draw_background(canvas, &node.style, x, y, w, h);
        
        if let Some(tr) = text_renderer {
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
            
            // 处理 white-space: nowrap 和 text-overflow: ellipsis
            let should_wrap = !matches!(node.style.white_space, WhiteSpace::NoWrap | WhiteSpace::Pre);
            let use_ellipsis = matches!(node.style.text_overflow, TextOverflow::Ellipsis);
            
            let text = &node.text;
            let text_w = tr.measure_text_with_spacing(text, size, letter_spacing);
            // 是否需要多行：含换行符，或允许换行且超出宽度
            let multiline = text.contains('\n') || (should_wrap && w > 0.0 && text_w > w);
            
            // 粗体：字体无独立 bold 字面时用轻微偏移二次描绘模拟（faux-bold）
            let is_bold = matches!(
                node.style.font_weight,
                FontWeight::Bold | FontWeight::W600 | FontWeight::W700 | FontWeight::W800 | FontWeight::W900
            );
            let bold_dx = if is_bold { 0.7 * sf } else { 0.0 };
            
            if multiline {
                // 多行：顶部对齐换行绘制
                draw_text_wrapped_advanced(
                    canvas, tr, text, x, y + size, size, w, h,
                    line_height, letter_spacing, &node.style, &paint
                );
                if is_bold {
                    draw_text_wrapped_advanced(
                        canvas, tr, text, x + bold_dx, y + size, size, w, h,
                        line_height, letter_spacing, &node.style, &paint
                    );
                }
            } else {
                // 单行：在盒内垂直居中（基线 ≈ 盒中心 + 0.34*字号）
                let baseline = y + h / 2.0 + size * 0.34;
                if w > 0.0 && use_ellipsis && text_w > w {
                    draw_text_with_ellipsis(canvas, tr, text, x, baseline, size, w, letter_spacing, &paint);
                    if is_bold {
                        draw_text_with_ellipsis(canvas, tr, text, x + bold_dx, baseline, size, w, letter_spacing, &paint);
                    }
                } else {
                    tr.draw_text_with_spacing(canvas, text, x, baseline, size, letter_spacing, &paint);
                    if is_bold {
                        tr.draw_text_with_spacing(canvas, text, x + bold_dx, baseline, size, letter_spacing, &paint);
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
    style: &NodeStyle,
    paint: &Paint,
) {
    if max_width <= 0.0 {
        tr.draw_text_with_spacing(canvas, text, x, y, size, letter_spacing, paint);
        return;
    }
    
    // 确保 line_height 至少等于 font_size
    let actual_line_height = line_height.max(size * 1.2);
    
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
        let mut line_start = 0;
        let mut current_width = 0.0;
        
        for (i, ch) in chars.iter().enumerate() {
            let char_width = tr.measure_char(*ch, size) + letter_spacing;
            
            // 检查是否需要换行
            if current_width + char_width > max_width && i > line_start {
                let line: String = chars[line_start..i].iter().collect();
                
                // 若下一行会超出容器高度：这是最后一行，把「剩余全部内容」画在当前行，
                // 宁可横向溢出也不丢字（修复 "92%" 被裁成 "92" 这类尾字符丢失）。
                // 容器底部（基线坐标系）约为 (y - size) + max_height
                let next_line_top = current_y + actual_line_height;
                if max_height > 0.0 && next_line_top > (y - size) + max_height {
                    let rest: String = chars[line_start..].iter().collect();
                    if use_ellipsis {
                        draw_text_with_ellipsis(canvas, tr, &rest, x, current_y, size, max_width, letter_spacing, paint);
                    } else {
                        tr.draw_text_with_spacing(canvas, &rest, x, current_y, size, letter_spacing, paint);
                    }
                    return;
                }
                
                // 绘制当前行
                tr.draw_text_with_spacing(canvas, &line, x, current_y, size, letter_spacing, paint);
                
                current_y += actual_line_height;
                line_start = i;
                current_width = char_width;
            } else {
                current_width += char_width;
            }
        }
        
        // 绘制段落的最后一行
        if line_start < chars.len() {
            let line: String = chars[line_start..].iter().collect();
            tr.draw_text_with_spacing(canvas, &line, x, current_y, size, letter_spacing, paint);
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
    paint: &Paint,
) {
    let ellipsis = "...";
    let ellipsis_width = tr.measure_text_with_spacing(ellipsis, size, letter_spacing);
    
    let text_width = tr.measure_text_with_spacing(text, size, letter_spacing);
    if text_width <= max_width {
        tr.draw_text_with_spacing(canvas, text, x, y, size, letter_spacing, paint);
        return;
    }
    
    // 找到合适的截断点
    let chars: Vec<char> = text.chars().collect();
    let mut current_width = 0.0;
    let mut truncate_at = chars.len();
    
    for (i, ch) in chars.iter().enumerate() {
        let char_width = tr.measure_char(*ch, size) + letter_spacing;
        if current_width + char_width + ellipsis_width > max_width {
            truncate_at = i;
            break;
        }
        current_width += char_width;
    }
    
    let truncated: String = chars[..truncate_at].iter().collect();
    let display_text = format!("{}{}", truncated, ellipsis);
    tr.draw_text_with_spacing(canvas, &display_text, x, y, size, letter_spacing, paint);
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
