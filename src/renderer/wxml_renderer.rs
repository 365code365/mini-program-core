//! WXML 渲染器 - 使用组件系统渲染微信小程序

use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::parser::wxss::{StyleSheet, ElementDesc};
use crate::text::TextRenderer;
use crate::ui::interaction::{InteractionManager, InteractiveElement, InteractionType};
use crate::ui::scroll_cache::ScrollCacheManager;
use crate::{Canvas, Color, Rect as GeoRect};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use taffy::prelude::*;

use super::components::{
    RenderNode, NodeStyle, ComponentContext, InheritedText, TextAlign, WhiteSpace,
    ViewComponent, TextComponent, ButtonComponent, IconComponent,
    ProgressComponent, SwitchComponent, CheckboxComponent, RadioComponent,
    SliderComponent, InputComponent, ImageComponent, VideoComponent,
    CanvasComponent, SwiperComponent, SwiperItemComponent, RichTextComponent,
    PickerComponent, PickerViewComponent, PickerViewColumnComponent,
    CheckboxGroupComponent, RadioGroupComponent,
    build_base_style, Tree, TextMeasure, measure_text_node,
};

#[derive(Debug, Clone)]
pub struct EventBinding {
    pub event_type: String,
    pub handler: String,
    pub data: HashMap<String, String>,
    pub bounds: GeoRect,
    /// 是否是 catch 事件（阻止冒泡）
    pub is_catch: bool,
}

pub struct CachedLayout {
    pub render_nodes: Vec<RenderNode>,
    pub taffy: Tree,
    pub content_height: f32,
    pub data: JsonValue,
}

pub struct WxmlRenderer {
    stylesheet: StyleSheet,
    screen_width: f32,
    screen_height: f32,
    event_bindings: Vec<EventBinding>,
    text_renderer: Option<std::sync::Arc<TextRenderer>>,
    scale_factor: f32,
    cache: Option<CachedLayout>,
    /// Scroll-view 离屏缓存管理器
    scroll_cache: ScrollCacheManager,
    /// 当前视口信息 (scroll_offset, viewport_height) - 用于虚拟列表
    current_viewport: Option<(f32, f32)>,
}

impl WxmlRenderer {
    pub fn new(stylesheet: StyleSheet, screen_width: f32, screen_height: f32) -> Self {
        Self::new_with_scale(stylesheet, screen_width, screen_height, 1.0)
    }
    
    pub fn new_with_scale(stylesheet: StyleSheet, screen_width: f32, screen_height: f32, scale_factor: f32) -> Self {
        // 共享进程内唯一的字体实例：避免每次创建渲染器都重新加载上百 MB 字体
        let text_renderer = crate::text::shared_fonts();
        
        Self { 
            stylesheet, 
            screen_width,
            screen_height,
            event_bindings: Vec::new(),
            text_renderer,
            scale_factor,
            cache: None,
            scroll_cache: ScrollCacheManager::new(),
            current_viewport: None,
        }
    }

    fn update_layout_if_needed(
        &mut self,
        nodes: &[WxmlNode],
        data: &JsonValue,
        viewport: Option<(f32, f32)>,
    ) {
        // 检查视口是否变化（用于虚拟列表）
        let viewport_changed = self.current_viewport != viewport;
        
        if let Some(cache) = &self.cache {
            if cache.data == *data && !viewport_changed {
                return; // Cache hit!
            }
        }
        
        // 更新当前视口
        self.current_viewport = viewport;
        
        // 数据变化，标记所有 scroll-view 缓存为脏
        self.scroll_cache.mark_all_dirty();
        
        let rendered = crate::parser::TemplateEngine::render_with_virtual_list(nodes, data, viewport);
        let mut taffy = Tree::new();
        
        let mut render_nodes = Vec::new();
        
        for (sib_i, node) in rendered.iter().enumerate() {
            if let Some(rn) = self.build_tree(&mut taffy, node, &[], &InheritedText::default(), sib_i, rendered.len()) {
                render_nodes.push(rn);
            }
        }
        
        // 构建正常布局树（包含所有节点，fixed 元素也参与布局计算）
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
        // 第二遍：按实际宽度修正换行文本高度后重新布局
        if self.correct_wrapped_text_heights(&mut taffy, &render_nodes) {
            self.compute_with_text(&mut taffy, root, Size::MAX_CONTENT);
        }
        
        // 获取实际内容高度
        let root_layout = taffy.layout(root).unwrap();
        let content_height = root_layout.size.height / self.scale_factor;
        
        self.cache = Some(CachedLayout {
            render_nodes,
            taffy,
            content_height,
            data: data.clone(),
        });
    }

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
        
        self.event_bindings.clear();
        // 不清除交互元素，保留 scroll controller 状态
        // interaction.clear_elements();  // 移除这行，避免每帧重建
        
        if let Some(cache) = self.cache.take() {
            let content_height = cache.content_height;
            // 渲染所有元素（fixed 元素会在 draw_with_interaction 中被跳过）
            // 不使用滚动偏移渲染，滚动在 present_to_buffer 中处理
            for rn in &cache.render_nodes {
                self.draw_with_interaction(canvas, &cache.taffy, rn, 0.0, 0.0, interaction, scroll_offset, viewport_height * self.scale_factor);
            }
            self.cache = Some(cache);
            return content_height;
        }
        
        0.0
    }
    
    /// 单独渲染 fixed 元素到指定的 canvas
    /// 这个方法应该在主内容渲染后调用，fixed_canvas 是一个覆盖在主内容上的透明层
    pub fn render_fixed_elements(
        &mut self,
        canvas: &mut Canvas,
        nodes: &[WxmlNode],
        data: &JsonValue,
        interaction: &mut InteractionManager,
        _viewport_height: f32, // Ignore content viewport height, use full screen height for fixed elements
    ) {
        // 使用已缓存的视口信息，不重新计算布局
        self.update_layout_if_needed(nodes, data, self.current_viewport);
        
        if let Some(cache) = self.cache.take() {
            // 收集 fixed 元素
            struct FixedNodeInfo {
                node: RenderNode,
            }
            
            fn collect_fixed(nodes: &[RenderNode], fixed_list: &mut Vec<FixedNodeInfo>) {
                for node in nodes {
                    if node.style.is_fixed {
                        fixed_list.push(FixedNodeInfo { node: node.clone() });
                    }
                    collect_fixed(&node.children, fixed_list);
                }
            }
            
            let mut fixed_nodes = Vec::new();
            collect_fixed(&cache.render_nodes, &mut fixed_nodes);
            
            // Sort by z-index
            fixed_nodes.sort_by(|a, b| a.node.style.z_index.cmp(&b.node.style.z_index));
            
            if !fixed_nodes.is_empty() {
                let sf = self.scale_factor;
                let vp_width = self.screen_width * sf;
                let vp_height = self.screen_height * sf; // Use full screen height for fixed elements
                
                for info in &fixed_nodes {
                    let rn = &info.node;
                    let layout = cache.taffy.layout(rn.taffy_node).unwrap();
                    let w = layout.size.width;
                    let h = layout.size.height;
                    
                    // 计算 fixed 元素的位置（相对于视口）
                    let actual_w = if rn.style.fixed_left.is_some() && rn.style.fixed_right.is_some() {
                        let left = rn.style.fixed_left.unwrap_or(0.0);
                        let right = rn.style.fixed_right.unwrap_or(0.0);
                        vp_width - left - right
                    } else {
                        w
                    };
                    
                    let x = rn.style.fixed_left.unwrap_or(0.0);
                    let y = if let Some(bottom) = rn.style.fixed_bottom {
                        // bottom 定位：从视口底部计算
                        vp_height - bottom - h
                    } else if let Some(top) = rn.style.fixed_top {
                        // top 定位：从视口顶部计算
                        top
                    } else {
                        0.0
                    };
                    
                    // 渲染 fixed 元素
                    self.draw_fixed_element_original(&cache.taffy, canvas, rn, x, y, actual_w, h, interaction, vp_height);
                }
            }
            
            self.cache = Some(cache);
        }
    }
    
    /// 使用原始 taffy 布局绘制 fixed 元素
    fn draw_fixed_element_original(
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
        if !Self::is_leaf_component(&node.tag) {
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
        for (et, handler, data, is_catch) in &node.events {
            self.event_bindings.push(EventBinding {
                event_type: et.clone(),
                handler: handler.clone(),
                data: data.clone(),
                bounds: logical_bounds,
                is_catch: *is_catch,
            });
        }
    }
    
    /// 递归绘制 fixed 元素的子节点
    fn draw_fixed_child_recursive(
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
        let mut node_to_draw = node.clone();
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
        if !Self::is_leaf_component(&node.tag) {
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
        for (et, handler, data, is_catch) in &node.events {
            self.event_bindings.push(EventBinding {
                event_type: et.clone(),
                handler: handler.clone(),
                data: data.clone(),
                bounds: logical_bounds,
                is_catch: *is_catch,
            });
        }
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
        self.event_bindings.clear();
        let rendered = crate::parser::TemplateEngine::render(nodes, data);
        let mut taffy = Tree::new();
        
        let mut render_nodes = Vec::new();
        for (sib_i, node) in rendered.iter().enumerate() {
            if let Some(rn) = self.build_tree(&mut taffy, node, &[], &InheritedText::default(), sib_i, rendered.len()) {
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
        if self.correct_wrapped_text_heights(&mut taffy, &render_nodes) {
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

    /// 测量给定 WXML+数据的内容总高度（逻辑像素），用于自适应画布尺寸。
    pub fn measure_content_height(&self, nodes: &[WxmlNode], data: &JsonValue) -> f32 {
        let rendered = crate::parser::TemplateEngine::render(nodes, data);
        let mut taffy = Tree::new();
        let mut render_nodes = Vec::new();
        for (sib_i, node) in rendered.iter().enumerate() {
            if let Some(rn) = self.build_tree(&mut taffy, node, &[], &InheritedText::default(), sib_i, rendered.len()) {
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
        if self.correct_wrapped_text_heights(&mut taffy, &render_nodes) {
            self.compute_with_text(&mut taffy, root, Size::MAX_CONTENT);
        }
        taffy.layout(root).unwrap().size.height / self.scale_factor
    }
    
    /// 第二遍布局修正：首遍 `compute_layout` 后文本节点的实际宽度已知，
    /// 对「宽度为百分比/100%」等在 build 阶段无法预知换行的文本重新计算换行行数，
    /// 据此修正其盒子高度，避免多行文本被压成一行高而与后续兄弟节点重叠。
    /// 返回是否有节点高度被修改（需要重新 compute_layout）。
    ///
    /// 这是对标准 CSS「文本按可用宽度自动换行、盒子高度随行数增长」语义的补齐。
    /// 用文本度量闭包计算布局：仅带 TextMeasure 上下文的（block/auto 宽）文本按可用
    /// 宽度解析换行与高度；其它叶子沿用各自 Style 里的显式尺寸（known dimensions）。
    fn compute_with_text(&self, taffy: &mut Tree, root: NodeId, available: Size<AvailableSpace>) {
        let tr = self.text_renderer.as_deref();
        let _ = taffy.compute_layout_with_measure(
            root,
            available,
            |known, avail, _id, ctx: Option<&mut TextMeasure>| match ctx {
                Some(tm) => measure_text_node(known, avail, tm, tr),
                None => Size {
                    width: known.width.unwrap_or(0.0),
                    height: known.height.unwrap_or(0.0),
                },
            },
        );
    }

    fn correct_wrapped_text_heights(&self, taffy: &mut Tree, nodes: &[RenderNode]) -> bool {
        let tr = match self.text_renderer.as_deref() { Some(t) => t, None => return false };
        let sf = self.scale_factor;
        let mut changed = false;
        for node in nodes {
            if node.tag == "text" && !node.text.is_empty() {
                let should_wrap = !matches!(node.style.white_space, WhiteSpace::NoWrap | WhiteSpace::Pre);
                if should_wrap {
                    if let Ok(layout) = taffy.layout(node.taffy_node) {
                        let box_w = layout.size.width;
                        let box_h = layout.size.height;
                        let pl = node.style.padding_left * sf;
                        let pr = node.style.padding_right * sf;
                        let pt = node.style.padding_top * sf;
                        let pb = node.style.padding_bottom * sf;
                        let avail = (box_w - pl - pr).max(1.0);
                        let size = node.style.font_size * sf;
                        let ls = node.style.letter_spacing * sf;
                        let line_height = node.style.line_height.map(|lh| lh * sf)
                            .unwrap_or_else(|| tr.natural_line_height_for(&node.text, size)).max(size);
                        let bold = matches!(
                            node.style.font_weight,
                            super::components::FontWeight::Bold | super::components::FontWeight::W600
                                | super::components::FontWeight::W700 | super::components::FontWeight::W800
                                | super::components::FontWeight::W900
                        ) && tr.has_bold_face();
                        let lines = count_wrapped_lines(tr, &node.text, avail, size, ls, bold);
                        let needed_h = lines as f32 * line_height + pt + pb;
                        if needed_h > box_h + 0.5 {
                            if let Ok(mut st) = taffy.style(node.taffy_node).cloned() {
                                st.size.height = length(needed_h);
                                st.min_size.height = length(needed_h);
                                taffy.set_style(node.taffy_node, st).ok();
                                changed = true;
                            }
                        }
                    }
                }
            }
            if self.correct_wrapped_text_heights(taffy, &node.children) {
                changed = true;
            }
        }
        changed
    }

    fn build_tree(&self, taffy: &mut Tree, node: &WxmlNode, ancestors: &[ElementDesc], inherited: &InheritedText, sib_index: usize, sib_count: usize) -> Option<RenderNode> {
        let sf = self.scale_factor;
        
        if node.node_type == WxmlNodeType::Text {
            let text = node.text_content.trim();
            if text.is_empty() { return None; }
            // 原始文本节点继承父级的字号/颜色/字重/对齐/行高
            let fs = inherited.font_size;
            // 默认行高取字体自然行高（≈浏览器 normal），无字体时回退 1.2 倍
            let natural_lh = self.text_renderer.as_deref()
                .map(|tr| tr.natural_line_height_for(text, fs * sf) / sf)
                .unwrap_or(fs * crate::text::NORMAL_LINE_HEIGHT_FACTOR);
            let line_h = inherited.line_height.unwrap_or(natural_lh);
            let tw = self.measure_text(text, fs * sf);
            // 居中/右对齐的文本撑满可用宽度，绘制时再按对齐做偏移（否则无法居中）
            let width_dim: Dimension = if matches!(inherited.align, TextAlign::Center | TextAlign::Right) {
                percent(1.0)
            } else {
                length(tw + 2.0 * sf)
            };
            let tn = taffy.new_leaf(Style {
                size: Size { width: width_dim, height: length(line_h * sf) },
                ..Default::default()
            }).unwrap();
            return Some(RenderNode {
                tag: "#text".into(), 
                text: text.into(), 
                attrs: HashMap::new(),
                taffy_node: tn,
                style: NodeStyle {
                    font_size: fs,
                    text_color: inherited.color,
                    font_weight: inherited.weight,
                    text_align: inherited.align,
                    line_height: inherited.line_height,
                    letter_spacing: inherited.letter_spacing,
                    opacity: 1.0,
                    ..Default::default()
                },
                children: vec![], 
                events: vec![],
            });
        }
        
        if node.node_type != WxmlNodeType::Element { return None; }

        let tag = node.tag_name.as_str();
        let mut ctx = ComponentContext {
            scale_factor: sf,
            screen_width: self.screen_width,
            screen_height: self.screen_height,
            stylesheet: &self.stylesheet,
            taffy,
            ancestors: ancestors.to_vec(),
            inherited: inherited.clone(),
            sibling_index: sib_index,
            sibling_count: sib_count,
        };
        
        let mut render_node = match tag {
            "text" => TextComponent::build(node, &mut ctx),
            "button" => ButtonComponent::build(node, &mut ctx),
            "icon" => IconComponent::build(node, &mut ctx),
            "progress" => ProgressComponent::build(node, &mut ctx),
            "switch" => SwitchComponent::build(node, &mut ctx),
            "checkbox" => CheckboxComponent::build(node, &mut ctx),
            "checkbox-group" => CheckboxGroupComponent::build(node, &mut ctx),
            "radio" => RadioComponent::build(node, &mut ctx),
            "radio-group" => RadioGroupComponent::build(node, &mut ctx),
            "slider" => SliderComponent::build(node, &mut ctx),
            "input" | "textarea" => InputComponent::build(node, &mut ctx),
            "image" => ImageComponent::build(node, &mut ctx),
            "video" => VideoComponent::build(node, &mut ctx),
            "canvas" => CanvasComponent::build(node, &mut ctx),
            "swiper" => SwiperComponent::build(node, &mut ctx),
            "swiper-item" => SwiperItemComponent::build(node, &mut ctx),
            "rich-text" => RichTextComponent::build(node, &mut ctx),
            "picker" => PickerComponent::build(node, &mut ctx),
            "picker-view" => PickerViewComponent::build(node, &mut ctx),
            "picker-view-column" => PickerViewColumnComponent::build(node, &mut ctx),
            _ => ViewComponent::build(node, &mut ctx),
        };
        
        if let Some(ref mut rn) = render_node {
            if !Self::is_leaf_component(tag) {
                // 扩展祖先链：当前节点作为子节点的父级，用于后代/子选择器匹配
                let mut child_ancestors = ancestors.to_vec();
                let node_classes: Vec<&str> = node.get_attr("class")
                    .map(|s| s.split_whitespace().collect())
                    .unwrap_or_default();
                child_ancestors.push(ElementDesc::new(
                    &node.tag_name,
                    node.get_attr("id"),
                    &node_classes,
                    &node.attributes,
                ));
                
                // 计算传递给子节点的继承文本样式（来自当前节点的计算样式）
                let child_inherited = InheritedText {
                    font_size: rn.style.font_size,
                    color: rn.style.text_color,
                    weight: rn.style.font_weight,
                    align: rn.style.text_align,
                    line_height: rn.style.line_height,
                    letter_spacing: rn.style.letter_spacing,
                };
                
                let mut children = vec![];
                for (sib_ci, c) in node.children.iter().enumerate() {
                    if let Some(cr) = self.build_tree(ctx.taffy, c, &child_ancestors, &child_inherited, sib_ci, node.children.len()) { 
                        children.push(cr); 
                    }
                }
                
                if !children.is_empty() {
                    let child_ids: Vec<NodeId> = children.iter().map(|c| c.taffy_node).collect();
                    let (mut ts, ns) = build_base_style(node, &mut ctx);
                    
                    // 对于 scroll-view，使用 Overflow::Visible 让子节点能够正确布局
                    // 裁剪在渲染时通过 canvas.clip_rect 处理
                    if tag == "scroll-view" {
                        ts.overflow.x = taffy::style::Overflow::Visible;
                        ts.overflow.y = taffy::style::Overflow::Visible;
                        
                        // 检查是否是横向滚动
                        let scroll_x = node.get_attr("scroll-x")
                            .map(|s| s == "true" || s == "{{true}}")
                            .unwrap_or(false);
                        
                        if scroll_x {
                            // 横向滚动：子元素横向排列
                            ts.flex_direction = FlexDirection::Row;
                            ts.flex_wrap = FlexWrap::NoWrap;
                        }
                        
                        // 为 scroll-view 的子元素设置 flex-shrink: 0，防止被压缩
                        for child in &children {
                            if let Ok(mut style) = ctx.taffy.style(child.taffy_node).cloned() {
                                style.flex_shrink = 0.0;
                                ctx.taffy.set_style(child.taffy_node, style).ok();
                            }
                        }
                    }
                    
                    let new_tn = ctx.taffy.new_with_children(ts, &child_ids).unwrap();
                    
                    rn.taffy_node = new_tn;
                    rn.children = children;
                    // 更新样式（保留原有样式中已设置的值，但用新样式覆盖）
                    rn.style = ns;
                }
            } else if tag == "swiper" {
                // swiper 是叶子（自绘），但仍需构建其 swiper-item 子树供组件绘制当前页
                let mut child_ancestors = ancestors.to_vec();
                let node_classes: Vec<&str> = node.get_attr("class")
                    .map(|s| s.split_whitespace().collect())
                    .unwrap_or_default();
                child_ancestors.push(ElementDesc::new(
                    &node.tag_name, node.get_attr("id"), &node_classes, &node.attributes,
                ));
                let child_inherited = InheritedText {
                    font_size: rn.style.font_size,
                    color: rn.style.text_color,
                    weight: rn.style.font_weight,
                    align: rn.style.text_align,
                    line_height: rn.style.line_height,
                    letter_spacing: rn.style.letter_spacing,
                };
                let mut children = vec![];
                for (sib_ci, c) in node.children.iter().enumerate() {
                    if let Some(cr) = self.build_tree(ctx.taffy, c, &child_ancestors, &child_inherited, sib_ci, node.children.len()) {
                        children.push(cr);
                    }
                }
                rn.children = children;
            }
        }
        
        render_node
    }
    
    fn is_leaf_component(tag: &str) -> bool {
        // rich-text / picker 不再是叶子：rich-text 自建带样式的文本片段子树；
        // picker 渲染其子元素（触发视图，如“当前选择：xxx”）而非合成占位 UI。
        matches!(tag, 
            "text" | "button" | "icon" | "progress" | "switch" | 
            "checkbox" | "radio" | "slider" | "input" | "textarea" | "image" | "video" | "canvas" |
            "picker-view-column" | "swiper"
        )
    }
    
    fn get_component_id(node: &RenderNode, bounds: &GeoRect) -> String {
        if let Some(id) = node.attrs.get("id") {
            if !id.is_empty() {
                return id.clone();
            }
        }
        format!("{}_{:.0}_{:.0}", node.tag, bounds.x, bounds.y)
    }

    fn draw_with_interaction(
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
        
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        // 渲染整个内容到 canvas，滚动在 present_to_buffer 中处理
        
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);
        
        let component_id = Self::get_component_id(node, &logical_bounds);
        
        // 应用交互状态
        let mut node_to_draw = node.clone();
        if let Some(state) = interaction.get_state(&component_id) {
            match node.tag.as_str() {
                "checkbox" | "switch" => {
                    node_to_draw.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                    // 更新颜色
                    let checkbox_color = node.attrs.get("color")
                        .and_then(|c| super::components::parse_color_str(c))
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
                        .and_then(|c| super::components::parse_color_str(c))
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
            let initial_value = node.attrs.get("value").cloned().unwrap_or_default();
            
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
                
                // 从缓存复制可见区域到主 Canvas
                canvas.save();
                canvas.clip_rect(GeoRect::new(x, y, w, h));
                
                if let Some(cache) = self.scroll_cache.get(&component_id) {
                    cache.blit_to(canvas, scroll_position, x, y, sf);
                }
                
                canvas.restore();
                
                // 注册子元素的交互区域（需要考虑滚动偏移）
                child_offset_y = -scroll_position * sf;
                let text_color = node.style.text_color.unwrap_or(Color::BLACK);
                for child in &node.children {
                    self.register_child_interactions(taffy, child, x, y + child_offset_y, text_color, interaction, scroll_position, h / sf);
                }
            } else {
                let text_color = node.style.text_color.unwrap_or(Color::BLACK);
                for child in &node.children { 
                    self.draw_child_with_interaction(canvas, taffy, child, x, y + child_offset_y, text_color, interaction, scroll_offset, viewport_height); 
                }
            }
        }

        // 记录事件绑定
        for (et, h, d, is_catch) in &node.events {
            self.event_bindings.push(EventBinding { 
                event_type: et.clone(), 
                handler: h.clone(), 
                data: d.clone(), 
                bounds: logical_bounds,
                is_catch: *is_catch,
            });
        }
    }
    
    /// 渲染子节点到离屏缓存（不处理交互状态，纯渲染）
    fn draw_child_to_cache(
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
        let mut node_to_draw = node.clone();
        if node_to_draw.style.text_color.is_none() {
            node_to_draw.style.text_color = Some(text_color);
        }
        
        // 应用交互状态
        if let Some(state) = interaction.get_state(&component_id) {
            match node.tag.as_str() {
                "checkbox" | "switch" => {
                    node_to_draw.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                    let checkbox_color = node.attrs.get("color")
                        .and_then(|c| super::components::parse_color_str(c))
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
                        .and_then(|c| super::components::parse_color_str(c))
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
    
    /// 注册 scroll-view 子元素的交互区域
    fn register_child_interactions(
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
        if !Self::is_leaf_component(&node.tag) {
            for child in &node.children {
                self.register_child_interactions(taffy, child, x, y, text_color, interaction, scroll_position, viewport_height);
            }
        }
        
        // 记录事件绑定
        for (et, h, d, is_catch) in &node.events {
            self.event_bindings.push(EventBinding {
                event_type: et.clone(),
                handler: h.clone(),
                data: d.clone(),
                bounds: logical_bounds,
                is_catch: *is_catch,
            });
        }
    }
    
    fn draw_child_with_interaction(
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
        
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);

        let text_color = node.style.text_color.unwrap_or(inherited_color);
        let component_id = Self::get_component_id(node, &logical_bounds);
        
        // 检查是否需要修改节点（只有交互组件才需要 clone）
        let needs_modification = node.style.text_color.is_none() 
            || matches!(node.tag.as_str(), "checkbox" | "switch" | "radio" | "slider" | "input" | "textarea")
            && interaction.get_state(&component_id).is_some();
        
        // 只在需要时才 clone
        let node_to_draw: std::borrow::Cow<RenderNode> = if needs_modification {
            let mut modified = node.clone();
            if modified.style.text_color.is_none() {
                modified.style.text_color = Some(text_color);
            }
            
            // 应用交互状态
            if let Some(state) = interaction.get_state(&component_id) {
                match node.tag.as_str() {
                    "checkbox" | "switch" => {
                        modified.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                        let checkbox_color = node.attrs.get("color")
                            .and_then(|c| super::components::parse_color_str(c))
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
                            .and_then(|c| super::components::parse_color_str(c))
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
            let initial_value = node.attrs.get("value").cloned().unwrap_or_default();
            
            let mut modified = node.clone();
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
            let has_overflow_hidden = node.style.overflow == super::components::Overflow::Hidden;
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

        for (et, h, d, is_catch) in &node.events {
            self.event_bindings.push(EventBinding { 
                event_type: et.clone(), 
                handler: h.clone(), 
                data: d.clone(), 
                bounds: logical_bounds,
                is_catch: *is_catch,
            });
        }
    }
    
    fn draw_component(&self, canvas: &mut Canvas, node: &RenderNode, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        match node.tag.as_str() {
            "#text" | "text" => TextComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "button" => ButtonComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "icon" => IconComponent::draw(node, canvas, x, y, w, h, sf),
            "progress" => ProgressComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "switch" => SwitchComponent::draw(node, canvas, x, y, w, h, sf),
            "checkbox" => CheckboxComponent::draw(node, canvas, x, y, w, h, sf),
            "checkbox-group" => CheckboxGroupComponent::draw(node, canvas, x, y, w, h, sf),
            "radio" => RadioComponent::draw(node, canvas, x, y, w, h, sf),
            "radio-group" => RadioGroupComponent::draw(node, canvas, x, y, w, h, sf),
            "slider" => SliderComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "input" | "textarea" => InputComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "image" => ImageComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "video" => VideoComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "canvas" => CanvasComponent::draw(node, canvas, x, y, w, h, sf),
            "swiper" => SwiperComponent::draw_with_text(node, canvas, x, y, w, h, sf, self.text_renderer.as_deref()),
            "rich-text" => RichTextComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "picker" => PickerComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "picker-view" => PickerViewComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            _ => ViewComponent::draw(node, canvas, x, y, w, h, sf),
        }
    }
    
    fn register_interactive_element(
        &self, 
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
        let is_fixed = is_in_fixed_container || original_node.style.is_fixed;
        
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
                    value: original_node.attrs.get("value").cloned().unwrap_or_default(),
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
                    value: original_node.attrs.get("value").cloned().unwrap_or_default(),
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
                let actual_value = original_node.attrs.get("value").cloned().unwrap_or_default();
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
            _ => {
                // view 等普通元素不需要注册为交互元素
                // 点击事件通过 event_bindings 处理
            }
        }
    }

    
    fn draw(&mut self, canvas: &mut Canvas, taffy: &Tree, node: &RenderNode, ox: f32, oy: f32) {
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);

        self.draw_component(canvas, node, x, y, w, h, sf);
        
        if !Self::is_leaf_component(&node.tag) {
            let text_color = node.style.text_color.unwrap_or(Color::BLACK);
            for child in &node.children { 
                // fixed 子元素不在正常流内绘制，改由视口固定层单独绘制
                if child.style.is_fixed { continue; }
                self.draw_with_color(canvas, taffy, child, x, y, text_color); 
            }
        }

        for (et, h, d, is_catch) in &node.events {
            self.event_bindings.push(EventBinding { 
                event_type: et.clone(), 
                handler: h.clone(), 
                data: d.clone(), 
                bounds: logical_bounds,
                is_catch: *is_catch,
            });
        }
    }
    
    fn draw_with_color(&mut self, canvas: &mut Canvas, taffy: &Tree, node: &RenderNode, ox: f32, oy: f32, inherited_color: Color) {
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);

        let text_color = node.style.text_color.unwrap_or(inherited_color);
        let mut node_with_color = node.clone();
        if node_with_color.style.text_color.is_none() {
            node_with_color.style.text_color = Some(text_color);
        }

        self.draw_component(canvas, &node_with_color, x, y, w, h, sf);
        
        if !Self::is_leaf_component(&node.tag) {
            for child in &node.children { 
                // fixed 子元素不在正常流内绘制，改由视口固定层单独绘制
                if child.style.is_fixed { continue; }
                self.draw_with_color(canvas, taffy, child, x, y, text_color); 
            }
        }

        for (et, h, d, is_catch) in &node.events {
            self.event_bindings.push(EventBinding { 
                event_type: et.clone(), 
                handler: h.clone(), 
                data: d.clone(), 
                bounds: logical_bounds,
                is_catch: *is_catch,
            });
        }
    }

    /// 收集 fixed 子树（含嵌套）到列表。
    fn collect_fixed_nodes<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
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
    fn draw_fixed_layer(&mut self, canvas: &mut Canvas, taffy: &mut Tree, roots: &[RenderNode], viewport_h: f32) {
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
                self.draw(canvas, taffy, node, base_x, base_y);
            }
        }
    }

    fn measure_text(&self, text: &str, size: f32) -> f32 {
        self.text_renderer.as_deref()
            .map(|tr| tr.measure_text(text, size))
            .unwrap_or(text.chars().count() as f32 * size * 0.6)
    }

    pub fn get_event_bindings(&self) -> &[EventBinding] { 
        &self.event_bindings 
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<&EventBinding> {
        self.event_bindings.iter().rev().find(|b| b.bounds.contains(&crate::Point::new(x, y)))
    }
    
    /// 命中测试并返回事件冒泡链。
    ///
    /// 对齐官方语义：`tap` 事件从最内层节点向外冒泡，`catchtap` 阻止继续冒泡。
    /// 由于渲染层是扁平的绑定列表，这里用包围盒面积升序近似节点由内到外的层级
    /// （子节点面积必然 <= 父节点）。遇到 `is_catch` 的绑定后停止（含该项）。
    ///
    /// 只返回与 `event_type` 匹配的绑定（点击对应 "tap"）。
    pub fn hit_test_bubble(&self, x: f32, y: f32, event_type: &str) -> Vec<EventBinding> {
        let p = crate::Point::new(x, y);
        let mut matched: Vec<EventBinding> = self.event_bindings.iter()
            .filter(|b| b.event_type == event_type && b.bounds.contains(&p))
            .cloned()
            .collect();
        
        // 面积升序：最内层（最小）在前
        matched.sort_by(|a, b| {
            let area_a = a.bounds.width * a.bounds.height;
            let area_b = b.bounds.width * b.bounds.height;
            area_a.partial_cmp(&area_b).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // 冒泡：从内到外，遇 catch 停止
        let mut chain = Vec::new();
        for b in matched {
            let stop = b.is_catch;
            chain.push(b);
            if stop {
                break;
            }
        }
        chain
    }
    
    /// 获取事件绑定数量
    pub fn event_count(&self) -> usize {
        self.event_bindings.len()
    }
    
    /// 打印所有事件绑定（调试用）
    pub fn debug_events(&self) {
        for (i, binding) in self.event_bindings.iter().enumerate() {
            println!("   [{}] {} -> {} bounds=({:.1},{:.1},{:.1},{:.1}) data={:?}", 
                i, binding.event_type, binding.handler,
                binding.bounds.x, binding.bounds.y, binding.bounds.width, binding.bounds.height,
                binding.data);
        }
    }
}

/// 按与 text.rs 绘制一致的贪心算法统计文本在给定可用宽度下的换行行数。
/// 用于第二遍布局修正文本盒子高度（含 `\n` 硬换行）。
fn count_wrapped_lines(tr: &TextRenderer, text: &str, max_width: f32, size: f32, letter_spacing: f32, bold: bool) -> usize {
    if max_width <= 0.0 {
        return text.split('\n').count().max(1);
    }
    let mut lines = 0usize;
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines += 1;
            continue;
        }
        let chars: Vec<char> = paragraph.chars().collect();
        let mut line_start = 0usize;
        let mut current_width = 0.0f32;
        for (i, ch) in chars.iter().enumerate() {
            let char_width = tr.measure_char_weighted(*ch, size, bold) + letter_spacing;
            if current_width + char_width > max_width && i > line_start {
                lines += 1;
                line_start = i;
                current_width = char_width;
            } else {
                current_width += char_width;
            }
        }
        lines += 1; // 段落最后一行
    }
    lines.max(1)
}
