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
            // slider 按下不做任何事：拖动交给手势仲裁（横向锁定才归它），点一下走 tap
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
                // 落在覆盖层上但这一点没有处理器：消费掉，不往下透（弹窗里的 form-type 按钮照样提交）
                let result = interaction.handle_click_scoped(x, y, true);
                if let Some(r) = &result {
                    prepare_focused_input(interaction, Some(renderer), text_renderer, scale_factor, x, y, r);
                    // 拨了覆盖层里的开关 / 单选 / 勾选：它们的 change 照发（这里只是「没有 tap 处理器」）。
                    // 从前这条分支直接 return —— fixed 底栏里的 radio-group 点了不触发 change（阅读器主题切换）
                    dispatch_component_events(app, renderer, interaction, x, y, Some(true), tap_ctx, Some(r));
                }
                // 表单提交 / navigator 跳转 / label 转发（label 转发出来的结果当作这次点击的结果）
                if let Some(r) = super::builtin_tap::after_click(app, renderer, interaction, result.as_ref(), x, y, Some(true), tap_ctx) {
                    prepare_focused_input(interaction, Some(renderer), text_renderer, scale_factor, x, y, &r);
                    return Some(r);
                }
                if result.is_none() {
                    super::video::on_tap(renderer, interaction, x, y, true);
                    // 同层渲染的 `<map>` 这类：点击交给宿主（nativeViewTap），它判点到了什么
                    super::engine_host_views::on_tap(renderer, interaction, x, y, true);
                }
                return result;
            }
        }
    }
    let _ = tabbar_y;
    
    if on_fixed_with_handler {
        // 检查交互元素（视口坐标，且只看覆盖层自己的元素）
        let result = interaction.handle_click_scoped(x, y, true);
        if let Some(r) = &result {
            prepare_focused_input(interaction, renderer, text_renderer, scale_factor, x, y, r);
        }
        if let Some(renderer) = renderer {
            dispatch_component_events(app, renderer, interaction, x, y, Some(true), tap_ctx, result.as_ref());
            if let Some(r) = super::builtin_tap::after_click(app, renderer, interaction, result.as_ref(), x, y, Some(true), tap_ctx) {
                prepare_focused_input(interaction, Some(renderer), text_renderer, scale_factor, x, y, &r);
                return Some(r);
            }
        }
        return result;
    }
    
    // scroll-view 里的子元素登记时已经减过它自己的滚动量（draw_interactive.rs 的
    // `child_offset_y`、scroll_live.rs 同理），命中只用页面内容坐标。
    // 从前这里又加了一次 scroll-view 的偏移：一滚过（scroll-into-view / 手指拖），
    // 点到的就是「两倍偏移」处的那一项 —— 往往在视口外，表现成整片列表点不动
    // （bank 收款人、restaurant 菜品）；横向的还把 x 偏移加到了 y 上
    let adjusted_y = actual_y;
    
    // 使用交互管理器处理点击
    if let Some(result) = interaction.handle_click(x, adjusted_y) {
        prepare_focused_input(interaction, renderer, text_renderer, scale_factor, x, adjusted_y, &result);
        
        if let Some(renderer) = renderer {
            dispatch_component_events(app, renderer, interaction, x, adjusted_y, Some(false), tap_ctx, Some(&result));
            // 按钮的 bindtap 先发，再提交 / 重置所在表单
            super::builtin_tap::after_click(app, renderer, interaction, Some(&result), x, adjusted_y, Some(false), tap_ctx);
        }
        
        return Some(result);
    } else {
        // 点击了非交互区域：输入框失焦，**而且这次点击照常派发**（与微信一致：先 blur，再 tap）。
        // 从前失焦之后直接 return，点的那个 bindtap 吞掉了 —— 输入框聚焦着点旁边的「×」清空，
        // 第一下只收了键盘、没清（news-app 搜索页，真机上反馈「点 × 清不掉」）
        if let Some(InteractionResult::InputBlur { id, value, .. }) = interaction.blur_input() {
            dispatch_input_event(app, renderer, &id, "blur", &value);
            if let Some(renderer) = renderer {
                dispatch_tap_chain(app, renderer, x, adjusted_y, Some(false), tap_ctx);
            }
            return Some(InteractionResult::InputBlur { id, value, dispatched: true });
        }
    }
    
    // 其余情况：按微信语义的捕获→冒泡链派发 tap
    if let Some(renderer) = renderer {
        dispatch_tap_chain(app, renderer, x, adjusted_y, Some(false), tap_ctx);
        if let Some(r) = super::builtin_tap::after_click(app, renderer, interaction, None, x, adjusted_y, Some(false), tap_ctx) {
            prepare_focused_input(interaction, Some(renderer), text_renderer, scale_factor, x, adjusted_y, &r);
            return Some(r);
        }
        // 点在 <video> 上：控制条的按钮 / 中间的播放键 / 显隐切换（事件下一帧由 host::video::tick 发）
        super::video::on_tap(renderer, interaction, x, adjusted_y, false);
        // 同层渲染的 `<map>` 这类：点击交给宿主（nativeViewTap），它判点到了什么（标记 → markertap）
        super::engine_host_views::on_tap(renderer, interaction, x, adjusted_y, false);
    }
    
    None
}

/// 刚聚焦的输入框：按渲染器登记的属性补齐编辑参数，并把光标放到点中的位置。
///
/// `hit_y` 是交给 `handle_click` 的那个纵坐标（与 `Focus.bounds` 同一坐标系）。
/// 正常流与覆盖层两条路径都要走这里 —— 从前只有正常流做了，弹窗里的输入框
/// maxlength / type 全不生效、光标也总在末尾。
#[allow(clippy::too_many_arguments)]
fn prepare_focused_input(
    interaction: &mut InteractionManager,
    renderer: Option<&WxmlRenderer>,
    text_renderer: Option<&crate::text::TextRenderer>,
    scale_factor: f64,
    x: f32,
    hit_y: f32,
    result: &InteractionResult,
) {
    let InteractionResult::Focus { id, bounds, click_x, .. } = result else { return };
    let sf = scale_factor as f32;
    let region = renderer.and_then(|r| r.input_region(id));
    // 兜底：老路径是从命中的事件绑定上带的 `data` 里取 maxlength / type
    let binding = renderer.and_then(|r| r.hit_test(x, hit_y));
    let attr = |k: &str| -> Option<String> {
        region
            .and_then(|r| r.attrs.get(k))
            .or_else(|| binding.and_then(|b| b.data.get(k)))
            .cloned()
    };
    let truthy = |k: &str| attr(k).map(|v| crate::renderer::components::attr_is_true(&v)).unwrap_or(false);
    let layout = interaction.text_layouts.get(id.as_str()).cloned();
    let Some(input) = interaction.focused_input.as_mut() else { return };
    if let Some(r) = region {
        input.multiline = r.tag == "textarea";
    }
    if let Some(ml) = attr("maxlength").and_then(|s| s.parse::<i32>().ok()) {
        input.maxlength = ml;
    }
    input.is_password = crate::renderer::components::is_password_input(attr("password").as_deref(), attr("type").as_deref());
    if let Some(t) = attr("type") {
        input.input_type = t;
    }
    // textarea 的右下角默认是「换行」（`return`），input 默认是「完成」
    input.confirm_type = attr("confirm-type")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| if input.multiline { "return" } else { "done" }.to_string());
    input.confirm_hold = truthy("confirm-hold");

    // 按上一帧画出来的排版定位（多行的折行、单行的 text-align / 字号 / 内边距 / 横向滚动都在里面）。
    // 画的是占位符、或者值已经变了就不能用：多行留在末尾，单行退回下面按默认字号估
    if let Some(l) = layout.filter(|l| l.text == input.value && !input.is_password) {
        let (pos, up) = l.hit(*click_x * sf, (hit_y - bounds.y) * sf);
        input.cursor_pos = pos;
        input.caret_upstream = up && input.multiline;
        return;
    }
    if input.multiline {
        return;
    }
    if let Some(tr) = text_renderer {
        // click_x 是逻辑坐标（相对于输入框左边缘）
        // 需要转换为物理坐标来匹配 measure_text 的结果
        let font_size = 16.0 * sf;
        let padding_left = 12.0 * sf;
        // 密码框画的是圆点：按圆点的宽度找光标，否则点在第 3 个点上光标会落到别处
        let char_widths: Vec<f32> = input
            .value
            .chars()
            .map(|c| tr.measure_text(&(if input.is_password { '•' } else { c }).to_string(), font_size))
            .collect();
        use crate::ui::interaction::calculate_cursor_position;
        input.cursor_pos = calculate_cursor_position(&input.value, &char_widths, *click_x * sf, padding_left, input.text_offset);
    }
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
pub(super) fn dispatch_component_events(
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
    // `change` 是组件自己的事件，**不冒泡**（微信 / Skyline）：只交给发出它的那个组件 ——
    // 开关 / 勾选框 / 滑块自己，或包着勾选框 / 单选框的 `checkbox-group` / `radio-group`。
    // 从前按 tap 那样走整条冒泡链：卡片里的开关一拨，外层 swiper 的 `bindchange` 也收到
    // `{value: true}`（智慧家首页的房间 swiper 被当成翻页）
    let dispatch_change = |app: &mut MiniApp, detail: serde_json::Value, owners: &[&str]| {
        let chain = renderer.dispatch_chain(x, hit_y, "change", scope);
        let own = chain
            .iter()
            .find(|b| b.phase == crate::renderer::components::EventPhase::Bubble && owners.contains(&b.tag.as_str()))
            .or_else(|| chain.iter().rev().find(|b| owners.contains(&b.tag.as_str())))
            .cloned();
        if let Some(b) = own {
            super::touch::dispatch_chain_to_js(app, std::slice::from_ref(&b), Some(b.clone()), "change", (x, hit_y), identifier, time_ms, detail);
        }
    };
    match result {
        Some(InteractionResult::Toggle { checked, .. }) => {
            // 在 `checkbox-group` 里：detail.value 是组内选中项的 value 数组（微信语义）；
            // 单独的 `<switch>`：detail.value 是布尔开关状态。
            match group_values(renderer, interaction, x, hit_y, scope, "checkbox-group") {
                Some(values) => dispatch_change(app, serde_json::json!({ "value": values }), &["checkbox-group"]),
                None => dispatch_change(app, serde_json::json!({ "value": checked }), &["switch", "checkbox"]),
            }
            dispatch_tap_chain(app, renderer, x, hit_y, scope, tap_ctx);
        }
        Some(InteractionResult::Select { value, .. }) => {
            dispatch_change(app, serde_json::json!({ "value": value }), &["radio-group", "radio"]);
            dispatch_tap_chain(app, renderer, x, hit_y, scope, tap_ctx);
        }
        // 点按 slider：跳到手指处发一次 change（detail.value 是 number），再照常冒泡 tap
        Some(InteractionResult::SliderChange { value, .. }) => {
            dispatch_change(app, serde_json::json!({ "value": super::form::number(*value) }), &["slider"]);
            dispatch_tap_chain(app, renderer, x, hit_y, scope, tap_ctx);
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

/// `bindinput` 的处理函数返回了一个字符串：用它替换输入框的内容（官方 component/input：
/// 「处理函数可以直接 return 一个字符串，将替换输入框的内容」）。在派发完 `input` 之后立刻取
pub fn take_input_replacement(app: &MiniApp) -> Option<String> {
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
