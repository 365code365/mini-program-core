//! 对外的渲染入口。
//!
//! 各入口的差别只在「给不给交互上下文、给不给滚动偏移与视口高」，
//! 内部都走同一条 布局(缓存) → 绘制调度 → 覆盖层 的链路。

use super::*;

impl WxmlRenderer {
    /// 渲染 WXML 节点，使用交互管理器处理状态
    pub fn render_with_interaction(
        &mut self, 
        canvas: &mut Canvas, 
        nodes: &[WxmlNode], 
        data: &JsonValue,
        interaction: &mut InteractionManager,
    ) {
        self.render_with_scroll_and_viewport(canvas, nodes, data, interaction, 0.0, self.screen_height);
    }
    
    /// 渲染 WXML 节点，支持滚动偏移（用于 fixed 定位）
    pub fn render_with_scroll(
        &mut self, 
        canvas: &mut Canvas, 
        nodes: &[WxmlNode], 
        data: &JsonValue,
        interaction: &mut InteractionManager,
        scroll_offset: f32,
    ) {
        self.render_with_scroll_and_viewport(canvas, nodes, data, interaction, scroll_offset, self.screen_height);
    }
    
    /// 渲染 WXML 节点，支持滚动偏移和自定义视口高度（用于 fixed 定位）
    /// 返回实际内容高度
    pub fn render_with_scroll_and_viewport(
        &mut self, 
        canvas: &mut Canvas, 
        nodes: &[WxmlNode], 
        data: &JsonValue,
        interaction: &mut InteractionManager,
        scroll_offset: f32,
        viewport_height: f32,
    ) -> f32 {
        // 传递视口信息给模板引擎，用于虚拟列表优化
        self.update_layout_if_needed(nodes, data, Some((scroll_offset, viewport_height)));
        
        self.animations_active = false;
        self.anim_marks = 0;
        self.animated_bounds.clear();
        self.event_bindings.clear();
        self.picker_regions.retain(|p| p.is_fixed);
        // 正常流的交互元素每帧重建（滚动控制器另存在 scroll_controllers 里，不受影响）。
        // 覆盖层的元素保留 —— 它可能整帧都不重绘，由 render_fixed_elements 单独重建。
        interaction.retain_only_fixed_elements();
        
        if let Some(cache) = self.cache.take() {
            let content_height = cache.content_height;
            // 损伤区重绘：只有这块矩形内的像素会被改写，其余保留上一帧。
            // 注意这里**不能 take()** —— 绘制期的 `cull_outside_viewport` 还要靠
            // `self.damage_clip` 把离损伤区太远的子树整棵剪掉。
            // 提前取空的话像素级裁剪照样正确，但那六百个节点还是会被逐个走一遍、
            // 各自发起绘制，白付几毫秒（局部重绘就只剩"少画"没有"少算"）。
            let damaged = self.damage_clip;
            if let Some(rect) = damaged {
                canvas.save();
                canvas.clip_rect(rect);
            }
            // 渲染所有元素（fixed 元素会在 draw_with_interaction 中被跳过）
            // 不使用滚动偏移渲染，滚动在 present_to_buffer 中处理
            for rn in &cache.render_nodes {
                self.draw_with_interaction(canvas, &cache.taffy, rn, 0.0, 0.0, interaction, scroll_offset, viewport_height * self.scale_factor);
            }
            if damaged.is_some() {
                canvas.restore();
            }
            // 用完即清：下一帧由宿主重新设置（不清的话整帧重绘会被上一帧的损伤区裁掉）
            self.damage_clip = None;
            self.cache = Some(cache);
            return content_height;
        }
        
        0.0
    }
    
    /// 兼容旧接口
    pub fn render(&mut self, canvas: &mut Canvas, nodes: &[WxmlNode], data: &JsonValue) {
        self.render_with_shell(canvas, nodes, data, |_| {});
    }

    /// 渲染页面，并在「正常流」与「fixed 覆盖层」之间插入宿主外壳绘制（如 tabBar）。
    ///
    /// 层叠顺序与浏览器一致：页面背景/内容 → 宿主外壳(tabBar) → 页面内 position:fixed
    /// 覆盖层（遮罩/弹窗）。这样全屏遮罩会同时压暗 tabBar，而 tabBar 又不会被页面背景覆盖。
    pub fn render_with_shell(
        &mut self,
        canvas: &mut Canvas,
        nodes: &[WxmlNode],
        data: &JsonValue,
        draw_shell: impl FnOnce(&mut Canvas),
    ) {
        self.animations_active = false;
        self.event_bindings.clear();
        let rendered = crate::parser::TemplateEngine::render_with_components(nodes, data, &self.component_templates);
        let mut taffy = Tree::new();
        
        let mut render_nodes = Vec::new();
        // 继承起点用 `page { … }`：这条路径（画廊、离屏渲染、双端对比的内置渲染）
        // 从前用的是内置默认值，与窗体那条路径不一致 —— 在 page 上定义字号/字色/字族的
        // 应用会出现「两个入口画出两种结果」。
        let page = self.page_style();
        for (sib_i, node) in rendered.iter().enumerate() {
            if let Some(rn) = self.build_tree(&mut taffy, node, &[], &page.inherited, sib_i, rendered.len(), false) {
                render_nodes.push(rn);
            }
        }
        
        let child_ids: Vec<NodeId> = render_nodes.iter().map(|n| n.taffy_node).collect();
        let root = taffy.new_with_children(
            Style {
                size: Size { width: length(self.screen_width * self.scale_factor), height: auto() },
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &child_ids,
        ).unwrap();
        
        self.compute_with_text(&mut taffy, root, Size::MAX_CONTENT);
        let mut need_relayout = self.correct_wrapped_text_heights(&mut taffy, &render_nodes);
        need_relayout |= self.correct_absolute_heights(
            &mut taffy,
            &render_nodes,
            canvas.height() as f32,
        );
        if need_relayout {
            self.compute_with_text(&mut taffy, root, Size::MAX_CONTENT);
        }
        
        // 正常流：跳过 fixed 子树（顶层与嵌套均由固定层处理）
        for rn in &render_nodes {
            if rn.style.is_fixed { continue; }
            self.draw(canvas, &taffy, rn, 0.0, 0.0);
        }
        // 宿主外壳（tabBar 等）：位于页面内容之上、页面 fixed 覆盖层之下
        draw_shell(canvas);
        // 固定层：把 position:fixed 元素钉在视口（画布高度即视口）
        let viewport_h = canvas.height() as f32;
        self.draw_fixed_layer(canvas, &mut taffy, &render_nodes, viewport_h);
    }

}
