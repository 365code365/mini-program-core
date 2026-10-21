//! 输入框编辑：键盘输入、失焦、文本选择
//!
//! `InteractionManager` 的一片，由 `interaction/mod.rs` 组合。
//!
//! 按键先拆成「移动光标」与「改值」两类：移动统一成 [`Motion`] + 是否扩展选区，
//! 改值统一落到 `FocusedInput::replace_range`（见 focused_edit.rs）。从前每个键一个分支、
//! 各自拼字符串，于是 textarea 的回车只能「确认并失焦」、`\n` 原样塞进值里画成豆腐块，
//! ↑↓、按词移动、撤销这些常用快捷键一个都没有。
use super::text_lines::{self, TextLine};
use super::*;

/// 光标往哪走
#[derive(Clone, Copy, Debug, PartialEq)]
enum Motion {
    CharLeft,
    CharRight,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    LineUp,
    LineDown,
    DocStart,
    DocEnd,
}

/// 移动类按键 → (动作, 是否扩展选区)
fn motion_of(key: &KeyInput) -> Option<(Motion, bool)> {
    use KeyInput as K;
    use Motion as M;
    Some(match key {
        K::Left => (M::CharLeft, false),
        K::Right => (M::CharRight, false),
        K::ShiftLeft => (M::CharLeft, true),
        K::ShiftRight => (M::CharRight, true),
        K::Home => (M::LineStart, false),
        K::End => (M::LineEnd, false),
        K::ShiftHome => (M::LineStart, true),
        K::ShiftEnd => (M::LineEnd, true),
        K::Up => (M::LineUp, false),
        K::Down => (M::LineDown, false),
        K::ShiftUp => (M::LineUp, true),
        K::ShiftDown => (M::LineDown, true),
        K::WordLeft => (M::WordLeft, false),
        K::WordRight => (M::WordRight, false),
        K::ShiftWordLeft => (M::WordLeft, true),
        K::ShiftWordRight => (M::WordRight, true),
        K::DocStart => (M::DocStart, false),
        K::DocEnd => (M::DocEnd, false),
        K::ShiftDocStart => (M::DocStart, true),
        K::ShiftDocEnd => (M::DocEnd, true),
        _ => return None,
    })
}

/// 行结构：多行输入框优先用绘制端写回的视觉行（与屏幕上看到的折行一致），
/// 没有或已过期（值变了还没重画）就按 `\n` 分行。单行输入框永远只有一行。
fn lines_for(layouts: &HashMap<String, TextLayout>, input: &FocusedInput) -> Vec<TextLine> {
    match layouts.get(&input.id) {
        Some(l) if input.multiline && l.text == input.value && !l.lines.is_empty() => l.lines.clone(),
        _ => {
            let chars: Vec<char> = input.value.chars().collect();
            text_lines::logical_lines(&chars)
        }
    }
}

/// 走一步之后光标停在哪：(下标, upstream, goal_x)
fn resolve(input: &FocusedInput, lines: &[TextLine], m: Motion) -> (usize, bool, Option<f32>) {
    let pos = input.cursor_pos;
    let up = input.caret_upstream;
    let len = input.char_len();
    let chars = || input.value.chars().collect::<Vec<char>>();
    match m {
        Motion::CharLeft => (pos.saturating_sub(1), false, None),
        Motion::CharRight => ((pos + 1).min(len), false, None),
        Motion::WordLeft => (text_lines::word_left(&chars(), pos), false, None),
        Motion::WordRight => (text_lines::word_right(&chars(), pos), false, None),
        Motion::LineStart => (text_lines::line_start(lines, pos, up), false, None),
        Motion::LineEnd => {
            let (p, u) = text_lines::line_end(lines, pos, up);
            (p, u, None)
        }
        Motion::LineUp => text_lines::vertical(lines, pos, up, input.goal_x, false, len),
        Motion::LineDown => text_lines::vertical(lines, pos, up, input.goal_x, true, len),
        Motion::DocStart => (0, false, None),
        Motion::DocEnd => (len, false, None),
    }
}

/// 移动光标；`extend` 为真时扩展选区（锚点不动，活动端跟着光标）
fn move_caret(input: &mut FocusedInput, lines: &[TextLine], m: Motion, extend: bool) {
    input.history.break_merge();
    let had = input.get_selection_range();
    let (target, up, goal) = resolve(input, lines, m);
    if extend {
        if input.selection_start.is_none() {
            input.selection_start = Some(input.cursor_pos);
        }
        input.cursor_pos = target;
        input.selection_end = Some(target);
    } else {
        input.clear_selection();
        // 有选区时 ←→ 只是收拢到选区那一头，不再多走一格（浏览器 / macOS 的行为）
        input.cursor_pos = match (had, m) {
            (Some((s, _)), Motion::CharLeft) => s,
            (Some((_, e)), Motion::CharRight) => e,
            _ => target,
        };
    }
    input.caret_upstream = up && input.cursor_pos == target;
    input.goal_x = if matches!(m, Motion::LineUp | Motion::LineDown) { goal } else { None };
}

/// 有选区删选区（自成一步撤销），否则删光标到 `m` 落点之间的那一段
fn delete_by(input: &mut FocusedInput, lines: &[TextLine], m: Motion, kind: EditKind) -> bool {
    if let Some((s, e)) = input.get_selection_range() {
        return input.delete_range(s, e, EditKind::Other);
    }
    let (target, _, _) = resolve(input, lines, m);
    input.delete_range(input.cursor_pos, target, kind)
}

/// `\r\n` / `\r` 统一成 `\n`（Windows 剪贴板、部分输入法给的是 `\r\n`）
fn normalize_newlines(text: &str) -> std::borrow::Cow<'_, str> {
    if text.contains('\r') {
        std::borrow::Cow::Owned(text.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        std::borrow::Cow::Borrowed(text)
    }
}

impl InteractionManager {
    /// 处理键盘输入
    pub fn handle_key_input(&mut self, key: KeyInput) -> Option<InteractionResult> {
        let focused = self.focused_input.as_ref()?;
        // 先处理会结束编辑的键（它们要动 `focused_input` 本身）
        let returns_newline = focused.multiline && focused.confirm_type == "return";
        match &key {
            KeyInput::Escape => return self.blur_input(),
            KeyInput::Enter if !returns_newline => return self.confirm_input(),
            KeyInput::Newline if !focused.multiline => return self.confirm_input(),
            _ => {}
        }
        let lines = lines_for(&self.text_layouts, focused);
        let input = self.focused_input.as_mut()?;

        if let Some((m, extend)) = motion_of(&key) {
            move_caret(input, &lines, m, extend);
            return None;
        }

        let changed = match key {
            KeyInput::Char(c) => {
                let c = if c == '\r' { '\n' } else { c };
                // 换行自成一步撤销：撤销一次退回一行，而不是把整段话都撤掉
                let kind = if c == '\n' { EditKind::Other } else { EditKind::Typing };
                input.insert_at_cursor(c.encode_utf8(&mut [0u8; 4]), kind)
            }
            KeyInput::Paste(text) => input.insert_at_cursor(&normalize_newlines(&text), EditKind::Other),
            // 走到这里的回车 / 换行都是「textarea 要换行」（确认的那几种上面已经返回了）
            KeyInput::Enter | KeyInput::Newline => input.insert_at_cursor("\n", EditKind::Other),
            KeyInput::Backspace => delete_by(input, &lines, Motion::CharLeft, EditKind::Deleting),
            KeyInput::Delete => delete_by(input, &lines, Motion::CharRight, EditKind::Deleting),
            KeyInput::DeleteWordBack => delete_by(input, &lines, Motion::WordLeft, EditKind::Other),
            KeyInput::DeleteWordForward => delete_by(input, &lines, Motion::WordRight, EditKind::Other),
            KeyInput::DeleteLineBack => {
                // 已经在行首：与 macOS 一样删掉前面那个换行，把两行接起来
                let at_start = text_lines::line_start(&lines, input.cursor_pos, input.caret_upstream) == input.cursor_pos;
                let m = if at_start { Motion::CharLeft } else { Motion::LineStart };
                delete_by(input, &lines, m, EditKind::Other)
            }
            KeyInput::Undo => match input.history.undo(&input.value, input.cursor_pos) {
                Some((v, c)) => {
                    input.restore(v, c);
                    true
                }
                None => false,
            },
            KeyInput::Redo => match input.history.redo(&input.value, input.cursor_pos) {
                Some((v, c)) => {
                    input.restore(v, c);
                    true
                }
                None => false,
            },
            KeyInput::SelectAll => {
                input.select_all();
                input.history.break_merge();
                return None;
            }
            KeyInput::Copy => {
                let (s, e) = input.get_selection_range()?;
                let text: String = input.value.chars().skip(s).take(e - s).collect();
                return Some(InteractionResult::CopyText { text });
            }
            KeyInput::Cut => {
                let (s, e) = input.get_selection_range()?;
                let text: String = input.value.chars().skip(s).take(e - s).collect();
                input.delete_range(s, e, EditKind::Other);
                self.states.insert(input.id.clone(), ComponentState { checked: false, value: input.value.clone() });
                return Some(InteractionResult::CutText { text, id: input.id.clone(), value: input.value.clone() });
            }
            // 移动类在上面已经处理；Escape / 确认类回车也已返回
            _ => return None,
        };
        if !changed {
            return None;
        }
        self.states.insert(input.id.clone(), ComponentState { checked: false, value: input.value.clone() });
        Some(InteractionResult::InputChange { id: input.id.clone(), value: input.value.clone() })
    }

    /// 输入法 / SDK 提交的一整串文字。
    ///
    /// - 整串恰好是一个换行（`\n` / `\r\n` / `\r`）：当作**回车键**。iOS 的 `insertText("\n")`、
    ///   Android 硬件键盘的回车、Flutter 多行输入的回车都是这样来的 —— textarea 换行，
    ///   input 触发 confirm。
    /// - 其余逐字插入（每个字符一次 `bindinput`，与原来一致）：多行里的换行保留，
    ///   单行里的换行与其它控制字符丢掉（浏览器往 `<input>` 里粘贴多行文本同样会去掉换行）。
    pub fn insert_text(&mut self, text: &str) -> Vec<InteractionResult> {
        let text = normalize_newlines(text);
        if text == "\n" {
            return self.handle_key_input(KeyInput::Enter).into_iter().collect();
        }
        let mut out = Vec::new();
        for c in text.chars() {
            if self.focused_input.is_none() {
                break;
            }
            if let Some(r) = self.handle_key_input(KeyInput::Char(c)) {
                out.push(r);
            }
        }
        out
    }

    /// 回车确认：`confirm-hold` 时保持聚焦，否则失焦（键盘收起）
    fn confirm_input(&mut self) -> Option<InteractionResult> {
        let input = self.focused_input.as_ref()?;
        let (id, value, hold) = (input.id.clone(), input.value.clone(), input.confirm_hold);
        if !hold {
            self.focused_input = None;
        }
        Some(InteractionResult::InputConfirm { id, value, keep_focus: hold })
    }

    /// 取消输入框聚焦
    pub fn blur_input(&mut self) -> Option<InteractionResult> {
        if let Some(input) = self.focused_input.take() {
            return Some(InteractionResult::InputBlur {
                id: input.id,
                value: input.value,
            });
        }
        None
    }

    /// 是否有输入框聚焦
    pub fn has_focused_input(&self) -> bool {
        self.focused_input.is_some()
    }

    /// 准备文本选择（鼠标按下时调用）
    /// 只移动光标到点击位置，清除之前的选择，记录锚点位置
    /// 实际选择在鼠标移动时才开始
    pub fn prepare_text_selection(&mut self, cursor_pos: usize) {
        if let Some(input) = &mut self.focused_input {
            // 清除之前的选择
            input.clear_selection();
            // 移动光标到点击位置
            input.cursor_pos = cursor_pos;
            // 记录锚点位置，但不开始选择
            self.selection_anchor = Some(cursor_pos);
            self.is_selecting_text = false; // 还没开始选择
        }
    }

    /// 开始文本选择（鼠标按下时调用，用于需要立即开始选择的情况）
    pub fn begin_text_selection(&mut self, cursor_pos: usize) {
        if let Some(input) = &mut self.focused_input {
            self.is_selecting_text = true;
            self.selection_anchor = Some(cursor_pos);
            input.selection_start = Some(cursor_pos);
            input.selection_end = Some(cursor_pos);
            input.cursor_pos = cursor_pos;
        }
    }

    /// 更新文本选择（鼠标移动时调用）
    pub fn update_text_selection(&mut self, cursor_pos: usize) {
        // 如果有锚点但还没开始选择，现在开始
        if let Some(anchor) = self.selection_anchor {
            if let Some(input) = &mut self.focused_input {
                // 只有当光标位置与锚点不同时才开始选择
                if cursor_pos != anchor {
                    self.is_selecting_text = true;
                    input.selection_start = Some(anchor);
                    input.selection_end = Some(cursor_pos);
                }
                input.cursor_pos = cursor_pos;
            }
        }
    }

    /// 结束文本选择（鼠标释放时调用）
    pub fn end_text_selection(&mut self) {
        self.is_selecting_text = false;
        self.selection_anchor = None;
        // 如果选择范围为空，清除选择
        if let Some(input) = &mut self.focused_input {
            if let (Some(start), Some(end)) = (input.selection_start, input.selection_end) {
                if start == end {
                    input.clear_selection();
                }
            }
        }
    }

    /// 是否正在选择文本（或准备选择）
    pub fn is_selecting(&self) -> bool {
        self.selection_anchor.is_some()
    }

    /// 是否真正在拖动选择（有实际的选择范围）
    pub fn is_dragging_selection(&self) -> bool {
        self.is_selecting_text
    }
}
