//! 页面主绘制路径：绘制的同时登记交互。
//!
//! 与 `draw` 那条纯绘制路径的区别是这里额外负责：
//! - 把事件绑定与可交互元素登记进本帧的表（供命中测试与宿主回读）
//! - 按压态样式的取用（`:active` / `hover-class`）
//! - `scroll-view` 的子画布与横向/纵向偏移
//! - 带 `transform`/`animation` 的子树转交离屏合成
//!
//! 顶层与子节点各有一份入口：顶层要处理 `position: fixed` 的分层与页面级滚动偏移，
//! 子节点只在父给定的原点里定位，两者的坐标语义不同，因此没有合并成一个函数。

use super::*;

impl WxmlRenderer {
    pub(super) fn draw_with_interaction(
        &mut self, 
        canvas: &mut Canvas, 
        taffy: &Tree, 
        node: &RenderNode, 
        ox: f32, 
        oy: f32,
        interaction: &mut InteractionManager,
        scroll_offset: f32,
        viewport_height: f32,
    ) {
        // 跳过 fixed 元素，它们会单独渲染
        if node.style.is_fixed {
            return;
        }
        // CSS 动画 / transform：由专门路径接管（含离屏仿射合成）
        if self.draw_with_transform(
            canvas,
            taffy,
            node,
            ox,
            oy,
            interaction,
            DrawKind::Top { scroll_offset, viewport_height },
        ) {
            return;
        }
        
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        // 渲染整个内容到 canvas，滚动在 present_to_buffer 中处理
        
        if self.cull_outside_viewport(node, x, y, w, h, scroll_offset, viewport_height) {
            return;
        }
        
        if node.tag == "swiper" {
            self.draw_swiper_container(canvas, taffy, node, x, y, w, h, interaction,
                DrawKind::Top { scroll_offset, viewport_height });
            return;
        }
        
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);
        
        let component_id = Self::get_component_id(node, &logical_bounds);
        
        // 应用交互状态
        let mut node_to_draw = Self::shallow_for_draw(node);
        // 按压态：`:active` / `hover-class` 在建树期已算好整套样式，这里按需切换，
        // 有 `transition` 就在常态与按压态之间插值
        self.apply_pressed_style(&mut node_to_draw, interaction, &component_id);
        let state_checked = interaction.get_state(&component_id).map(|s| s.checked);
        let switch_progress = if node.tag == "switch" {
            state_checked.map(|checked| self.toggle_progress(interaction, &component_id, checked))
        } else {
            None
        };
        if let Some(state) = interaction.get_state(&component_id) {
            match node.tag.as_str() {
                // switch 的配色由组件自己按进度求值（轨道关态是微信的浅灰，不是纯白）
                "switch" => {
                    node_to_draw.style.custom_data = switch_progress.unwrap_or(if state.checked { 1.0 } else { 0.0 });
                }
                "checkbox" => {
                    node_to_draw.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                    // 更新颜色
                    let checkbox_color = node.attrs.get("color")
                        .and_then(|c| crate::renderer::components::parse_color_str(c))
                        .unwrap_or(Color::from_hex(0x09BB07));
                    if state.checked {
                        node_to_draw.style.background_color = Some(checkbox_color);
                        node_to_draw.style.border_color = Some(checkbox_color);
                    } else {
                        node_to_draw.style.background_color = Some(Color::WHITE);
                        node_to_draw.style.border_color = Some(Color::from_hex(0xD1D1D1));
                    }
                }
                "radio" => {
                    node_to_draw.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                    // 更新颜色
                    let radio_color = node.attrs.get("color")
                        .and_then(|c| crate::renderer::components::parse_color_str(c))
                        .unwrap_or(Color::from_hex(0x09BB07));
                    if state.checked {
                        node_to_draw.style.background_color = Some(radio_color);
                        node_to_draw.style.border_color = Some(radio_color);
                    } else {
                        node_to_draw.style.background_color = Some(Color::WHITE);
                        node_to_draw.style.border_color = Some(Color::from_hex(0xD1D1D1));
                    }
                }
                "slider" => {
                    if let Ok(v) = state.value.parse::<f32>() {
                        node_to_draw.style.custom_data = v / 100.0;
                        if !node_to_draw.text.is_empty() {
                            node_to_draw.text = format!("{}", v as i32);
                        }
                    }
                }
                "input" | "textarea" => {
                    // 获取 placeholder
                    let placeholder = node.attrs.get("placeholder").cloned().unwrap_or_default();
                    
                    // 检查是否聚焦
                    let is_focused = interaction.focused_input.as_ref()
                        .map(|f| f.id == component_id)
                        .unwrap_or(false);
                    
                    if state.value.is_empty() && !is_focused {
                        // 没有输入值且未聚焦时显示 placeholder
                        node_to_draw.text = placeholder;
                        node_to_draw.style.text_color = Some(Color::from_hex(0xBFBFBF));
                    } else {
                        // 有输入值或聚焦时显示实际值（聚焦时即使为空也不显示 placeholder）
                        node_to_draw.text = state.value.clone();
                        node_to_draw.style.text_color = Some(Color::BLACK);
                    }
                }
                _ => {}
            }
        } else if matches!(node.tag.as_str(), "input" | "textarea") {
            // 输入框但还没有交互状态，显示 placeholder 或初始值
            let placeholder = node.attrs.get("placeholder").cloned().unwrap_or_default();
            let initial_value = Self::input_value_attr(node);
            
            if initial_value.is_empty() {
                node_to_draw.text = placeholder;
                node_to_draw.style.text_color = Some(Color::from_hex(0xBFBFBF));
            } else {
                node_to_draw.text = initial_value;
            }
        }
        
        // 注册交互元素
        self.register_interactive_element(node, &node_to_draw, &logical_bounds, interaction, taffy, false);

        // 绘制组件 - 特殊处理 input 和 button 组件
        match node.tag.as_str() {
                "input" | "textarea" => {
                    let focused = interaction.focused_input.as_ref()
                        .map(|f| f.id == component_id)
                        .unwrap_or(false);
                    let (cursor_pos, selection) = if focused {
                        let f = interaction.focused_input.as_ref().unwrap();
                        (f.cursor_pos, f.get_selection_range())
                    } else {
                        (0, None)
                    };
                    InputComponent::draw_with_selection(
                        &node_to_draw, canvas, self.text_renderer.as_deref(), 
                        x, y, w, h, sf, focused, cursor_pos, selection
                    );
                    
                    // 更新 text_offset（用于点击位置计算）
                    if focused {
                        if let Some(tr) = self.text_renderer.as_deref() {
                            let font_size = node_to_draw.style.font_size * sf;
                            let padding_left = 12.0 * sf;
                            let padding_right = 12.0 * sf;
                            let available_width = w - padding_left - padding_right;
                            
                            let text_width = tr.measure_text(&node_to_draw.text, font_size);
                            let mut text_offset = 0.0;
                        if text_width > available_width {
                            let cursor_text: String = node_to_draw.text.chars().take(cursor_pos).collect();
                            let cursor_x_in_text = tr.measure_text(&cursor_text, font_size);
                            
                            if cursor_x_in_text > available_width {
                                text_offset = available_width - cursor_x_in_text - font_size;
                            }
                        }
                        
                        if let Some(input) = &mut interaction.focused_input {
                            input.text_offset = text_offset;
                        }
                    }
                }
            }
            "button" => {
                let pressed = interaction.is_button_pressed(&component_id);
                ButtonComponent::draw_with_state(
                    &node_to_draw, canvas, self.text_renderer.as_deref(),
                    x, y, w, h, sf, pressed
                );
            }
            _ => {
                self.draw_component(canvas, &node_to_draw, x, y, w, h, sf);
            }
        }
        
        // 绘制子节点
        if !Self::is_leaf_component(&node.tag) {
            let is_scroll_view = node.tag == "scroll-view";
            let mut child_offset_y = 0.0;
            let scroll_position: f32;
            
            if is_scroll_view {
                // 计算 scroll-view 内容高度
                let mut content_height = 0.0f32;
                for child in &node.children {
                    let child_layout = taffy.layout(child.taffy_node).unwrap();
                    let child_bottom = child_layout.location.y + child_layout.size.height;
                    content_height = content_height.max(child_bottom);
                }
                
                // 获取滚动位置
                scroll_position = if let Some(controller) = interaction.get_scroll_controller(&component_id) {
                    controller.get_position()
                } else {
                    0.0
                };
                
                // 检查缓存是否需要更新
                let cache_needs_render = {
                    let cache = self.scroll_cache.get_or_create(
                        &component_id,
                        w as u32,
                        content_height.ceil() as u32,
                        (w / sf) as u32,
                        (h / sf) as u32,
                    );
                    cache.needs_render()
                };
                
                // 滑动手感的归因（`MINI_SCROLL_LOG=1`）：scroll-view 每帧的开销分三段
                // ——「重画离屏内容」「上屏 blit」「登记交互区域」。哪一段贵，改哪一段。
                let log_scroll = std::env::var("MINI_SCROLL_LOG").is_ok();
                let t_begin = std::time::Instant::now();
                if cache_needs_render {
                    // 创建临时 Canvas 用于渲染
                    let mut temp_canvas = Canvas::new(w as u32, content_height.ceil() as u32);
                    temp_canvas.clear(node.style.background_color.unwrap_or(Color::TRANSPARENT));
                    
                    let text_color = node.style.text_color.unwrap_or(Color::BLACK);
                    
                    // 渲染所有子元素到临时 Canvas
                    for child in &node.children {
                        self.draw_child_to_cache(&mut temp_canvas, taffy, child, 0.0, 0.0, text_color, interaction);
                    }
                    
                    // 将临时 Canvas 的内容复制到缓存
                    if let Some(cache) = self.scroll_cache.get_mut(&component_id) {
                        // 直接替换缓存的 canvas
                        cache.canvas = temp_canvas;
                        cache.mark_clean();
                    }
                }
                
                let t_content = std::time::Instant::now();
                // 从缓存复制可见区域到主 Canvas
                canvas.save();
                canvas.clip_rect(GeoRect::new(x, y, w, h));
                
                if let Some(cache) = self.scroll_cache.get(&component_id) {
                    cache.blit_to(canvas, scroll_position, x, y, sf);
                }
                
                canvas.restore();
                let t_blit = std::time::Instant::now();
                
                // 注册子元素的交互区域（需要考虑滚动偏移）
                child_offset_y = -scroll_position * sf;
                let text_color = node.style.text_color.unwrap_or(Color::BLACK);
                for child in &node.children {
                    self.register_child_interactions(taffy, child, x, y + child_offset_y, text_color, interaction, scroll_position, h / sf);
                }
                if log_scroll {
                    let ms = |a: std::time::Instant, b: std::time::Instant| (b - a).as_secs_f32() * 1000.0;
                    eprintln!(
                        "🎢 scroll-view {} 内容高 {:.0}：离屏重画 {:.2}ms（{}）  上屏 blit {:.2}ms  交互登记 {:.2}ms",
                        component_id, content_height,
                        ms(t_begin, t_content),
                        if cache_needs_render { "本帧重画" } else { "命中缓存" },
                        ms(t_content, t_blit),
                        t_blit.elapsed().as_secs_f32() * 1000.0,
                    );
                }
            } else {
                let text_color = node.style.text_color.unwrap_or(Color::BLACK);
                for child in &node.children { 
                    self.draw_child_with_interaction(canvas, taffy, child, x, y + child_offset_y, text_color, interaction, scroll_offset, viewport_height); 
                }
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
    
    /// 渲染子节点到离屏缓存（不处理交互状态，纯渲染）
    pub(super) fn draw_child_to_cache(
        &self,
        canvas: &mut Canvas,
        taffy: &Tree,
        node: &RenderNode,
        ox: f32,
        oy: f32,
        inherited_color: Color,
        interaction: &InteractionManager,
    ) {
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);
        let component_id = Self::get_component_id(node, &logical_bounds);
        
        let text_color = node.style.text_color.unwrap_or(inherited_color);
        let mut node_to_draw = Self::shallow_for_draw(node);
        if node_to_draw.style.text_color.is_none() {
            node_to_draw.style.text_color = Some(text_color);
        }
        
        // 应用交互状态
        if let Some(state) = interaction.get_state(&component_id) {
            match node.tag.as_str() {
                "checkbox" | "switch" => {
                    node_to_draw.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                    let checkbox_color = node.attrs.get("color")
                        .and_then(|c| crate::renderer::components::parse_color_str(c))
                        .unwrap_or(Color::from_hex(0x09BB07));
                    if state.checked {
                        node_to_draw.style.background_color = Some(checkbox_color);
                        node_to_draw.style.border_color = Some(checkbox_color);
                    } else {
                        node_to_draw.style.background_color = Some(Color::WHITE);
                        node_to_draw.style.border_color = Some(Color::from_hex(0xD1D1D1));
                    }
                }
                "radio" => {
                    node_to_draw.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                    let radio_color = node.attrs.get("color")
                        .and_then(|c| crate::renderer::components::parse_color_str(c))
                        .unwrap_or(Color::from_hex(0x09BB07));
                    if state.checked {
                        node_to_draw.style.background_color = Some(radio_color);
                        node_to_draw.style.border_color = Some(radio_color);
                    } else {
                        node_to_draw.style.background_color = Some(Color::WHITE);
                        node_to_draw.style.border_color = Some(Color::from_hex(0xD1D1D1));
                    }
                }
                "slider" => {
                    if let Ok(v) = state.value.parse::<f32>() {
                        node_to_draw.style.custom_data = v / 100.0;
                        if !node_to_draw.text.is_empty() {
                            node_to_draw.text = format!("{}", v as i32);
                        }
                    }
                }
                _ => {}
            }
        }
        
        // 绘制组件
        self.draw_component(canvas, &node_to_draw, x, y, w, h, sf);
        
        // 递归绘制子节点
        if !Self::is_leaf_component(&node.tag) {
            for child in &node.children {
                self.draw_child_to_cache(canvas, taffy, child, x, y, text_color, interaction);
            }
        }
    }
    
    pub(super) fn draw_child_with_interaction(
        &mut self, 
        canvas: &mut Canvas, 
        taffy: &Tree, 
        node: &RenderNode, 
        ox: f32, 
        oy: f32, 
        inherited_color: Color,
        interaction: &mut InteractionManager,
        scroll_offset: f32,
        viewport_height: f32,
    ) {
        // 跳过 fixed 元素，它们会单独渲染
        if node.style.is_fixed {
            return;
        }
        // CSS 动画 / transform：由专门路径接管（含离屏仿射合成）
        if self.draw_with_transform(
            canvas,
            taffy,
            node,
            ox,
            oy,
            interaction,
            DrawKind::Child { inherited: inherited_color, scroll_offset, viewport_height },
        ) {
            return;
        }
        
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        
        if self.cull_outside_viewport(node, x, y, w, h, scroll_offset, viewport_height) {
            return;
        }

        if node.tag == "swiper" {
            self.draw_swiper_container(canvas, taffy, node, x, y, w, h, interaction,
                DrawKind::Child { inherited: inherited_color, scroll_offset, viewport_height });
            return;
        }

        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);

        let text_color = node.style.text_color.unwrap_or(inherited_color);
        let component_id = Self::get_component_id(node, &logical_bounds);
        
        // 检查是否需要修改节点（只有交互组件才需要 clone）
        //
        // 按压态（`:active` / `hover-class`）也算：从前只有顶层绘制路径应用了它，
        // 而页面里绝大多数可点元素都是嵌套子节点 —— 按压过渡因此完全看不到效果。
        let has_pressed_variant = node.style.pressed_style.is_some();
        let needs_modification = node.style.text_color.is_none() 
            || has_pressed_variant
            || matches!(node.tag.as_str(), "checkbox" | "switch" | "radio" | "slider" | "input" | "textarea")
            && interaction.get_state(&component_id).is_some();
        
        let switch_progress = if node.tag == "switch" {
            interaction
                .get_state(&component_id)
                .map(|s| s.checked)
                .map(|checked| self.toggle_progress(interaction, &component_id, checked))
        } else {
            None
        };
        
        // 只在需要时才 clone
        let node_to_draw: std::borrow::Cow<RenderNode> = if needs_modification {
            let mut modified = Self::shallow_for_draw(node);
            if modified.style.text_color.is_none() {
                modified.style.text_color = Some(text_color);
            }
            // 按压态：`:active` / `hover-class` 在建树期已算好，这里按需切换/插值
            if has_pressed_variant {
                self.apply_pressed_style(&mut modified, interaction, &component_id);
            }
            
            // 应用交互状态
            if let Some(state) = interaction.get_state(&component_id) {
                match node.tag.as_str() {
                    // switch 的配色由组件自己按进度求值（轨道关态是微信的浅灰，不是纯白）
                    "switch" => {
                        modified.style.custom_data = switch_progress.unwrap_or(if state.checked { 1.0 } else { 0.0 });
                    }
                    "checkbox" => {
                        modified.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                        let checkbox_color = node.attrs.get("color")
                            .and_then(|c| crate::renderer::components::parse_color_str(c))
                            .unwrap_or(Color::from_hex(0x09BB07));
                        if state.checked {
                            modified.style.background_color = Some(checkbox_color);
                            modified.style.border_color = Some(checkbox_color);
                        } else {
                            modified.style.background_color = Some(Color::WHITE);
                            modified.style.border_color = Some(Color::from_hex(0xD1D1D1));
                        }
                    }
                    "radio" => {
                        modified.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                        let radio_color = node.attrs.get("color")
                            .and_then(|c| crate::renderer::components::parse_color_str(c))
                            .unwrap_or(Color::from_hex(0x09BB07));
                        if state.checked {
                            modified.style.background_color = Some(radio_color);
                            modified.style.border_color = Some(radio_color);
                        } else {
                            modified.style.background_color = Some(Color::WHITE);
                            modified.style.border_color = Some(Color::from_hex(0xD1D1D1));
                        }
                    }
                    "slider" => {
                        if let Ok(v) = state.value.parse::<f32>() {
                            modified.style.custom_data = v / 100.0;
                            if !modified.text.is_empty() {
                                modified.text = format!("{}", v as i32);
                            }
                        }
                    }
                    "input" | "textarea" => {
                        let placeholder = node.attrs.get("placeholder").cloned().unwrap_or_default();
                        let is_focused = interaction.focused_input.as_ref()
                            .map(|f| f.id == component_id)
                            .unwrap_or(false);
                        
                        if state.value.is_empty() && !is_focused {
                            modified.text = placeholder;
                            modified.style.text_color = Some(Color::from_hex(0xBFBFBF));
                        } else {
                            modified.text = state.value.clone();
                            modified.style.text_color = Some(Color::BLACK);
                        }
                    }
                    _ => {}
                }
            }
            std::borrow::Cow::Owned(modified)
        } else if matches!(node.tag.as_str(), "input" | "textarea") {
            // 输入框但还没有交互状态，显示 placeholder 或初始值
            let placeholder = node.attrs.get("placeholder").cloned().unwrap_or_default();
            let initial_value = Self::input_value_attr(node);
            
            let mut modified = Self::shallow_for_draw(node);
            if initial_value.is_empty() {
                modified.text = placeholder;
                modified.style.text_color = Some(Color::from_hex(0xBFBFBF));
            } else {
                modified.text = initial_value;
            }
            std::borrow::Cow::Owned(modified)
        } else {
            std::borrow::Cow::Borrowed(node)
        };

        // 注册交互元素（包括 scroll-view）
        self.register_interactive_element(node, &node_to_draw, &logical_bounds, interaction, taffy, false);

        // 绘制组件 - 特殊处理 input、button 和有点击事件的 view 组件
        match node.tag.as_str() {
                "input" | "textarea" => {
                    let focused = interaction.focused_input.as_ref()
                        .map(|f| f.id == component_id)
                        .unwrap_or(false);
                    let (cursor_pos, selection) = if focused {
                        let f = interaction.focused_input.as_ref().unwrap();
                        (f.cursor_pos, f.get_selection_range())
                    } else {
                        (0, None)
                    };
                    InputComponent::draw_with_selection(
                        &node_to_draw, canvas, self.text_renderer.as_deref(), 
                        x, y, w, h, sf, focused, cursor_pos, selection
                    );
                    
                    // 更新 text_offset（用于点击位置计算）
                    if focused {
                        if let Some(tr) = self.text_renderer.as_deref() {
                            let font_size = node_to_draw.style.font_size * sf;
                            let padding_left = 12.0 * sf;
                            let padding_right = 12.0 * sf;
                            let available_width = w - padding_left - padding_right;
                            
                            let text_width = tr.measure_text(&node_to_draw.text, font_size);
                            let mut text_offset = 0.0;
                            
                            if text_width > available_width {
                                let cursor_text: String = node_to_draw.text.chars().take(cursor_pos).collect();
                                let cursor_x_in_text = tr.measure_text(&cursor_text, font_size);
                                
                                if cursor_x_in_text > available_width {
                                    text_offset = available_width - cursor_x_in_text - font_size;
                                }
                            }
                            
                            if let Some(input) = &mut interaction.focused_input {
                                input.text_offset = text_offset;
                            }
                        }
                    }
                }
                "button" => {
                let pressed = interaction.is_button_pressed(&component_id);
                ButtonComponent::draw_with_state(
                    &node_to_draw, canvas, self.text_renderer.as_deref(),
                    x, y, w, h, sf, pressed
                );
            }
            _ => {
                self.draw_component(canvas, &node_to_draw, x, y, w, h, sf);
            }
        }
        
        if !Self::is_leaf_component(&node.tag) {
            let is_scroll_view = node.tag == "scroll-view";
            let has_overflow_hidden = node.style.overflow == crate::renderer::components::Overflow::Hidden;
            let mut child_offset_x = 0.0;
            let mut child_offset_y = 0.0;
            let scroll_position: f32;
            
            // 检查是否是横向滚动
            let is_horizontal_scroll = is_scroll_view && node.attrs.get("scroll-x")
                .map(|s| s == "true" || s == "{{true}}")
                .unwrap_or(false);
            
            // 对于 overflow: hidden 的容器，应用裁剪
            if has_overflow_hidden || is_scroll_view {
                canvas.save();
                canvas.clip_rect(GeoRect::new(x, y, w, h));
            }
            
            if is_scroll_view {
                scroll_position = if let Some(controller) = interaction.get_scroll_controller(&component_id) {
                    let pos = controller.get_position();
                    if is_horizontal_scroll {
                        child_offset_x = -pos * sf; // 横向滚动
                    } else {
                        child_offset_y = -pos * sf; // 纵向滚动
                    }
                    pos
                } else {
                    0.0
                };
            } else {
                scroll_position = 0.0;
            }

            // 对于 scroll-view，只渲染可见区域内的子元素（视口裁剪优化）
            if is_scroll_view {
                if is_horizontal_scroll {
                    // 横向滚动：检查水平方向的可见性
                    let viewport_left = scroll_position * sf;
                    let viewport_right = viewport_left + w;
                    
                    for child in &node.children {
                        let child_layout = taffy.layout(child.taffy_node).unwrap();
                        let child_left = child_layout.location.x;
                        let child_right = child_left + child_layout.size.width;
                        
                        // 只渲染与视口相交的子元素
                        if child_right >= viewport_left && child_left <= viewport_right {
                            self.draw_child_with_interaction(canvas, taffy, child, x + child_offset_x, y, text_color, interaction, scroll_offset, viewport_height); 
                        }
                    }
                } else {
                    // 纵向滚动：检查垂直方向的可见性
                    let viewport_top = scroll_position * sf;
                    let viewport_bottom = viewport_top + h;
                    
                    for child in &node.children {
                        let child_layout = taffy.layout(child.taffy_node).unwrap();
                        let child_top = child_layout.location.y;
                        let child_bottom = child_top + child_layout.size.height;
                        
                        // 只渲染与视口相交的子元素
                        if child_bottom >= viewport_top && child_top <= viewport_bottom {
                            self.draw_child_with_interaction(canvas, taffy, child, x, y + child_offset_y, text_color, interaction, scroll_offset, viewport_height); 
                        }
                    }
                }
            } else {
                for child in &node.children { 
                    self.draw_child_with_interaction(canvas, taffy, child, x, y + child_offset_y, text_color, interaction, scroll_offset, viewport_height); 
                }
            }

            if has_overflow_hidden || is_scroll_view {
                canvas.restore();
            }
        }

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
    
}
