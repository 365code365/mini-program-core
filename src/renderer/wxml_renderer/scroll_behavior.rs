//! scroll-view 登记时顺手落到滚动控制器上的手感属性（官方 component/scroll-view.html）：
//! - `bounces`（默认 true）、`paging-enabled`、`fast-deceleration`：属性与 ScrollViewContext 的写值（Context 优先）
//!   交给控制器（`ui/scroll_controller/behavior.rs`）。官方写「同时开启 enhanced 后生效」，这里写了就生效（Skyline 没有 enhanced 一说）；
//! - `show-scrollbar`（默认 true，**只在 enhanced 时**画，见 scrollbar.rs —— 没开 enhanced 的 scroll-view 一个像素都不变）；
//! - `min-drag-distance`：手指移动到这么远才开始滚（host/gesture.rs，只对写了的生效）；
//! - `enable-back-to-top`：没显式写 false 且盒子占屏 90% 以上时自动开（官方 2.27.3 起）；
//! - `scroll-anchoring`（Skyline 3.6.2 起默认 true）：视口上方插入 / 删掉内容时，视口里第一项留在原处。
//!   只认带身份的项（`id` 或 `wx:key`）——没身份的项分不出「是不是同一项」，不动；手指拖着、这一帧受控位置变了时不动；
//! - `reverse`：初始停在末尾（scroll_views.rs），内容变长时原本贴着底的接着贴底（聊天列表来新消息）。
//!
//! 每帧代价（每个画到的 scroll-view）：几个属性查找 + 控制器字段赋值；scroll-anchoring 在孩子里二分找视口里第一项。
use super::*;

/// 属性：写了按真假，没写取默认
fn attr_or(a: &HashMap<String, String>, k: &str, d: bool) -> bool {
    a.get(k).map(|v| crate::renderer::components::attr_is_true(v)).unwrap_or(d)
}

/// 孩子的身份（跨 setData 认得出是同一项）
fn identity(n: &RenderNode) -> Option<&str> {
    n.attrs
        .get("id")
        .filter(|s| !s.is_empty())
        .or_else(|| n.attrs.get(crate::parser::template::KEYED_ID_ATTR))
        .map(String::as_str)
}

impl WxmlRenderer {
    /// 手感开关落到控制器上；返回要不要画滚动条（`enhanced` + `show-scrollbar`）
    pub(super) fn apply_scroll_behavior(&self, node: &RenderNode, component_id: &str, interaction: &mut InteractionManager) -> bool {
        let a = &node.attrs;
        let sw = interaction.scroll_views.get(component_id).map(|s| s.ctx).unwrap_or_default();
        let enhanced = super::scroll_views::bool_attr(a, "enhanced");
        let bounces = sw.bounces.unwrap_or_else(|| attr_or(a, "bounces", true));
        let paging = sw.paging_enabled.unwrap_or_else(|| attr_or(a, "paging-enabled", false));
        let fast = sw.fast_deceleration.unwrap_or_else(|| attr_or(a, "fast-deceleration", false));
        let no_inertia = sw.deceleration_disabled.unwrap_or(false);
        if let Some(c) = interaction.get_scroll_controller_mut(component_id) {
            c.set_bounces(bounces);
            c.set_behavior(paging, fast, no_inertia);
        }
        // 横竖都能滚的横向控制器：同一套手感（ui/interaction/scroll_2d.rs；单向的是一次判空）
        if interaction.scroll_is_2d(component_id) {
            if let Some(c) = interaction.get_scroll_controller_mut(&crate::ui::interaction::scroll_x_key(component_id)) {
                c.set_bounces(bounces);
                c.set_behavior(paging, fast, no_inertia);
            }
        }
        // `min-drag-distance`（Skyline，官方默认 18）：只认写了的 —— 没写的 scroll-view 沿用宿主的方向锁定距离（4px），
        // 手感与从前一样（host/gesture.rs 读它）
        let min_drag = a.get("min-drag-distance").and_then(|v| super::scroll_views::parse_len(v, self.screen_width)).unwrap_or(0.0).max(0.0);
        if min_drag > 0.0 || interaction.scroll_views.get(component_id).is_some_and(|s| s.min_drag > 0.0) {
            if !interaction.scroll_views.contains_key(component_id) {
                interaction.scroll_views.insert(component_id.to_string(), Default::default());
            }
            if let Some(st) = interaction.scroll_views.get_mut(component_id) {
                st.min_drag = min_drag;
            }
        }
        sw.show_scrollbar.unwrap_or(enhanced && attr_or(a, "show-scrollbar", true))
    }

    /// `enable-back-to-top`：显式写了按写的；没写时盒子占屏 90% 以上自动开（只对纵向）
    pub(super) fn back_to_top(&self, a: &HashMap<String, String>, bounds: &GeoRect, horizontal: bool) -> bool {
        if horizontal {
            return false;
        }
        match a.get("enable-back-to-top") {
            Some(v) => crate::renderer::components::attr_is_true(v),
            None => bounds.width * bounds.height >= 0.9 * self.screen_width * self.screen_height,
        }
    }

    /// scroll-anchoring 与 reverse 贴底：受控位置（scroll-top / scroll-into-view）落完之后调。
    /// `controlled`：这一帧受控位置动过（那就以它为准，不另外挪）
    pub(super) fn keep_scroll_anchor(&self, node: &RenderNode, taffy: &Tree, component_id: &str, horizontal: bool, controlled: bool, interaction: &mut InteractionManager) {
        let a = &node.attrs;
        let anchoring = attr_or(a, "scroll-anchoring", true) && !horizontal;
        let reverse = super::scroll_views::bool_attr(a, "reverse");
        if !anchoring && !reverse {
            return;
        }
        let Some((pos, max, dragging)) = interaction.get_scroll_controller(component_id).map(|c| (c.get_position(), c.get_max_scroll(), c.is_dragging)) else {
            return;
        };
        let sf = self.scale_factor;
        let mut target = None;
        // 不用 entry(id.to_string())：每帧都会分配一个键（host/scroll_view.rs `state` 同理）
        if !interaction.scroll_views.contains_key(component_id) {
            interaction.scroll_views.insert(component_id.to_string(), Default::default());
        }
        let Some(st) = interaction.scroll_views.get_mut(component_id) else { return };
        // reverse：上一帧贴着底、内容变长了 —— 接着贴底
        if reverse && !controlled && !dragging && st.last_max > 0.0 && max > st.last_max + 0.5 && pos >= st.last_max - 1.0 {
            target = Some(max);
        }
        st.last_max = max;
        if anchoring && target.is_none() {
            // 视口里第一项（孩子按 y 排好的：二分）
            let top = pos * sf;
            let kids = &node.children;
            let i = kids.partition_point(|c| taffy.layout(c.taffy_node).map(|l| l.location.y + l.size.height <= top).unwrap_or(true));
            let now = kids.get(i).and_then(|c| Some((identity(c)?, taffy.layout(c.taffy_node).ok()?.location.y)));
            let mut re_anchor = None;
            if let Some((old_id, old_y)) = st.anchor.as_ref() {
                // 快路：视口里第一项还是它、位置没变（绝大多数帧）
                let same = now.is_some_and(|(id, y)| id == old_id && (y - old_y).abs() < 0.01);
                if !same && !controlled && !dragging && pos > 0.0 {
                    // 锚点那一项挪到哪了（滚过一项时也走这里：它没挪，dy = 0，换锚点）
                    let moved = kids.iter().find(|c| identity(c) == Some(old_id.as_str())).and_then(|c| taffy.layout(c.taffy_node).ok()).map(|l| l.location.y);
                    if let Some(y) = moved {
                        let dy = (y - old_y) / sf;
                        if dy.abs() > 0.01 {
                            target = Some(pos + dy);
                            re_anchor = Some(y);
                        }
                    }
                }
            }
            // 挪过了：锚点还是那一项，记它的新位置
            if let (Some(y), Some(a)) = (re_anchor, st.anchor.as_mut()) {
                a.1 = y;
            }
            if target.is_none() {
                match now {
                    Some((id, y)) => match &mut st.anchor {
                        Some((old, oy)) if old == id => *oy = y,
                        slot => *slot = Some((id.to_string(), y)),
                    },
                    None => st.anchor = None,
                }
            }
        }
        if let Some(t) = target {
            if let Some(c) = interaction.get_scroll_controller_mut(component_id) {
                c.set_position(t);
            }
        }
    }
}
