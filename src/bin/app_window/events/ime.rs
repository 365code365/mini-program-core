//! IME 输入处理

use mini_render::ui::interaction::{InteractionManager, InteractionResult};
use winit::event::Ime;

/// 处理 IME 事件
pub fn handle_ime_event(
    ime: Ime,
    interaction: &mut InteractionManager,
) -> Vec<InteractionResult> {
    let mut results = Vec::new();
    
    match ime {
        Ime::Commit(text) => {
            // 与 SDK 的 `text_input` 同一个入口：换行归一、控制字符过滤（从前这里一个都不滤，
            // 与按键那条路径的 `is_control` 过滤不一致）
            results.extend(interaction.insert_text(&text));
        }
        Ime::Preedit(text, cursor) => {
            if !text.is_empty() {
                println!("📝 IME Preedit: {} {:?}", text, cursor);
            }
        }
        Ime::Enabled => {
            println!("📝 IME Enabled");
        }
        Ime::Disabled => {
            println!("📝 IME Disabled");
        }
    }
    
    results
}
