//! Skyline 的吸顶组件：`<sticky-header>` / `<sticky-section>`，以及 `type="nested"` 的
//! `<nested-scroll-header>` / `<nested-scroll-body>`（后两个是普通容器，衔接在宿主的手势里）。
//!
//! 官方 component/sticky-header.html、sticky-section.html：
//! - sticky-header：`offset-top`（number，默认 0，吸顶时离视窗顶多少 px）、`allow-overlapping`
//!   （默认 false：与前一个 sticky-header 不重叠 —— 后一个顶上来时把前一个推走）、`padding`（长度 4 的数组，
//!   上右下左，px）、`bind:stickontopchange`（`{isStickOnTop}`，状态变了才发）；
//! - sticky-section：`push-pinned-header`（默认 true：吸住的头被本节底边推走）、`padding`。
//!
//! 这里只把它们变成「带 `is_sticky` 的容器」：吸住、被推走、只重画自己那一块都在
//! `wxml_renderer/sticky.rs`，与 CSS `position: sticky` 同一条路。
use super::base::*;
use super::view::ViewComponent;
use crate::parser::wxml::WxmlNode;

/// `padding="{{[8, 16, 8, 16]}}"`：模板把数组写成 `8,16,8,16` 或 `[8,16,8,16]`，都认；不足 4 个按 CSS 简写补齐
fn padding_attr(node: &WxmlNode) -> Option<[f32; 4]> {
    let raw = node.get_attr("padding")?;
    let v: Vec<f32> = raw
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    Some(match v.len() {
        0 => return None,
        1 => [v[0]; 4],
        2 => [v[0], v[1], v[0], v[1]],
        3 => [v[0], v[1], v[2], v[1]],
        _ => [v[0], v[1], v[2], v[3]],
    })
}

/// 四个组件共用：按 view 建，再按属性补 padding（属性写了就盖掉 CSS 的 padding）与吸顶偏移
pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
    let mut rn = ViewComponent::build(node, ctx)?;
    let sf = ctx.scale_factor;
    let mut style = ctx.taffy.style(rn.taffy_node).ok()?.clone();
    if let Some([t, r, b, l]) = padding_attr(node) {
        style.padding = taffy::geometry::Rect {
            top: taffy::style::LengthPercentage::length(t * sf),
            right: taffy::style::LengthPercentage::length(r * sf),
            bottom: taffy::style::LengthPercentage::length(b * sf),
            left: taffy::style::LengthPercentage::length(l * sf),
        };
        rn.style.padding_top = t;
        rn.style.padding_right = r;
        rn.style.padding_bottom = b;
        rn.style.padding_left = l;
    }
    if node.tag_name == "sticky-header" {
        let off = node.get_attr("offset-top").and_then(|s| s.trim().parse::<f32>().ok()).unwrap_or(0.0);
        rn.style.is_sticky = true;
        rn.style.is_positioned = true;
        rn.style.fixed_top = Some(off * sf);
        rn.style.fixed_bottom = None;
        style.position = taffy::style::Position::Relative;
        style.inset.top = taffy::style::LengthPercentageAuto::auto();
        style.inset.bottom = taffy::style::LengthPercentageAuto::auto();
    }
    ctx.taffy.set_style(rn.taffy_node, style).ok()?;
    Some(rn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(attrs: &[(&str, &str)]) -> WxmlNode {
        let mut n = WxmlNode::new_element("sticky-header");
        for (k, v) in attrs {
            n.attributes.insert(k.to_string(), v.to_string());
        }
        n
    }

    #[test]
    fn padding_数组的几种写法() {
        assert_eq!(padding_attr(&node(&[("padding", "[8, 16, 4, 2]")])), Some([8.0, 16.0, 4.0, 2.0]));
        assert_eq!(padding_attr(&node(&[("padding", "8,16,4,2")])), Some([8.0, 16.0, 4.0, 2.0]));
        assert_eq!(padding_attr(&node(&[("padding", "10")])), Some([10.0; 4]));
        assert_eq!(padding_attr(&node(&[("padding", "6 12")])), Some([6.0, 12.0, 6.0, 12.0]));
        assert_eq!(padding_attr(&node(&[])), None);
    }
}
