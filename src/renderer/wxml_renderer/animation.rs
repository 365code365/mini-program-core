//! 动画求值与 transform 合成。
//!
//! 三件事：CSS `@keyframes` / `transition` 的时间线求值、`wx.createAnimation()`
//! 载荷（`animation="{{data}}"`）的逐步插值、以及把两者的结果连同静态 `transform`
//! 合成成「本帧要画的节点 + 一个仿射矩阵」交给离屏合成路径。
//!
//! 也管「本帧有没有动画在跑」这个标记 —— 宿主据此决定是否续帧，
//! 以及动画元素的包围盒（损伤区，宿主据此只重绘小块而不是整屏）。

use super::*;

impl WxmlRenderer {
    /// 取走本帧记录的动画节点包围盒（物理像素，画布坐标）
    pub fn take_animated_bounds(&mut self) -> Vec<GeoRect> {
        std::mem::take(&mut self.animated_bounds)
    }

    /// 标记「本帧有动画在跑」，同时推进计数器
    pub(super) fn mark_animating(&mut self) {
        self.animations_active = true;
        self.anim_marks += 1;
    }

    /// 直接设置「本帧有动画」标记。
    ///
    /// `render_fixed_elements` 是接着页面那一趟累加的，宿主要分别知道
    /// 「页面在动」和「fixed 覆盖层在动」（后者不能走局部重绘），
    /// 于是需要在两趟之间清零再合并回来。
    pub fn set_animations_active(&mut self, active: bool) {
        self.animations_active = active;
    }

    /// 限定本帧只重绘这个矩形（物理像素，画布坐标）。每帧渲染前设置，渲染后自动清空。
    pub fn set_damage_clip(&mut self, rect: Option<GeoRect>) {
        self.damage_clip = rect;
    }

    /// 登记 `usingComponents` 声明的自定义组件模板（标签名 → 组件 WXML 根节点）。
    /// 换页时由宿主重新设置：组件是按页面声明的。
    pub fn set_component_templates(&mut self, templates: crate::parser::template::ComponentTemplates) {
        self.component_templates = templates;
        self.cache = None; // 模板变了，上一棵树不再可复用
    }

    /// 本帧是否存在仍在推进的 CSS 动画。宿主用它决定「继续按刷新率出帧」还是「空闲休眠」。
    pub fn has_active_animations(&self) -> bool {
        self.animations_active
    }

    /// 显式设置动画时钟（秒）。用于静态截图/测试等需要确定性时间的场景；
    /// 不设置时使用进程内全局时钟。
    pub fn set_animation_time(&mut self, seconds: f32) {
        self.anim_time = Some(seconds);
    }

    pub(super) fn animation_time(&self) -> f32 {
        self.anim_time.unwrap_or_else(crate::renderer::anim::now_secs)
    }

    /// 这一刻的动画时钟（秒）。宿主自己画的动画（提示框里的转圈）跟它走，
    /// `set_animation_time` 定住时快照里的转角也定住，像素才可比
    pub fn animation_clock(&self) -> f32 {
        self.animation_time()
    }

    /// 取（并缓存）动画名对应的时间轴
    pub(super) fn timeline_for(&mut self, name: &str) -> Option<&crate::renderer::anim::Timeline> {
        if !self.timelines.contains_key(name) {
            let built = self
                .stylesheet
                .keyframes_named(name)
                .map(|rule| {
                    crate::renderer::anim::Timeline::from_rule(rule, self.screen_width, self.scale_factor)
                })
                .filter(|tl| !tl.is_empty());
            self.timelines.insert(name.to_string(), built);
        }
        self.timelines.get(name).and_then(|t| t.as_ref())
    }

    /// 求节点此刻的 CSS 动画覆盖值；顺带标记「本帧还有动画在跑」。
    pub(super) fn animated_values(&mut self, node: &RenderNode) -> Option<crate::renderer::anim::AnimatedValues> {
        let spec = node.style.animation.clone()?;
        let now = self.animation_time();
        // 无限循环 / 未结束的动画都要求宿主继续出帧
        let still_running = !spec.iterations.is_finite()
            || now < spec.delay + spec.duration * spec.iterations;
        if still_running {
            self.mark_animating();
        }
        let progress = spec.progress_at(now)?;
        // 元素自身的计算值作为隐式 0%/100% 关键帧（CSS 语义）
        let base = crate::renderer::anim::FrameValues {
            offset: 0.0,
            opacity: Some(node.style.opacity),
            transform: Some(node.style.transform.unwrap_or_else(crate::renderer::components::Transform::new)),
            background_color: node.style.background_color,
            text_color: node.style.text_color,
        };
        let timeline = self.timeline_for(&spec.name)?;
        let values = timeline.sample_with_base(progress, &base);
        if values.is_empty() {
            None
        } else {
            Some(values)
        }
    }

    /// 把动画求值结果 + 静态 transform 合成成「本帧要用的节点」。
    /// 返回 None 表示该节点本帧无需特殊处理（走普通绘制路径）。
    pub(super) fn resolve_animated_node(
        &mut self,
        node: &RenderNode,
    ) -> Option<(RenderNode, crate::renderer::components::Transform)> {
        let values = self.animated_values(node);
        let js = self.js_animated_values(node);
        let has_values = values.is_some() || js.is_some();
        let mut transform = values
            .as_ref()
            .and_then(|v| v.transform)
            .or(node.style.transform);
        if let Some(j) = &js {
            if j.has_transform {
                transform = Some(j.transform);
            }
        }
        if !has_values && transform.is_none() {
            return None;
        }
        let mut resolved = node.clone();
        // 动画值已在此处落地，避免离屏重绘时再次求值（否则会无限递归）。
        // `animation` 属性（wx.createAnimation 的载荷）同样要摘掉 —— 只清 style
        // 不清属性的话，重入这条路径时又会解析出动画，直接栈溢出。
        resolved.style.animation = None;
        resolved.style.transform = None;
        resolved.attrs.remove("animation");
        if let Some(v) = &values {
            if let Some(op) = v.opacity {
                resolved.style.opacity = (node.style.opacity * op).clamp(0.0, 1.0);
            }
            if let Some(bg) = v.background_color {
                resolved.style.background_color = Some(bg);
                resolved.style.background_gradient = None;
            }
            if let Some(color) = v.text_color {
                resolved.style.text_color = Some(color);
            }
        }
        // JS 动画（wx.createAnimation）优先级高于 CSS 动画：它是逻辑层刚下发的目标态
        if let Some(j) = &js {
            if let Some(op) = j.opacity {
                resolved.style.opacity = (node.style.opacity * op).clamp(0.0, 1.0);
            }
            if let Some(bg) = j.background_color {
                resolved.style.background_color = Some(bg);
                resolved.style.background_gradient = None;
            }
        }
        Some((resolved, transform.unwrap_or_else(crate::renderer::components::Transform::new)))
    }

    /// 处理节点的 CSS 动画与 `transform`。返回 true 表示本节点（含子树）已绘制完成。
    ///
    /// 三条路径：
    /// 1. 只有颜色/透明度在动 → 直接用求值后的节点走普通绘制；
    /// 2. 纯平移 → 加偏移绘制（子树、命中区都自然跟随）；
    /// 3. 含缩放/旋转/倾斜 → 子树画到离屏画布再仿射贴回。
    pub(super) fn draw_with_transform(
        &mut self,
        canvas: &mut Canvas,
        taffy: &Tree,
        node: &RenderNode,
        ox: f32,
        oy: f32,
        interaction: &mut InteractionManager,
        kind: DrawKind,
    ) -> bool {
        let marks_before = self.anim_marks;
        // `:active { transform: ... }` 的按压态要在这里先烘焙，才能进离屏仿射
        let baked = self.bake_pressed_for_transform(taffy, node, ox, oy, interaction);
        let node = baked.as_ref().unwrap_or(node);
        let Some((resolved, transform)) = self.resolve_animated_node(node) else { return false };
        // 这个节点自己有动画在跑的话，记下它的包围盒（含 transform 溢出余量），
        // 宿主据此只重绘这些小块而不是整屏。按计数器判断而不是看全局标记 ——
        // 后者被前一个动画节点置真后，后续节点就无法自证「我在动」。
        if self.anim_marks != marks_before && !self.drawing_offscreen {
            if let Ok(layout) = taffy.layout(node.taffy_node) {
                let (x, y) = (ox + layout.location.x, oy + layout.location.y);
                let (w, h) = (layout.size.width, layout.size.height);
                self.animated_bounds.push(animated_bound(taffy, node, (x, y, w, h), &transform));
                // 动画底图：这块只因动画重画时，留下 / 贴回它底下那层（backdrop.rs）
                self.backdrop_at_animated(canvas, node.taffy_node);
            }
        }

        if crate::renderer::compose::is_identity(&transform) {
            self.dispatch_draw(canvas, taffy, &resolved, ox, oy, interaction, kind);
            return true;
        }
        if crate::renderer::compose::is_translate_only(&transform) {
            self.dispatch_draw(
                canvas,
                taffy,
                &resolved,
                ox + transform.translate_x,
                oy + transform.translate_y,
                interaction,
                kind,
            );
            return true;
        }

        let layout = taffy.layout(node.taffy_node).unwrap();
        let (x, y) = (ox + layout.location.x, oy + layout.location.y);
        let (w, h) = (layout.size.width, layout.size.height);
        let pad = crate::renderer::compose::transform_padding(w, h, &transform);
        let tw = (w + pad * 2.0).ceil() as u32;
        let th = (h + pad * 2.0).ceil() as u32;
        if w <= 0.0 || h <= 0.0 || tw == 0 || th == 0 || tw > 4096 || th > 4096 {
            // 尺寸异常（含 0 尺寸叶子）时退回不变换绘制，避免分配巨大离屏画布
            self.dispatch_draw(canvas, taffy, &resolved, ox, oy, interaction, kind);
            return true;
        }

        // 透明度是**成组**的（CSS opacity 连子树一起淡）。离屏那一趟不走 paint_opacity_group，
        // 从前透明度只落在这个节点自己的背景上、孩子照旧不透明画进离屏、再按 1.0 贴回：
        // page-container 居中弹层离场（scale 0.9 + opacity 0）时白底淡没了，标题、按钮原样留在遮罩上（「关闭留残影」）。
        // 改成子树按不透明画进离屏，贴回时整块乘透明度
        // 子树内容不随动画变（只转 / 只缩放）：离屏按内容缓存，每帧只贴回（transform_cache.rs）
        let cached = self.transform_cacheable(node, &resolved, baked.is_some(), interaction);
        let (resolved, group_opacity) = Self::take_group_opacity(resolved);
        // 变换后的子树坐标与屏幕不再一一对应：命中区与事件绑定用一次性容器承接，
        // 不污染真实的交互命中表（旋转/缩放子树内的点击是已知限制）。
        let draw_sub = |r: &mut Self, off: &mut Canvas| {
            let mut scratch = InteractionManager::new();
            let bindings_before = r.event_bindings.len();
            let was_offscreen = r.drawing_offscreen;
            r.drawing_offscreen = true;
            r.dispatch_draw(off, taffy, &resolved, pad - layout.location.x, pad - layout.location.y, &mut scratch, kind);
            r.drawing_offscreen = was_offscreen;
            // 离屏那一趟注册的绑定用的是离屏画布的局部坐标，直接留下会命中错位置，先丢掉
            r.event_bindings.truncate(bindings_before);
        };
        let mut fresh = None;
        if cached {
            self.ensure_transform_offscreen(taffy, node, (tw, th), draw_sub);
        } else {
            // 离屏只画贴回去时会被采到的那一块（见 compose::offscreen_for）
            let mut offscreen = crate::renderer::compose::offscreen_for(canvas, (tw, th), (x - pad, y - pad), (x + w / 2.0, y + h / 2.0), &transform);
            draw_sub(self, &mut offscreen);
            fresh = Some(offscreen);
        }
        // 再按**未变换**的真实位置重新注册一遍。
        //
        // 从前到这里就结束了（注释写作「旋转/缩放子树内的点击是已知限制」）——
        // 结果是带 `scale`/`rotate` 动画的容器里所有按钮都是死的：弹窗用
        // `animation: popIn`（含 scale）做入场，弹窗里的关闭按钮、领取按钮全都点不动，
        // 而且点击会穿透到下层页面。
        // 用未变换几何做命中是个近似（浏览器按变换后的几何命中），但对
        // 「围绕中心缩放/旋转」这类实际用法足够接近，比彻底没有绑定好得多。
        let inherited = match kind {
            DrawKind::Child { inherited, .. } => inherited,
            DrawKind::Top { .. } => Color::BLACK,
        };
        let (scroll_offset, viewport_height) = match kind {
            DrawKind::Top { scroll_offset, viewport_height }
            | DrawKind::Child { scroll_offset, viewport_height, .. } => (scroll_offset, viewport_height),
        };
        self.register_child_interactions(
            taffy,
            &resolved,
            ox,
            oy,
            inherited,
            interaction,
            scroll_offset,
            viewport_height / self.scale_factor,
        );

        let src = match fresh.as_ref() {
            Some(c) => Some(c),
            None => self.transform_cache.canvas(node.taffy_node),
        };
        if let Some(src) = src {
            crate::renderer::compose::blit_transformed(canvas, src, (x - pad, y - pad), (x + w / 2.0, y + h / 2.0), &transform, group_opacity);
        }
        true
    }

    /// 离屏仿射那条路的成组透明度：节点按不透明画（返回的节点 opacity = 1），透明度交给贴回那一步。
    /// 不透明的节点原样返回（贴回乘 1.0，与从前逐字节相同）
    fn take_group_opacity(mut resolved: RenderNode) -> (RenderNode, f32) {
        let op = resolved.style.opacity.clamp(0.0, 1.0);
        if op >= 0.999 {
            return (resolved, 1.0);
        }
        resolved.style.opacity = 1.0;
        (resolved, op)
    }

    /// 无交互上下文的绘制路径（静态渲染 / 宿主外壳）上的动画与 transform 处理。
    pub(super) fn draw_transformed_plain(
        &mut self,
        canvas: &mut Canvas,
        taffy: &Tree,
        node: &RenderNode,
        ox: f32,
        oy: f32,
        inherited: Color,
    ) -> bool {
        let Some((resolved, transform)) = self.resolve_animated_node(node) else { return false };
        if crate::renderer::compose::is_identity(&transform) {
            self.draw_with_color(canvas, taffy, &resolved, ox, oy, inherited);
            return true;
        }
        if crate::renderer::compose::is_translate_only(&transform) {
            self.draw_with_color(
                canvas,
                taffy,
                &resolved,
                ox + transform.translate_x,
                oy + transform.translate_y,
                inherited,
            );
            return true;
        }
        let layout = taffy.layout(node.taffy_node).unwrap();
        let (x, y) = (ox + layout.location.x, oy + layout.location.y);
        let (w, h) = (layout.size.width, layout.size.height);
        let pad = crate::renderer::compose::transform_padding(w, h, &transform);
        let tw = (w + pad * 2.0).ceil() as u32;
        let th = (h + pad * 2.0).ceil() as u32;
        if w <= 0.0 || h <= 0.0 || tw == 0 || th == 0 || tw > 4096 || th > 4096 {
            self.draw_with_color(canvas, taffy, &resolved, ox, oy, inherited);
            return true;
        }
        // 离屏只画贴回去时会被采到的那一块（见 compose::offscreen_for）
        let mut offscreen = crate::renderer::compose::offscreen_for(canvas, (tw, th), (x - pad, y - pad), (x + w / 2.0, y + h / 2.0), &transform);
        let bindings_before = self.event_bindings.len();
        self.draw_with_color(
            &mut offscreen,
            taffy,
            &resolved,
            pad - layout.location.x,
            pad - layout.location.y,
            inherited,
        );
        self.event_bindings.truncate(bindings_before);
        crate::renderer::compose::blit_transformed(
            canvas,
            &offscreen,
            (x - pad, y - pad),
            (x + w / 2.0, y + h / 2.0),
            &transform,
            1.0,
        );
        true
    }

}

/// 一个在动的节点这一帧最多画到哪（宿主下一帧擦旧的、画新的都按它）。
///
/// 从前只按节点自己的框 + `transform_padding` 算，漏了两样（tests/sdk_damage_grow.rs 的转圈抓到的）：
/// - **平移**：离屏仿射贴回时整块按 (tx, ty) 挪，`transform_padding` 只管缩放 / 旋转撑出去的那圈；
/// - **溢出框的孩子**：转圈里那个点超出了框，跟着转到框外，那几行每帧留着上一帧的像素。
///
/// 现在取「框 ∪ 子树里各节点的框」：只平移时整块挪 |t|；有缩放 / 旋转 / 倾斜时，按子树里离变换原点（框心）
/// 最远的那一点（× 放大倍数）为半径取正方形。外面再留 2px（双线性采样）。
///
/// 「最远的那一点」按**画得出来的形状**算，不按外包框的角：圆角盒子离框心最远的是四个圆角上的点
/// （角上圆心的距离 + 半径），阴影另按外扩 + 模糊余量 + 偏移算。播放页的唱片是个圆（border-radius 50%），
/// 从前按外包框的角算半径（√2 × 边长的一半），每帧重画 932×932，现在 702×702（像素少一半）：
/// 方框角上那一圈转到哪都是透明的，重画了也是原样
fn animated_bound(taffy: &Tree, node: &RenderNode, (x, y, w, h): (f32, f32, f32, f32), t: &crate::renderer::components::Transform) -> GeoRect {
    fn extent(taffy: &Tree, n: &RenderNode, px: f32, py: f32, b: &mut (f32, f32, f32, f32)) {
        for c in &n.children {
            let Ok(l) = taffy.layout(c.taffy_node) else { continue };
            let (x, y) = (px + l.location.x, py + l.location.y);
            *b = (b.0.min(x), b.1.min(y), b.2.max(x + l.size.width), b.3.max(y + l.size.height));
            extent(taffy, c, x, y, b);
        }
    }
    let mut b = (x, y, x + w, y + h);
    extent(taffy, node, x, y, &mut b);
    let shift = t.translate_x.abs().max(t.translate_y.abs()).ceil() + 2.0;
    if crate::renderer::compose::is_translate_only(t) {
        return GeoRect::new(b.0 - shift, b.1 - shift, b.2 - b.0 + shift * 2.0, b.3 - b.1 + shift * 2.0);
    }
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let mut far = 0.0f32;
    ink_far(taffy, node, (x, y, w, h), (cx, cy), &mut far);
    let skew = 1.0 + t.skew_x.to_radians().tan().abs() + t.skew_y.to_radians().tan().abs();
    let r = (far * t.scale_x.abs().max(t.scale_y.abs()).max(1.0) * skew).ceil() + shift;
    GeoRect::new(cx - r, cy - r, r * 2.0, r * 2.0)
}

/// 子树里离 `c` 最远的墨迹点有多远（`n` 的框是 `(x, y, w, h)`，页面坐标）。圆角盒子 = 内缩矩形 ⊕ 圆，
/// 最远点在某个角的圆弧上：角上圆心到 `c` 的距离 + 那个角的半径（直角就是角点本身）
fn ink_far(taffy: &Tree, n: &RenderNode, (x, y, w, h): (f32, f32, f32, f32), c: (f32, f32), far: &mut f32) {
    fn rounded(c: (f32, f32), (x, y, w, h): (f32, f32, f32, f32), r: [f32; 4]) -> f32 {
        let centers = [(x + r[0], y + r[0]), (x + w - r[1], y + r[1]), (x + w - r[2], y + h - r[2]), (x + r[3], y + h - r[3])];
        centers.iter().zip(r).map(|((px, py), r)| ((px - c.0).powi(2) + (py - c.1).powi(2)).sqrt() + r).fold(0.0f32, f32::max)
    }
    let radii = crate::renderer::components::get_border_radii_clamped(&n.style, w, h);
    *far = far.max(rounded(c, (x, y, w, h), radii));
    if let Some(s) = n.style.box_shadow.as_ref().filter(|s| !s.inset) {
        // 与 draw_outer_shadow 同一个形状：按 spread 外扩、半径 = border-radius + spread（不超过短边一半），
        // 模糊在形状外再铺 pad_for(blur) 像素，整块按偏移挪
        let (sw, sh) = ((w + s.spread * 2.0).max(0.0), (h + s.spread * 2.0).max(0.0));
        let sr = (n.style.border_radius + s.spread).max(0.0).min(sw.min(sh) / 2.0);
        let d = rounded(c, (x - s.spread + s.offset_x, y - s.spread + s.offset_y, sw, sh), [sr; 4]);
        *far = far.max(d + crate::renderer::components::shadow::pad_for(s.blur) as f32);
    }
    for ch in &n.children {
        let Ok(l) = taffy.layout(ch.taffy_node) else { continue };
        ink_far(taffy, ch, (x + l.location.x, y + l.location.y, l.size.width, l.size.height), c, far);
    }
}
