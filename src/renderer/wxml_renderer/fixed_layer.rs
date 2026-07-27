//! `position: fixed` 覆盖层。
//!
//! 覆盖层画在独立画布上、用**视口坐标**（不含滚动偏移），所以它与页面的重绘节奏
//! 是分开的：页面滚动时覆盖层完全不用重画（它钉在视口上）。
//!
//! 命中判定同样要分开：落在覆盖层区域内的点击一律由覆盖层消费，不能穿到下层页面。

use super::*;

impl WxmlRenderer {
    /// 单独渲染 fixed 元素到指定的 canvas（覆盖在主内容之上的透明层）。
    ///
    /// 与静态渲染走**同一套** `draw_fixed_layer`：先按视口约束（left+right → 宽、
    /// top+bottom → 高）对 fixed 子树重排，再钉到视口坐标。
    /// 此前窗体自己实现了一套简化版，直接拿「相对根容器」的布局尺寸当 fixed 尺寸 ——
    /// `top:0;bottom:0` 的全屏遮罩会被撑到整页内容高（近 2000px），于是遮罩铺满屏幕
    /// 而居中的弹窗被推到视口下方看不见（首页新人券弹窗就是这么"消失"的）。
    pub fn render_fixed_elements(
        &mut self,
        canvas: &mut Canvas,
        nodes: &[WxmlNode],
        data: &JsonValue,
        interaction: &mut InteractionManager,
        _viewport_height: f32, // fixed 层以画布高度为视口高
    ) {
        // 使用已缓存的视口信息，不重新计算布局
        self.update_layout_if_needed(nodes, data, self.current_viewport);
        interaction.clear_fixed_elements();
        
        if let Some(mut cache) = self.cache.take() {
            let viewport_h = canvas.height() as f32;
            let roots = std::mem::take(&mut cache.render_nodes);
            self.registering_fixed = true;
            self.fixed_hit_regions.clear();
            self.draw_fixed_layer_inner(canvas, &mut cache.taffy, &roots, viewport_h, Some(interaction));
            self.registering_fixed = false;
            cache.render_nodes = roots;
            self.cache = Some(cache);
        }
    }
    
    /// 使用原始 taffy 布局绘制 fixed 元素
    pub(super) fn draw_fixed_element_original(
        &mut self,
        taffy: &Tree,
        canvas: &mut Canvas,
        node: &RenderNode,
        fixed_x: f32,
        fixed_y: f32,
        fixed_w: f32,
        fixed_h: f32,
        interaction: &mut InteractionManager,
        viewport_height: f32,
    ) {
        let sf = self.scale_factor;
        let logical_bounds = GeoRect::new(fixed_x / sf, fixed_y / sf, fixed_w / sf, fixed_h / sf);
        
        // 绘制 fixed 元素的背景
        self.draw_component(canvas, node, fixed_x, fixed_y, fixed_w, fixed_h, sf);
        
        // 注册交互元素
        self.register_interactive_element(node, node, &logical_bounds, interaction, taffy, true);

        // 绘制子节点 - 子节点位置相对于 fixed 元素
        if Self::draws_children(node) {
            let text_color = node.style.text_color.unwrap_or(Color::BLACK);
            for child in &node.children {
                // 获取子节点在原始布局中相对于父节点的位置
                let child_layout = taffy.layout(child.taffy_node).unwrap();
                let child_x = fixed_x + child_layout.location.x;
                let child_y = fixed_y + child_layout.location.y;
                let child_w = child_layout.size.width;
                let child_h = child_layout.size.height;
                
                self.draw_fixed_child_recursive(taffy, canvas, child, child_x, child_y, child_w, child_h, text_color, interaction, viewport_height);
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
    
    /// 递归绘制 fixed 元素的子节点
    pub(super) fn draw_fixed_child_recursive(
        &mut self,
        taffy: &Tree,
        canvas: &mut Canvas,
        node: &RenderNode,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        inherited_color: Color,
        interaction: &mut InteractionManager,
        viewport_height: f32,
    ) {
        // Viewport culling
        if y > viewport_height || y + h < 0.0 {
            return;
        }

        let sf = self.scale_factor;
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);
        
        let text_color = node.style.text_color.unwrap_or(inherited_color);
        let mut node_to_draw = Self::shallow_for_draw(node);
        if node_to_draw.style.text_color.is_none() {
            node_to_draw.style.text_color = Some(text_color);
        }
        
        // 注册交互元素
        self.register_interactive_element(node, &node_to_draw, &logical_bounds, interaction, taffy, true);

        // 绘制组件 - 特殊处理 button 以支持按下状态
        let component_id = Self::get_component_id(node, &logical_bounds);
        match node.tag.as_str() {
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
        
        // 递归绘制子节点
        if Self::draws_children(node) {
            let is_scroll_view = node.tag == "scroll-view";
            let mut child_offset_y = 0.0;
            let scroll_position: f32;
            
            if is_scroll_view {
                canvas.save();
                canvas.clip_rect(GeoRect::new(x, y, w, h));
                
                scroll_position = if let Some(controller) = interaction.get_scroll_controller(&component_id) {
                    let pos = controller.get_position();
                    child_offset_y = -pos * sf; // 转换为物理像素
                    pos
                } else {
                    0.0
                };
            } else {
                scroll_position = 0.0;
            }

            // 对于 scroll-view，只渲染可见区域内的子元素
            if is_scroll_view {
                let viewport_top = scroll_position * sf;
                let viewport_bottom = viewport_top + h;
                
                for child in &node.children {
                    let child_layout = taffy.layout(child.taffy_node).unwrap();
                    let child_top = child_layout.location.y;
                    let child_bottom = child_top + child_layout.size.height;
                    
                    // 只渲染与视口相交的子元素
                    if child_bottom >= viewport_top && child_top <= viewport_bottom {
                        let child_x = x + child_layout.location.x;
                        let child_y = y + child_layout.location.y + child_offset_y;
                        let child_w = child_layout.size.width;
                        let child_h = child_layout.size.height;
                        
                        self.draw_fixed_child_recursive(taffy, canvas, child, child_x, child_y, child_w, child_h, text_color, interaction, viewport_height);
                    }
                }
            } else {
                for child in &node.children {
                    let child_layout = taffy.layout(child.taffy_node).unwrap();
                    let child_x = x + child_layout.location.x;
                    let child_y = y + child_layout.location.y + child_offset_y;
                    let child_w = child_layout.size.width;
                    let child_h = child_layout.size.height;
                    
                    self.draw_fixed_child_recursive(taffy, canvas, child, child_x, child_y, child_w, child_h, text_color, interaction, viewport_height);
                }
            }

            if is_scroll_view {
                canvas.restore();
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
    
    /// 收集 fixed 子树（含嵌套）到列表。
    pub(super) fn collect_fixed_nodes<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
        for child in &node.children {
            if child.style.is_fixed {
                out.push(child);
            }
            // fixed 子树内部不再单独收集（其内部随父一起按视口绘制）
            if !child.style.is_fixed {
                Self::collect_fixed_nodes(child, out);
            }
        }
    }

    /// 简单渲染路径下的 fixed 层：把 position:fixed 元素钉在视口而非内容流底部，
    /// 与浏览器 position:fixed 语义一致。
    ///
    /// 关键点：fixed 元素的 top/bottom/left/right 需相对「视口」解析，而 Taffy 是相对
    /// 根容器（内容全高）解析的。因此这里先按视口把该元素的宽/高约束出来，再以视口尺寸
    /// 为可用空间对其子树单独重排，最后按视口把整棵子树钉到目标位置——这样 top:0;bottom:0
    /// 的全屏遮罩才会是视口高、其居中的对话框才落在可见区内。
    pub(super) fn draw_fixed_layer(&mut self, canvas: &mut Canvas, taffy: &mut Tree, roots: &[RenderNode], viewport_h: f32) {
        // `registering_fixed` 必须置上：覆盖层里的事件绑定要标成 `is_fixed`，
        // 否则它们会以**内容坐标**混进正常流那张表里。页面一滚，遮罩/弹窗的绑定
        // 就停在内容里的旧位置，反而挡住那块位置上真正的元素 —— 点击落到弹窗的
        // 处理函数上。宿主那条路径（render_fixed_elements）一直是置上的，
        // 这条静态路径（画廊、离屏渲染、双端对比）漏了。
        let prev = self.registering_fixed;
        self.registering_fixed = true;
        self.draw_fixed_layer_inner(canvas, taffy, roots, viewport_h, None);
        self.registering_fixed = prev;
    }

    /// fixed 覆盖层绘制。`interaction` 为 Some 时同时注册交互元素与命中区（宿主运行态），
    /// 为 None 时纯绘制（静态渲染/截图）。
    pub(super) fn draw_fixed_layer_inner(
        &mut self,
        canvas: &mut Canvas,
        taffy: &mut Tree,
        roots: &[RenderNode],
        viewport_h: f32,
        mut interaction: Option<&mut InteractionManager>,
    ) {
        let sf = self.scale_factor;
        let vp_width = self.screen_width * sf;
        let mut fixed_ids: Vec<(NodeId, NodeStyle)> = Vec::new();
        {
            let mut fixed_nodes: Vec<&RenderNode> = Vec::new();
            for root in roots {
                if root.style.is_fixed {
                    fixed_nodes.push(root);
                }
                Self::collect_fixed_nodes(root, &mut fixed_nodes);
            }
            fixed_nodes.sort_by(|a, b| a.style.z_index.cmp(&b.style.z_index));
            for n in fixed_nodes {
                fixed_ids.push((n.taffy_node, n.style.clone()));
            }
        }

        // 建立 taffy_node -> RenderNode 引用查找（fixed 子树根）
        fn find_node<'a>(nodes: &'a [RenderNode], id: NodeId) -> Option<&'a RenderNode> {
            for n in nodes {
                if n.taffy_node == id { return Some(n); }
                if let Some(found) = find_node(&n.children, id) { return Some(found); }
            }
            None
        }

        for (id, st) in &fixed_ids {
            // 当前（相对根容器的）布局，用作缺省尺寸
            let cur = match taffy.layout(*id) { Ok(l) => *l, Err(_) => continue };
            let mut target_w = cur.size.width;
            let mut target_h = cur.size.height;
            // 视口约束：left+right → 宽；top+bottom → 高
            if let (Some(l), Some(r)) = (st.fixed_left, st.fixed_right) {
                target_w = (vp_width - l - r).max(0.0);
            }
            if let (Some(t), Some(b)) = (st.fixed_top, st.fixed_bottom) {
                target_h = (viewport_h - t - b).max(0.0);
            }

            // 以视口约束尺寸对该 fixed 子树单独重排（绝对定位，不影响主流）
            if let Ok(mut style) = taffy.style(*id).cloned() {
                style.position = Position::Relative;
                style.inset = Rect { top: auto(), right: auto(), bottom: auto(), left: auto() };
                style.size = Size { width: length(target_w), height: length(target_h) };
                if taffy.set_style(*id, style).is_ok() {
                    self.compute_with_text(
                        taffy,
                        *id,
                        Size {
                            width: AvailableSpace::Definite(target_w),
                            height: AvailableSpace::Definite(target_h),
                        },
                    );
                }
            }

            let new_layout = match taffy.layout(*id) { Ok(l) => *l, Err(_) => continue };
            let w = new_layout.size.width;
            let h = new_layout.size.height;

            let pinned_x = if let Some(left) = st.fixed_left {
                left
            } else if let Some(right) = st.fixed_right {
                vp_width - right - w
            } else {
                cur.location.x
            };
            let pinned_y = if let Some(bottom) = st.fixed_bottom {
                viewport_h - bottom - h
            } else if let Some(top) = st.fixed_top {
                top
            } else {
                cur.location.y
            };

            if let Some(node) = find_node(roots, *id) {
                // 重排后该节点作为子树根，location 约为 0，直接以钉住点为原点绘制
                let base_x = pinned_x - new_layout.location.x;
                let base_y = pinned_y - new_layout.location.y;
                // 记录遮挡区域（逻辑坐标）：落在这里的点击不允许穿透到下层
                if self.registering_fixed && w > 0.0 && h > 0.0 {
                    self.fixed_hit_regions.push(GeoRect::new(
                        pinned_x / sf,
                        pinned_y / sf,
                        w / sf,
                        h / sf,
                    ));
                }
                match interaction.as_deref_mut() {
                    Some(im) => {
                        // 子树根自身的 is_fixed 需要清掉，否则会被「跳过 fixed」的分支拦下
                        let mut root = node.clone();
                        root.style.is_fixed = false;
                        self.draw_with_interaction(canvas, taffy, &root, base_x, base_y, im, 0.0, viewport_h);
                    }
                    None => self.draw(canvas, taffy, node, base_x, base_y),
                }
            }
        }
    }

}
