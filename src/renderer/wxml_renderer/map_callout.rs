//! `<map>` 的自定义气泡（官方 component/map「marker 上的自定义气泡 customCallout」）：
//! `<map>` 下 `slot="callout"` 的 cover-view 里，带 `marker-id` 的每个子节点就是那个标记的气泡内容。
//!
//! 地图由宿主画（同层渲染，`components::host_view`），标记在屏上的位置只有宿主知道，所以分两半：
//! - **布局**（[`place_slot`]）：slot 节点不占流、挪到画面外（页面上不画、点不到 —— 微信里它也只当气泡显示）；
//!   里面每个带 marker-id 的子节点各自绝对定位，宽高按自己的内容（一个气泡一张图）。
//! - **出图**（[`WxmlRenderer::map_custom_callouts`]）：按 `<snapshot>` 那条静态路径把每个气泡离屏画成 RGBA，
//!   宿主（`host::engine_map_callouts`）存成 PNG 交给地图，地图按 marker 的 `customCallout`（display / anchorX / anchorY）
//!   贴在标记上方。
//!
//! 取舍：气泡内容只在布局树整棵重建时重新出图（与 `sameLayerViews` 同一个时机）；只改了气泡里文字的 setData
//! 走增量布局，宿主手里还是旧图。气泡里的点击不分子节点，整块算 `callouttap`（宿主判）。
use super::*;

/// 挪出画面的距离（逻辑像素 × sf 之后也远远在画布外）
const OFFSCREEN: f32 = -100_000.0;

fn is_slot(n: &RenderNode) -> bool {
    n.attrs.get("slot").map(|s| s.trim() == "callout").unwrap_or(false)
}

fn set_abs(taffy: &mut Tree, node: taffy::NodeId, left: f32) {
    if let Ok(mut st) = taffy.style(node).cloned() {
        st.position = taffy::style::Position::Absolute;
        st.inset.left = taffy::style::LengthPercentageAuto::length(left);
        st.inset.top = taffy::style::LengthPercentageAuto::length(0.0);
        st.inset.right = taffy::style::LengthPercentageAuto::auto();
        st.inset.bottom = taffy::style::LengthPercentageAuto::auto();
        let _ = taffy.set_style(node, st);
    }
}

/// 布局时（`layout.rs` 建 `<map>` 的子节点之后）：见模块注释
pub(super) fn place_slot(children: &[RenderNode], taffy: &mut Tree) {
    for c in children.iter().filter(|c| is_slot(c)) {
        set_abs(taffy, c.taffy_node, OFFSCREEN);
        for m in c.children.iter().filter(|m| m.attrs.contains_key("marker-id")) {
            set_abs(taffy, m.taffy_node, 0.0);
        }
    }
}

fn find_map<'a>(nodes: &'a [RenderNode], key: &str) -> Option<&'a RenderNode> {
    for n in nodes {
        if n.tag == "map" && crate::renderer::components::host_view::key("map", &n.attrs) == key {
            return Some(n);
        }
        if let Some(f) = find_map(&n.children, key) {
            return Some(f);
        }
    }
    None
}

impl WxmlRenderer {
    /// 键为 [key] 的同层 `<map>` 的自定义气泡：(marker-id, 宽, 高, RGBA)，物理像素。没有 `slot="callout"` 的给空
    pub fn map_custom_callouts(&mut self, key: &str) -> Vec<(i64, u32, u32, Vec<u8>)> {
        let Some(cache) = self.cache.take() else { return Vec::new() };
        let mut out = Vec::new();
        if let Some(map) = find_map(&cache.render_nodes, key) {
            for slot in map.children.iter().filter(|c| is_slot(c)) {
                for m in &slot.children {
                    let Some(id) = m.attrs.get("marker-id").and_then(|s| s.trim().parse::<f64>().ok()) else { continue };
                    if let Ok((w, h, px)) = self.draw_snapshot(&cache.taffy, m) {
                        out.push((id as i64, w, h, px));
                    }
                }
            }
        }
        self.cache = Some(cache);
        out
    }
}
