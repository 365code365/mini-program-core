//! `<button>` 的点击态（默认 `hover-class="button-hover"`）。
//!
//! 按压态在绘制时**整份替换**常态（`wxml_renderer/pressed.rs` 的 `bake_pressed_style`），
//! 它缺了按钮的默认值就画成一个没样式的块：曾经一按下去文字跑到左边、圆角没了、
//! primary 的绿底变成内置 `.button-hover` 那层灰。这里锁住两条：
//! 1. 按压态带着常态的全部默认外观（居中、字号、圆角、类型色、边框）；
//! 2. 只变官方文档写的那几样：`opacity:.7`，默认按钮的底换成 rgba(0,0,0,.1)。

use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::parser::wxss::{StyleSheet, WxssParser};
use crate::renderer::components::*;
use crate::Color;

fn button(attrs: &[(&str, &str)], label: &str) -> WxmlNode {
    let text = WxmlNode {
        node_type: WxmlNodeType::Text,
        tag_name: String::new(),
        attributes: Default::default(),
        children: vec![],
        text_content: label.to_string(),
    };
    WxmlNode {
        node_type: WxmlNodeType::Element,
        tag_name: "button".to_string(),
        attributes: attrs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        children: vec![text],
        text_content: String::new(),
    }
}

/// 与渲染器同一份样式表：页面 CSS + 内置 `.button-hover`（`WxmlRenderer::new` 里补的）
fn sheet(page_css: &str) -> StyleSheet {
    let mut ss = WxssParser::new(page_css).parse().unwrap_or_default();
    ss.ensure_button_hover();
    ss
}

fn build(node: &WxmlNode, sheet: &StyleSheet) -> RenderNode {
    let mut taffy: Tree = Tree::new();
    let mut ctx = ComponentContext {
        scale_factor: 2.0,
        screen_width: 375.0,
        screen_height: 667.0,
        stylesheet: sheet,
        taffy: &mut taffy,
        ancestors: Vec::new(),
        inherited: Default::default(),
        sibling_index: 0,
        sibling_count: 1,
        has_positioned_ancestor: false,
    };
    ButtonComponent::build(node, &mut ctx).expect("button build")
}

fn pressed(rn: &RenderNode) -> &NodeStyle {
    rn.style.pressed_style.as_deref().expect("button 默认带 button-hover 点击态")
}

#[test]
fn 按下去仍居中_字号圆角不变() {
    for ty in ["primary", "default", "warn"] {
        let rn = build(&button(&[("type", ty)], ty), &sheet(""));
        let (n, p) = (&rn.style, pressed(&rn));
        assert!(p.text_align == TextAlign::Center, "{ty}：按下去文字不能跑到左边");
        assert_eq!(p.font_size, n.font_size, "{ty}：字号");
        assert_eq!(p.border_radius, n.border_radius, "{ty}：圆角");
        assert_eq!(p.text_color, n.text_color, "{ty}：文字色（只整体变淡，不换色）");
        assert_eq!(p.border_width, n.border_width, "{ty}：边框");
        assert!((p.opacity - 0.7).abs() < 1e-6, "{ty}：button-hover 的 opacity:.7，实际 {}", p.opacity);
    }
}

#[test]
fn primary_warn_plain_按下去保持类型色_默认按钮换成内置灰底() {
    let ss = sheet("");
    for attrs in [
        vec![("type", "primary")],
        vec![("type", "warn")],
        vec![("type", "primary"), ("plain", "true")],
        vec![("type", "warn"), ("plain", "true"), ("size", "mini")],
    ] {
        let rn = build(&button(&attrs, "x"), &ss);
        assert_eq!(pressed(&rn).background_color, rn.style.background_color, "{attrs:?}：类型色压过 .button-hover");
    }
    // 默认按钮的底色（`wx-button`）特异性低于 `.button-hover`，被换成 rgba(0,0,0,.1)
    let rn = build(&button(&[("type", "default")], "x"), &ss);
    assert_eq!(rn.style.background_color, Some(Color::from_hex(0xF8F8F8)));
    let bg = pressed(&rn).background_color.expect("默认按钮按下有底色");
    assert_eq!((bg.r, bg.g, bg.b), (0, 0, 0));
    assert!((24..=26).contains(&bg.a), "rgba(0,0,0,.1)，实际 alpha {}", bg.a);
}

#[test]
fn 页面写的背景与点击态照常生效() {
    // 页面覆盖了 `.button-hover`：按页面的来，类型色不再顶上
    let rn = build(&button(&[("type", "primary")], "x"), &sheet(".button-hover{background-color:#06ad56;}"));
    assert_eq!(pressed(&rn).background_color, Some(Color::from_hex(0x06AD56)));
    // 自定义 hover-class
    let rn = build(
        &button(&[("type", "primary"), ("hover-class", "down")], "x"),
        &sheet(".down{background-color:#123456;}"),
    );
    assert_eq!(pressed(&rn).background_color, Some(Color::from_hex(0x123456)));
    assert!(pressed(&rn).text_align == TextAlign::Center, "自定义 hover-class 也保持居中");
    // 页面给常态写了背景：按下去还是这个底，只变淡
    let rn = build(&button(&[("class", "b")], "x"), &sheet(".b{background-color:#ff0000;}"));
    assert_eq!(pressed(&rn).background_color, Some(Color::from_hex(0xFF0000)));
    // hover-class="none" 关掉点击态
    let rn = build(&button(&[("hover-class", "none")], "x"), &sheet(""));
    assert!(rn.style.pressed_style.is_none());
}

#[test]
fn 禁用的按钮没有点击态() {
    let rn = build(&button(&[("type", "primary"), ("disabled", "true")], "x"), &sheet(""));
    assert!(rn.style.pressed_style.is_none(), "微信里 disabled 的按钮按下去没有 button-hover");
}
