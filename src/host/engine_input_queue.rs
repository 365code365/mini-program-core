//! 输入法连着送来的几下：页面在上一下的回调里 `setData` 过，就等它画完一帧再送下一下。
//!
//! 为什么：输入框的值是受控的（`elements.rs` `sync_input_value`）—— 页面改了 `value` 属性才以页面为准，
//! 而「改没改」是拿这一帧的属性与**上一次画的时候**见到的比。中文输入法点「发送」时常常在同一拍里先上屏
//! 组字区的字、再发编辑器动作：引擎还没画过一帧，`bindinput` 里 `setData({draft:'在吗'})` 的值没人见过，
//! 紧接着 `bindconfirm` 里 `setData({draft:''})`，下一帧属性是 `''` —— 与上次见到的 `''` 一样，判成「没改」，
//! 框里留着「在吗」，下一句接在后面（轻聊聊天页「在吗周末一起吃饭吗」）。真机上两下之间隔着帧就没事，
//! 所以这里只是把「中间隔一帧」补上：排队的输入在 pump 画完之后按序送出，又 setData 了就再等下一帧。
//!
//! 只有确认键（`enter` / `confirm` / `return`）排队；排着的时候后面来的也排在它后面（保持顺序）。
//! 逐字上屏、退格、方向键照旧当场生效，不多等一帧。
use super::engine::MiniEngine;

pub enum QueuedInput {
    Text(String),
    Key(String),
}

impl MiniEngine {
    /// 这一下要不要排队：前面已有排着的（保持顺序），或这是确认键、而页面数据脏着还没画过（受控值没对上账）。
    ///
    /// 只拦确认键：会把值设回「上次见过的样子」的是确认回调（发送后清空）。打字、退格、方向键不拦 ——
    /// Flutter 插件一次输入法更新会连发好几个 `left` / `backspace` 再送字，每个都等一帧就是肉眼可见的迟钝
    pub(super) fn queue_input_if_pending(&mut self, ev: QueuedInput) -> Option<QueuedInput> {
        let single_line = self.interaction.focused_input.as_ref().is_some_and(|f| !f.multiline);
        let confirm = match &ev {
            QueuedInput::Key(k) => matches!(k.as_str(), "enter" | "confirm" | "return"),
            // 单行输入框收到的「\n」也是回车（text_input 的约定）
            QueuedInput::Text(t) => single_line && (t == "\n" || t == "\r\n"),
        };
        if !self.input_queue.is_empty() || (confirm && self.page_data_dirty && self.interaction.has_focused_input()) {
            self.input_queue.push_back(ev);
            return None;
        }
        Some(ev)
    }

    /// pump 画完之后：按序送出排着的输入；送出的那一下又让页面 setData 了，剩下的等下一帧
    pub(super) fn flush_input_queue(&mut self) {
        while let Some(ev) = self.input_queue.pop_front() {
            match ev {
                QueuedInput::Text(s) => self.text_input_now(&s),
                QueuedInput::Key(k) => self.key_now(&k),
            }
            if self.page_data_dirty {
                break;
            }
        }
    }

    /// 还有排着的输入（宿主要续帧）
    pub(super) fn input_queue_pending(&self) -> bool {
        !self.input_queue.is_empty()
    }
}
