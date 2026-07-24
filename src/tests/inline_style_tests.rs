use crate::parser::wxml::WxmlNode;
use crate::renderer::components::{build_base_style, ComponentContext};
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
    // 100% -> Dimension::Percent(1.0)
    if let Dimension::Percent(p) = style.size.width {
        assert_eq!(p, 1.0);
    } else {
        panic!("Width should be percent, got {:?}", style.size.width);
    }
}
