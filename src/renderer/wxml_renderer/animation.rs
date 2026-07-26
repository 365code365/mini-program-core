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

    /// 求 `animation="{{animData}}"`（`wx.createAnimation().export()`）此刻的样式覆盖值。
    ///
    /// 载荷形如 `{actions:[{animates:[{type,args}],option:{transition:{duration,delay,timingFunction}}}]}`，
    /// 每个 action 是一「步」：按 delay/duration 顺序播放，步与步之间对目标值做插值。
    /// 起始时刻按载荷指纹记忆 —— 同一份 export 在后续帧里继续播，换了新的 export 就重新开始。
    pub(super) fn js_animated_values(&mut self, node: &RenderNode) -> Option<JsAnimValues> {
        let raw = node.attrs.get("animation")?;
        if !raw.contains("actions") {
            return None;
        }
        // 属性里的对象被序列化成单引号 JSON（见 expr::render_value），这里还原
        let parsed: JsonValue = serde_json::from_str(&raw.replace('\'', "\"")).ok()?;
        let actions = parsed.get("actions")?.as_array()?;
        if actions.is_empty() {
            return None;
        }

        let fingerprint = {
            // FNV-1a：只用来判断「还是不是同一份 export」
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for b in raw.as_bytes() {
                h ^= *b as u64;
                h = h.wrapping_mul(0x1000_0000_01b3);
            }
            h
        };
        let now = self.animation_time();
        let start = match self.js_animations.get(&fingerprint) {
            Some(s) => *s,
            None => {
                if self.js_animations.len() > 64 {
                    self.js_animations.clear();
                }
                self.js_animations.insert(fingerprint, now);
                now
            }
        };
        let elapsed = now - start;

        let sf = self.scale_factor;
        let mut state = JsAnimValues {
            transform: crate::renderer::components::Transform::new(),
            has_transform: false,
            opacity: None,
            background_color: None,
        };
        let mut cursor = 0.0f32;
        let mut finished = true;
        for action in actions {
            let tr = action.get("option").and_then(|o| o.get("transition"));
            let ms = |k: &str, d: f32| {
                tr.and_then(|t| t.get(k)).and_then(|v| v.as_f64()).unwrap_or(d as f64) as f32 / 1000.0
            };
            let duration = ms("duration", 400.0).max(0.0);
            let delay = ms("delay", 0.0).max(0.0);
            let curve = tr
                .and_then(|t| t.get("timingFunction"))
                .and_then(|v| v.as_str())
                .map(crate::renderer::components::parse_timing_function)
                .unwrap_or((0.0, 0.0, 1.0, 1.0)); // createAnimation 默认 linear

            let target = Self::apply_js_animates(&state, action.get("animates"), sf);
            let step_start = cursor + delay;
            let step_end = step_start + duration;
            if elapsed >= step_end || duration <= 0.0 {
                state = target;
                cursor = step_end;
                continue;
            }
            finished = false;
            let p = if elapsed <= step_start {
                0.0
            } else {
                (elapsed - step_start) / duration
            };
            let eased = crate::renderer::anim::cubic_bezier(p, curve.0, curve.1, curve.2, curve.3);
            state = JsAnimValues {
                transform: crate::renderer::anim::lerp_transform(state.transform, target.transform, eased),
                has_transform: state.has_transform || target.has_transform,
                opacity: match (state.opacity, target.opacity) {
                    (Some(a), Some(b)) => Some(a + (b - a) * eased),
                    (None, Some(b)) => Some(1.0 + (b - 1.0) * eased),
                    (a, None) => a,
                },
                background_color: match (state.background_color, target.background_color) {
                    (Some(a), Some(b)) => Some(crate::renderer::anim::lerp_color(a, b, eased)),
                    (None, Some(b)) => Some(b),
                    (a, None) => a,
                },
            };
            break;
        }
        if !finished {
            self.mark_animating();
        }
        Some(state)
    }

    /// 把一步里的 animates 累积到状态上（每步给的是绝对目标值，未提及的属性沿用上一步）
    pub(super) fn apply_js_animates(
        base: &JsAnimValues,
        animates: Option<&JsonValue>,
        sf: f32,
    ) -> JsAnimValues {
        let mut out = base.clone();
        let Some(list) = animates.and_then(|a| a.as_array()) else { return out };
        for item in list {
            let ty = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let args = item.get("args").and_then(|v| v.as_array());
            let num = |i: usize, d: f32| {
                args.and_then(|a| a.get(i))
                    .and_then(|v| v.as_f64())
                    .map(|v| v as f32)
                    .unwrap_or(d)
            };
            match ty {
                // translate 的参数是逻辑 px，Transform 用物理 px
                "translate" => {
                    out.transform.translate_x = num(0, 0.0) * sf;
                    out.transform.translate_y = num(1, 0.0) * sf;
                    out.has_transform = true;
                }
                "translateX" => { out.transform.translate_x = num(0, 0.0) * sf; out.has_transform = true; }
                "translateY" => { out.transform.translate_y = num(0, 0.0) * sf; out.has_transform = true; }
                "rotate" => { out.transform.rotate = num(0, 0.0); out.has_transform = true; }
                "scale" => {
                    let sx = num(0, 1.0);
                    out.transform.scale_x = sx;
                    out.transform.scale_y = num(1, sx);
                    out.has_transform = true;
                }
                "scaleX" => { out.transform.scale_x = num(0, 1.0); out.has_transform = true; }
                "scaleY" => { out.transform.scale_y = num(0, 1.0); out.has_transform = true; }
                "skew" => {
                    out.transform.skew_x = num(0, 0.0);
                    out.transform.skew_y = num(1, 0.0);
                    out.has_transform = true;
                }
                "opacity" => out.opacity = Some(num(0, 1.0).clamp(0.0, 1.0)),
                "backgroundColor" => {
                    if let Some(c) = args
                        .and_then(|a| a.first())
                        .and_then(|v| v.as_str())
                        .and_then(crate::renderer::components::parse_color_str)
                    {
                        out.background_color = Some(c);
                    }
                }
                // width/height 会引发重排，与 CSS 动画同样的取舍：不支持
                _ => {}
            }
        }
        out
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
                let pad = crate::renderer::compose::transform_padding(w, h, &transform).max(2.0);
                self.animated_bounds.push(GeoRect::new(
                    x - pad,
                    y - pad,
                    w + pad * 2.0,
                    h + pad * 2.0,
                ));
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

        let mut offscreen = Canvas::new(tw, th);
        offscreen.clear(Color::TRANSPARENT);
        // 变换后的子树坐标与屏幕不再一一对应：命中区与事件绑定用一次性容器承接，
        // 不污染真实的交互命中表（旋转/缩放子树内的点击是已知限制）。
        let mut scratch = InteractionManager::new();
        let bindings_before = self.event_bindings.len();
        let was_offscreen = self.drawing_offscreen;
        self.drawing_offscreen = true;
        self.dispatch_draw(
            &mut offscreen,
            taffy,
            &resolved,
            pad - layout.location.x,
            pad - layout.location.y,
            &mut scratch,
            kind,
        );
        self.drawing_offscreen = was_offscreen;
        // 离屏那一趟注册的绑定用的是离屏画布的局部坐标，直接留下会命中错位置，先丢掉
        self.event_bindings.truncate(bindings_before);
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
        let mut offscreen = Canvas::new(tw, th);
        offscreen.clear(Color::TRANSPARENT);
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
