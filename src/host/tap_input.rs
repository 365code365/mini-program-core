//! 输入框事件派发：`bindinput` / focus / blur / confirm / keyboardheightchange（按输入框的组件 id 找绑定）。
//! 从 tap.rs 拆出（**纯搬迁**，那边到了 500 行），对外路径照旧是 `host::tap::dispatch_input_event` 等（tap.rs 再导出）。
use crate::renderer::WxmlRenderer;
use crate::runtime::MiniApp;

/// `bindinput` 的处理函数返回了一个字符串：用它替换输入框的内容（官方 component/input：
/// 「处理函数可以直接 return 一个字符串，将替换输入框的内容」）。在派发完 `input` 之后立刻取
pub fn take_input_replacement(app: &MiniApp) -> Option<String> {
    // textarea：「bind:input 处理函数的返回值并不会反映到 textarea 上」（官方 textarea.html）
    if crate::ui::interaction::edit_note::last_edit_multiline() {
        return None;
    }
    let raw = app
        .eval("typeof __lastHandlerResult === 'string' ? JSON.stringify(__lastHandlerResult) : ''")
        .ok()?;
    if raw.is_empty() {
        return None;
    }
    serde_json::from_str::<String>(&raw).ok()
}

pub fn dispatch_input_event(
    app: &mut MiniApp,
    renderer: Option<&WxmlRenderer>,
    component_id: &str,
    event_type: &str,
    value: &str,
) -> bool {
    // 微信的 detail：input 是 `{value, cursor, keyCode}`，focus 是 `{value, height}`，
    // textarea 的 blur 是 `{value, cursor}`、input 的 blur 与 confirm 是 `{value}`。
    // 光标 / keyCode / 失焦时内容变没变由产生结果的那一侧记下（ui/interaction/edit_note.rs）
    use crate::ui::interaction::edit_note;
    let mut detail = serde_json::json!({ "value": value });
    match event_type {
        "input" => {
            let note = edit_note::take_edit(component_id, value);
            detail["cursor"] = serde_json::json!(note.as_ref().map(|n| n.cursor).unwrap_or_else(|| value.chars().count()));
            detail["keyCode"] = serde_json::json!(note.map(|n| n.key_code).unwrap_or(0));
        }
        "focus" => {
            // 软键盘高度：桌面宿主没有软键盘，微信在无键盘时也给 0（SDK 用宿主报来的高度，engine_input.rs）
            detail["height"] = serde_json::json!(0);
        }
        "blur" => {
            if let Some(n) = edit_note::take_blur(component_id) {
                if n.multiline {
                    detail["cursor"] = serde_json::json!(n.cursor);
                } else if n.changed {
                    // input 的 bindchange：「键盘非聚焦状态内容改变时触发」，失焦时内容与聚焦时不同才发。
                    // 顺序按 DOM（change 先于 blur），微信真机顺序待验
                    dispatch_input_detail(app, renderer, component_id, "change", serde_json::json!({ "value": value }));
                }
            }
        }
        _ => {}
    }
    dispatch_input_detail(app, renderer, component_id, event_type, detail)
}

/// 按输入框的组件 id 派发它身上的 `bind<event_type>`，detail 由调用方给
/// （`keyboardheightchange` 的 `{height, duration}` 走这里，见 engine_keyboard.rs）
pub fn dispatch_input_detail(
    app: &mut MiniApp,
    renderer: Option<&WxmlRenderer>,
    component_id: &str,
    event_type: &str,
    detail: serde_json::Value,
) -> bool {
    let Some(renderer) = renderer else { return false };
    let binding = match renderer.binding_for(component_id, event_type) {
        Some(b) => b,
        // 兜底：滚动后重算出来的 `component_id` 可能与聚焦那一刻不同（id 含坐标）。
        // 此时若全页只有唯一一条该类绑定，仍按它派发；有多条则宁可不发，避免串台。
        // 只认输入框自己的绑定：`change` 这类名字别的组件也有（picker 的 bindchange），没写 bindchange 的输入框
        // 失焦时退到「全页唯一一条 change」就派给了 picker（train 的乘车人页：手机号写进了票种下标）
        None => {
            let mut it = renderer
                .get_event_bindings()
                .iter()
                .filter(|b| b.event_type == event_type && matches!(b.tag.as_str(), "input" | "textarea"));
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
