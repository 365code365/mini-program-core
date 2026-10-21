//! 多行输入框的「视觉行」：折行结果的数据形状，以及在它上面走光标。
//!
//! 折行本身（要量字宽）在绘制端做（`renderer/components/input/textarea.rs`），
//! 结果写回 [`InteractionManager::text_layouts`](super::InteractionManager)；
//! 这里只放与字体无关的部分：按行移动、点击定位、按词移动。
//!
//! ## 下标约定
//! 一行是字符下标区间 `[start, end)`，**不含**结束这一行的 `\n`：
//! - 硬换行：下一行从 `end + 1` 开始（跳过 `\n`），光标可以停在 `end`（`\n` 之前）；
//! - 软换行（折行）：下一行从 `end` 开始，`end` 这个下标同时是「上一行末」与「下一行首」，
//!   靠 `FocusedInput::caret_upstream` 区分，默认算下一行首（浏览器同样如此）。

/// 一条视觉行
#[derive(Clone, Debug, PartialEq)]
pub struct TextLine {
    pub start: usize,
    pub end: usize,
    /// 行首相对内容区左边的偏移（`text-align` 居中 / 靠右时非 0），设备像素
    pub x0: f32,
    /// 每个字符边界相对行首的 x：`xs[i]` 是第 `start + i` 个字符**之前**的位置，
    /// 长度 = `end - start + 1`
    pub xs: Vec<f32>,
}

impl TextLine {
    fn len(&self) -> usize {
        self.end - self.start
    }

    /// 本行第 `col` 个边界的 x（相对内容区）
    fn x_at(&self, col: usize) -> f32 {
        self.x0 + self.xs.get(col.min(self.len())).copied().unwrap_or(0.0)
    }

    /// 离 `x` 最近的边界
    fn nearest_col(&self, x: f32) -> usize {
        let mut best = 0;
        let mut best_d = f32::MAX;
        for col in 0..=self.len() {
            let d = (self.x_at(col) - x).abs();
            if d < best_d {
                best_d = d;
                best = col;
            }
        }
        best
    }
}

/// 一个多行输入框最近一次绘制的排版结果
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextLayout {
    /// 排版用的文本。与输入框当前的值不同就说明它过期了，或者画的是占位符 —— 不能拿来定位
    pub text: String,
    pub lines: Vec<TextLine>,
    /// 行高（设备像素）
    pub line_height: f32,
    /// 内容区左上角相对盒子左上角（设备像素，就是 padding-left / padding-top）
    pub origin: (f32, f32),
    /// 纵向滚动量（设备像素，≥ 0）：光标所在行超出可视区时整体上移
    pub scroll_y: f32,
}

impl TextLayout {
    /// 点击定位：`(x, y)` 是相对盒子左上角的设备像素。返回 (下标, 是否画在上一行末)
    pub fn hit(&self, x: f32, y: f32) -> (usize, bool) {
        if self.lines.is_empty() || self.line_height <= 0.0 {
            return (0, false);
        }
        let row = ((y - self.origin.1 + self.scroll_y) / self.line_height).floor();
        let li = (row.max(0.0) as usize).min(self.lines.len() - 1);
        let line = &self.lines[li];
        let col = line.nearest_col(x - self.origin.0);
        let upstream = col == line.len() && is_soft_end(&self.lines, li);
        (line.start + col, upstream)
    }
}

/// 没有绘制结果时（还没画过、单测里）的退路：只按 `\n` 分行，每个字符算 1 个单位宽
pub fn logical_lines(chars: &[char]) -> Vec<TextLine> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (i, &c) in chars.iter().enumerate() {
        if c == '\n' {
            lines.push(unit_line(start, i));
            start = i + 1;
        }
    }
    lines.push(unit_line(start, chars.len()));
    lines
}

fn unit_line(start: usize, end: usize) -> TextLine {
    TextLine { start, end, x0: 0.0, xs: (0..=end - start).map(|i| i as f32).collect() }
}

/// 第 `li` 行是不是软换行结束的（下一行紧接着从 `end` 开始）
pub fn is_soft_end(lines: &[TextLine], li: usize) -> bool {
    lines.get(li + 1).is_some_and(|n| n.start == lines[li].end)
}

/// 光标所在的行
pub fn caret_line(lines: &[TextLine], pos: usize, upstream: bool) -> usize {
    let mut li = lines.iter().rposition(|l| l.start <= pos).unwrap_or(0);
    if upstream && li > 0 && lines[li].start == pos && is_soft_end(lines, li - 1) {
        li -= 1;
    }
    li
}

/// 光标的 x（相对内容区，设备像素）
pub fn caret_x(lines: &[TextLine], pos: usize, upstream: bool) -> f32 {
    let li = caret_line(lines, pos, upstream);
    lines.get(li).map(|l| l.x_at(pos.saturating_sub(l.start))).unwrap_or(0.0)
}

/// 当前行行首
pub fn line_start(lines: &[TextLine], pos: usize, upstream: bool) -> usize {
    lines.get(caret_line(lines, pos, upstream)).map(|l| l.start).unwrap_or(0)
}

/// 当前行行尾：(下标, 是否画在本行末)。折行的那一行，行尾下标与下一行首相同，要带上 upstream
pub fn line_end(lines: &[TextLine], pos: usize, upstream: bool) -> (usize, bool) {
    let li = caret_line(lines, pos, upstream);
    match lines.get(li) {
        Some(l) => (l.end, is_soft_end(lines, li)),
        None => (pos, false),
    }
}

/// ↑↓ 一行。`goal_x` 是连续按 ↑↓ 时保持的那一列。
///
/// 返回 (新下标, upstream, 新的 goal_x)。第一行再往上到全文开头、最后一行再往下到结尾 ——
/// 与 macOS / Chrome 的 textarea 一致。
pub fn vertical(
    lines: &[TextLine],
    pos: usize,
    upstream: bool,
    goal_x: Option<f32>,
    down: bool,
    total_len: usize,
) -> (usize, bool, Option<f32>) {
    if lines.is_empty() {
        return (if down { total_len } else { 0 }, false, goal_x);
    }
    let li = caret_line(lines, pos, upstream);
    let x = goal_x.unwrap_or_else(|| caret_x(lines, pos, upstream));
    let target = if down { li + 1 } else { li.wrapping_sub(1) };
    let Some(line) = lines.get(target) else {
        return (if down { total_len } else { 0 }, false, Some(x));
    };
    let col = line.nearest_col(x);
    let up = col == line.len() && is_soft_end(lines, target);
    (line.start + col, up, Some(x))
}

/// 断词用的字符类别：空白 / 西文词 / CJK / 标点。同类连续算一个词
fn word_class(c: char) -> u8 {
    if c.is_whitespace() {
        0
    } else if crate::text::is_cjk_char(c) {
        2
    } else if c.is_alphanumeric() || c == '_' {
        1
    } else {
        3
    }
}

/// 往左一个词：先跳过空白，再跳过一整段同类字符（mac ⌥← 的落点：词首）
pub fn word_left(chars: &[char], pos: usize) -> usize {
    let mut i = pos.min(chars.len());
    while i > 0 && word_class(chars[i - 1]) == 0 {
        i -= 1;
    }
    if i > 0 {
        let k = word_class(chars[i - 1]);
        while i > 0 && word_class(chars[i - 1]) == k {
            i -= 1;
        }
    }
    i
}

/// 往右一个词：先跳过空白，再跳过一整段同类字符（mac ⌥→ 的落点：词尾）
pub fn word_right(chars: &[char], pos: usize) -> usize {
    let n = chars.len();
    let mut i = pos.min(n);
    while i < n && word_class(chars[i]) == 0 {
        i += 1;
    }
    if i < n {
        let k = word_class(chars[i]);
        while i < n && word_class(chars[i]) == k {
            i += 1;
        }
    }
    i
}
