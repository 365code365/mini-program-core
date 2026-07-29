//! 输入框光标（聚焦指示线）与内边距。
//!
//! 三件事以前都不对，而且都属于「看着像小事、用起来很别扭」：
//! 1. 光标是 1 **设备**像素的描边线段 —— 2x 屏上只有半个逻辑像素，细到看不出它在闪；
//!    微信是 2 逻辑像素宽、两端圆角、高度略高于字面（约 1.15em）并垂直居中。
//! 2. 闪烁相位以**进程启动**为零点：点进输入框的瞬间可能正好在「隐藏」的半周期里，
//!    看起来像「点了没反应」。微信是一聚焦就立刻显示，再从那一刻数拍子。
//! 3. 文字/占位符/光标的左缩进写死 12px，不看元素**实际**的 padding ——
//!    页面自己设了 `padding-left` 时排版按 CSS、文字却固定缩进 12px，两边对不齐。

use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::parser::wxss::{StyleSheet, WxssParser};
use crate::renderer::components::*;
use crate::{Canvas, Color};
use taffy::prelude::*;

fn node_of(attrs: &[(&str, &str)]) -> WxmlNode {
    WxmlNode {
        node_type: WxmlNodeType::Element,
        tag_name: "input".to_string(),
        attributes: attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        children: vec![],
        text_content: String::new(),
    }
}

fn css(s: &str) -> StyleSheet {
    WxssParser::new(s).parse().unwrap_or_default()
}

/// 画一个聚焦的输入框，返回 (光标矩形, 输入框矩形, 字号) —— 都是设备像素
fn draw_focused(
    attrs: &[(&str, &str)],
    sheet: &StyleSheet,
    sf: f32,
    cursor_pos: usize,
) -> ((f32, f32, f32, f32), (f32, f32, f32, f32), f32) {
    let node = node_of(attrs);
    let mut taffy: Tree = Tree::new();
    let mut ctx = ComponentContext {
        scale_factor: sf,
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
    let rn = InputComponent::build(&node, &mut ctx).expect("input build");
    let font_size = rn.style.font_size * sf;
    // 盒子尺寸：宽给定，高按 build 算出来的
    let (x, y, w, h) = (20.0 * sf, 30.0 * sf, 300.0 * sf, 40.0 * sf);
    let tr = crate::text::shared_fonts();
    let mut canvas = Canvas::new(750, 400);
    canvas.clear(Color::WHITE);
    InputComponent::draw_with_cursor(
        &rn,
        &mut canvas,
        tr.as_deref(),
        x,
        y,
        w,
        h,
        sf,
        true,
        cursor_pos,
    );
    let caret = last_caret_rect().expect("聚焦时应记下光标矩形");
    (caret, (x, y, w, h), font_size)
}

#[test]
fn caret_is_two_logical_px_wide_with_rounded_ends() {
    let sheet = css("");
    let sf = 2.0;
    let (caret, _, _) = draw_focused(&[("value", "abc")], &sheet, sf, 3);
    assert!(
        (caret.2 - 2.0 * sf).abs() < 0.01,
        "光标宽度应为 2 逻辑像素（{} 设备像素），实际 {}",
        2.0 * sf,
        caret.2
    );
}

#[test]
fn caret_is_taller_than_the_glyphs_and_vertically_centered() {
    let sheet = css("");
    let sf = 2.0;
    let (caret, box_rect, font_size) = draw_focused(&[("value", "abc")], &sheet, sf, 3);
    assert!(
        caret.3 > font_size,
        "光标应略高于字面：高 {} vs 字号 {}",
        caret.3,
        font_size
    );
    assert!(
        (caret.3 - font_size * 1.15).abs() < 0.01,
        "高度应为 1.15em，实际 {}",
        caret.3
    );
    let caret_center = caret.1 + caret.3 / 2.0;
    let box_center = box_rect.1 + box_rect.3 / 2.0;
    assert!(
        (caret_center - box_center).abs() < 0.01,
        "光标应垂直居中：光标中心 {caret_center} vs 盒子中心 {box_center}"
    );
}

#[test]
fn caret_sits_inside_the_input_box() {
    let sheet = css("");
    let sf = 2.0;
    let (caret, b, _) = draw_focused(&[("value", "hello")], &sheet, sf, 5);
    assert!(caret.0 >= b.0 && caret.0 + caret.2 <= b.0 + b.2, "水平越界 {caret:?} / {b:?}");
    assert!(caret.1 >= b.1 && caret.1 + caret.3 <= b.1 + b.3, "垂直越界 {caret:?} / {b:?}");
}

#[test]
fn caret_moves_right_as_the_cursor_advances() {
    let sheet = css("");
    let sf = 2.0;
    let (at0, _, _) = draw_focused(&[("value", "abcdef")], &sheet, sf, 0);
    let (at3, _, _) = draw_focused(&[("value", "abcdef")], &sheet, sf, 3);
    let (at6, _, _) = draw_focused(&[("value", "abcdef")], &sheet, sf, 6);
    assert!(at0.0 < at3.0 && at3.0 < at6.0, "光标 x 应随位置递增: {} {} {}", at0.0, at3.0, at6.0);
}

#[test]
fn text_indent_follows_the_elements_own_padding() {
    // 页面自己写了 padding-left 时，光标（以及文字）的起点要跟着走，
    // 不能固定 12px —— 否则排版按 CSS、文字按 12px，两边错开
    let sf = 2.0;
    let narrow = css(".f{ padding-left:4px; padding-right:4px; }");
    let wide = css(".f{ padding-left:40px; padding-right:4px; }");
    let (c_narrow, b, _) = draw_focused(&[("class", "f")], &narrow, sf, 0);
    let (c_wide, _, _) = draw_focused(&[("class", "f")], &wide, sf, 0);
    // 空值时光标在文本起点，起点 = 盒子左边 + padding-left
    assert!(
        (c_narrow.0 + c_narrow.2 / 2.0 - (b.0 + 4.0 * sf)).abs() < 0.5,
        "padding-left:4px 时光标应在 {}，实际 {}",
        b.0 + 4.0 * sf,
        c_narrow.0 + c_narrow.2 / 2.0
    );
    assert!(
        c_wide.0 > c_narrow.0 + 30.0 * sf,
        "padding-left 变大时光标要跟着右移：{} -> {}",
        c_narrow.0,
        c_wide.0
    );
}

// ───────────────────────────── 闪烁 ─────────────────────────────

#[test]
fn blink_shows_in_the_first_half_of_each_cycle() {
    let half = cursor_blink_interval_ms();
    assert!(blink_visible_at(0), "相位零点必须可见");
    assert!(blink_visible_at(half - 1));
    assert!(!blink_visible_at(half), "后半周期隐藏");
    assert!(!blink_visible_at(half * 2 - 1));
    assert!(blink_visible_at(half * 2), "下一周期又可见");
}

#[test]
fn blink_interval_matches_the_platform_convention() {
    // 530ms 半周期（≈1.06s 一个完整心跳），与 iOS/微信一致
    assert_eq!(cursor_blink_interval_ms(), 530);
}

#[test]
fn focusing_makes_the_caret_visible_right_away() {
    // 聚焦重置相位零点：不能出现「点进去正好在隐藏半周期、看着像没反应」
    reset_cursor_blink();
    assert!(
        cursor_blink_visible(),
        "reset_cursor_blink() 之后光标必须立刻可见"
    );
}
