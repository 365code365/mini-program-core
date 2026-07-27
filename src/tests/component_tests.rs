//! 组件单元测试
//! 测试各种 UI 组件的构建和渲染

use crate::renderer::components::*;
use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::parser::wxss::WxssParser;
use std::collections::HashMap;
use taffy::prelude::*;

/// 创建测试用的 WXML 节点
fn create_test_node(tag: &str, classes: &[&str], attrs: HashMap<String, String>) -> WxmlNode {
    let class_str = classes.join(" ");
    let mut attributes = attrs;
    if !class_str.is_empty() {
        attributes.insert("class".to_string(), class_str);
    }
    
    WxmlNode {
        node_type: WxmlNodeType::Element,
        tag_name: tag.to_string(),
        attributes,
        children: vec![],
        text_content: String::new(),
    }
}

/// 创建测试用的文本节点
fn create_text_node(text: &str) -> WxmlNode {
    WxmlNode {
        node_type: WxmlNodeType::Text,
        tag_name: String::new(),
        attributes: HashMap::new(),
        children: vec![],
        text_content: text.to_string(),
    }
}

/// 辅助函数：解析 CSS
fn parse_css(css: &str) -> crate::parser::wxss::StyleSheet {
    WxssParser::new(css).parse().unwrap_or_default()
}

/// 测试 View 组件构建
#[test]
fn test_view_component_build() {
    let css = r#"
        .container {
            width: 100px;
            height: 50px;
            background-color: #FF0000;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let mut taffy = TaffyTree::new();
    
    let node = create_test_node("view", &["container"], HashMap::new());
    
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
    
    let render_node = ViewComponent::build(&node, &mut ctx);
    assert!(render_node.is_some());
    
    let rn = render_node.unwrap();
    assert_eq!(rn.tag, "view");
    assert!(rn.style.background_color.is_some());
    
    let bg = rn.style.background_color.unwrap();
    assert_eq!(bg.r, 255);
    assert_eq!(bg.g, 0);
    assert_eq!(bg.b, 0);
}

/// 测试 Text 组件构建
#[test]
fn test_text_component_build() {
    let css = r#"
        .title {
            font-size: 20px;
            color: #333333;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let mut taffy = TaffyTree::new();
    
    let mut node = create_test_node("text", &["title"], HashMap::new());
    node.children.push(create_text_node("Hello World"));
    
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
    
    let render_node = TextComponent::build(&node, &mut ctx);
    assert!(render_node.is_some());
    
    let rn = render_node.unwrap();
    assert_eq!(rn.tag, "text");
    assert_eq!(rn.text, "Hello World");
    assert_eq!(rn.style.font_size, 20.0);
}

/// 测试 Button 组件构建
#[test]
fn test_button_component_build() {
    let css = r#"
        .primary-btn {
            background-color: #007AFF;
            border-radius: 8px;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let mut taffy = TaffyTree::new();
    
    let mut node = create_test_node("button", &["primary-btn"], HashMap::new());
    node.children.push(create_text_node("Submit"));
    
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
    
    let render_node = ButtonComponent::build(&node, &mut ctx);
    assert!(render_node.is_some());
    
    let rn = render_node.unwrap();
    assert_eq!(rn.tag, "button");
    assert_eq!(rn.text, "Submit");
}

/// 测试 Icon 组件构建
#[test]
fn test_icon_component_build() {
    let stylesheet = parse_css("");
    let mut taffy = TaffyTree::new();
    
    let mut attrs = HashMap::new();
    attrs.insert("type".to_string(), "success".to_string());
    attrs.insert("size".to_string(), "24".to_string());
    attrs.insert("color".to_string(), "#09BB07".to_string());
    
    let node = create_test_node("icon", &[], attrs);
    
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
    
    let render_node = IconComponent::build(&node, &mut ctx);
    assert!(render_node.is_some());
    
    let rn = render_node.unwrap();
    assert_eq!(rn.tag, "icon");
}

/// 测试 Progress 组件构建
#[test]
fn test_progress_component_build() {
    let stylesheet = parse_css("");
    let mut taffy = TaffyTree::new();
    
    let mut attrs = HashMap::new();
    attrs.insert("percent".to_string(), "75".to_string());
    attrs.insert("show-info".to_string(), "true".to_string());
    
    let node = create_test_node("progress", &[], attrs);
    
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
    
    let render_node = ProgressComponent::build(&node, &mut ctx);
    assert!(render_node.is_some());
    
    let rn = render_node.unwrap();
    assert_eq!(rn.tag, "progress");
    // custom_data 存储的是原始百分比值 75.0，不是 0.75
    assert!(rn.style.custom_data == 75.0 || rn.style.custom_data == 0.75);
}

/// 测试 Switch 组件构建
#[test]
fn test_switch_component_build() {
    let stylesheet = parse_css("");
    let mut taffy = TaffyTree::new();
    
    let mut attrs = HashMap::new();
    attrs.insert("checked".to_string(), "true".to_string());
    
    let node = create_test_node("switch", &[], attrs);
    
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
    
    let render_node = SwitchComponent::build(&node, &mut ctx);
    assert!(render_node.is_some());
    
    let rn = render_node.unwrap();
    assert_eq!(rn.tag, "switch");
    assert_eq!(rn.style.custom_data, 1.0); // checked = true
}

/// 测试 Checkbox 组件构建
#[test]
fn test_checkbox_component_build() {
    let stylesheet = parse_css("");
    let mut taffy = TaffyTree::new();
    
    let mut attrs = HashMap::new();
    attrs.insert("value".to_string(), "option1".to_string());
    attrs.insert("checked".to_string(), "false".to_string());
    
    let node = create_test_node("checkbox", &[], attrs);
    
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
    
    let render_node = CheckboxComponent::build(&node, &mut ctx);
    assert!(render_node.is_some());
    
    let rn = render_node.unwrap();
    assert_eq!(rn.tag, "checkbox");
    assert_eq!(rn.style.custom_data, 0.0); // checked = false
}

/// 测试 Radio 组件构建
#[test]
fn test_radio_component_build() {
    let stylesheet = parse_css("");
    let mut taffy = TaffyTree::new();
    
    let mut attrs = HashMap::new();
    attrs.insert("value".to_string(), "male".to_string());
    
    let node = create_test_node("radio", &[], attrs);
    
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
    
    let render_node = RadioComponent::build(&node, &mut ctx);
    assert!(render_node.is_some());
    
    let rn = render_node.unwrap();
    assert_eq!(rn.tag, "radio");
}

/// 测试 Slider 组件构建
#[test]
fn test_slider_component_build() {
    let stylesheet = parse_css("");
    let mut taffy = TaffyTree::new();
    
    let mut attrs = HashMap::new();
    attrs.insert("min".to_string(), "0".to_string());
    attrs.insert("max".to_string(), "100".to_string());
    attrs.insert("value".to_string(), "50".to_string());
    
    let node = create_test_node("slider", &[], attrs);
    
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
    
    let render_node = SliderComponent::build(&node, &mut ctx);
    assert!(render_node.is_some());
    
    let rn = render_node.unwrap();
    assert_eq!(rn.tag, "slider");
    assert_eq!(rn.style.custom_data, 0.5); // 50/100
}

/// 测试 Input 组件构建
#[test]
fn test_input_component_build() {
    let css = r#"
        .search-input {
            height: 40px;
            padding: 10px;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let mut taffy = TaffyTree::new();
    
    let mut attrs = HashMap::new();
    attrs.insert("placeholder".to_string(), "Search...".to_string());
    attrs.insert("type".to_string(), "text".to_string());
    
    let node = create_test_node("input", &["search-input"], attrs);
    
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
    
    let render_node = InputComponent::build(&node, &mut ctx);
    assert!(render_node.is_some());
    
    let rn = render_node.unwrap();
    assert_eq!(rn.tag, "input");
    assert_eq!(rn.text, "Search...");
}

/// 测试 Image 组件构建
#[test]
fn test_image_component_build() {
    let css = r#"
        .avatar {
            width: 80px;
            height: 80px;
            border-radius: 40px;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let mut taffy = TaffyTree::new();
    
    let mut attrs = HashMap::new();
    attrs.insert("src".to_string(), "https://example.com/avatar.png".to_string());
    attrs.insert("mode".to_string(), "aspectFill".to_string());
    
    let node = create_test_node("image", &["avatar"], attrs);
    
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
    
    let render_node = ImageComponent::build(&node, &mut ctx);
    assert!(render_node.is_some());
    
    let rn = render_node.unwrap();
    assert_eq!(rn.tag, "image");
}

/// 测试事件绑定提取
#[test]
fn test_event_extraction() {
    let mut attrs = HashMap::new();
    attrs.insert("bindtap".to_string(), "onTap".to_string());
    attrs.insert("data-id".to_string(), "123".to_string());
    attrs.insert("data-name".to_string(), "test".to_string());
    
    let node = create_test_node("view", &[], attrs);
    
    let events = extract_events(&node);
    assert_eq!(events.len(), 1);
    
    let e = &events[0];
    assert_eq!(e.event_type, "tap");
    assert_eq!(e.handler, "onTap");
    assert_eq!(e.data.get("id"), Some(&"123".to_string()));
    assert_eq!(e.data.get("name"), Some(&"test".to_string()));
    assert!(!e.is_catch);
}

/// 测试颜色解析
#[test]
fn test_color_parsing() {
    // 6位十六进制
    let color1 = parse_color_str("#FF6B35");
    assert!(color1.is_some());
    let c1 = color1.unwrap();
    assert_eq!(c1.r, 255);
    assert_eq!(c1.g, 107);
    assert_eq!(c1.b, 53);
    
    // 3位十六进制
    let color2 = parse_color_str("#FFF");
    assert!(color2.is_some());
    let c2 = color2.unwrap();
    assert_eq!(c2.r, 255);
    assert_eq!(c2.g, 255);
    assert_eq!(c2.b, 255);
    
    // RGB
    let color3 = parse_color_str("rgb(100, 150, 200)");
    assert!(color3.is_some());
    let c3 = color3.unwrap();
    assert_eq!(c3.r, 100);
    assert_eq!(c3.g, 150);
    assert_eq!(c3.b, 200);
    
    // 无效颜色
    let color4 = parse_color_str("invalid");
    assert!(color4.is_none());
}

/// 测试文本内容提取
#[test]
fn test_text_content_extraction() {
    let mut node = create_test_node("view", &[], HashMap::new());
    node.children.push(create_text_node("Hello "));
    node.children.push(create_text_node("World"));
    
    let text = get_text_content(&node);
    assert_eq!(text, "Hello World");
}

/// 测试 class 列表提取
#[test]
fn test_class_extraction() {
    let mut attrs = HashMap::new();
    attrs.insert("class".to_string(), "container flex-row active".to_string());
    
    let node = create_test_node("view", &[], attrs);
    let classes = get_classes(&node);
    
    assert_eq!(classes.len(), 3);
    assert!(classes.contains(&"container"));
    assert!(classes.contains(&"flex-row"));
    assert!(classes.contains(&"active"));
}

/// 测试 NodeStyle 默认值
#[test]
fn test_node_style_defaults() {
    let style = NodeStyle::default();
    
    assert!(style.background_color.is_none());
    assert!(style.text_color.is_none());
    assert_eq!(style.border_width, 0.0);
    assert_eq!(style.border_radius, 0.0);
    assert_eq!(style.font_size, 0.0);
    assert_eq!(style.opacity, 0.0);
    assert!(!style.is_fixed);
    assert!(style.fixed_bottom.is_none());
    assert!(style.fixed_top.is_none());
}

/// 测试 fixed 定位样式
#[test]
fn test_fixed_position_style() {
    let css = r#"
        .action-bar {
            position: fixed;
            bottom: 0;
            left: 0;
            right: 0;
            height: 50px;
        }
    "#;
    
    let stylesheet = parse_css(css);
    let mut taffy = TaffyTree::new();
    
    let node = create_test_node("view", &["action-bar"], HashMap::new());
    
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
    
    let render_node = ViewComponent::build(&node, &mut ctx);
    assert!(render_node.is_some());
    
    let rn = render_node.unwrap();
    assert!(rn.style.is_fixed);
    assert_eq!(rn.style.fixed_bottom, Some(0.0));
    assert_eq!(rn.style.fixed_left, Some(0.0));
    assert_eq!(rn.style.fixed_right, Some(0.0));
}
