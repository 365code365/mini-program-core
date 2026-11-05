//! 编辑结果的附带信息：`bindinput` 的 `cursor` / `keyCode`、失焦时的 `cursor` 与「内容变没变」（input 的 `bindchange`）。
//!
//! 官方（component/input.html、textarea.html）：
//! - input `bind:input` detail `{value, cursor, keyCode}`，textarea 同；
//! - textarea `bind:blur` detail `{value, cursor}`（input 的 blur 只写了 `{value, encryptedValue?, encryptError?}`，不带 cursor）；
//! - input `bind:change`「键盘非聚焦状态内容改变时触发」`{value}`：失焦时与聚焦那一刻的值比，变了才发；
//! - textarea 的 `bind:input` 处理函数的返回值**不会**反映到 textarea 上（只有 input 会替换内容）。
//!
//! 为什么用线程局部传而不是改派发函数的签名：结果（`InteractionResult`）产生与派发
//! （`host::tap::dispatch_input_event`）中间隔着桌面窗体 / SDK 两套调用方、十来处调用点，
//! 而派发那一侧手里没有 `InteractionManager`。产生与派发总在同一线程上紧挨着发生（SDK 一个引擎一个线程），
//! 所以这里按 (输入框 id, 值) 记一笔、派发时取走，对不上就退回从前的算法（光标 = 末尾、keyCode = 0）。
use std::cell::RefCell;
use std::collections::VecDeque;

/// 一次改值：这一步之后的光标与按键
#[derive(Clone, Debug, PartialEq)]
pub struct EditNote {
    pub id: String,
    pub value: String,
    pub cursor: usize,
    pub key_code: u32,
    pub multiline: bool,
}

/// 一次失焦
#[derive(Clone, Debug, PartialEq)]
pub struct BlurNote {
    pub id: String,
    pub cursor: usize,
    /// 与聚焦时的值不同（input 据此发 `bindchange`）
    pub changed: bool,
    pub multiline: bool,
}

thread_local! {
    static EDITS: RefCell<VecDeque<EditNote>> = const { RefCell::new(VecDeque::new()) };
    static LAST_MULTILINE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static BLUR: RefCell<Option<BlurNote>> = const { RefCell::new(None) };
}

/// 一次输入法提交可以是一整串（每个字一次 bindinput）：排队，派发时按 (id, 值) 对上
const CAP: usize = 64;

pub(super) fn note_edit(n: EditNote) {
    LAST_MULTILINE.with(|m| m.set(n.multiline));
    EDITS.with(|q| {
        let mut q = q.borrow_mut();
        if q.len() >= CAP {
            q.pop_front();
        }
        q.push_back(n);
    });
}

/// 派发 `bindinput` 时取走与这一次 (id, 值) 对应的那笔
pub fn take_edit(id: &str, value: &str) -> Option<EditNote> {
    EDITS.with(|q| {
        let mut q = q.borrow_mut();
        let i = q.iter().position(|n| n.id == id && n.value == value)?;
        // 排在它前面的是派发方没取的（被合并 / 丢了的）：一起扔掉
        q.drain(..i);
        q.pop_front()
    })
}

/// 刚派发的 `bindinput` 是不是 textarea 的（它的返回值不替换内容）
pub fn last_edit_multiline() -> bool {
    LAST_MULTILINE.with(|m| m.get())
}

pub(super) fn note_blur(n: BlurNote) {
    BLUR.with(|b| *b.borrow_mut() = Some(n));
}

/// 派发 `bindblur` 时取走（id 对不上返回 None）
pub fn take_blur(id: &str) -> Option<BlurNote> {
    BLUR.with(|b| {
        let mut b = b.borrow_mut();
        if b.as_ref().is_some_and(|n| n.id == id) { b.take() } else { None }
    })
}

/// 改值之后记一笔（handle_key_input / 剪切）
pub(super) fn note_input_edit(f: &super::FocusedInput, key_code: u32) {
    note_edit(EditNote { id: f.id.clone(), value: f.value.clone(), cursor: f.cursor_pos, key_code, multiline: f.multiline });
}

/// 失焦（blur_input / 确认收键盘）时记一笔
pub(super) fn note_input_blur(f: &super::FocusedInput) {
    note_blur(BlurNote { id: f.id.clone(), cursor: f.cursor_pos, changed: f.value != f.value_at_focus, multiline: f.multiline });
}

thread_local! {
    static SWITCHED: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
}

/// 点另一个输入框换焦点：旧的那个失焦（记下失焦信息与它的 (id, 值)，宿主取走派 bindblur）
pub(super) fn note_switch(prev: &super::FocusedInput) {
    note_input_blur(prev);
    SWITCHED.with(|s| *s.borrow_mut() = Some((prev.id.clone(), prev.value.clone())));
}

/// 取走「因换焦点而失焦」的那个输入框 (id, 值)
pub fn take_switch() -> Option<(String, String)> {
    SWITCHED.with(|s| s.borrow_mut().take())
}

/// 按键 → `keyCode`（官方只写「keyCode 为键值」，没给表；按 DOM 的 keyCode：退格 8、回车 13、Delete 46，
/// 可打印字符给它的字符码 —— 安卓微信实测给的是字符码，iOS 待验）。粘贴 / 剪切 / 撤销这类不是按键的给 0
pub(super) fn key_code_of(key: &super::KeyInput) -> u32 {
    use super::KeyInput as K;
    match key {
        K::Backspace | K::DeleteWordBack | K::DeleteLineBack => 8,
        K::Delete | K::DeleteWordForward => 46,
        K::Enter | K::Newline => 13,
        K::Char('\n') | K::Char('\r') => 13,
        K::Char(c) => *c as u32,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 按_id_与值对上_前面没取的一并丢掉() {
        let n = |v: &str, c| EditNote { id: "a".into(), value: v.into(), cursor: c, key_code: 0, multiline: false };
        note_edit(n("x", 1));
        note_edit(n("xy", 2));
        assert_eq!(take_edit("a", "xy").map(|e| e.cursor), Some(2));
        assert_eq!(take_edit("a", "x"), None, "排在前面的已经扔了");
        note_blur(BlurNote { id: "a".into(), cursor: 2, changed: true, multiline: true });
        assert_eq!(take_blur("b"), None);
        assert!(take_blur("a").is_some_and(|b| b.changed && b.cursor == 2));
    }
}
