//! 交互结果处理

use mini_render::ui::interaction::InteractionResult;
use mini_render::renderer::WxmlRenderer;
use mini_render::runtime::MiniApp;
use winit::window::Window;
use std::sync::Arc;

/// 处理交互结果
pub fn handle_interaction_result(
    result: &InteractionResult,
    window: Option<&Arc<Window>>,
    renderer: Option<&WxmlRenderer>,
    app: &mut MiniApp,
    clipboard: &mut Option<arboard::Clipboard>,
    scroll_position: f32,
    scale_factor: f64,
) {
    match result {
        InteractionResult::Toggle { id, checked } => {
            println!("🔘 Toggle {}: {}", id, checked);
        }
        InteractionResult::Select { id, value } => {
            println!("🔘 Select {}: {}", id, value);
        }
        InteractionResult::SliderChange { id, value } => {
            println!("🎚️ Slider {}: {}", id, value);
        }
        InteractionResult::SliderEnd { id } => {
            println!("🎚️ Slider {} released", id);
        }
        InteractionResult::Focus { id, bounds, click_x: _, is_fixed } => {
            println!("📝 Focus: {} at ({:.0}, {:.0}, {:.0}x{:.0}) fixed={}", id, bounds.x, bounds.y, bounds.width, bounds.height, is_fixed);
            if let Some(window) = window {
                window.set_ime_allowed(true);
                let sf = scale_factor;
                
                // 计算 IME 位置
                // 如果是 fixed 元素，bounds.y 已经是视口坐标，不需要减去 scroll_position
                // 如果是普通元素，bounds.y 是内容坐标，需要减去 scroll_position 得到视口坐标
                let viewport_y = if *is_fixed {
                    bounds.y
                } else {
                    bounds.y - scroll_position
                };
                
                // macOS IME: position 是光标位置，size 是光标区域
                // 将光标位置设置在输入框内部底部，这样候选框会紧贴输入框下方
                let padding_left = 12.0 * sf as f32; // 与 input.rs 中的 padding 一致
                let ime_x = ((bounds.x + padding_left) * sf as f32) as f64;
                // 光标 y 位置设置在输入框底部边缘
                let ime_y = ((viewport_y + bounds.height) * sf as f32) as f64;
                
                println!("📝 IME cursor: ({:.0}, {:.0})", ime_x, ime_y);
                
                // size 设置为光标大小（1x字体高度）
                let cursor_height = (16.0 * sf) as f64; // 默认字体大小
                window.set_ime_cursor_area(
                    winit::dpi::PhysicalPosition::new(ime_x, ime_y),
                    winit::dpi::PhysicalSize::new(1.0, cursor_height),
                );
            }
        }
        InteractionResult::InputChange { id, value } => {
            println!("📝 Input {}: {}", id, value);
            if let Some(renderer) = renderer {
                for binding in renderer.get_event_bindings() {
                    if binding.event_type == "input" {
                        let mut event_data = binding.data.clone();
                        event_data.insert("value".to_string(), value.clone());
                        let data_json = serde_json::to_string(&event_data).unwrap_or("{}".to_string());
                        let call_code = format!("__callPageMethod('{}', {})", binding.handler, data_json);
                        app.eval(&call_code).ok();
                        break;
                    }
                }
            }
        }
        InteractionResult::InputBlur { id, value } => {
            println!("📝 Blur {}: {}", id, value);
            if let Some(window) = window {
                window.set_ime_allowed(false);
            }
            if let Some(renderer) = renderer {
                for binding in renderer.get_event_bindings() {
                    if binding.event_type == "blur" {
                        let mut event_data = binding.data.clone();
                        event_data.insert("value".to_string(), value.clone());
                        let data_json = serde_json::to_string(&event_data).unwrap_or("{}".to_string());
                        let call_code = format!("__callPageMethod('{}', {})", binding.handler, data_json);
                        app.eval(&call_code).ok();
                        break;
                    }
                }
            }
        }
        InteractionResult::InputConfirm { id, value } => {
            println!("📝 Confirm {}: {}", id, value);
            if let Some(window) = window {
                window.set_ime_allowed(false);
            }
            if let Some(renderer) = renderer {
                for binding in renderer.get_event_bindings() {
                    if binding.event_type == "confirm" {
                        let mut event_data = binding.data.clone();
                        event_data.insert("value".to_string(), value.clone());
                        let data_json = serde_json::to_string(&event_data).unwrap_or("{}".to_string());
                        let call_code = format!("__callPageMethod('{}', {})", binding.handler, data_json);
                        app.eval(&call_code).ok();
                        break;
                    }
                }
            }
        }
        InteractionResult::ButtonClick { id, bounds: _ } => {
            println!("🔘 Button clicked: {}", id);
        }
        InteractionResult::CopyText { text } => {
            println!("📋 Copy: {}", text);
            if let Some(ref mut cb) = clipboard {
                if let Err(e) = cb.set_text(text) {
                    println!("❌ Clipboard copy failed: {}", e);
                } else {
                    println!("✅ Copied to clipboard");
                }
            }
        }
        InteractionResult::CutText { text, id, value } => {
            println!("✂️ Cut from {}: {} (remaining: {})", id, text, value);
            if let Some(ref mut cb) = clipboard {
                if let Err(e) = cb.set_text(text) {
                    println!("❌ Clipboard cut failed: {}", e);
                } else {
                    println!("✅ Cut to clipboard");
                }
            }
        }
    }
}

/// 检查并获取导航请求
pub fn check_navigation(app: &mut MiniApp) -> Option<super::navigation::NavigationRequest> {
    use super::navigation::NavigationRequest;
    
    if let Ok(nav_str) = app.eval("JSON.stringify(__pendingNavigation || null)") {
        if nav_str != "null" && !nav_str.is_empty() {
            if let Ok(nav) = serde_json::from_str::<serde_json::Value>(&nav_str) {
                if let Some(nav_type) = nav.get("type").and_then(|v| v.as_str()) {
                    let url = nav.get("url").and_then(|v| v.as_str()).unwrap_or("");
                    let result = match nav_type {
                        "navigateTo" => Some(NavigationRequest::NavigateTo { url: url.to_string() }),
                        "navigateBack" => Some(NavigationRequest::NavigateBack),
                        "switchTab" => Some(NavigationRequest::SwitchTab { url: url.to_string() }),
                        "redirectTo" => Some(NavigationRequest::RedirectTo { url: url.to_string() }),
                        "reLaunch" => Some(NavigationRequest::ReLaunch { url: url.to_string() }),
                        _ => None,
                    };
                    // 清除导航请求
                    app.eval("__pendingNavigation = null").ok();
                    return result;
                }
            }
        }
    }
    None
}

/// 打印 JS 输出
pub fn print_js_output(app: &MiniApp) {
    if let Ok(output) = app.eval("__print_buffer.splice(0).join('\\n')") {
        if !output.is_empty() && output != "undefined" {
            for line in output.lines() {
                println!("   {}", line);
            }
        }
    }
}
