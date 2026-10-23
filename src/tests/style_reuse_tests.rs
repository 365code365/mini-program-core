//! 建树复用 `build` 交出来的基础样式（`TagSpec::base_style_only`）的前提：这几种标签的 `build`
//! 交出来的样式与 `build_base_style` 现算的**完全一样**。有人改了 view 的 `build`（哪怕只动一个字段）
//! 而没把表里那一格改成 false，这里先红 —— 否则容器会悄悄用上 build 改过的样式。
use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::parser::wxss::WxssParser;
use crate::renderer::components::registry::{spec_for, TAGS};
use crate::renderer::components::*;
use std::collections::HashMap;

const CSS: &str = r#"
    .c { width: 120rpx; padding: 8px 4px; background-color: #123456; color: #ff0000; font-size: 15px;
         border: 2px solid #00ff00; border-radius: 6px; line-height: 1.5; position: relative; }
    .c:active { background-color: #654321; }
    .h { opacity: 0.6; color: #0000ff; }
    view .c { margin-top: 3px; }
"#;

fn node(tag: &str) -> WxmlNode {
    let mut attributes = HashMap::new();
    attributes.insert("class".to_string(), "c".to_string());
    attributes.insert("style".to_string(), "height: 40px; letter-spacing: 1px".to_string());
    attributes.insert("hover-class".to_string(), "h".to_string());
    WxmlNode { node_type: WxmlNodeType::Element, tag_name: tag.to_string(), attributes, children: vec![], text_content: String::new() }
}

/// 两种算法要用的输入完全一样（祖先链、继承、兄弟位置都相同）
fn with_ctx<R>(sheet: &crate::parser::wxss::StyleSheet, taffy: &mut Tree, f: impl FnOnce(&mut ComponentContext) -> R) -> R {
    let attrs = HashMap::new();
    let mut ctx = ComponentContext {
        scale_factor: 2.0,
        screen_width: 375.0,
        screen_height: 667.0,
        stylesheet: sheet,
        taffy,
        ancestors: vec![crate::parser::wxss::ElementDesc::new("view", None, &["page"], &attrs)],
        inherited: Default::default(),
        sibling_index: 1,
        sibling_count: 3,
        has_positioned_ancestor: false,
    };
    f(&mut ctx)
}

fn paint_fields(s: &NodeStyle) -> String {
    format!(
        "{:?} {:?} {} {:?} {:?} {} {} {} {}",
        s.background_color, s.text_color, s.font_size, s.line_height, s.letter_spacing, s.opacity,
        s.padding_top, s.padding_left, s.is_positioned
    )
}

#[test]
fn 标了只用基础样式的标签_build_交出来的就是基础样式() {
    let sheet = WxssParser::new(CSS).parse().unwrap();
    // 表里标了的 + 未登记标签（兜底也标了）
    let mut tags: Vec<&str> = TAGS.iter().filter(|s| s.base_style_only).flat_map(|s| s.tags.iter().copied()).collect();
    tags.push("web-view");
    assert!(tags.contains(&"view") && tags.contains(&"scroll-view"), "{tags:?}");
    for tag in tags {
        assert!(spec_for(tag).base_style_only, "{tag}");
        let n = node(tag);
        let mut t1 = Tree::new();
        let rn = with_ctx(&sheet, &mut t1, |ctx| (spec_for(tag).build)(&n, ctx)).expect("建出节点");
        let built = t1.style(rn.taffy_node).unwrap().clone();
        let mut t2 = Tree::new();
        let (ts, ns) = with_ctx(&sheet, &mut t2, |ctx| build_base_style(&n, ctx));
        assert_eq!(format!("{built:?}"), format!("{ts:?}"), "<{tag}> 的 taffy 样式");
        assert_eq!(paint_fields(&rn.style), paint_fields(&ns), "<{tag}> 的绘制样式");
        let (a, b) = (rn.style.pressed_style.as_deref().expect("有按压态"), ns.pressed_style.as_deref().expect("有按压态"));
        assert_eq!(paint_fields(a), paint_fields(b), "<{tag}> 的按压态");
    }
}
