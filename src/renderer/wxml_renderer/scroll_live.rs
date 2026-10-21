//! 顶层 scroll-view 的「不走缓存」画法：与嵌套 scroll-view 同一套（裁剪 → 按滚动位置偏移 →
//! 只画与视口相交的子节点 → 子节点走带交互的绘制路径）。
//!
//! 顶层 scroll-view 平时把整段内容画进一张离屏缓存、滚动时只 blit（`draw_interactive.rs`）。
//! 但缓存里的 swiper 一换页、一拖动，整块缓存就得重画 —— 那是整段内容高的一次光栅化，
//! 每帧一次。swiper 在 uni-app 产物里很常见（整页套一个 `<scroll-view class="page">`），
//! 所以有 swiper 的顶层 scroll-view 直接改走这条路：每帧只画可见的那几个子节点，
//! swiper 的分页、拖动、指示点、事件绑定都和嵌套时一模一样。
//!
//! 代码与 `draw_child_with_interaction` 里嵌套 scroll-view 那段一致（那段是逐字节基线
//! 守着的老代码，这里另写一份而不是把它挪出来，免得动到没有 swiper 的页面）。

use super::*;

impl WxmlRenderer {
    /// 子树里有没有某个标签（含嵌套 scroll-view 里的：缓存会把它们一起画进去）
    pub(super) fn subtree_has_tag(node: &RenderNode, tag: &str) -> bool {
        node.children.iter().any(|c| c.tag == tag || Self::subtree_has_tag(c, tag))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_scroll_view_live(
        &mut self,
        canvas: &mut Canvas,
        taffy: &Tree,
        node: &RenderNode,
        (x, y, w, h): (f32, f32, f32, f32),
        component_id: &str,
        interaction: &mut InteractionManager,
        scroll_offset: f32,
        viewport_height: f32,
    ) {
        let sf = self.scale_factor;
        let text_color = node.style.text_color.unwrap_or(Color::BLACK);
        let horizontal = super::scroll_views::scroll_axes(&node.attrs).0;
        canvas.save();
        canvas.clip_rect(GeoRect::new(x, y, w, h));
        let pos = interaction.get_scroll_controller(component_id).map(|c| c.get_position()).unwrap_or(0.0);
        if !horizontal && pos < 0.0 {
            self.draw_refresher_gap(canvas, node, component_id, interaction, (x, y, w), -pos * sf);
        }
        let (dx, dy) = if horizontal { (-pos * sf, 0.0) } else { (0.0, -pos * sf) };
        let (view_start, view_len) = (pos * sf, if horizontal { w } else { h });
        for child in &node.children {
            let Ok(l) = taffy.layout(child.taffy_node) else { continue };
            let (s, len) = if horizontal { (l.location.x, l.size.width) } else { (l.location.y, l.size.height) };
            if s + len >= view_start && s <= view_start + view_len {
                self.draw_child_with_interaction(canvas, taffy, child, x + dx, y + dy, text_color, interaction, scroll_offset, viewport_height);
            }
        }
        canvas.restore();
    }
}
