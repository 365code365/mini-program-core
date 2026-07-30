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
    _scroll_position: f32,
    _scale_factor: f64,
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
            mini_render::host::tap::dispatch_input_event(app, renderer, id, "focus", value);
            if let Some(window) = window {
                window.set_ime_allowed(true);
            }
            // 候选词面板的位置**不在这里设**：光标会随打字移动，只在聚焦时设一次
            // 就永远停在最初那个位置。改由宿主每帧按真实光标矩形同步
            // （`MiniAppWindow::sync_ime_cursor_area`）。
            //
            // 旧实现还有一个更直接的毛病：它把光标区域报成「输入框底边往下
            // 16pt 高的一个盒子」，而系统是把面板摆在这个盒子**之下** ——
            // 于是候选条离输入框凭空多出一段距离，就是「输入提示距离太远」。
        }
        InteractionResult::InputChange { id, value } => {
            println!("📝 Input {}: {}", id, value);
            mini_render::host::tap::dispatch_input_event(app, renderer, id, "input", value);
        }
        InteractionResult::InputBlur { id, value } => {
            println!("📝 Blur {}: {}", id, value);
            if let Some(window) = window {
                window.set_ime_allowed(false);
            }
            mini_render::host::tap::dispatch_input_event(app, renderer, id, "blur", value);
        }
        InteractionResult::InputConfirm { id, value } => {
            println!("📝 Confirm {}: {}", id, value);
            if let Some(window) = window {
                window.set_ime_allowed(false);
            }
            mini_render::host::tap::dispatch_input_event(app, renderer, id, "confirm", value);
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
