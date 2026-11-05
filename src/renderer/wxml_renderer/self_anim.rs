//! 组件自己带的动画：`<button loading>` 的转圈、`<progress active>` 的进度。
//!
//! 两者都只重画自己的盒子：在动的那一帧 `mark_animating()` 并把盒子记进 `animated_bounds`
//! （与 swiper / CSS 动画节点同一条规则，见 animation.rs），宿主据此只重画这一小块，不整帧。
//! 每帧代价：先按标签判（不是 button / progress 的节点只多一次字符串比较），button 再判 `loading`
//! 属性，progress 再判 `active`；都不带时不查表、不分配。
//!
//! 时钟用渲染器的动画时钟：无头快照 `--time` 定住时钟时转圈停在那一格（像素可比），
//! progress 直接画终态（定住的钟让它永远停在 0 的话基线没法比）。
//!
//! 已知取舍：画进 scroll-view 离屏缓存的这两个组件不动（缓存不逐帧重画），转圈停在第 0 格、
//! progress 直接画终态。顶层页面与逐帧画的容器里都正常动。

use super::*;
use crate::renderer::components::progress_anim;

/// 这一帧组件自己动画的状态（交给 `draw_interactive_component`）
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SelfAnim {
    /// button loading 转圈此刻在第几格（0~11）
    pub spin: Option<u32>,
    /// progress 此刻画到的百分比
    pub percent: Option<f32>,
}

impl WxmlRenderer {
    /// 带交互的两条绘制路径上，button / progress 走到时调一次（画没画都调：盒子在损伤区外时
    /// 也要续上「在动」，否则下一帧宿主不再重画它，转圈就停了）
    pub(super) fn self_anim(&mut self, node: &RenderNode, component_id: &str, (x, y, w, h): (f32, f32, f32, f32)) -> SelfAnim {
        let mut out = SelfAnim::default();
        let moving = match node.tag.as_str() {
            "button" if crate::renderer::components::button_is_loading(node) => {
                // WeUI loading：`steps(12)`，1 秒一圈
                out.spin = Some(((self.animation_time() * 12.0).floor().max(0.0) as u64 % 12) as u32);
                true
            }
            "progress" if node.attrs.contains_key("active") => {
                let live = self.anim_time.is_none() && !self.drawing_offscreen;
                let (p, run) = progress_anim::frame(component_id, &node.attrs, node.style.custom_data, self.animation_time(), live);
                out.percent = Some(p);
                run
            }
            _ => false,
        };
        if moving {
            self.mark_animating();
            if !self.drawing_offscreen && w > 0.0 && h > 0.0 {
                self.animated_bounds.push(GeoRect::new(x - 1.0, y - 1.0, w + 2.0, h + 2.0));
            }
        }
        out
    }
}
