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
    // 宽度仍交给布局引擎按包含块解析（父节点通常就是整宽）：
    // 只有**高度**的百分比会在无定位祖先时按视口折算，因为父节点 auto 高度时
    // 百分比没有参照物会塌成 0。
    if let Dimension::Percent(p) = style.size.width {
        assert_eq!(p, 1.0);
    } else {
        panic!("Width should be percent, got {:?}", style.size.width);
    }
}
