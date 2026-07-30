//! 鼠标事件处理

use crate::ui::interaction::{InteractionManager, InteractionResult, InteractionType};
use crate::renderer::WxmlRenderer;
use crate::runtime::MiniApp;
use crate::ui::scroll_controller::ScrollController;
use super::tabbar::tabbar_height;

pub const LOGICAL_HEIGHT: u32 = 667;

/// 鼠标按下事件处理
pub fn handle_mouse_pressed(
    x: f32,
    y: f32,
    scroll: &mut ScrollController,
    interaction: &mut InteractionManager,
    timestamp: u64,
) -> bool {
    let mouse_pos = (x, y);
    // 考虑滚动偏移
    let actual_y = y + scroll.get_position();
    
    // 首先检查固定元素（使用原始坐标，且只看覆盖层自己的元素）
    if let Some(element) = interaction.hit_test_fixed(x, y) {
        let element = element.clone();
        if !element.disabled && element.interaction_type != InteractionType::ScrollArea {
            interaction.set_button_pressed(element.id.clone(), element.bounds);
        }
        match element.interaction_type {
            InteractionType::Switch | InteractionType::Checkbox | InteractionType::Radio => {
                if !element.disabled {
                    if let Some(_result) = interaction.handle_click_scoped(x, y, true) {
                        return true;
                    }
                }
            }
            _ => {}
        }
        return true; // Fixed element consumed click
    }

    // 然后检查普通元素（使用滚动后的坐标）
    if let Some(element) = interaction.hit_test_flow(x, actual_y) {
        let element = element.clone();
        // 任何可点元素都进入按压态（`:active` / `hover-class` 靠它生效），
        // 滚动区域除外 —— 那是拖动，不是按压。
        if !element.disabled && element.interaction_type != InteractionType::ScrollArea {
            interaction.set_button_pressed(element.id.clone(), element.bounds);
        }
        
        match element.interaction_type {
            InteractionType::Slider => {
                if !element.disabled {
                    if let Some(_result) = interaction.handle_click(x, actual_y) {
                        return true;
                    }
                }
                return true;
            }
            InteractionType::Button => {
                if !element.disabled {
                    interaction.set_button_pressed(element.id.clone(), element.bounds);
                    return true;
                }
            }
            InteractionType::ScrollArea => {
                if !element.is_fixed {
                    if let Some(controller) = interaction.get_scroll_controller_mut(&element.id) {
                        // 根据滚动方向使用 x 或 y
                        let drag_pos = if element.is_horizontal { x } else { y };
                        controller.begin_drag(drag_pos, timestamp);
                        interaction.dragging_scroll_area = Some(element.id.clone());
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    
    // 如果不是在拖动滑块或 ScrollArea，才开始滚动拖动
    if !interaction.is_dragging_slider() && interaction.dragging_scroll_area.is_none() {
        scroll.begin_drag(mouse_pos.1, timestamp);
    }
    
    false
}

/// 处理内容区域点击
#[allow(clippy::too_many_arguments)]
pub fn handle_content_click(
    x: f32,
    y: f32,
    scroll_pos: f32,
    has_tabbar: bool,
    interaction: &mut InteractionManager,
    renderer: Option<&WxmlRenderer>,
    app: &mut MiniApp,
    scale_factor: f64,
    text_renderer: Option<&crate::text::TextRenderer>,
    // tap_ctx: 本次 tap 所属触点的标识与页面时间戳（写进事件对象）
    tap_ctx: (u32, u64),
) -> Option<InteractionResult> {
    let actual_y = y + scroll_pos;
    let tabbar_y = if has_tabbar { (LOGICAL_HEIGHT - tabbar_height()) as f32 } else { LOGICAL_HEIGHT as f32 };
    
    // ── 覆盖层优先，且到此为止 ──
    // `position: fixed` 层画在页面之上，坐标是视口坐标。点击必须先只在它里面找：
    // 命中了就派发并返回；即使没命中任何处理器，只要落在覆盖层的范围内
    // （典型就是弹窗的半透明遮罩）也要把点击吞掉，不能穿透到下层页面。
    //
    // 从前是「拿全局 hit_test 的结果，再用 bounds 是否落在视口内来猜它是不是 fixed」——
    // 页面顶部的普通元素同样满足这个条件，于是弹窗弹着也能点到底下的商品。
    let mut on_fixed_with_handler = false;
    if let Some(renderer) = renderer {
        if renderer.fixed_layer_hit(x, y) {
            on_fixed_with_handler = !renderer.dispatch_chain(x, y, "tap", Some(true)).is_empty();
            if !on_fixed_with_handler {
                // 落在覆盖层上但这一点没有处理器：消费掉，不往下透
                return interaction.handle_click_scoped(x, y, true);
            }
        }
    }
    let _ = tabbar_y;
    
    if on_fixed_with_handler {
        // 检查交互元素（视口坐标，且只看覆盖层自己的元素）
        let result = interaction.handle_click_scoped(x, y, true);
        if let Some(renderer) = renderer {
            dispatch_component_events(app, renderer, interaction, x, y, Some(true), tap_ctx, result.as_ref());
        }
        return result;
    }
    
    // 检查是否点击在 scroll-view 内部，如果是，需要调整坐标
    let mut adjusted_y = actual_y;
    if let Some(element) = interaction.hit_test(x, actual_y) {
        if element.interaction_type == InteractionType::ScrollArea {
            // 点击在 scroll-view 上，需要加上 scroll-view 的滚动偏移
            if let Some(controller) = interaction.get_scroll_controller(&element.id) {
                let scroll_offset = controller.get_position();
                // 计算相对于 scroll-view 内部的坐标
                adjusted_y = actual_y + scroll_offset;
            }
        }
    }
    
    // 使用交互管理器处理点击
    if let Some(result) = interaction.handle_click(x, adjusted_y) {
        // 处理输入框光标位置
        if let InteractionResult::Focus { click_x, .. } = &result {
            if let Some(focused) = &interaction.focused_input {
                if let Some(tr) = text_renderer {
                    // click_x 是逻辑坐标（相对于输入框左边缘）
                    // 需要转换为物理坐标来匹配 measure_text 的结果
                    let sf = scale_factor as f32;
                    let font_size = 16.0 * sf;
                    let padding_left = 12.0 * sf;
                    let click_x_physical = *click_x * sf;
                    let text_offset = focused.text_offset;
                    
                    let mut char_widths = Vec::new();
                    for c in focused.value.chars() {
                        let char_str = c.to_string();
                        let width = tr.measure_text(&char_str, font_size);
                        char_widths.push(width);
                    }
                    
                    use crate::ui::interaction::calculate_cursor_position;
                    let cursor_pos = calculate_cursor_position(&focused.value, &char_widths, click_x_physical, padding_left, text_offset);
                    
                    if let Some(input) = &mut interaction.focused_input {
                        input.cursor_pos = cursor_pos;
                    }
                }
            }
            
            // 设置输入框的 maxlength 和 input_type
            if let Some(renderer) = renderer {
                if let Some(binding) = renderer.hit_test(x, adjusted_y) {
                    if let Some(input) = &mut interaction.focused_input {
                        // 从 binding.data 或渲染节点获取属性
                        if let Some(maxlength_str) = binding.data.get("maxlength") {
                            if let Ok(maxlength) = maxlength_str.parse::<i32>() {
                                input.maxlength = maxlength;
                            }
                        }
                        if let Some(input_type) = binding.data.get("type") {
                            input.input_type = input_type.clone();
                        }
                    }
                }
            }
        }
        
        if let Some(renderer) = renderer {
            dispatch_component_events(app, renderer, interaction, x, adjusted_y, Some(false), tap_ctx, Some(&result));
        }
        
        return Some(result);
    } else {
        // 点击了非交互区域，让输入框失去焦点
        if interaction.has_focused_input() {
            return interaction.blur_input();
        }
    }
    
    // 其余情况：按微信语义的捕获→冒泡链派发 tap
    if let Some(renderer) = renderer {
        dispatch_tap_chain(app, renderer, x, adjusted_y, Some(false), tap_ctx);
    }
    
    None
}

/// 命中了一个有状态的组件（开关/勾选/按钮/输入框）之后，把该发的事件发出去。
///
/// 从前这里是「`renderer.hit_test` 取该点最上层的**任意一条**绑定，
/// 用 `__callPageMethod(handler, dataset)` 调一下」，三处硬伤：
///   - 不分事件类型：点一下普通输入框会拿它的 `bindinput` 当 tap 调用，
///     `e.detail.value` 是 undefined，页面里 `.trim()` 直接抛异常；
///   - 不冒泡、事件对象是 dataset 拼的（没有 touches、target/currentTarget 不分）；
///   - 不带组件归属：组件模板里的绑定会打到页面的同名方法上。
///
/// 现在按微信语义分开发：`change`（勾选/开关/单选）走 change 链，
/// tap 走统一的 tap 链，两者都可能同时发生（把开关包在带 bindtap 的 view 里就是）。
/// 输入框的 focus/input/blur/confirm 不在这里 —— 由 `interaction_handler` 负责。
#[allow(clippy::too_many_arguments)]
fn dispatch_component_events(
    app: &mut MiniApp,
    renderer: &WxmlRenderer,
    interaction: &InteractionManager,
    x: f32,
    hit_y: f32,
    scope: Option<bool>,
    tap_ctx: (u32, u64),
    result: Option<&InteractionResult>,
) {
    let (identifier, time_ms) = tap_ctx;
    let dispatch_change = |app: &mut MiniApp, detail: serde_json::Value| {
        super::touch::dispatch_to_js(
            app, renderer, "change", (x, hit_y), (x, hit_y), scope, identifier, time_ms, detail,
        );
    };
    match result {
        Some(InteractionResult::Toggle { checked, .. }) => {
            // 在 `checkbox-group` 里：detail.value 是组内选中项的 value 数组（微信语义）；
            // 单独的 `<switch>`：detail.value 是布尔开关状态。
            let detail = match group_values(renderer, interaction, x, hit_y, scope, "checkbox-group") {
                Some(values) => serde_json::json!({ "value": values }),
                None => serde_json::json!({ "value": checked }),
            };
            dispatch_change(app, detail);
            dispatch_tap_chain(app, renderer, x, hit_y, scope, tap_ctx);
        }
        Some(InteractionResult::Select { value, .. }) => {
            dispatch_change(app, serde_json::json!({ "value": value }));
            dispatch_tap_chain(app, renderer, x, hit_y, scope, tap_ctx);
        }
        Some(InteractionResult::SliderChange { value, .. }) => {
            dispatch_change(app, serde_json::json!({ "value": value }));
        }
        // 输入框的焦点事件由 interaction_handler 派发（它才有输入框的当前文本）
        Some(InteractionResult::Focus { .. }) => {}
        // 按钮、以及「点在没有状态的地方」：都是一次普通 tap
        _ => dispatch_tap_chain(app, renderer, x, hit_y, scope, tap_ctx),
    }
}

/// 命中点所在的 `checkbox-group` / `radio-group` 里，当前选中项的 value 集合。
/// 不在这类组里返回 `None`（那就是单独的 switch/checkbox，走布尔语义）。
fn group_values(
    renderer: &WxmlRenderer,
    interaction: &InteractionManager,
    x: f32,
    hit_y: f32,
    scope: Option<bool>,
    group_tag: &str,
) -> Option<Vec<String>> {
    let group = renderer
        .dispatch_chain(x, hit_y, "change", scope)
        .into_iter()
        .find(|b| b.tag == group_tag)?;
    let kind = if group_tag == "checkbox-group" {
        InteractionType::Checkbox
    } else {
        InteractionType::Radio
    };
    Some(interaction.checked_values_in(&group.bounds, kind))
}

/// 把一次 tap 交给统一的事件派发出口（捕获→冒泡、catch、mut-bind、完整事件对象）。
/// `hit_y` 是对应坐标系下的纵坐标（覆盖层用视口坐标，正常流含滚动偏移）。
fn dispatch_tap_chain(
    app: &mut MiniApp,
    renderer: &WxmlRenderer,
    x: f32,
    hit_y: f32,
    scope: Option<bool>,
    tap_ctx: (u32, u64),
) {
    let (identifier, time_ms) = tap_ctx;
    // tap 的 detail 是点击点坐标（微信语义）
    let detail = serde_json::json!({ "x": x, "y": hit_y });
    let hit = super::touch::dispatch_to_js(
        app, renderer, "tap", (x, hit_y), (x, hit_y), scope, identifier, time_ms, detail,
    );
    if hit {
        println!("👆 tap");
    }
}

pub fn dispatch_input_event(
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
