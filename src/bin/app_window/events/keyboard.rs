//! 键盘事件处理

use mini_render::ui::interaction::{InteractionManager, InteractionResult, KeyInput};
use winit::keyboard::{PhysicalKey, KeyCode, ModifiersState};

/// 处理键盘输入，返回是否已处理和可能的交互结果
pub fn handle_keyboard_input(
    physical_key: PhysicalKey,
    modifiers: ModifiersState,
    interaction: &mut InteractionManager,
    clipboard: &mut Option<arboard::Clipboard>,
) -> (bool, Option<InteractionResult>) {
    if !interaction.has_focused_input() {
        return (false, None);
    }
    let PhysicalKey::Code(code) = physical_key else {
        return (false, None);
    };
    let key_input = map_editing_key(code, modifiers, cfg!(target_os = "macos"), || {
        clipboard.as_mut().and_then(|cb| cb.get_text().ok()).unwrap_or_default()
    });
    match key_input {
        Some(ki) => (true, interaction.handle_key_input(ki)),
        None => (false, None),
    }
}

/// 键码 + 修饰键 → 编辑意图。按平台习惯分两套（`mac` 单独传进来，便于两套都能测）：
///
/// | 意图 | macOS | 其它平台 |
/// |---|---|---|
/// | 按词移动 / 删除 | ⌥← ⌥→ / ⌥⌫ ⌥⌦ | Ctrl+← Ctrl+→ / Ctrl+⌫ Ctrl+⌦ |
/// | 行首 / 行尾 | ⌘← ⌘→、Home End | Home End |
/// | 删到行首 | ⌘⌫ | — |
/// | 全文首 / 尾 | ⌘↑ ⌘↓、Ctrl+Home/End | Ctrl+Home Ctrl+End |
/// | 撤销 / 重做 | ⌘Z / ⇧⌘Z | Ctrl+Z / Ctrl+Y、Ctrl+Shift+Z |
/// | 只换行 | ⇧↩ ⌥↩ | Shift+Enter Alt+Enter |
///
/// 全选 / 复制 / 剪切 / 粘贴 / 撤销认 ⌘ 也认 Ctrl（mac 上按 Ctrl+C 也复制，与从前一致）。
/// 加 Shift 的移动一律是扩展选区。返回 `None` 的键交回默认处理（滚动页面等）。
pub fn map_editing_key(
    code: KeyCode,
    modifiers: ModifiersState,
    mac: bool,
    paste_text: impl FnOnce() -> String,
) -> Option<KeyInput> {
    use KeyInput as K;
    let cmd = modifiers.contains(ModifiersState::SUPER);
    let ctrl = modifiers.contains(ModifiersState::CONTROL);
    let alt = modifiers.contains(ModifiersState::ALT);
    let shift = modifiers.contains(ModifiersState::SHIFT);
    let primary = cmd || ctrl;
    let word = if mac { alt } else { ctrl };
    let line = mac && cmd;
    let pick = |plain: K, extend: K| if shift { extend } else { plain };
    Some(match code {
        KeyCode::Backspace if word => K::DeleteWordBack,
        KeyCode::Backspace if line => K::DeleteLineBack,
        KeyCode::Backspace => K::Backspace,
        KeyCode::Delete if word => K::DeleteWordForward,
        KeyCode::Delete => K::Delete,
        KeyCode::ArrowLeft if line => pick(K::Home, K::ShiftHome),
        KeyCode::ArrowLeft if word => pick(K::WordLeft, K::ShiftWordLeft),
        KeyCode::ArrowLeft => pick(K::Left, K::ShiftLeft),
        KeyCode::ArrowRight if line => pick(K::End, K::ShiftEnd),
        KeyCode::ArrowRight if word => pick(K::WordRight, K::ShiftWordRight),
        KeyCode::ArrowRight => pick(K::Right, K::ShiftRight),
        KeyCode::ArrowUp if line => pick(K::DocStart, K::ShiftDocStart),
        KeyCode::ArrowUp => pick(K::Up, K::ShiftUp),
        KeyCode::ArrowDown if line => pick(K::DocEnd, K::ShiftDocEnd),
        KeyCode::ArrowDown => pick(K::Down, K::ShiftDown),
        KeyCode::Home if ctrl => pick(K::DocStart, K::ShiftDocStart),
        KeyCode::Home => pick(K::Home, K::ShiftHome),
        KeyCode::End if ctrl => pick(K::DocEnd, K::ShiftDocEnd),
        KeyCode::End => pick(K::End, K::ShiftEnd),
        KeyCode::Enter | KeyCode::NumpadEnter if shift || alt => K::Newline,
        KeyCode::Enter | KeyCode::NumpadEnter => K::Enter,
        KeyCode::Escape => K::Escape,
        KeyCode::KeyA if primary => K::SelectAll,
        KeyCode::KeyC if primary => K::Copy,
        KeyCode::KeyX if primary => K::Cut,
        KeyCode::KeyV if primary => K::Paste(paste_text()),
        KeyCode::KeyZ if primary => pick(K::Undo, K::Redo),
        KeyCode::KeyY if ctrl && !mac => K::Redo,
        _ => return None,
    })
}

/// 处理文本输入字符
pub fn handle_text_input(
    text: &str,
    ctrl: bool,
    interaction: &mut InteractionManager,
) -> Vec<InteractionResult> {
    let mut results = Vec::new();
    
    if ctrl || !interaction.has_focused_input() {
        return results;
    }
    
    for c in text.chars() {
        if c.is_control() { continue; }
        if let Some(result) = interaction.handle_key_input(KeyInput::Char(c)) {
            results.push(result);
        }
    }
    
    results
}

/// 处理默认键盘事件（非输入框聚焦时）
pub fn handle_default_keyboard(
    physical_key: PhysicalKey,
    interaction: &mut InteractionManager,
) -> Option<DefaultKeyAction> {
    if let PhysicalKey::Code(code) = physical_key {
        match code {
            KeyCode::Escape => {
                if interaction.has_focused_input() {
                    return Some(DefaultKeyAction::BlurInput);
                } else {
                    return Some(DefaultKeyAction::Exit);
                }
            }
            KeyCode::Backspace => {
                if !interaction.has_focused_input() {
                    return Some(DefaultKeyAction::NavigateBack);
                }
            }
            KeyCode::ArrowUp => return Some(DefaultKeyAction::ScrollUp),
            KeyCode::ArrowDown => return Some(DefaultKeyAction::ScrollDown),
            KeyCode::PageUp => return Some(DefaultKeyAction::PageUp),
            KeyCode::PageDown => return Some(DefaultKeyAction::PageDown),
            _ => {}
        }
    }
    None
}

/// 默认键盘动作
pub enum DefaultKeyAction {
    Exit,
    NavigateBack,
    BlurInput,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(code: KeyCode, mods: ModifiersState, mac: bool) -> Option<KeyInput> {
        map_editing_key(code, mods, mac, || "clip".to_string())
    }

    const NONE: ModifiersState = ModifiersState::empty();
    const SHIFT: ModifiersState = ModifiersState::SHIFT;
    const CTRL: ModifiersState = ModifiersState::CONTROL;
    const ALT: ModifiersState = ModifiersState::ALT;
    const CMD: ModifiersState = ModifiersState::SUPER;

    #[test]
    fn mac_uses_option_for_words_and_command_for_lines() {
        assert_eq!(map(KeyCode::ArrowLeft, ALT, true), Some(KeyInput::WordLeft));
        assert_eq!(map(KeyCode::ArrowRight, ALT | SHIFT, true), Some(KeyInput::ShiftWordRight));
        assert_eq!(map(KeyCode::ArrowLeft, CMD, true), Some(KeyInput::Home));
        assert_eq!(map(KeyCode::ArrowRight, CMD | SHIFT, true), Some(KeyInput::ShiftEnd));
        assert_eq!(map(KeyCode::ArrowUp, CMD, true), Some(KeyInput::DocStart));
        assert_eq!(map(KeyCode::Backspace, ALT, true), Some(KeyInput::DeleteWordBack));
        assert_eq!(map(KeyCode::Backspace, CMD, true), Some(KeyInput::DeleteLineBack));
        assert_eq!(map(KeyCode::KeyZ, CMD, true), Some(KeyInput::Undo));
        assert_eq!(map(KeyCode::KeyZ, CMD | SHIFT, true), Some(KeyInput::Redo));
        assert_eq!(map(KeyCode::KeyV, CMD, true), Some(KeyInput::Paste("clip".into())));
    }

    #[test]
    fn other_platforms_use_ctrl_for_words() {
        assert_eq!(map(KeyCode::ArrowLeft, CTRL, false), Some(KeyInput::WordLeft));
        assert_eq!(map(KeyCode::Backspace, CTRL, false), Some(KeyInput::DeleteWordBack));
        assert_eq!(map(KeyCode::Home, CTRL, false), Some(KeyInput::DocStart));
        assert_eq!(map(KeyCode::KeyY, CTRL, false), Some(KeyInput::Redo));
        assert_eq!(map(KeyCode::ArrowLeft, CMD, false), Some(KeyInput::Left), "非 mac 的 Super 不当 ⌘ 用");
    }

    #[test]
    fn enter_variants_and_plain_keys() {
        assert_eq!(map(KeyCode::Enter, NONE, true), Some(KeyInput::Enter));
        assert_eq!(map(KeyCode::NumpadEnter, NONE, false), Some(KeyInput::Enter));
        assert_eq!(map(KeyCode::Enter, SHIFT, true), Some(KeyInput::Newline));
        assert_eq!(map(KeyCode::Enter, ALT, false), Some(KeyInput::Newline));
        assert_eq!(map(KeyCode::ArrowUp, NONE, false), Some(KeyInput::Up));
        assert_eq!(map(KeyCode::ArrowDown, SHIFT, false), Some(KeyInput::ShiftDown));
        assert_eq!(map(KeyCode::KeyQ, NONE, true), None, "普通字母交给文本输入");
    }
}
