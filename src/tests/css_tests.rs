//! CSS 解析单元测试
//! 测试 WXSS 样式解析功能

use crate::parser::wxss::{WxssParser, StyleSheet, StyleValue, LengthUnit, rpx_to_px};

/// 辅助函数：解析 CSS 字符串
fn parse_css(css: &str) -> StyleSheet {
    WxssParser::new(css).parse().unwrap_or_default()
}

/// 测试基本 CSS 解析
#[test]
fn test_basic_css_parsing() {
    let css = r#"
        .container {
            width: 100px;
            height: 200px;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let styles = stylesheet.get_styles(&["container"], "view");
    
    assert!(styles.contains_key("width"));
    assert!(styles.contains_key("height"));
    
    if let Some(StyleValue::Length(w, LengthUnit::Px)) = styles.get("width") {
        assert_eq!(*w, 100.0);
    } else {
        panic!("width should be 100px");
    }
}

/// 测试 rpx 单位解析
#[test]
fn test_rpx_parsing() {
    let css = r#"
        .box {
            width: 750rpx;
            padding: 20rpx;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let styles = stylesheet.get_styles(&["box"], "view");
    
    if let Some(StyleValue::Length(w, LengthUnit::Rpx)) = styles.get("width") {
        assert_eq!(*w, 750.0);
        // 在 375px 屏幕上，750rpx = 375px
        assert_eq!(rpx_to_px(*w, 375.0), 375.0);
    } else {
        panic!("width should be 750rpx");
    }
}

/// 测试颜色解析 - 十六进制
#[test]
fn test_hex_color_parsing() {
    let css = r#"
        .text {
            color: #FF6B35;
            background-color: #fff;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let styles = stylesheet.get_styles(&["text"], "text");
    
    if let Some(StyleValue::Color(c)) = styles.get("color") {
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 107);
        assert_eq!(c.b, 53);
    } else {
        panic!("color should be #FF6B35");
    }
    
    if let Some(StyleValue::Color(c)) = styles.get("background-color") {
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 255);
        assert_eq!(c.b, 255);
    } else {
        panic!("background-color should be #fff");
    }
}

/// 测试 RGB 颜色解析
#[test]
fn test_rgb_color_parsing() {
    let css = r#"
        .box {
            background-color: rgb(100, 150, 200);
        }
    "#;
    
    let stylesheet = parse_css(css);
    let styles = stylesheet.get_styles(&["box"], "view");
    
    if let Some(StyleValue::Color(c)) = styles.get("background-color") {
        assert_eq!(c.r, 100);
        assert_eq!(c.g, 150);
        assert_eq!(c.b, 200);
    } else {
        panic!("background-color should be rgb(100, 150, 200)");
    }
}

/// 测试 Flexbox 属性解析
#[test]
fn test_flexbox_parsing() {
    let css = r#"
        .flex-container {
            display: flex;
            flex-direction: row;
            justify-content: space-between;
            align-items: center;
            flex-wrap: wrap;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let styles = stylesheet.get_styles(&["flex-container"], "view");
    
    if let Some(StyleValue::String(s)) = styles.get("display") {
        assert_eq!(s, "flex");
    }
    
    if let Some(StyleValue::String(s)) = styles.get("flex-direction") {
        assert_eq!(s, "row");
    }
    
    if let Some(StyleValue::String(s)) = styles.get("justify-content") {
        assert_eq!(s, "space-between");
    }
    
    if let Some(StyleValue::String(s)) = styles.get("align-items") {
        assert_eq!(s, "center");
    }
}

/// 测试 position 属性解析
#[test]
fn test_position_parsing() {
    let css = r#"
        .fixed-bar {
            position: fixed;
            bottom: 0;
            left: 0;
            right: 0;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let styles = stylesheet.get_styles(&["fixed-bar"], "view");
    
    if let Some(StyleValue::String(s)) = styles.get("position") {
        assert_eq!(s, "fixed");
    }
    
    if let Some(StyleValue::Length(v, _)) = styles.get("bottom") {
        assert_eq!(*v, 0.0);
    }
}

/// 测试多类选择器
#[test]
fn test_multiple_classes() {
    let css = r#"
        .base {
            padding: 10px;
        }
        .highlight {
            color: #FF0000;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let styles = stylesheet.get_styles(&["base", "highlight"], "view");
    
    assert!(styles.contains_key("padding"));
    assert!(styles.contains_key("color"));
}

/// 测试标签选择器
#[test]
fn test_tag_selector() {
    let css = r#"
        button {
            background-color: #007AFF;
            border-radius: 8px;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let styles = stylesheet.get_styles(&[], "button");
    
    if let Some(StyleValue::Color(c)) = styles.get("background-color") {
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 122);
        assert_eq!(c.b, 255);
    }
}

/// 测试边框简写解析
#[test]
fn test_border_shorthand() {
    let css = r#"
        .bordered {
            border: 1px solid #333;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let styles = stylesheet.get_styles(&["bordered"], "view");
    
    // border 简写应该被解析
    assert!(styles.contains_key("border"));
}

/// 测试字体属性
#[test]
fn test_font_properties() {
    let css = r#"
        .text {
            font-size: 16px;
            font-weight: bold;
            line-height: 1.5;
            letter-spacing: 2px;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let styles = stylesheet.get_styles(&["text"], "text");
    
    if let Some(StyleValue::Length(v, _)) = styles.get("font-size") {
        assert_eq!(*v, 16.0);
    }
    
    if let Some(StyleValue::String(s)) = styles.get("font-weight") {
        assert_eq!(s, "bold");
    }
    
    if let Some(StyleValue::Number(v)) = styles.get("line-height") {
        assert_eq!(*v, 1.5);
    }
}

/// 测试百分比值
#[test]
fn test_percent_values() {
    let css = r#"
        .full-width {
            width: 100%;
            height: 50%;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let styles = stylesheet.get_styles(&["full-width"], "view");
    
    if let Some(StyleValue::Length(v, LengthUnit::Percent)) = styles.get("width") {
        assert_eq!(*v, 100.0);
    }
    
    if let Some(StyleValue::Length(v, LengthUnit::Percent)) = styles.get("height") {
        assert_eq!(*v, 50.0);
    }
}

/// 测试 auto 值
#[test]
fn test_auto_value() {
    let css = r#"
        .auto-margin {
            margin: auto;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let styles = stylesheet.get_styles(&["auto-margin"], "view");
    
    if let Some(StyleValue::Auto) = styles.get("margin") {
        // OK
    } else {
        panic!("margin should be auto");
    }
}

/// 测试 text-decoration
#[test]
fn test_text_decoration() {
    let css = r#"
        .strikethrough {
            text-decoration: line-through;
        }
        .underlined {
            text-decoration: underline;
        }
    "#;
    
    let stylesheet = parse_css(css);
    
    let styles1 = stylesheet.get_styles(&["strikethrough"], "text");
    if let Some(StyleValue::String(s)) = styles1.get("text-decoration") {
        assert_eq!(s, "line-through");
    }
    
    let styles2 = stylesheet.get_styles(&["underlined"], "text");
    if let Some(StyleValue::String(s)) = styles2.get("text-decoration") {
        assert_eq!(s, "underline");
    }
}

/// 测试 overflow 属性
#[test]
fn test_overflow() {
    let css = r#"
        .scroll-view {
            overflow: scroll;
        }
        .hidden-overflow {
            overflow: hidden;
        }
    "#;
    
    let stylesheet = parse_css(css);
    
    let styles1 = stylesheet.get_styles(&["scroll-view"], "view");
    if let Some(StyleValue::String(s)) = styles1.get("overflow") {
        assert_eq!(s, "scroll");
    }
    
    let styles2 = stylesheet.get_styles(&["hidden-overflow"], "view");
    if let Some(StyleValue::String(s)) = styles2.get("overflow") {
        assert_eq!(s, "hidden");
    }
}

/// 测试 z-index
#[test]
fn test_z_index() {
    let css = r#"
        .overlay {
            z-index: 100;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let styles = stylesheet.get_styles(&["overlay"], "view");
    
    if let Some(StyleValue::Number(v)) = styles.get("z-index") {
        assert_eq!(*v, 100.0);
    }
}

/// 测试 opacity
#[test]
fn test_opacity() {
    let css = r#"
        .transparent {
            opacity: 0.5;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let styles = stylesheet.get_styles(&["transparent"], "view");
    
    if let Some(StyleValue::Number(v)) = styles.get("opacity") {
        assert_eq!(*v, 0.5);
    }
}

/// 测试 rpx_to_px 转换函数
#[test]
fn test_rpx_to_px_conversion() {
    // 标准 375px 屏幕
    assert_eq!(rpx_to_px(750.0, 375.0), 375.0);
    assert_eq!(rpx_to_px(375.0, 375.0), 187.5);
    assert_eq!(rpx_to_px(100.0, 375.0), 50.0);
    
    // 414px 屏幕 (iPhone Plus)
    assert_eq!(rpx_to_px(750.0, 414.0), 414.0);
    
    // 320px 屏幕 (iPhone SE)
    assert_eq!(rpx_to_px(750.0, 320.0), 320.0);
}

/// 测试灰色系颜色解析 (#F5F5F5)
#[test]
fn test_gray_color_parsing() {
    let css = r#"
        .category-nav {
            background-color: #F5F5F5;
        }
        .category-item {
            background-color: #f5f5f5;
        }
    "#;
    
    let stylesheet = parse_css(css);
    
    // 测试大写
    let styles1 = stylesheet.get_styles(&["category-nav"], "view");
    if let Some(StyleValue::Color(c)) = styles1.get("background-color") {
        assert_eq!(c.r, 245, "category-nav red should be 245");
        assert_eq!(c.g, 245, "category-nav green should be 245");
        assert_eq!(c.b, 245, "category-nav blue should be 245");
    } else {
        panic!("category-nav background-color should be #F5F5F5");
    }
    
    // 测试小写
    let styles2 = stylesheet.get_styles(&["category-item"], "view");
    if let Some(StyleValue::Color(c)) = styles2.get("background-color") {
        assert_eq!(c.r, 245, "category-item red should be 245");
        assert_eq!(c.g, 245, "category-item green should be 245");
        assert_eq!(c.b, 245, "category-item blue should be 245");
    } else {
        panic!("category-item background-color should be #f5f5f5");
    }
}

/// 测试复合类选择器 (.class1.class2)
#[test]
fn test_compound_class_selector() {
    let css = r#"
        .category-item {
            background-color: #F5F5F5;
        }
        .category-item.active {
            background-color: #ffffff;
        }
    "#;
    
    let stylesheet = parse_css(css);
    
    // 只有 category-item 类
    let styles1 = stylesheet.get_styles(&["category-item"], "view");
    if let Some(StyleValue::Color(c)) = styles1.get("background-color") {
        assert_eq!(c.r, 245, "without active: red should be 245");
        assert_eq!(c.g, 245, "without active: green should be 245");
        assert_eq!(c.b, 245, "without active: blue should be 245");
    } else {
        panic!("category-item should have background-color #F5F5F5");
    }
    
    // 同时有 category-item 和 active 类
    let styles2 = stylesheet.get_styles(&["category-item", "active"], "view");
    if let Some(StyleValue::Color(c)) = styles2.get("background-color") {
        assert_eq!(c.r, 255, "with active: red should be 255");
        assert_eq!(c.g, 255, "with active: green should be 255");
        assert_eq!(c.b, 255, "with active: blue should be 255");
    } else {
        panic!("category-item.active should have background-color #ffffff");
    }
}

/// 测试选择器优先级（特异性）
#[test]
fn test_selector_specificity() {
    let css = r#"
        .item {
            color: #000;
        }
        .item.highlight {
            color: #FF0000;
        }
        .item.highlight.important {
            color: #00FF00;
        }
    "#;
    
    let stylesheet = parse_css(css);
    
    // 单类选择器
    let styles1 = stylesheet.get_styles(&["item"], "view");
    if let Some(StyleValue::Color(c)) = styles1.get("color") {
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
    }
    
    // 双类选择器应该覆盖单类
    let styles2 = stylesheet.get_styles(&["item", "highlight"], "view");
    if let Some(StyleValue::Color(c)) = styles2.get("color") {
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
    }
    
    // 三类选择器应该覆盖双类
    let styles3 = stylesheet.get_styles(&["item", "highlight", "important"], "view");
    if let Some(StyleValue::Color(c)) = styles3.get("color") {
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 255);
        assert_eq!(c.b, 0);
    }
}

/// 简写与细项同时中选时，细项必须赢 —— 而且**每次运行结果都要一样**。
///
/// 回归的是一个真实 bug：`.dot{border:2rpx solid #ccc}` 与
/// `.dot.on{border-color:#FF6B35}` 两条规则合并后，级联结果里同时存在
/// `border`（简写）和 `border-color`（细项）两个键。样式落地时按 HashMap 遍历顺序
/// 应用，而 Rust 的 HashMap 每进程一个随机 hash 种子 ——
/// 于是同一份源码每次运行渲染结果都可能不同（地址页那个选中圆点时橙时灰）。
#[test]
fn test_shorthand_longhand_cascade_is_deterministic() {
    use crate::renderer::components::{build_base_style, ComponentContext};
    use crate::parser::wxml::WxmlNode;
    use crate::renderer::components::Tree;

    let css = r#"
        .dot { width: 18px; height: 18px; border: 2px solid #CCCCCC; }
        .dot.on { border-color: #FF6B35; }
    "#;
    let stylesheet = parse_css(css);

    let mut node = WxmlNode::new_element("view");
    node.attributes.insert("class".to_string(), "dot on".to_string());

    // 跑多次：每次都是一个新 HashMap（同进程内种子相同，但遍历顺序仍取决于插入/容量），
    // 关键是断言结果稳定且等于 CSS 语义要求的那个值
    for _ in 0..8 {
        let mut taffy = Tree::new();
        let mut ctx = ComponentContext {
            taffy: &mut taffy,
            stylesheet: &stylesheet,
            screen_width: 375.0,
            screen_height: 667.0,
            scale_factor: 1.0,
            ancestors: Vec::new(),
            inherited: Default::default(),
            sibling_index: 0,
            sibling_count: 1,
        };
        let (_ts, ns) = build_base_style(&node, &mut ctx);
        let c = ns.border_color.expect("border-color 应该落地");
        assert_eq!(
            (c.r, c.g, c.b), (0xFF, 0x6B, 0x35),
            "细项 border-color 必须覆盖简写 border 里的颜色"
        );
        assert!((ns.border_width - 2.0).abs() < 0.01, "简写里的宽度应保留");
    }
}
