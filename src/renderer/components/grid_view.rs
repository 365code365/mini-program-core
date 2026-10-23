//! `<grid-view>`（Skyline，官方 component/grid-view）：网格（`type="aligned"`，默认）与瀑布流（`type="masonry"`）。
//!
//! - 属性：`cross-axis-count`（列数，默认 2）、`max-cross-axis-extent`（单列最大宽；给了就按容器宽推列数）、
//!   `main-axis-gap`（行间距）、`cross-axis-gap`（列间距）、`padding="[上, 右, 下, 左]"`（px）。
//! - aligned：taffy 的 CSS Grid，`repeat(列数, 1fr)` + 两个 gap，同一行等高（与 Skyline 的对齐网格一致）。
//! - masonry：先按同样的列宽排一遍拿到每一项的自然高度（`align-items: start`，不被同行拉高），
//!   布局之后（`masonry_pass`）把每一项依次放进当前最矮的那一列，改成绝对定位，容器高取最高的那列，再排一遍。
//!   这与 Skyline 瀑布流「按顺序填最短列」的规则一致。
//!
//! 取舍：Skyline 里 grid-view 只能放在 `<scroll-view type="custom">` 里；这里放在哪都按同样规则排。
//! 列数按**容器宽**推（`max-cross-axis-extent`）时，建树期还不知道容器宽，先按屏幕宽算。
use super::base::*;
use super::ViewComponent;
use crate::parser::wxml::WxmlNode;
use taffy::prelude::*;

fn num(attrs: &std::collections::HashMap<String, String>, k: &str) -> Option<f32> {
    let v = attrs.get(k)?;
    let t = v.trim().trim_start_matches("{{").trim_end_matches("}}").trim();
    t.parse::<f32>().ok()
}

/// `padding="[10, 20, 10, 20]"` / `"[10]"` / `"10"` → (上, 右, 下, 左)
pub fn padding_of(attrs: &std::collections::HashMap<String, String>) -> [f32; 4] {
    let Some(raw) = attrs.get("padding") else { return [0.0; 4] };
    let v: Vec<f32> = raw
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter_map(|s| s.parse::<f32>().ok())
        .collect();
    match v.len() {
        0 => [0.0; 4],
        1 => [v[0]; 4],
        2 => [v[0], v[1], v[0], v[1]],
        3 => [v[0], v[1], v[2], v[1]],
        _ => [v[0], v[1], v[2], v[3]],
    }
}

fn columns(attrs: &std::collections::HashMap<String, String>, container_w: f32) -> u16 {
    if let Some(ext) = num(attrs, "max-cross-axis-extent").filter(|e| *e > 0.0) {
        return (container_w / ext).ceil().clamp(1.0, 64.0) as u16;
    }
    num(attrs, "cross-axis-count").map(|n| n.round().clamp(1.0, 64.0) as u16).unwrap_or(2)
}

pub fn is_masonry(attrs: &std::collections::HashMap<String, String>) -> bool {
    attrs.get("type").map(|t| t.trim() == "masonry").unwrap_or(false)
}

pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
    let rn = ViewComponent::build(node, ctx)?;
    if let Ok(st) = ctx.taffy.style(rn.taffy_node) {
        let mut st = st.clone();
        apply_layout(&rn.attrs, &mut st, ctx.scale_factor, ctx.screen_width);
        ctx.taffy.set_style(rn.taffy_node, st).ok();
    }
    Some(rn)
}

/// 把网格语义落到样式上。带子节点时建树会重算一遍基础样式（覆盖 build 里设的），
/// 所以 `build_tree` 在那之后也调一次（与 swiper / picker-view 同一个位置）
pub fn apply_layout(attrs: &std::collections::HashMap<String, String>, st: &mut Style, sf: f32, screen_w: f32) {
    let pad = padding_of(attrs);
    let cols = columns(attrs, screen_w - pad[1] - pad[3]);
    let main_gap = num(attrs, "main-axis-gap").unwrap_or(0.0).max(0.0) * sf;
    let cross_gap = num(attrs, "cross-axis-gap").unwrap_or(0.0).max(0.0) * sf;
    st.display = Display::Grid;
    st.grid_template_columns = taffy::style_helpers::evenly_sized_tracks(cols);
    st.gap = Size { width: length(cross_gap), height: length(main_gap) };
    if pad.iter().any(|p| *p > 0.0) {
        st.padding = Rect { top: length(pad[0] * sf), right: length(pad[1] * sf), bottom: length(pad[2] * sf), left: length(pad[3] * sf) };
    }
    if is_masonry(attrs) {
        st.align_items = Some(AlignItems::START);
    }
}

/// 布局之后：把瀑布流 grid-view 的子项放进最矮的列。有改动返回 true（调用方再排一遍）
pub fn masonry_pass(taffy: &mut Tree, nodes: &[RenderNode], sf: f32) -> bool {
    let mut changed = false;
    for n in nodes {
        if n.tag == "grid-view" && is_masonry(&n.attrs) && !n.children.is_empty() {
            changed |= place(taffy, n, sf);
        }
        changed |= masonry_pass(taffy, &n.children, sf);
    }
    changed
}

fn place(taffy: &mut Tree, n: &RenderNode, sf: f32) -> bool {
    let Ok(lay) = taffy.layout(n.taffy_node).cloned() else { return false };
    let pad = padding_of(&n.attrs).map(|p| p * sf);
    let inner_w = lay.size.width - pad[1] - pad[3];
    let cols = columns(&n.attrs, inner_w / sf) as usize;
    let main_gap = num(&n.attrs, "main-axis-gap").unwrap_or(0.0).max(0.0) * sf;
    let cross_gap = num(&n.attrs, "cross-axis-gap").unwrap_or(0.0).max(0.0) * sf;
    let col_w = ((inner_w - cross_gap * (cols as f32 - 1.0)) / cols as f32).max(0.0);
    let mut heights = vec![0.0f32; cols];
    for c in &n.children {
        let Ok(cl) = taffy.layout(c.taffy_node) else { continue };
        // 摆的是**外边距盒**（与对齐网格里一格装下 margin 一致）：绝对定位的 inset 定的是外边距边，
        // 宽度要扣掉左右外边距，占的列高要算上上下外边距 —— 不扣的话带 margin 的项整体往右下错开、伸出容器
        let m = cl.margin;
        let h = cl.size.height + m.top + m.bottom;
        let (col, top) = heights.iter().enumerate().fold((0, f32::MAX), |acc, (i, v)| if *v < acc.1 { (i, *v) } else { acc });
        let Ok(st) = taffy.style(c.taffy_node) else { continue };
        let mut st = st.clone();
        st.position = Position::Absolute;
        st.inset = Rect {
            left: length(pad[3] + col as f32 * (col_w + cross_gap)),
            top: length(pad[0] + top),
            right: auto(),
            bottom: auto(),
        };
        st.size.width = length((col_w - m.left - m.right).max(0.0));
        taffy.set_style(c.taffy_node, st).ok();
        heights[col] = top + h + main_gap;
    }
    let content = heights.iter().cloned().fold(0.0, f32::max) - if n.children.is_empty() { 0.0 } else { main_gap };
    if let Ok(st) = taffy.style(n.taffy_node) {
        let mut st = st.clone();
        st.display = Display::Block;
        st.size.height = length(content.max(0.0) + pad[0] + pad[2]);
        taffy.set_style(n.taffy_node, st).ok();
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padding_forms() {
        let mut a = std::collections::HashMap::new();
        a.insert("padding".to_string(), "[10, 20, 30, 40]".to_string());
        assert_eq!(padding_of(&a), [10.0, 20.0, 30.0, 40.0]);
        a.insert("padding".to_string(), "[8]".to_string());
        assert_eq!(padding_of(&a), [8.0; 4]);
        a.insert("cross-axis-count".to_string(), "3".to_string());
        assert_eq!(columns(&a, 375.0), 3);
        a.insert("max-cross-axis-extent".to_string(), "100".to_string());
        assert_eq!(columns(&a, 375.0), 4, "按单列最大宽推列数");
    }
}
