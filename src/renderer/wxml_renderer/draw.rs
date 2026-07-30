//! 绘制调度的公共部分：视口裁剪、绘制分派、以及**不带交互**的绘制路径。
//!
//! 页面主路径（带交互登记的那条）在 `draw_interactive`；这里放两边共用的东西
//! 和「没有交互上下文」的场景（画廊、离屏度量）用的 `draw` / `draw_with_color`。
//!
//! 视口裁剪在这里：整页画布可能上万像素高，只画「视口 ± 余量」那一条带。

use super::*;

impl WxmlRenderer {
    pub(super) fn shallow_for_draw(node: &RenderNode) -> RenderNode {
        RenderNode {
            tag: node.tag.clone(),
            text: node.text.clone(),
            attrs: node.attrs.clone(),
            taffy_node: node.taffy_node,
            style: node.style.clone(),
            children: Vec::new(),
            events: Vec::new(),
        }
    }

    /// 视口裁剪：子树完全落在可见区之外时跳过整棵绘制。
    ///
    /// 宿主把整页内容画进一张「内容高」的长画布，再按滚动位置 blit 可见的一屏。
    /// 页面有 CSS 动画时每帧都要重绘，于是首页那种 6800px 高的长页面每帧都在
    /// 光栅化 5M 像素（实测 49.6ms/帧 ≈ 20FPS，肉眼就是卡）。
    /// 可见区之外的内容 blit 时根本读不到，绘制它纯属浪费。
    ///
    /// 留一段余量并跳过带 transform/动画的节点：它们的实际绘制范围可能超出布局盒
    /// （离屏仿射、位移），不能只按盒子判断。
    pub(super) fn cull_outside_viewport(
        &self,
        node: &RenderNode,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        scroll_offset: f32,
        viewport_height: f32,
    ) -> bool {
        // 离屏合成时坐标是画布局部坐标，和滚动空间没有对应关系，不能用它裁剪
        if self.drawing_offscreen {
            return false;
        }
        if node.style.transform.is_some() || node.style.animation.is_some() {
            return false;
        }
        const MARGIN: f32 = VIEWPORT_CULL_MARGIN_PX;
        // 横向：画布宽就是屏宽，横滑列表里被推到屏幕外的项同样不可见
        let canvas_w = self.screen_width * self.scale_factor;
        if x + w < -MARGIN || x > canvas_w + MARGIN {
            return true;
        }
        // 局部重绘帧：离损伤区太远的子树连遍历都不必要。
        //
        // 从前只按视口裁剪，于是一个 45x33 的倒计时损伤区，也要把视口内
        // 六百来个节点全部走一遍、各自发起绘制，最后才在像素级被裁掉 ——
        // 白付约 3ms。整帧预算只有 6.94ms，这一笔是掉帧的直接原因之一。
        //
        // 余量沿用视口裁剪的同一个值：阴影、溢出的绝对定位子节点可能画到
        // 布局盒之外，判据必须比盒子宽松（`tools/damage-check.sh` 逐像素校验这一点）。
        if let Some(d) = self.damage_clip {
            if x + w < d.x - MARGIN
                || x > d.x + d.width + MARGIN
                || y + h < d.y - MARGIN
                || y > d.y + d.height + MARGIN
            {
                return true;
            }
        }
        if viewport_height <= 0.0 {
            return false;
        }
        let visible_top = scroll_offset * self.scale_factor - MARGIN;
        let visible_bottom = scroll_offset * self.scale_factor + viewport_height + MARGIN;
        y + h < visible_top || y > visible_bottom
    }

    /// 绘制 swiper 容器：裁剪到自身盒子 → 整行按当前页横向偏移 → 逐项走普通绘制 → 指示点。
    ///
    /// 只有"当前页在哪儿"是 swiper 特有的，item 内容（图片、浮层、flex、动画、
    /// 绝对定位）全部复用通用绘制路径，不再自成一套。
    pub(super) fn draw_swiper_container(
        &mut self,
        canvas: &mut Canvas,
        taffy: &Tree,
        node: &RenderNode,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        interaction: &mut InteractionManager,
        kind: DrawKind,
    ) {
        let sf = self.scale_factor;
        draw_background(canvas, &node.style, x, y, w, h);
        if node.children.is_empty() || w <= 0.0 || h <= 0.0 {
            return;
        }
        let current = SwiperComponent::current_index(node, x, y);
        // 换页是滑过去的（与微信一致）：拿到 prev→current 之间的浮点页号。
        // 只有滑动进行中才要求继续出帧 —— 从前是「有多于一项就每帧重绘」，
        // 而 swiper 平时根本不动，等于让带轮播的页面永远整屏重绘。
        let (page_pos, sliding) = SwiperComponent::slide_position(node, x, y)
            .unwrap_or((current as f32, false));
        if sliding {
            self.mark_animating();
        }

        canvas.save();
        canvas.clip_rect(GeoRect::new(x, y, w, h));
        // 子项在 taffy 里排成一行/一列（每项一屏），整体沿主轴移动 page_pos 屏
        let vertical = SwiperComponent::is_vertical(node);
        let (dx, dy) = if vertical {
            (0.0, -page_pos * h)
        } else {
            (-page_pos * w, 0.0)
        };
        // 滑动中要画相邻两页（否则中间过程一侧是空白）；静止时只画当前页 ——
        // 非当前页被裁剪后完全不可见，逐帧重采样它们的图片纯属浪费。
        let first = page_pos.floor().max(0.0) as usize;
        let last = if sliding { page_pos.ceil().max(0.0) as usize } else { first };
        for idx in first..=last.min(node.children.len().saturating_sub(1)) {
            if let Some(child) = node.children.get(idx) {
                self.dispatch_draw(canvas, taffy, child, x + dx, y + dy, interaction, kind);
            }
        }
        canvas.restore();

        SwiperComponent::draw_indicators(node, canvas, x, y, w, h, sf, current);
    }

    /// 分发到对应的绘制路径（顶层节点 / 子节点两套上下文）
    pub(super) fn dispatch_draw(
        &mut self,
        canvas: &mut Canvas,
        taffy: &Tree,
        node: &RenderNode,
        ox: f32,
        oy: f32,
        interaction: &mut InteractionManager,
        kind: DrawKind,
    ) {
        match kind {
            DrawKind::Top { scroll_offset, viewport_height } => {
                self.draw_with_interaction(canvas, taffy, node, ox, oy, interaction, scroll_offset, viewport_height)
            }
            DrawKind::Child { inherited, scroll_offset, viewport_height } => self
                .draw_child_with_interaction(canvas, taffy, node, ox, oy, inherited, interaction, scroll_offset, viewport_height),
        }
    }

    pub(super) fn draw_component(&self, canvas: &mut Canvas, node: &RenderNode, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        // 标签 → 绘制实现与耗时归因分类都来自同一张表（components::registry）。
        // 从前这里是两个各自维护的 match：一个 19 条分派、一个 10 条归因名 ——
        // 加组件时漏掉归因那条，它的耗时就悄悄落进「其它」。
        let spec = crate::renderer::components::spec_for(node.tag.as_str());
        // 绘制耗时按组件类型归因（`MINI_DRAW_LOG=1`）：一帧 8ms 里到底是图片重采样贵、
        // 还是上百个文字光栅化贵，只有摊开才知道该优化哪里。关掉时零开销。
        let _t = crate::renderer::draw_profile::Timer::start(spec.profile);
        let mut ctx = crate::renderer::components::DrawCtx {
            node,
            canvas,
            text: self.text_renderer.as_deref(),
            x,
            y,
            w,
            h,
            sf,
        };
        (spec.draw)(&mut ctx);
    }
    
    pub(super) fn draw(&mut self, canvas: &mut Canvas, taffy: &Tree, node: &RenderNode, ox: f32, oy: f32) {
        if self.draw_transformed_plain(canvas, taffy, node, ox, oy, Color::BLACK) {
            return;
        }
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);

        self.draw_component(canvas, node, x, y, w, h, sf);
        
        if Self::draws_children(node) {
            let text_color = node.style.text_color.unwrap_or(Color::BLACK);
            for child in &node.children { 
                // fixed 子元素不在正常流内绘制，改由视口固定层单独绘制
                if child.style.is_fixed { continue; }
                self.draw_with_color(canvas, taffy, child, x, y, text_color); 
            }
        }

        self.push_event_bindings(node, logical_bounds);
    }
    
    pub(super) fn draw_with_color(&mut self, canvas: &mut Canvas, taffy: &Tree, node: &RenderNode, ox: f32, oy: f32, inherited_color: Color) {
        if self.draw_transformed_plain(canvas, taffy, node, ox, oy, inherited_color) {
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
        let mut node_with_color = Self::shallow_for_draw(node);
        if node_with_color.style.text_color.is_none() {
            node_with_color.style.text_color = Some(text_color);
        }

        self.draw_component(canvas, &node_with_color, x, y, w, h, sf);
        
        if Self::draws_children(node) {
            for child in &node.children { 
                // fixed 子元素不在正常流内绘制，改由视口固定层单独绘制
                if child.style.is_fixed { continue; }
                self.draw_with_color(canvas, taffy, child, x, y, text_color); 
            }
        }

        self.push_event_bindings(node, logical_bounds);
    }

}
