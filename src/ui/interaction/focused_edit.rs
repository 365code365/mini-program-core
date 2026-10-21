//! `FocusedInput` 的构造与「区间替换」——所有改值的按键最后都落到这两三个函数上。
//!
//! 以前每个按键分支各写一遍「转成 `Vec<char>` → 增删 → 拼回去 → 同步状态」，
//! 选区、maxlength、撤销任何一处要改就得改七八个地方。现在只有 [`FocusedInput::replace_range`]
//! 真正改值，撤销记录也只在这里记，不会漏。

use super::{EditHistory, EditKind, FocusedInput};
use crate::Rect;

impl FocusedInput {
    /// 刚聚焦的输入框：光标在末尾、无选区。maxlength / type / 多行等属性由宿主按渲染器登记的
    /// 属性补上（`host::tap::prepare_focused_input`）
    pub fn new(id: String, value: String, bounds: Rect, is_fixed: bool) -> Self {
        let cursor_pos = value.chars().count();
        Self {
            id,
            value,
            cursor_pos,
            selection_start: None,
            selection_end: None,
            is_password: false,
            bounds,
            is_fixed,
            text_offset: 0.0,
            maxlength: 140,
            input_type: "text".to_string(),
            multiline: false,
            confirm_type: "done".to_string(),
            confirm_hold: false,
            caret_upstream: false,
            goal_x: None,
            history: EditHistory::default(),
        }
    }

    /// 值的字符数（光标、选区、maxlength 都按 `char` 计）
    pub fn char_len(&self) -> usize {
        self.value.chars().count()
    }

    /// 把 `[start, end)`（字符下标）换成 `text`，光标落在插入内容之后，清掉选区。
    /// 改之前记一步撤销（同类连续编辑合并，见 `EditHistory`）。
    pub(super) fn replace_range(&mut self, start: usize, end: usize, text: &str, kind: EditKind) {
        let (start, end) = (start.min(end), start.max(end));
        self.history.record(kind, &self.value, self.cursor_pos);
        let chars: Vec<char> = self.value.chars().collect();
        let end = end.min(chars.len());
        let start = start.min(end);
        let mut out = String::with_capacity(self.value.len() + text.len());
        out.extend(&chars[..start]);
        out.push_str(text);
        out.extend(&chars[end..]);
        self.value = out;
        self.cursor_pos = start + text.chars().count();
        self.clear_selection();
        self.caret_upstream = false;
        self.goal_x = None;
    }

    /// 在光标处插入（有选区就替换选区）。不合类型的字符先滤掉，超出 maxlength 的部分截断。
    /// 什么都没插进去（全被滤掉 / 已经满了）返回 false，值不变。
    pub(super) fn insert_at_cursor(&mut self, text: &str, kind: EditKind) -> bool {
        let filtered: String = text.chars().filter(|&c| self.validate_char(c)).collect();
        if filtered.is_empty() {
            return false;
        }
        let (start, end) = self.get_selection_range().unwrap_or((self.cursor_pos, self.cursor_pos));
        let room = if self.maxlength < 0 {
            usize::MAX
        } else {
            (self.maxlength as usize).saturating_sub(self.char_len() - (end - start))
        };
        if room == 0 {
            return false;
        }
        let insert: String = filtered.chars().take(room).collect();
        self.replace_range(start, end, &insert, kind);
        true
    }

    /// 删除 `[a, b)`（顺序不限）。区间为空返回 false
    pub(super) fn delete_range(&mut self, a: usize, b: usize, kind: EditKind) -> bool {
        if a == b {
            return false;
        }
        self.replace_range(a, b, "", kind);
        true
    }

    /// 撤销 / 重做恢复出来的状态
    pub(super) fn restore(&mut self, value: String, cursor: usize) {
        self.cursor_pos = cursor.min(value.chars().count());
        self.value = value;
        self.clear_selection();
        self.caret_upstream = false;
        self.goal_x = None;
    }
}
