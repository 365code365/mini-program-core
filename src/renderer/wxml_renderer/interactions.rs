//! 可交互元素的登记。
//!
//! 与事件绑定表不同，这里登记的是**有内部状态**的元素（勾选框、开关、滑块、输入框、
//! 滚动区、按钮、以及带按压态的普通 view）：宿主的 `InteractionManager` 拿它们
//! 做命中、维护选中/焦点/滚动位置，并在下一帧回读。
//!
//! 每帧重建（滚动会改变几何），但覆盖层部分保留 —— 覆盖层可能整帧不重绘。

use super::*;

impl WxmlRenderer {
    /// 注册 scroll-view 子元素的交互区域
    pub(super) fn register_child_interactions(
        &mut self,
        taffy: &Tree,
        node: &RenderNode,
        ox: f32,
        oy: f32,
        inherited_color: Color,
        interaction: &mut InteractionManager,
        scroll_position: f32,
        viewport_height: f32,
    ) {
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        
        // 检查是否在可见区域内
        let logical_y = y / sf;
        let logical_h = h / sf;
        let viewport_top = scroll_position;
        let viewport_bottom = scroll_position + viewport_height;
        
        // 只注册可见区域内的元素
        if logical_y + logical_h < viewport_top || logical_y > viewport_bottom {
            return;
        }
        
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);
        let text_color = node.style.text_color.unwrap_or(inherited_color);
        
        // 注册交互元素
        self.register_interactive_element(node, node, &logical_bounds, interaction, taffy, false);
        
        // 递归注册子元素
        if Self::draws_children(node) {
            for child in &node.children {
                self.register_child_interactions(taffy, child, x, y, text_color, interaction, scroll_position, viewport_height);
            }
        }
        
        // 记录事件绑定
        for e in &node.events {
            self.event_bindings.push(EventBinding {
                event_type: e.event_type.clone(),
                handler: e.handler.clone(),
                id: node.attrs.get("id").cloned().unwrap_or_default(),
                data: e.data.clone(),
                bounds: logical_bounds,
                is_catch: e.is_catch,
                phase: e.phase,
                mut_bind: e.mut_bind,
                is_fixed: self.registering_fixed,
            });
        }
    }
    
    pub(super) fn register_interactive_element(
        &mut self, 
        original_node: &RenderNode, 
        drawn_node: &RenderNode,
        bounds: &GeoRect, 
        interaction: &mut InteractionManager,
        taffy: &Tree,
        is_in_fixed_container: bool
    ) {
        let disabled = original_node.attrs.get("disabled")
            .map(|s| s == "true" || s == "{{true}}")
            .unwrap_or(false);
        
        let id = Self::get_component_id(original_node, bounds);
        // 覆盖层子树里的元素统统算 fixed：坐标是视口坐标，而且它们在页面之上。
        // 只看 `style.is_fixed` 的话只有子树根算 fixed，里面的按钮会被当成正常流元素。
        let is_fixed = is_in_fixed_container || original_node.style.is_fixed || self.registering_fixed;

        // picker 的「点一下弹出面板」由宿主接管：这里只登记它的位置与选项。
        // 它同时也走下面 `_ =>` 的按压态登记（微信里 picker 也吃 `hover-class`）。
        if original_node.tag == "picker" {
            let binding = Self::build_picker_binding(original_node, bounds, &id, disabled, is_fixed);
            self.picker_regions.push(binding);
        }
        
        match original_node.tag.as_str() {
            "scroll-view" => {
                // 检查是否是横向滚动
                let scroll_x = original_node.attrs.get("scroll-x")
                    .map(|s| s == "true" || s == "{{true}}")
                    .unwrap_or(false);
                
                let mut content_height = 0.0;
                let mut content_width = 0.0;
                
                // 计算内容尺寸：所有子节点的边界最大值
                for child in original_node.children.iter() {
                    if let Ok(layout) = taffy.layout(child.taffy_node) {
                        let bottom = layout.location.y + layout.size.height;
                        let right = layout.location.x + layout.size.width;
                        if bottom > content_height {
                            content_height = bottom;
                        }
                        if right > content_width {
                            content_width = right;
                        }
                    }
                }
                
                // 转换为逻辑像素
                let logical_content_height = content_height / self.scale_factor;
                let logical_content_width = content_width / self.scale_factor;
                
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::ScrollArea,
                    id,
                    bounds: *bounds,
                    checked: false,
                    value: String::new(),
                    disabled,
                    min: 0.0,
                    max: 0.0,
                    content_height: logical_content_height,
                    viewport_height: bounds.height,
                    content_width: logical_content_width,
                    viewport_width: bounds.width,
                    is_horizontal: scroll_x,
                    is_fixed,
                });
            }
            "checkbox" => {
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Checkbox,
                    id,
                    bounds: *bounds,
                    checked: drawn_node.style.custom_data > 0.5,
                    value: Self::input_value_attr(original_node),
                    disabled,
                    min: 0.0,
                    max: 1.0,
                    content_height: 0.0,
                    viewport_height: 0.0,
                    content_width: 0.0,
                    viewport_width: 0.0,
                    is_horizontal: false,
                    is_fixed,
                });
            }
            "radio" => {
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Radio,
                    id,
                    bounds: *bounds,
                    checked: drawn_node.style.custom_data > 0.5,
                    value: Self::input_value_attr(original_node),
                    disabled,
                    min: 0.0,
                    max: 1.0,
                    content_height: 0.0,
                    viewport_height: 0.0,
                    content_width: 0.0,
                    viewport_width: 0.0,
                    is_horizontal: false,
                    is_fixed,
                });
            }
            "switch" => {
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Switch,
                    id,
                    bounds: *bounds,
                    checked: drawn_node.style.custom_data > 0.5,
                    value: original_node.text.clone(),
                    disabled,
                    min: 0.0,
                    max: 1.0,
                    content_height: 0.0,
                    viewport_height: 0.0,
                    content_width: 0.0,
                    viewport_width: 0.0,
                    is_horizontal: false,
                    is_fixed,
                });
            }
            "slider" => {
                let min = original_node.attrs.get("min").and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let max = original_node.attrs.get("max").and_then(|s| s.parse().ok()).unwrap_or(100.0);
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Slider,
                    id,
                    bounds: *bounds,
                    checked: false,
                    value: format!("{}", (drawn_node.style.custom_data * 100.0) as i32),
                    disabled,
                    min,
                    max,
                    content_height: 0.0,
                    viewport_height: 0.0,
                    content_width: 0.0,
                    viewport_width: 0.0,
                    is_horizontal: false,
                    is_fixed,
                });
            }
            "input" | "textarea" => {
                // 只使用原始 value 属性，不使用 placeholder
                let actual_value = Self::input_value_attr(original_node);
                // 如果已有状态，使用状态中的值
                let current_value = interaction.get_state(&id)
                    .map(|s| s.value.clone())
                    .unwrap_or(actual_value);
                
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Input,
                    id,
                    bounds: *bounds,
                    checked: false,
                    value: current_value,
                    disabled,
                    min: 0.0,
                    max: 0.0,
                    content_height: 0.0,
                    viewport_height: 0.0,
                    content_width: 0.0,
                    viewport_width: 0.0,
                    is_horizontal: false,
                    is_fixed,
                });
            }
            "button" => {
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Button,
                    id,
                    bounds: *bounds,
                    checked: false,
                    value: original_node.text.clone(),
                    disabled,
                    min: 0.0,
                    max: 0.0,
                    content_height: 0.0,
                    viewport_height: 0.0,
                    content_width: 0.0,
                    viewport_width: 0.0,
                    is_horizontal: false,
                    is_fixed,
                });
            }
            "picker" => {
                // 有按压态时也要登记（与下面普通元素同理）
                if original_node.style.pressed_style.is_some() {
                    interaction.register_element(InteractiveElement {
                        interaction_type: InteractionType::View,
                        id,
                        bounds: *bounds,
                        checked: false,
                        value: String::new(),
                        disabled,
                        min: 0.0,
                        max: 0.0,
                        content_height: 0.0,
                        viewport_height: 0.0,
                        content_width: 0.0,
                        viewport_width: 0.0,
                        is_horizontal: false,
                        is_fixed,
                    });
                }
            }
            _ => {
                // 普通元素的点击走 event_bindings，本来不必登记为交互元素。
                // 但**有按压态样式**（`:active` 命中或写了 `hover-class`）的必须登记 ——
                // 按压态是按元素 id 记录的，不登记的话宿主永远找不到这个元素，
                // `:active` / `hover-class` 就完全不会生效（微信里它们对任意 view 都生效）。
                if original_node.style.pressed_style.is_some() {
                    interaction.register_element(InteractiveElement {
                        interaction_type: InteractionType::View,
                        id,
                        bounds: *bounds,
                        checked: false,
                        value: String::new(),
                        disabled,
                        min: 0.0,
                        max: 0.0,
                        content_height: 0.0,
                        viewport_height: 0.0,
                        content_width: 0.0,
                        viewport_width: 0.0,
                        is_horizontal: false,
                        is_fixed,
                    });
                }
            }
        }
    }

    
}
