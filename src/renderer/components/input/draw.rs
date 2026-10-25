//! input 组件的绘制：背景、边框、文字、选区与光标（单行）。
//!
//! 从 594 行的 input.rs 按职责切开（构建留在 `mod.rs`）。盒子部分抽成 [`InputComponent::draw_box`]
//! 给 textarea 共用；textarea 的多行排版在 `textarea.rs`。
use super::*;

impl InputComponent {
    
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
        Self::draw_with_cursor(node, canvas, text_renderer, x, y, w, h, sf, false, 0);
    }
    
    pub fn draw_with_cursor(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        sf: f32,
        focused: bool,
        cursor_pos: usize,
    ) {
        Self::draw_with_selection(node, canvas, text_renderer, x, y, w, h, sf, focused, cursor_pos, None);
    }

    /// 同 [`draw_with_selection`](Self::draw_with_selection)，另返回这一帧单行文字的排版（文字起点、每个字的右边界）：
    /// 点击定位光标按看到的位置算（text-align 靠右 / 居中、真实字号与内边距、横向滚动都在里面）
    #[allow(clippy::too_many_arguments)]
    pub fn draw_single_line(
        node: &RenderNode,
        canvas: &mut Canvas,
        text_renderer: Option<&TextRenderer>,
        (x, y, w, h): (f32, f32, f32, f32),
        sf: f32,
        focused: bool,
        cursor_pos: usize,
        selection: Option<(usize, usize)>,
    ) -> Option<crate::ui::interaction::text_lines::TextLayout> {
        Self::draw_with_selection(node, canvas, text_renderer, x, y, w, h, sf, focused, cursor_pos, selection);
        let tr = text_renderer?;
        let style = &node.style;
        let fs = style.font_size * sf;
        let (pl, pr) = (style.padding_left * sf, style.padding_right * sf);
        let avail = w - pl - pr;
        let mut xs = vec![0.0f32];
        let mut acc = String::new();
        // 画的是占位符时，点击定位按空值算（光标只能在开头）
        let value: &str = if node.attrs.contains_key(super::PLACEHOLDER_SHOWN) { "" } else { &node.text };
        for c in value.chars() {
            acc.push(c);
            xs.push(tr.measure_text(&acc, fs));
        }
        let tw = *xs.last().unwrap_or(&0.0);
        let x0 = Self::line_dx(style.text_align, tw, avail, focused, &xs, cursor_pos, fs);
        let line = crate::ui::interaction::text_lines::TextLine { start: 0, end: xs.len() - 1, x0, xs };
        Some(crate::ui::interaction::text_lines::TextLayout {
            text: value.to_string(),
            lines: vec![line],
            line_height: h.max(1.0),
            origin: (pl, 0.0),
            scroll_y: 0.0,
        })
    }

    /// 单行文字的水平起点（相对内容区左边）：放得下按 text-align，放不下按光标横向滚动（与绘制同一算式）
    fn line_dx(align: TextAlign, tw: f32, avail: f32, focused: bool, xs: &[f32], cursor: usize, fs: f32) -> f32 {
        if tw <= avail {
            return match align {
                TextAlign::Center => (avail - tw) / 2.0,
                TextAlign::Right => avail - tw,
                _ => 0.0,
            };
        }
        let cx = xs.get(cursor).copied().unwrap_or(tw);
        if focused && cx > avail { avail - cx - fs } else { 0.0 }
    }
    
    pub fn draw_with_selection(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        sf: f32,
        focused: bool,
        cursor_pos: usize,
        selection: Option<(usize, usize)>,
    ) {
        // textarea 是多行排版（折行、`\n`、按行放光标），与单行的横向滚动完全两套几何
        if node.tag == "textarea" {
            Self::draw_textarea(node, canvas, text_renderer, x, y, w, h, sf, focused, cursor_pos, selection, false);
            return;
        }
        Self::draw_box(node, canvas, x, y, w, h, focused);
        let style = &node.style;
        
        // 计算文本位置
        let font_size = style.font_size * sf;
        // 用元素**实际**的内边距，不是写死的 12px。
        //
        // 布局（taffy）用的是 `style.padding`（页面没写时 build 期给 8/12 的默认值），
        // 而这里以前写死 12 —— 页面自己设了 `padding-left` 时两边就对不上：
        // 盒子按 CSS 排版，文字/占位符/光标却固定缩进 12px，看起来「输入提示离左边太远（或太近）」。
        // HTML 端的 `.wx-input` 是 `padding:0` + 外层盒子的 padding，只有按实际值取才对得齐。
        let padding_left = style.padding_left * sf;
        let padding_right = style.padding_right * sf;
        let text_x = x + padding_left;
        
        // 根据 text-align 和 vertical-align 计算位置
        let text_y = match style.vertical_align {
            VerticalAlign::Top => y + font_size + 4.0 * sf,
            VerticalAlign::Bottom => y + h - 4.0 * sf,
            _ => y + (h + font_size) / 2.0 - 2.0 * sf, // Middle/Baseline - 垂直居中
        };
        
        if let Some(tr) = text_renderer {
            // 设置裁剪区域，防止文本溢出输入框
            canvas.save();
            let clip_x = x + padding_left;
            let clip_y = y;
            let clip_w = w - padding_left - padding_right;
            let clip_h = h;
            canvas.clip_rect(GeoRect::new(clip_x, clip_y, clip_w, clip_h));
            
            // 画的是占位符：光标、选区、横向滚动都按空值算，占位符自己按 text-align 摆
            let ph = node.attrs.contains_key(super::PLACEHOLDER_SHOWN);
            let value: &str = if ph { "" } else { &node.text };
            // 先计算文本偏移（用于所有绘制）
            let text_width = if !value.is_empty() {
                tr.measure_text(value, font_size)
            } else {
                0.0
            };
            let available_width = w - padding_left - padding_right;
            
            let mut text_offset = 0.0;
            if focused && text_width > available_width {
                let cursor_text: String = value.chars().take(cursor_pos).collect();
                let cursor_x_in_text = tr.measure_text(&cursor_text, font_size);
                
                if cursor_x_in_text > available_width {
                    text_offset = available_width - cursor_x_in_text - font_size;
                }
            }
            
            // 文字整体的水平起点：text-align 居中 / 靠右（放得下时）、超长时按光标滚动。
            // 文字、选区、光标三者共用这一个起点 —— 从前只有文字按 text-align 摆，光标与选区永远从左边算：
            // 靠右的金额框（房贷计算「1000 万元」）光标停在最左边，离数字老远
            let align_dx = match style.text_align {
                TextAlign::Center if text_width <= available_width => (available_width - text_width) / 2.0,
                TextAlign::Right if text_width <= available_width => available_width - text_width,
                _ => text_offset,
            };
            let text_offset = align_dx;
            // 绘制选中背景（需要考虑 text_offset）
            if let Some((sel_start, sel_end)) = selection {
                if sel_start != sel_end && focused {
                    let start_text: String = value.chars().take(sel_start).collect();
                    let sel_text: String = value.chars().skip(sel_start).take(sel_end - sel_start).collect();
                    
                    let sel_x = text_x + tr.measure_text(&start_text, font_size) + text_offset;
                    let sel_w = tr.measure_text(&sel_text, font_size);
                    let sel_y = y + (h - font_size) / 2.0 - 2.0 * sf;
                    let sel_h = font_size + 4.0 * sf;
                    
                    let sel_paint = Paint::new()
                        .with_color(Color::new(7, 193, 96, 80)) // 半透明绿色
                        .with_style(PaintStyle::Fill);
                    canvas.draw_rect(&GeoRect::new(sel_x, sel_y, sel_w, sel_h), &sel_paint);
                }
            }
            
            // 绘制文本
            let color = style.text_color.unwrap_or(Color::BLACK);
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
            
            if !node.text.is_empty() {
                // 根据 text-align 调整 x 位置（占位符按它自己的宽对齐）
                let final_x = if ph {
                    let pw = tr.measure_text(&node.text, font_size);
                    text_x + match style.text_align {
                        TextAlign::Center if pw <= available_width => (available_width - pw) / 2.0,
                        TextAlign::Right if pw <= available_width => available_width - pw,
                        _ => 0.0,
                    }
                } else {
                    text_x + align_dx
                };
                
                tr.draw_text(canvas, &node.text, final_x, text_y, font_size, &paint);
            }
            
            // ───────────────── 光标 ─────────────────
            //
            // 只在没有选中（或选中范围为空）时显示，按 CURSOR_BLINK_INTERVAL_MS 闪烁。
            //
            // 几何按微信/iOS 对齐，以前是「1 设备像素的描边线段，高度正好等于字号」：
            // - 1 设备像素在 2x 屏上只有半个逻辑像素，细到几乎看不见（也就更看不出闪烁）；
            //   微信的光标是 **2 逻辑像素**宽，两端带圆角；
            // - 高度取字号会显得比文字矮一截，微信是略高于字面（约 1.15em）并垂直居中。
            if focused && selection.map(|(s, e)| s == e).unwrap_or(true) {
                let cursor_text: String = value.chars().take(cursor_pos).collect();
                let cursor_x = text_x + tr.measure_text(&cursor_text, font_size) + text_offset;
                let caret_w = (2.0 * sf).max(2.0);
                let caret_h = font_size * 1.15;
                let caret_x = cursor_x - caret_w / 2.0;
                let caret_y = y + (h - caret_h) / 2.0;
                // 位置每帧都记（不管这一拍显不显示）：输入法候选框要靠它定位
                set_last_caret_rect(Some((caret_x, caret_y, caret_w, caret_h)));
                if should_show_cursor() {
                    let paint = Paint::new()
                        .with_color(Color::from_hex(0x07C160))
                        .with_style(PaintStyle::Fill)
                        .with_anti_alias(true);
                    let mut path = Path::new();
                    path.add_round_rect(caret_x, caret_y, caret_w, caret_h, caret_w / 2.0);
                    canvas.draw_path(&path, &paint);
                }
            }
            
            // 恢复裁剪区域
            canvas.restore();
        }
    }

    /// 输入框自己的盒子：阴影、背景、边框（聚焦时高亮）、逐边边框。input 与 textarea 共用
    pub(super) fn draw_box(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, focused: bool) {
        let style = &node.style;
        
        // 获取圆角值（支持四角独立设置）
        let radius_tl = style.border_radius_tl.unwrap_or(style.border_radius);
        let radius_tr = style.border_radius_tr.unwrap_or(style.border_radius);
        let radius_br = style.border_radius_br.unwrap_or(style.border_radius);
        let radius_bl = style.border_radius_bl.unwrap_or(style.border_radius);
        let has_radius = radius_tl > 0.0 || radius_tr > 0.0 || radius_br > 0.0 || radius_bl > 0.0;
        let uniform_radius = radius_tl == radius_tr && radius_tr == radius_br && radius_br == radius_bl;
        
        // 绘制盒子阴影
        if let Some(shadow) = &style.box_shadow {
            draw_box_shadow(canvas, shadow, x, y, w, h, style.border_radius);
        }
        
        // 绘制背景
        if let Some(bg) = style.background_color {
            // 应用透明度
            let bg = if style.opacity < 1.0 {
                Color::new(bg.r, bg.g, bg.b, (bg.a as f32 * style.opacity) as u8)
            } else {
                bg
            };
            
            let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
            if has_radius {
                let mut path = Path::new();
                if uniform_radius {
                    path.add_round_rect(x, y, w, h, radius_tl);
                } else {
                    path.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
                }
                canvas.draw_path(&path, &paint);
            } else {
                canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
            }
        }
        
        // 绘制边框 - 聚焦时高亮（除非有自定义边框颜色）
        let border_color = if focused {
            Color::from_hex(0x07C160) // 微信绿色
        } else {
            style.border_color.unwrap_or(Color::from_hex(0xD9D9D9))
        };
        
        if style.border_width > 0.0 {
            stroke_round_rect_ring(
                canvas, x, y, w, h,
                [radius_tl, radius_tr, radius_br, radius_bl],
                style.border_width, border_color,
            );
        }
        // 逐边边框（`border-bottom` 那条下划线是表单最常见的写法）。
        // input 自己画盒子，不走通用的背景/边框绘制，所以这里要显式补一次 ——
        // 漏了它的话「下划线式输入框」在引擎里完全看不见（登录页的手机号/验证码就是）。
        draw_side_borders(canvas, style, x, y, w, h);
    }
}
