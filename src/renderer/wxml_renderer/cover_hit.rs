//! 同层组件（`<map>` 这类）在某一点上**有没有被页面自己的元素盖住**。
//!
//! 地图铺满屏、上面绝对定位一圈 cover-view 按钮（钱包 / 定位 / 扫码开锁）是地图类小程序的标准写法。
//! 按下点落在地图框里不等于按在地图上：盖在上面的按钮先收（微信里 cover-view 就是盖在地图上的那层）。
//! 从前只看框（`engine_host_views::gesture_view_at`），宿主点名 `sameLayerGestures` 之后整串触摸都给了地图，
//! 真机上地图上的按钮一个都点不动；桌面不点名手势，所以无头全绿。
//!
//! ## 怎么判「在上面」
//! 正常流的绘制顺序就是树的先序（`draw_interactive` 不按 z-index 给兄弟排序，只有 fixed 层排），所以：
//! 先序里排在组件**之后**、又不是它祖先的节点（含它自己的子节点），框（再按祖先的 overflow 裁）盖住这一点就算盖住。
//! 与浏览器的命中一致：盖在上面的元素没绑事件也照样挡（点在说明卡片上不该拖动地图）。
//! - 正常流的组件：fixed 子树不在这里看 —— 调用方先问 `fixed_layer_hit`（覆盖层整块都在正常流上面）。
//! - fixed 层里的组件：只看排在它后面的 fixed 节点（覆盖层之间按 z-index 排，这里按文档顺序近似）。
//!
//! 坐标推法与 `query.rs` 的 `walk_query` 一致（fixed 根的钉住原点、scroll-view 偏移、swiper 翻页），
//! 组件按 `get_component_id` 认 —— 与 `QueryHit.component_id` 同一个算式，改一边要改另一边。
use super::*;

struct CoverWalk<'a> {
    target: &'a str,
    /// 正常流的组件给内容坐标，fixed 的给视口坐标（与 `QueryHit.rect` 同一坐标系）
    p: crate::Point,
    /// 已经走过组件本身：之后的节点画在它上面
    target_fixed: Option<bool>,
    covered: bool,
}

impl WxmlRenderer {
    /// `component_id` 那个节点在 `p` 处是否被画在它之后的节点盖住。还没布局过、找不到这个节点都算没盖住
    pub fn covered_at(&self, component_id: &str, p: crate::Point, interaction: &InteractionManager) -> bool {
        let Some(cache) = self.cache.as_ref() else { return false };
        let mut w = CoverWalk { target: component_id, p, target_fixed: None, covered: false };
        for n in &cache.render_nodes {
            self.walk_cover(n, &cache.taffy, (0.0, 0.0), false, None, interaction, &mut w);
            if w.covered {
                break;
            }
        }
        w.covered
    }

    #[allow(clippy::too_many_arguments)]
    fn walk_cover(
        &self,
        n: &RenderNode,
        taffy: &Tree,
        origin: (f32, f32),
        in_fixed: bool,
        clip: Option<GeoRect>,
        interaction: &InteractionManager,
        w: &mut CoverWalk,
    ) {
        let Ok(l) = taffy.layout(n.taffy_node) else { return };
        let sf = self.scale_factor;
        let pinned = (n.style.is_fixed && !in_fixed && self.fixed_origins.0 == self.layout_generation)
            .then(|| self.fixed_origins.1.get(&n.taffy_node).copied())
            .flatten();
        let (x, y) = match pinned {
            Some((bx, by)) => (bx + l.location.x, by + l.location.y),
            None => (origin.0 + l.location.x, origin.1 + l.location.y),
        };
        let fixed = in_fixed || n.style.is_fixed;
        // fixed 根跳出祖先的裁剪（它按视口定位）
        let clip = if n.style.is_fixed && !in_fixed { None } else { clip };
        let rect = GeoRect::new(x / sf, y / sf, l.size.width / sf, l.size.height / sf);
        match w.target_fixed {
            None => {
                if Self::get_component_id(n, &rect) == w.target {
                    w.target_fixed = Some(fixed);
                }
            }
            // 组件之后画的、同一层（正常流 / 覆盖层）的节点
            Some(tf) if tf == fixed => {
                if rect.contains(&w.p) && clip.is_none_or(|c| c.contains(&w.p)) {
                    w.covered = true;
                    return;
                }
            }
            Some(_) => {}
        }
        let clips = n.tag == "scroll-view" || n.tag == "swiper" || n.style.overflow != crate::renderer::components::Overflow::Visible;
        let child_clip = if clips { Some(clip.map_or(rect, |c| intersect(c, rect))) } else { clip };
        // scroll-view 的子节点随它的滚动偏移移动（与 query.rs 同一个算式）
        let (mut cx, mut cy) = (x, y);
        if n.tag == "scroll-view" {
            let id = Self::get_component_id(n, &rect);
            if let Some(c) = interaction.get_scroll_controller(&id) {
                if super::scroll_views::scroll_axes(&n.attrs).0 {
                    cx -= c.get_position() * sf;
                } else {
                    cy -= c.get_position() * sf;
                    if super::scroll_views::scroll_both(&n.attrs) {
                        cx -= interaction.scroll_x_of(&id) * sf;
                    }
                }
            }
        }
        let swiper_id = (n.tag == "swiper").then(|| Self::get_component_id(n, &rect));
        for (i, c) in n.children.iter().enumerate() {
            let (dx, dy) = swiper_id.as_deref().map(|id| self.swiper_item_offset(taffy, n, id, i)).unwrap_or((0.0, 0.0));
            self.walk_cover(c, taffy, (cx + dx, cy + dy), fixed, child_clip, interaction, w);
            if w.covered {
                return;
            }
        }
    }
}

fn intersect(a: GeoRect, b: GeoRect) -> GeoRect {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = (a.x + a.width).min(b.x + b.width);
    let y1 = (a.y + a.height).min(b.y + b.height);
    GeoRect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
}
