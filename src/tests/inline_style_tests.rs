use crate::parser::wxml::WxmlNode;
use crate::renderer::components::{build_base_style, dim_percent, ComponentContext};
use crate::parser::wxss::StyleSheet;
use taffy::prelude::*;

#[test]
fn test_inline_style_parsing() {
    let stylesheet = StyleSheet::new();
    let mut taffy = TaffyTree::new();
    
    let mut ctx = ComponentContext {
        scale_factor: 1.0,
        screen_width: 375.0,
        screen_height: 667.0,
        stylesheet: &stylesheet,
        taffy: &mut taffy,
        ancestors: Vec::new(),
        inherited: Default::default(),
    sibling_index: 0,
    sibling_count: 1,
            has_positioned_ancestor: false,
    };
    
    let mut node = WxmlNode::new_element("view");
    node.attributes.insert("style".to_string(), "position: fixed; bottom: 100rpx; width: 100%;".to_string());
    
    let (style, node_style) = build_base_style(&node, &mut ctx);
    
    // Check position
    assert_eq!(style.position, Position::Absolute);
    assert!(node_style.is_fixed);
    
    // Check bottom
    assert_eq!(node_style.fixed_bottom, Some(50.0)); // 100rpx = 50px (at 375 width)
    
    // Check width
    // `position: fixed` 的包含块永远是视口：百分比宽按视口折成像素（375 × 1.0）。
    // （absolute 不同：宽度仍交给布局引擎按父节点解析，只有无定位祖先时的高度按视口折算）
    assert_eq!(dim_percent(style.size.width), None, "fixed 的百分比宽已按视口折算");
    assert_eq!(style.size.width, taffy::prelude::length(375.0));
}
