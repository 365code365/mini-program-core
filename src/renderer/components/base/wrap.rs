//! 断行单元判定与逐行切分
//!
//! 由 `base/mod.rs` 组合。**纯搬迁**：从 1925 行的 base.rs 按职责切开，一行代码没改。
use super::*;

/// 一个字符是否属于「不可拆分的西文词」。与 [`min_unit_width`] 同一套规则，
/// 保证 min-content 宽度和实际断行位置不会互相打架。

pub fn is_wrap_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'
}

/// 段落断行：把一段（不含 `\n` 的）文本切成若干行，返回每行的字符下标区间 `[start, end)`。
///
/// 与浏览器 `white-space: normal` + `overflow-wrap: normal` 对齐的三条规则：
/// 1. 连续西文词是不可拆分单元：放不下也不从词中间切断，让它独占一行横向溢出
///    （Chrome 就是这么处理 `HOT` 挤在 32px 盒子里的情形）；CJK 逐字可断。
/// 2. 行尾空白不参与「放不下」的判断，也不进入行区间 —— 否则右/居中对齐会被空格推偏。
/// 3. 换行处的空白被吃掉，不会跑到下一行行首。
///
/// `measure` 由调用方提供（同一套字形度量），确保度量、行数统计、绘制三处结果一致。
pub fn wrap_paragraph_lines(
    chars: &[char],
    max_width: f32,
    mut measure: impl FnMut(&[char]) -> f32,
) -> Vec<(usize, usize)> {
    if chars.is_empty() {
        return vec![(0, 0)];
    }
    // 去掉区间末尾的空白（规则 2）
    let trim_end = |end: usize| -> usize {
        let mut e = end;
        while e > 0 && chars[e - 1].is_whitespace() {
            e -= 1;
        }
        e
    };

    let mut lines: Vec<(usize, usize)> = Vec::new();
    let mut line_start = 0usize;
    let mut line_w = 0.0f32; // 本行已确定的宽度，不含挂起的行尾空白
    let mut pending_ws = 0.0f32;
    let mut i = 0usize;

    while i < chars.len() {
        let unit_start = i;
        if chars[i].is_whitespace() {
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            pending_ws += measure(&chars[unit_start..i]);
            continue;
        }
        if is_wrap_word_char(chars[i]) {
            while i < chars.len() && is_wrap_word_char(chars[i]) {
                i += 1;
            }
        } else {
            i += 1;
        }
        let unit_w = measure(&chars[unit_start..i]);
        if line_w > 0.0 && line_w + pending_ws + unit_w > max_width + WRAP_TOLERANCE_PX {
            lines.push((line_start, trim_end(unit_start)));
            line_start = unit_start;
            line_w = unit_w;
        } else {
            line_w += pending_ws + unit_w;
        }
        pending_ws = 0.0;
    }
    lines.push((line_start, trim_end(chars.len())));
    lines
}

/// 统计文本在给定宽度下的换行行数（含显式换行符），供度量使用。
pub fn count_wrapped_text_lines(tr: &crate::text::TextRenderer, text: &str, max_width: f32, size: f32, ls: f32, bold: bool) -> usize {
    if max_width <= 0.0 { return text.split('\n').count().max(1); }
    let mut total = 0usize;
    for para in text.split('\n') {
        if para.is_empty() { total += 1; continue; }
        let chars: Vec<char> = para.chars().collect();
        let measure = |s: &[char]| -> f32 {
            s.iter().map(|c| tr.measure_char_weighted(*c, size, bold) + ls).sum()
        };
        total += wrap_paragraph_lines(&chars, max_width, measure).len();
    }
    total.max(1)
}
