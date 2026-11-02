//! 交互态样式：按压（CSS `:active` / 小程序 `hover-class`）与 switch 拨动。
//!
//! 按压态的整套样式在建树期就算好了（见 `components::base`），这里只负责
//! 「当前该用常态还是按压态、有 `transition` 时插值到几分」。
//! 代价是按压只影响绘制类属性（背景/颜色/透明度/transform）——
//! 按一下就重排整页在纯软件光栅上太贵。

use super::*;

impl WxmlRenderer {
    /// 开关类组件（switch）的状态过渡进度：点击后在 `TOGGLE_DURATION` 内从旧值滑到新值。
    ///
    /// 微信的开关滑块是带缓动的位移（weui 用 cubic-bezier(.4,.4,.25,1.35) 略带回弹），
    /// 直接跳变会失去"拨动"手感。
    ///
    /// `bounds`：开关的框（画布坐标）。在滑的时候记进动画包围盒，宿主才重画这一块 ——
    /// 页面上还有别的动画在记包围盒时，不记的话开关那块没人重画，滑块停在半路、最后一跳到位。
    pub(super) fn toggle_progress(
        &mut self,
        interaction: &InteractionManager,
        id: &str,
        checked: bool,
        bounds: (f32, f32, f32, f32),
    ) -> f32 {
        const TOGGLE_DURATION: f32 = 0.3;
        // 拨动时刻是两帧之间按墙上时钟记的，而帧时刻是 vsync 时刻、落后墙上时钟最多
        // `RESYNC_MS`（frame_clock::Anchor）：抬手后的第一帧 elapsed 常是负的几毫秒。
        // 从前负的一律给终态 —— 开关先「啪」地跳到开、下一帧回到关、再滑过去（iOS 上「打开时抖一下」）。
        // 这个范围里的负数按「刚开始」算；更负的是动画钟被定住了（无头快照 `--time`），照旧给终态
        const CLOCK_LAG: f32 = crate::frame_clock::RESYNC_MS as f32 / 1000.0;
        let target = if checked { 1.0 } else { 0.0 };
        // 只认「真正拨动」的时刻；按压起点（`transitions`）不算，否则按一下就假拨一次
        let Some(started) = interaction.toggled_at.get(id).copied() else { return target };
        let elapsed = self.animation_time() - started;
        let live_clock = self.anim_time.is_none();
        if elapsed >= TOGGLE_DURATION || (elapsed < 0.0 && !(live_clock && elapsed > -CLOCK_LAG)) {
            return target;
        }
        let elapsed = elapsed.max(0.0);
        self.mark_animating();
        if !self.drawing_offscreen {
            // 滑块投影 0 2px 3px：往下、往外各多出 5 个逻辑像素
            let pad = 6.0 * self.scale_factor;
            let (x, y, w, h) = bounds;
            self.animated_bounds.push(GeoRect::new(x - pad, y - pad, w + pad * 2.0, h + pad * 2.0));
        }
        let eased = crate::renderer::anim::cubic_bezier(elapsed / TOGGLE_DURATION, 0.4, 0.4, 0.25, 1.35);
        if checked { eased.clamp(0.0, 1.0) } else { (1.0 - eased).clamp(0.0, 1.0) }
    }

    /// 按压态过渡进度：0 = 常态，1 = 按压态。
    ///
    /// 有 `transition` 时按缓动插值，没有就直接给终态。
    /// 插值期间标记「本帧有动画」，宿主才会继续出帧把过渡走完。
    pub(super) fn pressed_progress(
        &mut self,
        interaction: &InteractionManager,
        component_id: &str,
        spec: Option<crate::renderer::components::TransitionSpec>,
    ) -> f32 {
        let is_pressed = interaction.is_button_pressed(component_id);
        match spec {
            Some(t) if t.duration > 0.0 => {
                match interaction.transitions.get(component_id).copied() {
                    Some(start) => {
                        let elapsed = self.animation_time() - start - t.delay;
                        let raw = (elapsed / t.duration).clamp(0.0, 1.0);
                        if elapsed < t.duration {
                            self.mark_animating();
                        }
                        let eased = crate::renderer::anim::cubic_bezier(
                            raw, t.curve.0, t.curve.1, t.curve.2, t.curve.3,
                        );
                        if is_pressed { eased } else { 1.0 - eased }
                    }
                    // 没有记录过起始时刻：直接给终态
                    None => if is_pressed { 1.0 } else { 0.0 },
                }
            }
            _ => if is_pressed { 1.0 } else { 0.0 },
        }
    }

    /// 把按压态样式按给定进度落到节点上（只动绘制类属性：背景/文字/边框/透明度/transform）。
    pub(super) fn bake_pressed_style(
        node_to_draw: &mut RenderNode,
        pressed: &crate::renderer::components::NodeStyle,
        progress: f32,
    ) {
        if progress <= 0.001 {
            return;
        }
        if progress >= 0.999 {
            let keep = node_to_draw.style.pressed_style.take();
            let transition = node_to_draw.style.transition;
            node_to_draw.style = pressed.clone();
            node_to_draw.style.pressed_style = keep;
            node_to_draw.style.transition = transition;
            return;
        }
        let base = &mut node_to_draw.style;
        base.background_color = lerp_color_opt(base.background_color, pressed.background_color, progress);
        base.text_color = lerp_color_opt(base.text_color, pressed.text_color, progress);
        base.border_color = lerp_color_opt(base.border_color, pressed.border_color, progress);
        base.opacity += (pressed.opacity - base.opacity) * progress;
        if pressed.transform.is_some() || base.transform.is_some() {
            let from = base.transform.unwrap_or_else(crate::renderer::components::Transform::new);
            let to = pressed.transform.unwrap_or_else(crate::renderer::components::Transform::new);
            base.transform = Some(crate::renderer::anim::lerp_transform(from, to, progress));
        }
    }

    /// 把按压态样式应用到待绘制节点上（绘制路径调用）。
    pub(super) fn apply_pressed_style(
        &mut self,
        node_to_draw: &mut RenderNode,
        interaction: &InteractionManager,
        component_id: &str,
    ) {
        let Some(pressed) = node_to_draw.style.pressed_style.clone() else { return };
        let spec = node_to_draw.style.transition;
        let progress = self.pressed_progress(interaction, component_id, spec);
        Self::bake_pressed_style(node_to_draw, &pressed, progress);
    }

    /// 按压态里带 `transform`（`:active { transform: scale(.96) }` 这种）时，
    /// 提前把整份按压态**烘焙**进一个临时节点，交给 transform 合成路径。
    ///
    /// 不能留给绘制期的 `apply_pressed_style`：那时已经错过了离屏仿射入口，
    /// scale/rotate 会被直接丢掉（表现为「按压只变色不缩放」）。
    /// 烘焙时同时清掉 `pressed_style`，因为离屏重绘用的是画布局部坐标，
    /// `get_component_id` 算出来的 id 与命中表里的不一致，按 id 再查一次会查不到，
    /// 按压态会整体丢失。
    pub(super) fn bake_pressed_for_transform(
        &mut self,
        taffy: &Tree,
        node: &RenderNode,
        ox: f32,
        oy: f32,
        interaction: &InteractionManager,
    ) -> Option<RenderNode> {
        let pressed = node.style.pressed_style.as_ref()?;
        if pressed.transform.is_none() && node.style.transform.is_none() {
            return None;
        }
        if pressed.transform == node.style.transform {
            return None;
        }
        let layout = taffy.layout(node.taffy_node).ok()?;
        let sf = self.scale_factor;
        let bounds = GeoRect::new(
            (ox + layout.location.x) / sf,
            (oy + layout.location.y) / sf,
            layout.size.width / sf,
            layout.size.height / sf,
        );
        let component_id = Self::get_component_id(node, &bounds);
        let progress = self.pressed_progress(interaction, &component_id, node.style.transition);
        if progress <= 0.001 {
            return None;
        }
        let pressed = pressed.clone();
        let mut baked = node.clone();
        Self::bake_pressed_style(&mut baked, &pressed, progress);
        baked.style.pressed_style = None;
        Some(baked)
    }

}
