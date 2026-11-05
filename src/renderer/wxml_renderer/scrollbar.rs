//! scroll-view 的滚动条（`enhanced` + `show-scrollbar`，或 ScrollViewContext 写了 `showScrollbar = true`）。
//!
//! 外观按 iOS 的滚动指示条：3px 宽的圆角条、黑色 35% 透明度，离边 2px；长度 = 视口 / 内容 × 视口（最短 20px）。
//! 滚动时出现，停下 0.5s 后用 0.3s 渐隐（官方没写时长，待验；Android 真机上是系统滚动条的样子，没做）。
//!
//! **只重画滚动条那一条**：scroll-view 在动的帧它整个盒子本来就在重画（损伤区 = 盒子），滚动条画在内容之上不另加；
//! 停下之后保持 / 渐隐的那几帧把滚动条的框记进动画包围盒（与 self_anim / swiper 同一条规则），宿主只清、只画这一条。
//! 没开的 scroll-view 每帧只多一次登记表查找，一个像素都不变。
use super::*;

const HOLD_SECS: f32 = 0.5;
const FADE_SECS: f32 = 0.3;
/// 逻辑 px
const THICK: f32 = 3.0;
const INSET: f32 = 2.0;
const MIN_LEN: f32 = 20.0;

impl WxmlRenderer {
    /// scroll-view 的内容画完之后调（与 `draw_scroll_stickies` 同几处）
    pub(super) fn draw_scrollbar(&mut self, canvas: &mut Canvas, component_id: &str, (x, y, w, h): (f32, f32, f32, f32), interaction: &mut InteractionManager) {
        let Some(horizontal) = self.scroll_views.iter().find(|s| s.component_id == component_id && s.scrollbar).map(|s| s.horizontal) else {
            return;
        };
        if self.drawing_offscreen {
            return;
        }
        let Some((pos, max, moving)) = interaction.get_scroll_controller(component_id).map(|c| (c.get_position(), c.get_max_scroll(), c.is_dragging || c.is_animating())) else {
            return;
        };
        if max <= 0.5 {
            return;
        }
        let now = self.animation_time();
        let Some(st) = interaction.scroll_views.get_mut(component_id) else { return };
        // 默认 (0, 0)：还没动过（打开页面时不闪一下）
        let never = st.bar == (0.0, 0.0);
        if moving || (pos - st.bar.0).abs() > 0.01 {
            st.bar = (pos, now);
        } else if never {
            return;
        }
        let age = (now - st.bar.1).max(0.0);
        let alpha = if age < HOLD_SECS { 1.0 } else { (1.0 - (age - HOLD_SECS) / FADE_SECS).clamp(0.0, 1.0) };
        if alpha <= 0.0 {
            return;
        }
        let sf = self.scale_factor;
        let view = if horizontal { w } else { h };
        let content = view + max * sf;
        let track = view - 2.0 * INSET * sf;
        // 越界（橡皮筋）时条变短，与 iOS 一样
        let over = if pos < 0.0 { -pos } else { (pos - max).max(0.0) } * sf;
        let len = (view * view / content - over).max(MIN_LEN * sf).min(track);
        let p = (pos / max).clamp(0.0, 1.0);
        let off = INSET * sf + p * (track - len);
        let t = THICK * sf;
        let r = if horizontal {
            GeoRect::new(x + off, y + h - (INSET + THICK) * sf, len, t)
        } else {
            GeoRect::new(x + w - (INSET + THICK) * sf, y + off, t, len)
        };
        canvas.save();
        canvas.clip_rect(GeoRect::new(x, y, w, h));
        canvas.fill_round_rect(r.x, r.y, r.width, r.height, t / 2.0, Color::new(0, 0, 0, (0.35 * alpha * 255.0).round() as u8));
        canvas.restore();
        // 还亮着 / 在渐隐：要下一帧，宿主只重画这一条
        self.mark_animating();
        self.animated_bounds.push(GeoRect::new(r.x - 1.0, r.y - 1.0, r.width + 2.0, r.height + 2.0));
    }
}
