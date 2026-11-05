//! Skyline 的列表构造器 `<list-builder>` / `<grid-builder>` 与 `<list-view>` 的布局（官方 component/list-builder、
//! grid-builder、list-view）。
//!
//! - 展开：模板阶段按 `list`（+ `child-count`）把 `slot:item` / `slot:index` 的子模板逐项展开（parser/template/builder.rs）；
//! - 布局：list-builder / list-view 竖排，`padding="[上, 右, 下, 左]"`（px）；list-builder `type="static"`（默认）时
//!   没写高度的项按 `child-height` 定高；grid-builder 与 grid-view 同一套（components::grid_view：
//!   type aligned / masonry、cross-axis-count、max-cross-axis-extent、main-axis-gap、cross-axis-gap、padding）；
//! - `bind:itembuild` / `bind:itemdispose`（detail `{index}`）：按项进出「视口裁剪窗」派发（host/builder_events.rs）。
//!
//! 取舍（写明）：项是**全部**建树、布局的（与 wx:for 同样的代价），没有按需建树 / 回收 ——
//! itembuild / itemdispose 只是按可见窗口派发的通知；`initial-child-count` 不影响建树。
//! 官方只支持放在 `<scroll-view type="custom">` 里，这里放在哪都按同样规则排、窗口按最近的 scroll-view 与页面视口求。
use super::base::*;
use super::grid_view::padding_of;
use std::collections::HashMap;
use taffy::prelude::*;

/// 模板展开时记在构造器节点上的「每项几个根节点」（slot 子模板可以有不止一个根），
/// 渲染器据此把孩子下标换算成项序号
pub const PER_ITEM_ATTR: &str = "__per_item";

pub fn is_builder(tag: &str) -> bool {
    matches!(tag, "list-builder" | "grid-builder")
}

/// 竖排列表的样式（list-builder / list-view）。带孩子时建树重算了基础样式，所以由 `build_tree` 在那之后调
/// （与 grid-view 同一个位置）；孩子的定高也在这里落
pub fn apply_list_layout(tag: &str, attrs: &HashMap<String, String>, st: &mut Style, children: &[RenderNode], taffy: &mut Tree, sf: f32) {
    let pad = padding_of(attrs);
    if pad.iter().any(|p| *p != 0.0) {
        st.padding = Rect { top: length(pad[0] * sf), right: length(pad[1] * sf), bottom: length(pad[2] * sf), left: length(pad[3] * sf) };
    }
    st.flex_direction = FlexDirection::Column;
    let static_h = (tag == "list-builder" && attrs.get("type").map(|t| t.trim() != "dynamic").unwrap_or(true))
        .then(|| attrs.get("child-height").and_then(|v| v.trim().trim_start_matches('[').trim_end_matches(']').trim().parse::<f32>().ok()))
        .flatten()
        .filter(|h| *h > 0.0);
    for c in children {
        let Ok(cs) = taffy.style(c.taffy_node) else { continue };
        let need_h = static_h.is_some() && dim_is_auto(cs.size.height);
        if cs.flex_shrink == 0.0 && !need_h {
            continue;
        }
        let mut cs = cs.clone();
        // 列表项不被列表的高度压扁（与 scroll-view 的孩子一样）
        cs.flex_shrink = 0.0;
        if need_h {
            cs.size.height = length(static_h.unwrap_or(0.0) * sf);
        }
        taffy.set_style(c.taffy_node, cs).ok();
    }
}

/// 构造器节点上每项的根节点数（没记就是 1）
pub fn per_item(attrs: &HashMap<String, String>) -> usize {
    attrs.get(PER_ITEM_ATTR).and_then(|v| v.parse::<usize>().ok()).filter(|n| *n > 0).unwrap_or(1)
}
