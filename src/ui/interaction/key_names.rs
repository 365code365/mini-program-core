//! 按键名 → [`KeyInput`]。
//!
//! 移动端 SDK 的 `key(name)`（C 的 `mr_app_key`、JNI 的 `nativeKey`、Flutter 的 `sendKey`）
//! 与桌面无头的 `--key <name>` 共用这一张表。以前两处各抄一份，SDK 那份少了 `esc`，
//! 新加的键也只加到一边 —— 宿主照文档发了却没反应，又不报错。
//!
//! 名字不分大小写，修饰键用 `shift+` 前缀；不认识的名字返回 `None`（调用方决定是否告警）。

use super::KeyInput;

/// 全部认得的名字（`--help` 与文档用）
pub const KEY_NAMES: &[&str] = &[
    "enter", "confirm", "return", "newline", "shift+enter",
    "backspace", "delete", "escape", "esc",
    "left", "right", "up", "down", "home", "end",
    "shift+left", "shift+right", "shift+up", "shift+down", "shift+home", "shift+end",
    "word-left", "word-right", "shift+word-left", "shift+word-right",
    "doc-start", "doc-end", "shift+doc-start", "shift+doc-end",
    "delete-word-back", "delete-word-forward", "delete-line-back",
    "selectall", "copy", "cut", "undo", "redo",
];

impl KeyInput {
    /// 按名字取功能键。`paste` 不在这里：它要带文本，SDK 走 `text_input`
    pub fn from_name(name: &str) -> Option<KeyInput> {
        let lower = name.trim().to_ascii_lowercase();
        Some(match lower.as_str() {
            "enter" | "confirm" | "return" => KeyInput::Enter,
            "newline" | "shift+enter" => KeyInput::Newline,
            "backspace" => KeyInput::Backspace,
            "delete" => KeyInput::Delete,
            "escape" | "esc" => KeyInput::Escape,
            "left" => KeyInput::Left,
            "right" => KeyInput::Right,
            "up" => KeyInput::Up,
            "down" => KeyInput::Down,
            "home" => KeyInput::Home,
            "end" => KeyInput::End,
            "shift+left" => KeyInput::ShiftLeft,
            "shift+right" => KeyInput::ShiftRight,
            "shift+up" => KeyInput::ShiftUp,
            "shift+down" => KeyInput::ShiftDown,
            "shift+home" => KeyInput::ShiftHome,
            "shift+end" => KeyInput::ShiftEnd,
            "word-left" => KeyInput::WordLeft,
            "word-right" => KeyInput::WordRight,
            "shift+word-left" => KeyInput::ShiftWordLeft,
            "shift+word-right" => KeyInput::ShiftWordRight,
            "doc-start" => KeyInput::DocStart,
            "doc-end" => KeyInput::DocEnd,
            "shift+doc-start" => KeyInput::ShiftDocStart,
            "shift+doc-end" => KeyInput::ShiftDocEnd,
            "delete-word-back" => KeyInput::DeleteWordBack,
            "delete-word-forward" => KeyInput::DeleteWordForward,
            "delete-line-back" => KeyInput::DeleteLineBack,
            "selectall" | "select-all" => KeyInput::SelectAll,
            "copy" => KeyInput::Copy,
            "cut" => KeyInput::Cut,
            "undo" => KeyInput::Undo,
            "redo" => KeyInput::Redo,
            _ => return None,
        })
    }
}
