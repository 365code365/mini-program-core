//! 输入框的撤销 / 重做（⌘Z / Ctrl+Z、⇧⌘Z / Ctrl+Y）。
//!
//! 存的是**整串快照**而不是操作：输入框的值很短（默认 maxlength 140），快照最简单也最不容易错 ——
//! 操作日志要处理选区替换、maxlength 截断、类型过滤各种组合，回放时任何一处不对称就会把值改坏。
//!
//! 连续打字合成一步（浏览器也是这样）：否则打了一句话要按几十次撤销。
//! 光标移动、换行、粘贴、删除方式变了，都会打断合并。

/// 一步编辑属于哪类：同类连续的编辑合成一步撤销
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    /// 逐字输入（不含换行）
    Typing,
    /// 逐字删除（退格 / 删除键）
    Deleting,
    /// 粘贴、剪切、换行、按词删除……每次自成一步
    Other,
}

#[derive(Clone, Debug)]
struct Snapshot {
    value: String,
    cursor: usize,
}

/// 撤销栈的深度上限：再多也没人会按那么多次，留着只是占内存
const LIMIT: usize = 100;

#[derive(Clone, Debug, Default)]
pub struct EditHistory {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// 上一步编辑的类别；`None` 表示被打断了（下一步一定新开一格）
    last: Option<EditKind>,
}

impl EditHistory {
    /// 编辑**之前**调用：记下改之前的值与光标
    pub fn record(&mut self, kind: EditKind, value: &str, cursor: usize) {
        self.redo.clear();
        let merge = kind != EditKind::Other && self.last == Some(kind);
        if !merge {
            if self.undo.len() >= LIMIT {
                self.undo.remove(0);
            }
            self.undo.push(Snapshot { value: value.to_string(), cursor });
        }
        self.last = Some(kind);
    }

    /// 光标移动等非编辑操作：下一次打字另起一步
    pub fn break_merge(&mut self) {
        self.last = None;
    }

    /// 撤销：返回要恢复成的 (值, 光标)。`value` / `cursor` 是当前状态，存进重做栈
    pub fn undo(&mut self, value: &str, cursor: usize) -> Option<(String, usize)> {
        let s = self.undo.pop()?;
        self.redo.push(Snapshot { value: value.to_string(), cursor });
        self.last = None;
        Some((s.value, s.cursor))
    }

    /// 重做：撤销的反方向
    pub fn redo(&mut self, value: &str, cursor: usize) -> Option<(String, usize)> {
        let s = self.redo.pop()?;
        self.undo.push(Snapshot { value: value.to_string(), cursor });
        self.last = None;
        Some((s.value, s.cursor))
    }
}
