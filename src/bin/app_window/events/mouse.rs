//! 鼠标事件处理

use mini_render::ui::interaction::{InteractionManager, InteractionResult, InteractionType};
use mini_render::renderer::WxmlRenderer;
use mini_render::runtime::MiniApp;
use mini_render::ui::scroll_controller::ScrollController;
use super::super::tabbar::tabbar_height;

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
    
    // 首先检查固定元素（使用原始坐标）
    if let Some(element) = interaction.hit_test(x, y) {
        let element = element.clone();
        if element.is_fixed {
            match element.interaction_type {
                InteractionType::Button => {
                    if !element.disabled {
                        interaction.set_button_pressed(element.id.clone(), element.bounds);
                        return true;
                    }
                }
                InteractionType::Switch | InteractionType::Checkbox | InteractionType::Radio => {
                    if !element.disabled {
                        if let Some(_result) = interaction.handle_click(x, y) { // Fixed elements use screen coords
                            return true;
                        }
                    }
                }
                _ => {}
            }
            return true; // Fixed element consumed click
        }
    }

    // 然后检查普通元素（使用滚动后的坐标）
    if let Some(element) = interaction.hit_test(x, actual_y) {
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

/// 鼠标释放事件处理
pub fn handle_mouse_released(
    scroll: &mut ScrollController,
    interaction: &mut InteractionManager,
) -> bool {
    // 清除按钮按下状态
    interaction.clear_button_pressed();
    
    // 结束滑块拖动
    if let Some(_result) = interaction.handle_mouse_release() {
        // 结果会在外部处理
    }
    
    // 结束 ScrollArea 拖动
    if let Some(id) = &interaction.dragging_scroll_area.clone() {
        if let Some(controller) = interaction.get_scroll_controller_mut(id) {
            controller.end_drag();
        }
        interaction.dragging_scroll_area = None;
        return true; // 触发重绘
    }
    
    scroll.end_drag()
}

/// 处理内容区域点击
pub fn handle_content_click(
    x: f32,
    y: f32,
    scroll_pos: f32,
    has_tabbar: bool,
    interaction: &mut InteractionManager,
    renderer: Option<&WxmlRenderer>,
    app: &mut MiniApp,
    scale_factor: f64,
    text_renderer: Option<&mini_render::text::TextRenderer>,
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
    let mut fixed_binding = None;
    if let Some(renderer) = renderer {
        if renderer.fixed_layer_hit(x, y) {
            let chain = renderer.hit_test_bubble_fixed(x, y, "tap");
            if let Some(b) = chain.first() {
                fixed_binding = Some((b.event_type.clone(), b.handler.clone(), b.data.clone(), b.bounds));
            } else {
                // 落在覆盖层上但这一点没有处理器：消费掉，不往下透
                return interaction.handle_click_scoped(x, y, true);
            }
        }
    }
    let _ = tabbar_y;
    
    if let Some((event_type, handler, data, _bounds)) = fixed_binding {
        // 检查交互元素（视口坐标，且只看覆盖层自己的元素）
        if let Some(result) = interaction.handle_click_scoped(x, y, true) {
            let should_call_js = matches!(&result, 
                InteractionResult::ButtonClick { .. } |
                InteractionResult::Toggle { .. } |
                InteractionResult::Select { .. }
            );
            
            if should_call_js {
                println!("👆 {} -> {}", event_type, handler);
                let data_json = serde_json::to_string(&data).unwrap_or("{}".to_string());
                let call_code = format!("__callPageMethod('{}', {})", handler, data_json);
                app.eval(&call_code).ok();
            }
            
            return Some(result);
        }
        
        // 如果没有交互元素，直接调用事件处理
        println!("👆 {} -> {}", event_type, handler);
        let data_json = serde_json::to_string(&data).unwrap_or("{}".to_string());
        let call_code = format!("__callPageMethod('{}', {})", handler, data_json);
        app.eval(&call_code).ok();
        return None;
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
                    
                    use mini_render::ui::interaction::calculate_cursor_position;
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
        
        let should_call_js = matches!(&result,
            InteractionResult::ButtonClick { .. } |
            InteractionResult::Toggle { .. } |
            InteractionResult::Select { .. } |
            InteractionResult::Focus { .. }
        );
        
        if should_call_js {
            if let Some(renderer) = renderer {
                if let Some(binding) = renderer.hit_test(x, adjusted_y) {
                    println!("👆 {} -> {}", binding.event_type, binding.handler);
                    let data_json = serde_json::to_string(&binding.data).unwrap_or("{}".to_string());
                    let call_code = format!("__callPageMethod('{}', {})", binding.handler, data_json);
                    app.eval(&call_code).ok();
                }
            }
        }
        
        return Some(result);
    } else {
        // 点击了非交互区域，让输入框失去焦点
        if interaction.has_focused_input() {
            return interaction.blur_input();
        }
    }
    
    // 检查其他事件绑定 —— 使用冒泡链分发（bindtap 冒泡，catchtap 阻止）
    if let Some(renderer) = renderer {
        let chain = renderer.hit_test_bubble(x, adjusted_y, "tap");
        for binding in &chain {
            println!("👆 {} -> {} (bubble)", binding.event_type, binding.handler);
            let data_json = serde_json::to_string(&binding.data).unwrap_or("{}".to_string());
            let call_code = format!("__callPageMethod('{}', {})", binding.handler, data_json);
            app.eval(&call_code).ok();
        }
    }
    
    None
}
