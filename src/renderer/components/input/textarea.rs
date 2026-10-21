//! textarea 的多行绘制：折行、按行放光标与选区、光标所在行超出可视区时纵向滚动。
//!
//! 以前 textarea 与 input 共用单行绘制：整串值一次 `draw_text`、文字垂直居中。
//! 值里一旦有 `\n`，字体里没有它的字形，画出来就是豆腐块（「212121□□assa□」），
//! 文字也不会换行 —— 打满一行之后只会横向滚走。
//!
//! 折行规则对齐浏览器 textarea 的 UA 样式 `white-space: pre-wrap; overflow-wrap: break-word`：
//! - `\n` 是硬换行；连续西文词不从中间断，放不下就整词挪到下一行；
//! - 一个词独占一行也放不下时才逐字切开（break-word —— 与 `<text>` 的 normal 不同）；
//! - 空白保留，挂在行尾，不会因为行尾空格把下一个词挤下去以外的方式触发换行。
//!
//! 排版结果（[`TextLayout`]）返回给调用方写回交互状态：↑↓ / Home / End 按看到的行走，
//! 点击按看到的行定位光标，都靠它。

use super::*;
use crate::ui::interaction::text_lines::{self, TextLayout, TextLine};

/// 一个字符的前进宽度。控制字符（`\t`、`\r`……）按空格量、按空格画：
/// 编辑端已经不让它们进值了，但 `setData` 给的初始值里可能带着
fn advance(tr: &TextRenderer, c: char, size: f32) -> f32 {
    tr.measure_char(if c.is_control() { ' ' } else { c }, size)
}

/// 按 `pre-wrap` + `break-word` 把整串值切成视觉行（下标约定见 `ui::interaction::text_lines`）
pub fn layout_textarea_lines(chars: &[char], max_w: f32, mut measure: impl FnMut(char) -> f32, align: TextAlign) -> Vec<TextLine> {
    let mut lines = Vec::new();
    let limit = max_w + WRAP_TOLERANCE_PX;
    let mut para_start = 0;
    loop {
        let para_end = chars[para_start..].iter().position(|&c| c == '\n').map(|i| para_start + i).unwrap_or(chars.len());
        let mut line_start = para_start;
        let mut xs = vec![0.0f32];
        let mut w = 0.0f32;
        let mut i = para_start;
        while i < para_end {
            let c = chars[i];
            if c.is_whitespace() {
                // 空白挂在行尾：不参与「放不放得下」
                w += measure(c);
                xs.push(w);
                i += 1;
                continue;
            }
            let j = if is_wrap_word_char(c) {
                let mut j = i;
                while j < para_end && is_wrap_word_char(chars[j]) {
                    j += 1;
                }
                j
            } else {
                i + 1
            };
            let widths: Vec<f32> = chars[i..j].iter().map(|&c| measure(c)).collect();
            let unit_w: f32 = widths.iter().sum();
            if i > line_start && w + unit_w > limit {
                lines.push(finish_line(line_start, i, std::mem::replace(&mut xs, vec![0.0]), chars, max_w, align));
                line_start = i;
                w = 0.0;
            }
            for (k, cw) in widths.into_iter().enumerate() {
                // break-word：整词独占一行也放不下，才逐字切
                if unit_w > limit && i + k > line_start && w + cw > limit {
                    lines.push(finish_line(line_start, i + k, std::mem::replace(&mut xs, vec![0.0]), chars, max_w, align));
                    line_start = i + k;
                    w = 0.0;
                }
                w += cw;
                xs.push(w);
            }
            i = j;
        }
        lines.push(finish_line(line_start, para_end, xs, chars, max_w, align));
        if para_end >= chars.len() {
            break;
        }
        para_start = para_end + 1;
    }
    lines
}

/// 收尾一行：按 `text-align` 算行首偏移（行尾挂着的空白不算进对齐宽度，浏览器同样如此）
fn finish_line(start: usize, end: usize, xs: Vec<f32>, chars: &[char], max_w: f32, align: TextAlign) -> TextLine {
    let mut visible = end;
    while visible > start && chars[visible - 1].is_whitespace() {
        visible -= 1;
    }
    let width = xs[visible - start];
    let x0 = match align {
        TextAlign::Center => ((max_w - width) / 2.0).max(0.0),
        TextAlign::Right => (max_w - width).max(0.0),
        _ => 0.0,
    };
    TextLine { start, end, x0, xs }
}

impl InputComponent {
    /// 画一个 textarea。`upstream`：光标在软换行处时画在上一行末（见 `FocusedInput::caret_upstream`）。
    ///
    /// 返回本次的排版（没有字体时为 `None`），调用方写回 `InteractionManager::text_layouts`。
    #[allow(clippy::too_many_arguments)]
    pub fn draw_textarea(
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
        upstream: bool,
    ) -> Option<TextLayout> {
        Self::draw_box(node, canvas, x, y, w, h, focused);
        let tr = text_renderer?;
        let style = &node.style;
        let fs = style.font_size * sf;
        let (pl, pr) = (style.padding_left * sf, style.padding_right * sf);
        let (pt, pb) = (style.padding_top * sf, style.padding_bottom * sf);
        let content_w = (w - pl - pr).max(0.0);
        let content_h = (h - pt - pb).max(0.0);
        // 行高：CSS 写了就用；没写按含 CJK 的 normal —— 不随内容在中西文之间切换，
        // 否则打下第一个汉字的瞬间所有行一起跳高
        let line_h = style.line_height.map(|lh| lh * sf).unwrap_or_else(|| tr.natural_line_height(fs)).max(1.0);

        let chars: Vec<char> = node.text.chars().collect();
        let lines = layout_textarea_lines(&chars, content_w, |c| advance(tr, c, fs), style.text_align);
        let cursor_pos = cursor_pos.min(chars.len());
        let caret_li = text_lines::caret_line(&lines, cursor_pos, upstream);
        // 光标所在行露出来：只在它越过底边时往上推（无状态，每帧按光标算）
        let scroll_y = if focused { ((caret_li + 1) as f32 * line_h - content_h).max(0.0) } else { 0.0 };
        let (cx, cy) = (x + pl, y + pt);
        // 第 i 行基线：与 `<text>` 多行一致，首行基线 = 内容区顶 + 字号
        let baseline = |i: usize| cy + fs + i as f32 * line_h - scroll_y;
        let caret_w = (2.0 * sf).max(2.0);

        canvas.save();
        // 裁到盒子：滚上去的行可以露在上内边距里（浏览器的滚动容器是 padding box）。
        // 横向多留一个光标宽，行首的光标不会被切掉一半
        canvas.clip_rect(GeoRect::new(x - caret_w, y, w + 2.0 * caret_w, h));

        // ── 选区：逐行一块 ──
        if let (true, Some((s, e))) = (focused, selection.filter(|(s, e)| s != e)) {
            let paint = Paint::new().with_color(Color::new(7, 193, 96, 80)).with_style(PaintStyle::Fill);
            for (i, l) in lines.iter().enumerate() {
                let (from, to) = (s.max(l.start), e.min(l.end));
                // 选区跨过这一行的换行符：行尾补一小段，看得出换行也被选中了
                let newline = e > l.end && s <= l.end && !text_lines::is_soft_end(&lines, i) && i + 1 < lines.len();
                if from > to || (from == to && !newline) {
                    continue;
                }
                let x1 = cx + l.x0 + l.xs[from - l.start];
                let x2 = cx + l.x0 + l.xs[to - l.start] + if newline { fs * 0.3 } else { 0.0 };
                let top = baseline(i) - fs;
                canvas.draw_rect(&GeoRect::new(x1, top, x2 - x1, fs + 4.0 * sf), &paint);
            }
        }

        // ── 文字：只画露在盒子里的行 ──
        let color = style.text_color.unwrap_or(Color::BLACK);
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
        for (i, l) in lines.iter().enumerate() {
            let b = baseline(i);
            if b - fs > y + h || b + line_h < y || l.start == l.end {
                continue;
            }
            let text: String = chars[l.start..l.end].iter().map(|&c| if c.is_control() { ' ' } else { c }).collect();
            tr.draw_text(canvas, &text, cx + l.x0, b, fs, &paint);
        }

        // ── 光标：几何与单行输入框相同（2 逻辑像素宽、1.15em 高），竖直位置跟着所在行 ──
        if focused && selection.map(|(s, e)| s == e).unwrap_or(true) {
            let caret_h = fs * 1.15;
            let caret_x = cx + text_lines::caret_x(&lines, cursor_pos, upstream) - caret_w / 2.0;
            // 单行输入框里光标中心 = 基线 - 字号/2 + 2sf，这里沿用，两种输入框里的光标与字的相对位置一样
            let caret_y = baseline(caret_li) - (fs + caret_h) / 2.0 + 2.0 * sf;
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
        canvas.restore();

        Some(TextLayout { text: node.text.clone(), lines, line_height: line_h, origin: (pl, pt), scroll_y })
    }
}
