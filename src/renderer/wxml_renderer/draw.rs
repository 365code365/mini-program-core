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
        if viewport_height <= 0.0 {
            return false;
        }
        let visible_top = scroll_offset * self.scale_factor - MARGIN;
        let visible_bottom = scroll_offset * self.scale_factor + viewport_height + MARGIN;
        y + h < visible_top || y > visible_bottom
    }

    /// 局部重绘帧里，这个盒子离损伤区太远、画了也会被像素级裁掉。
    ///
    /// 只用来**跳过绘制**，不能用来剪掉整棵子树：事件绑定与交互元素每帧清空重建，
    /// 子树不遍历就不登记，损伤区帧之后远处的元素就点不动了（直到下一次整帧）。
    ///
    /// 余量沿用视口裁剪的同一个值：阴影、溢出的绝对定位子节点可能画到
    /// 布局盒之外，判据必须比盒子宽松（`tools/damage-check.sh` 逐像素校验这一点）。
    pub(super) fn outside_damage(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        const MARGIN: f32 = VIEWPORT_CULL_MARGIN_PX;
        // 离屏合成时坐标是离屏画布的局部坐标，与损伤区不在同一坐标系
        if self.drawing_offscreen {
            return false;
        }
        match self.damage_clip {
            Some(d) => {
                x + w < d.x - MARGIN
                    || x > d.x + d.width + MARGIN
                    || y + h < d.y - MARGIN
                    || y > d.y + d.height + MARGIN
            }
            None => false,
        }
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

    /// `opacity < 1` 时先把子树画进透明离屏，再按这个透明度贴回去。
    ///
    /// 这样背景、文字、图片一起变淡，和 CSS / 微信一致。逐个颜色乘 alpha 做不到：
    /// 子节点（按钮里的 `<text>`）根本看不到父节点的 opacity。
    /// `draw` 用平移后的原点再走一遍原来的绘制函数；绑定坐标用 `binding_shift` 换回页面坐标。
    pub(super) fn paint_opacity_group(
        &mut self,
        canvas: &mut Canvas,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        opacity: f32,
        shadow: Option<&crate::renderer::components::BoxShadow>,
        mut draw: impl FnMut(&mut Self, &mut Canvas, f32, f32),
    ) {
        let mut pad = 2.0f32;
        if let Some(s) = shadow {
            pad = pad.max(s.blur + s.spread + s.offset_x.abs().max(s.offset_y.abs()) + 2.0);
        }
        let tw = (w + pad * 2.0).ceil() as u32;
        let th = (h + pad * 2.0).ceil() as u32;
        if tw == 0 || th == 0 || tw > 4096 || th > 4096 {
            // 大到不值得离屏：退回逐颜色乘 alpha，同时别再进组合成（否则递归）
            let was = self.bypass_opacity_group;
            self.bypass_opacity_group = true;
            draw(self, canvas, 0.0, 0.0);
            self.bypass_opacity_group = was;
            return;
        }
        let mut off = Canvas::new(tw, th);
        off.clear(Color::new(0, 0, 0, 0));
        // 离屏只画贴回去时落在目标裁剪区里的那一块（目标裁剪区换算到离屏坐标）。贴回去本来就只贴
        // 裁剪区里的像素（`blend_pixels_opacity` 按 draw_bounds 裁），裁剪区里的像素与整块画逐字节相同
        // （局部重画靠的就是这一条）。从前离屏整块画：补条带只要一百多行，首页头图里那段
        // `opacity: 0.9` 的两行字照样整块光栅化，一加上一条带 10~16ms
        let (ox, oy) = ((x - pad) as i32, (y - pad) as i32);
        let (bx0, bx1, by0, by1) = canvas.draw_bounds();
        let (cx0, cy0) = ((bx0 - ox).clamp(0, tw as i32), (by0 - oy).clamp(0, th as i32));
        let (cx1, cy1) = ((bx1 - ox).clamp(cx0, tw as i32), (by1 - oy).clamp(cy0, th as i32));
        if (cx0, cy0, cx1, cy1) != (0, 0, tw as i32, th as i32) {
            off.clip_rect(GeoRect::new(cx0 as f32, cy0 as f32, (cx1 - cx0) as f32, (cy1 - cy0) as f32));
        }
        let was_off = self.drawing_offscreen;
        let was_flat = self.flattening_opacity;
        let was_shift = self.binding_shift;
        let sf = self.scale_factor.max(0.001);
        self.drawing_offscreen = true;
        self.flattening_opacity = true;
        self.binding_shift = (was_shift.0 + (x - pad) / sf, was_shift.1 + (y - pad) / sf);
        draw(self, &mut off, pad - x, pad - y);
        self.drawing_offscreen = was_off;
        self.flattening_opacity = was_flat;
        self.binding_shift = was_shift;
        canvas.blend_pixels_opacity((x - pad) as i32, (y - pad) as i32, off.pixels(), tw as usize, opacity);
    }

    /// 像素盒换成逻辑坐标，并加上离屏绘制的页面偏移（平时偏移是 0）
    pub(super) fn page_bounds(&self, x: f32, y: f32, w: f32, h: f32) -> GeoRect {
        let sf = self.scale_factor.max(0.001);
        GeoRect::new(
            x / sf + self.binding_shift.0,
            y / sf + self.binding_shift.1,
            w / sf,
            h / sf,
        )
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
        // 同层渲染的原生组件（`<map>` 这类，宿主画好帧交过来）：画在框的背景之上、子节点之下
        if crate::renderer::components::host_view::is_same_layer(node.tag.as_str()) {
            crate::renderer::components::host_view::draw(node, ctx.canvas, x, y, w, h);
        } else if crate::renderer::components::native_placeholder::label_of(node.tag.as_str()).is_some()
            && !self.is_host_native(node.tag.as_str())
        {
            // 宿主没接管的原生组件（map / camera / web-view ...）：画占位，不留一块看不出是什么的空白
            crate::renderer::components::native_placeholder::draw(node, ctx.canvas, ctx.text, x, y, w, h, sf);
        }
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
        let logical_bounds = self.page_bounds(x, y, w, h);

        self.draw_component(canvas, node, x, y, w, h, sf);
        // swiper：与带交互的路径同一套分页（G-13：从前这里把每一项都排开画出来）
        if node.tag == "swiper" {
            let text_color = node.style.text_color.unwrap_or(Color::BLACK);
            self.draw_swiper_plain(canvas, taffy, node, (x, y, w, h), |r, c, child, ox, oy| {
                r.draw_with_color(c, taffy, child, ox, oy, text_color)
            });
            self.push_event_bindings(node, logical_bounds);
            return;
        }
        if node.tag == "picker-view" {
            let text_color = node.style.text_color.unwrap_or(Color::BLACK);
            self.draw_picker_view_with(canvas, taffy, node, (x, y, w, h), false, |r, c, item, ox, oy| {
                r.draw_with_color(c, taffy, item, ox, oy, text_color)
            });
            self.push_event_bindings(node, logical_bounds);
            return;
        }
        
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
        let logical_bounds = self.page_bounds(x, y, w, h);

        let text_color = node.style.text_color.unwrap_or(inherited_color);
        let mut node_with_color = Self::shallow_for_draw(node);
        if node_with_color.style.text_color.is_none() {
            node_with_color.style.text_color = Some(text_color);
        }

        self.draw_component(canvas, &node_with_color, x, y, w, h, sf);
        if node.tag == "swiper" {
            self.draw_swiper_plain(canvas, taffy, node, (x, y, w, h), |r, c, child, ox, oy| {
                r.draw_with_color(c, taffy, child, ox, oy, text_color)
            });
            self.push_event_bindings(node, logical_bounds);
            return;
        }
        if node.tag == "picker-view" {
            self.draw_picker_view_with(canvas, taffy, node, (x, y, w, h), false, |r, c, item, ox, oy| {
                r.draw_with_color(c, taffy, item, ox, oy, text_color)
            });
            self.push_event_bindings(node, logical_bounds);
            return;
        }
        
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
