//! `<switch type="checkbox">` 的像素：选中态必须看得见，且用 `color` 属性上色。
//!
//! 曾经的 bug：checkbox 形态只在 CSS 写了 `background-color` 时才填底，再画一个白勾 ——
//! 交互层又不替 switch 改写背景色（开关配色归组件按进度求值），于是选中后只剩
//! 「白勾压在白卡片上」，看着像点了没反应；`color="#fa9d3b"` 也从没被读过。

use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::parser::wxss::StyleSheet;
use crate::renderer::components::*;
use crate::{Canvas, Color};

const OX: f32 = 10.0;
const OY: f32 = 10.0;

/// build + 在白底上画一个开关，返回画布与盒子尺寸（sf = 1，默认 24x24）
fn draw_switch(attrs: &[(&str, &str)]) -> (Canvas, f32, f32) {
    let node = WxmlNode {
        node_type: WxmlNodeType::Element,
        tag_name: "switch".into(),
        attributes: attrs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        children: vec![],
        text_content: String::new(),
    };
    let sheet = StyleSheet::default();
    let mut taffy: Tree = Tree::new();
    let mut ctx = ComponentContext {
        scale_factor: 1.0,
        screen_width: 375.0,
        screen_height: 667.0,
        stylesheet: &sheet,
        taffy: &mut taffy,
        ancestors: Vec::new(),
        inherited: Default::default(),
        sibling_index: 0,
        sibling_count: 1,
        has_positioned_ancestor: false,
    };
    let rn = SwitchComponent::build(&node, &mut ctx).expect("switch build");
    let size = taffy.style(rn.taffy_node).unwrap().size;
    let (w, h) = (size.width.value(), size.height.value());
    let mut canvas = Canvas::new(48, 48);
    canvas.clear(Color::WHITE);
    SwitchComponent::draw(&rn, &mut canvas, OX, OY, w, h, 1.0);
    (canvas, w, h)
}

fn px(c: &Canvas, dx: f32, dy: f32) -> (u8, u8, u8) {
    let p = c.get_pixel((OX + dx) as u32, (OY + dy) as u32);
    (p.r, p.g, p.b)
}

fn near(a: (u8, u8, u8), b: u32) -> bool {
    let b = ((b >> 16) as u8, (b >> 8) as u8, b as u8);
    let d = |x: u8, y: u8| (x as i32 - y as i32).abs();
    d(a.0, b.0) <= 2 && d(a.1, b.1) <= 2 && d(a.2, b.2) <= 2
}

#[test]
fn checked_checkbox_switch_fills_with_color_attr() {
    let (c, w, h) = draw_switch(&[("type", "checkbox"), ("checked", "true"), ("color", "#fa9d3b")]);
    assert_eq!((w, h), (24.0, 24.0));
    // 盒子上沿中间：勾画不到的地方，应是 color 填底
    assert!(near(px(&c, 12.0, 3.0), 0xFA9D3B), "选中底色应是 color：{:?}", px(&c, 12.0, 3.0));
    // 勾的拐点（cx - 0.05w, cy + 0.2h）是白的：白勾压在彩色底上才看得见
    assert!(near(px(&c, 10.8, 16.8), 0xFFFFFF), "勾应是白色：{:?}", px(&c, 10.8, 16.8));
}

#[test]
fn checked_checkbox_switch_defaults_to_switch_green() {
    // 不写 color 时与 switch 同一个默认色（微信文档 #04BE02）
    let (c, _, _) = draw_switch(&[("type", "checkbox"), ("checked", "{{true}}")]);
    assert!(near(px(&c, 12.0, 3.0), 0x04BE02), "默认底色：{:?}", px(&c, 12.0, 3.0));
}

#[test]
fn unchecked_checkbox_switch_is_white_box_with_grey_border() {
    let (c, _, _) = draw_switch(&[("type", "checkbox"), ("color", "#fa9d3b")]);
    assert!(near(px(&c, 12.0, 12.0), 0xFFFFFF), "未选中内部白：{:?}", px(&c, 12.0, 12.0));
    assert!(near(px(&c, 12.0, 0.5), 0xD1D1D1), "未选中灰边：{:?}", px(&c, 12.0, 0.5));
}
