//! 横竖两个方向同时滚的 scroll-view（`scroll-x` + `scroll-y`）在渲染器这边的两件事：
//!
//! - 横向那一半的受控位置：`scroll-left` 变了、`scroll-into-view` 变了（横向也对齐到目标），落到 `"{id}#x"` 控制器上；
//! - 子节点的绘制偏移：[`WxmlRenderer::scroll_child_offset`] 给出 `(dx, dy)`，绘制、SelectorQuery、
//!   组件 id 换算（form.rs / builder_window.rs / query.rs）都用它，三处算出来的位置才对得上。
//!
//! 纵向那一半与单向的 scroll-view 完全一样（scroll_views.rs）。

use super::scroll_views::{bool_attr, parse_len, scroll_axes, scroll_both, ScrollViewInfo, SCROLL_WITH_ANIMATION_SECS};
use super::*;
use crate::ui::interaction::{scroll_x_key, InteractionManager};

impl WxmlRenderer {
    /// scroll-view 子节点相对它的内容原点要挪多少（物理像素，向左 / 向上为负）。
    /// 单向的只有一个轴，双向的两个轴都有
    pub(super) fn scroll_child_offset(attrs: &HashMap<String, String>, id: &str, interaction: &InteractionManager, sf: f32) -> (f32, f32) {
        let Some(c) = interaction.get_scroll_controller(id) else { return (0.0, 0.0) };
        let pos = c.get_position();
        if scroll_axes(attrs).0 {
            return (-pos * sf, 0.0);
        }
        let dx = if scroll_both(attrs) { -interaction.scroll_x_of(id) * sf } else { 0.0 };
        (dx, -pos * sf)
    }

    /// 双向 scroll-view 横向的受控位置：`scroll-into-view` 变了（横向也对齐过去）或 `scroll-left` 变了。
    /// 在 `apply_controlled_position` **之前**调：那边会把 `last_into_view` 记成这一帧的值
    pub(super) fn apply_controlled_left(&self, node: &RenderNode, taffy: &Tree, info: &ScrollViewInfo, interaction: &mut InteractionManager) {
        let a = &node.attrs;
        let into = a.get("scroll-into-view").map(|s| s.trim()).filter(|s| !s.is_empty());
        let left = a.get("scroll-left").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let animate = bool_attr(a, "scroll-with-animation");
        let st = interaction.scroll_views.entry(info.component_id.clone()).or_default();
        let first = !st.seen;
        let into_changed = into.is_some() && st.last_into_view.as_deref() != into;
        let left_changed = left.is_some() && st.last_scroll_left != left;
        st.last_scroll_left = left.clone();
        let target = if into_changed {
            into.and_then(|id| self.into_view_left(node, taffy, id, info, interaction))
        } else if left_changed {
            left.as_deref().and_then(|v| parse_len(v, self.screen_width))
        } else {
            None
        };
        let Some(target) = target else { return };
        if let Some(c) = interaction.get_scroll_controller_mut(&scroll_x_key(&info.component_id)) {
            if c.is_dragging {
                return;
            }
            if animate && !first {
                c.animate_to(target, SCROLL_WITH_ANIMATION_SECS);
            } else {
                c.set_position(target);
            }
        }
    }

    /// `scroll-into-view` 横向的目标：与纵向同一套对齐（start / center / end / nearest）。
    /// `scroll-into-view-offset` 只有一个数，只加在纵向上（ScrollViewContext.scrollIntoView 的 offset 同理）
    fn into_view_left(&self, node: &RenderNode, taffy: &Tree, id: &str, info: &ScrollViewInfo, interaction: &InteractionManager) -> Option<f32> {
        let sf = self.scale_factor;
        let (x, _, w, _) = super::scroll_views::find_anchor(&node.children, taffy, id, 0.0, 0.0)?;
        let (start, size, view) = (x / sf, w / sf, info.bounds.width);
        let cur = interaction.scroll_x_of(&info.component_id);
        let target = match node.attrs.get("scroll-into-view-alignment").map(|s| s.trim()).unwrap_or("start") {
            "center" => start + size / 2.0 - view / 2.0,
            "end" => start + size - view,
            "nearest" => {
                if start >= cur && start + size <= cur + view {
                    return None;
                } else if start < cur {
                    start
                } else {
                    start + size - view
                }
            }
            _ => start,
        };
        Some(target)
    }
}
