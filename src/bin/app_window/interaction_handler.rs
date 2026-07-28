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
        InteractionResult::Focus { id, bounds, click_x: _, is_fixed, value } => {
            println!("📝 Focus: {} at ({:.0}, {:.0}, {:.0}x{:.0}) fixed={}", id, bounds.x, bounds.y, bounds.width, bounds.height, is_fixed);
            dispatch_input_event(app, renderer, id, "focus", value);
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
            dispatch_input_event(app, renderer, id, "input", value);
        }
        InteractionResult::InputBlur { id, value } => {
            println!("📝 Blur {}: {}", id, value);
            if let Some(window) = window {
                window.set_ime_allowed(false);
            }
            dispatch_input_event(app, renderer, id, "blur", value);
        }
        InteractionResult::InputConfirm { id, value } => {
            println!("📝 Confirm {}: {}", id, value);
            if let Some(window) = window {
                window.set_ime_allowed(false);
            }
            dispatch_input_event(app, renderer, id, "confirm", value);
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

/// 把输入框事件（focus / input / blur / confirm）按微信语义派发给逻辑层。
///
/// 两个必须这样做的理由：
///
/// 1. **`detail.value` 一定是字符串。** 旧实现把值塞进 dataset 再走
///    `__callPageMethod`，而 dataset 会按字面量还原类型（`data-id="12"` → 数字 12），
///    于是输入 `1212121` 到了页面里变成数字，`e.detail.value.trim()` 直接
///    `TypeError: not a function`。微信里 `e.detail.value` 永远是字符串。
/// 2. **绑定按组件精确匹配。** 旧实现遍历绑定表取第一条 `event_type=="input"` 就 break，
///    一个页面有两个输入框时所有输入都打到第一个。
///
/// 返回是否真的派发出去（没有对应 `bind*` 时为 false）。
fn dispatch_input_event(
    app: &mut MiniApp,
    renderer: Option<&WxmlRenderer>,
    component_id: &str,
    event_type: &str,
    value: &str,
) -> bool {
    let Some(renderer) = renderer else { return false };
    let binding = match renderer.binding_for(component_id, event_type) {
        Some(b) => b,
        // 兜底：滚动后重算出来的 `component_id` 可能与聚焦那一刻不同（id 含坐标）。
        // 此时若全页只有唯一一条该类绑定，仍按它派发；有多条则宁可不发，避免串台。
        None => {
            let mut it = renderer
                .get_event_bindings()
                .iter()
                .filter(|b| b.event_type == event_type);
            match (it.next(), it.next()) {
                (Some(only), None) => only,
                _ => return false,
            }
        }
    };

    let node = serde_json::json!({
        "id": binding.id,
        "dataset": binding.data,
        "offsetLeft": binding.bounds.x,
        "offsetTop": binding.bounds.y,
    });
    // 微信的 detail：input 是 `{value, cursor, keyCode}`，focus 是 `{value, height}`，
    // blur / confirm 是 `{value}`。
    let mut detail = serde_json::json!({ "value": value });
    match event_type {
        "input" => {
            detail["cursor"] = serde_json::json!(value.chars().count());
            detail["keyCode"] = serde_json::json!(0);
        }
        "focus" => {
            // 软键盘高度：桌面宿主没有软键盘，微信在无键盘时也给 0
            detail["height"] = serde_json::json!(0);
        }
        _ => {}
    }
    let event = serde_json::json!({
        "type": event_type,
        "target": node,
        "currentTarget": node,
        "detail": detail,
        "touches": [],
        "changedTouches": [],
    });
    let handler = serde_json::to_string(&binding.handler).unwrap_or_else(|_| "''".into());
    let owner = serde_json::to_string(&binding.owner).unwrap_or_else(|_| "''".into());
    app.eval(&format!("__dispatchEvent({handler}, {event}, {owner})")).ok();
    true
}

/// 把 `<image>` 的 `bindload` / `binderror` 派发到逻辑层（微信语义）。
///
/// `load` 的 `detail` 是图片**原始**像素尺寸 `{width, height}`，`error` 是 `{errMsg}`。
/// 走 `__dispatchEvent` 而不是 `__callPageMethod`：既带上组件归属（组件模板里的
/// `bindload` 要发给组件实例），`detail` 也不会被 dataset 的字面量还原改掉类型。
pub fn dispatch_image_event(
    app: &mut MiniApp,
    e: &mini_render::renderer::ImageEvent,
    time_ms: u64,
) {
    use mini_render::renderer::ImageEventDetail;
    let node = serde_json::json!({ "id": e.id, "dataset": e.data });
    let detail = match e.detail {
        ImageEventDetail::Load { width, height } => {
            serde_json::json!({ "width": width, "height": height })
        }
        ImageEventDetail::Error => serde_json::json!({ "errMsg": "GET_IMAGE_FAILED" }),
    };
    let event = serde_json::json!({
        "type": e.event_type,
        "timeStamp": time_ms,
        "target": node,
        "currentTarget": node,
        "detail": detail,
        "touches": [],
        "changedTouches": [],
    });
    let handler = serde_json::to_string(&e.handler).unwrap_or_else(|_| "''".into());
    let owner = serde_json::to_string(&e.owner).unwrap_or_else(|_| "''".into());
    let r = app.eval(&format!("__dispatchEvent({handler}, {event}, {owner})"));
    if mini_render::renderer::components::image_net::log_enabled() {
        eprintln!("🖼 ⚑ 派发 {} -> {} 结果={:?}", e.event_type, e.handler, r);
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
