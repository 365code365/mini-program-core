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
        
        let logical_bounds = self.page_bounds(x, y, w, h);
        
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
        // 交互状态落到本帧节点上（三条绘制路径同一份实现，见 interactive.rs）
        Self::apply_interaction_state(
            node, &mut node_to_draw, interaction, &component_id, switch_progress,
        );
        if self.flattening_opacity {
            self.flattening_opacity = false;
            node_to_draw.style.opacity = 1.0;
        } else if !self.drawing_offscreen
            && !self.bypass_opacity_group
            && node_to_draw.style.opacity < 0.999
        {
            let op = node_to_draw.style.opacity;
            let shadow = node_to_draw.style.box_shadow.clone();
            self.paint_opacity_group(canvas, x, y, w, h, op, shadow.as_ref(), |r, c, dx, dy| {
                r.draw_with_interaction(
                    c, taffy, node, ox + dx, oy + dy, interaction, scroll_offset, viewport_height,
                );
            });
            return;
        }

        // 注册交互元素
        self.register_interactive_element(node, &node_to_draw, &logical_bounds, interaction, taffy, false);

        // 绘制组件 - 特殊处理 input 和 button 组件
        if !self.outside_damage(x, y, w, h) {
            match node.tag.as_str() {
                "input" | "textarea" | "button" => self.draw_interactive_component(
                    canvas, node, &node_to_draw, interaction, &component_id, x, y, w, h, sf,
                ),
                _ => self.draw_component(canvas, &node_to_draw, x, y, w, h, sf),
            }
        }
        
        // 绘制子节点
        if Self::draws_children(node) {
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

        self.push_event_bindings(node, logical_bounds);
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
        
        let logical_bounds = self.page_bounds(x, y, w, h);
        let component_id = Self::get_component_id(node, &logical_bounds);
        
        let text_color = node.style.text_color.unwrap_or(inherited_color);
        let mut node_to_draw = Self::shallow_for_draw(node);
        if node_to_draw.style.text_color.is_none() {
            node_to_draw.style.text_color = Some(text_color);
        }
        
        // 应用交互状态（与另外两条绘制路径同一份实现，见 interactive.rs）
        Self::apply_interaction_state(node, &mut node_to_draw, interaction, &component_id, None);
        
        // 绘制组件
        self.draw_component(canvas, &node_to_draw, x, y, w, h, sf);
        
        // 递归绘制子节点
        if Self::draws_children(node) {
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

        let logical_bounds = self.page_bounds(x, y, w, h);

        let text_color = node.style.text_color.unwrap_or(inherited_color);
        let component_id = Self::get_component_id(node, &logical_bounds);
        
        // 检查是否需要修改节点（只有交互组件才需要 clone）
        //
        // 按压态（`:active` / `hover-class`）也算：从前只有顶层绘制路径应用了它，
        // 而页面里绝大多数可点元素都是嵌套子节点 —— 按压过渡因此完全看不到效果。
        let has_pressed_variant = node.style.pressed_style.is_some();
        let needs_modification = node.style.text_color.is_none() 
            || has_pressed_variant
            || crate::renderer::components::spec_for(node.tag.as_str()).stateful
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
            
            // 应用交互状态（唯一一份实现，见 interactive.rs）
            Self::apply_interaction_state(
                node, &mut modified, interaction, &component_id, switch_progress,
            );
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
        let mut node_to_draw = node_to_draw;
        if self.flattening_opacity {
            self.flattening_opacity = false;
            let mut owned = node_to_draw.into_owned();
            owned.style.opacity = 1.0;
            node_to_draw = std::borrow::Cow::Owned(owned);
        } else if !self.drawing_offscreen
            && !self.bypass_opacity_group
            && node_to_draw.style.opacity < 0.999
        {
            let op = node_to_draw.style.opacity;
            let shadow = node_to_draw.style.box_shadow.clone();
            self.paint_opacity_group(canvas, x, y, w, h, op, shadow.as_ref(), |r, c, dx, dy| {
                r.draw_child_with_interaction(
                    c, taffy, node, ox + dx, oy + dy, text_color, interaction,
                    scroll_offset, viewport_height,
                );
            });
            return;
        }

        // 注册交互元素（包括 scroll-view）
        self.register_interactive_element(node, &node_to_draw, &logical_bounds, interaction, taffy, false);

        // 绘制组件 - 特殊处理 input、button 和有点击事件的 view 组件
        if !self.outside_damage(x, y, w, h) {
            match node.tag.as_str() {
                "input" | "textarea" | "button" => self.draw_interactive_component(
                    canvas, node, &node_to_draw, interaction, &component_id, x, y, w, h, sf,
                ),
                _ => self.draw_component(canvas, &node_to_draw, x, y, w, h, sf),
            }
        }
        
        if Self::draws_children(node) {
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

        self.push_event_bindings(node, logical_bounds);
    }
    
}
