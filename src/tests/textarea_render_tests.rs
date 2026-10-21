//! textarea 的多行排版与绘制。
//!
//! 以前 textarea 走单行输入框的绘制：整串值一次画出、垂直居中，`\n` 画成豆腐块，
//! 文字也不折行。这里把「折行规则」「换行不出字形」「光标跟着行走」「光标行露在可视区」
//! 四件事各钉一条可判定的断言。

use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::parser::wxss::{StyleSheet, WxssParser};
use crate::renderer::components::*;
use crate::renderer::components::TextAlign;
use crate::ui::interaction::text_lines::{self, TextLayout};
use crate::{Canvas, Color};

/// 每个字符 10px 的假度量：折行结果只取决于规则，不取决于机器上的字体
fn wrap(text: &str, width: f32) -> Vec<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    layout_textarea_lines(&chars, width, |_| 10.0, TextAlign::Left).iter().map(|l| (l.start, l.end)).collect()
}

#[test]
fn hard_newlines_split_lines_and_skip_the_newline_itself() {
    assert_eq!(wrap("ab\ncd", 100.0), vec![(0, 2), (3, 5)]);
    assert_eq!(wrap("ab\n", 100.0), vec![(0, 2), (3, 3)], "末尾换行后面还有一个空行（光标要能停在那）");
    assert_eq!(wrap("", 100.0), vec![(0, 0)]);
    assert_eq!(wrap("\n\n", 100.0), vec![(0, 0), (1, 1), (2, 2)]);
}

#[test]
fn words_move_to_the_next_line_whole_and_spaces_hang() {
    // 「hello 」正好 60px：行尾空格挂着，不把下一行挤成三行
    assert_eq!(wrap("hello world", 60.0), vec![(0, 6), (6, 11)]);
}

#[test]
fn a_word_longer_than_the_box_breaks_anywhere() {
    // textarea 的 UA 样式是 overflow-wrap: break-word：独占一行也放不下才逐字切
    assert_eq!(wrap("abcdefghij", 45.0), vec![(0, 4), (4, 8), (8, 10)]);
    assert_eq!(wrap("ab abcdefghij", 45.0), vec![(0, 3), (3, 7), (7, 11), (11, 13)], "先整词挪行，再切");
}

#[test]
fn cjk_breaks_between_any_two_characters() {
    assert_eq!(wrap("你好世界", 25.0), vec![(0, 2), (2, 4)]);
}

#[test]
fn text_align_offsets_each_line_ignoring_hanging_spaces() {
    let chars: Vec<char> = "ab  ".chars().collect();
    let lines = layout_textarea_lines(&chars, 100.0, |_| 10.0, TextAlign::Center);
    assert_eq!(lines[0].x0, 40.0, "居中按可见宽 20px 算，行尾空格不算");
    let lines = layout_textarea_lines(&chars, 100.0, |_| 10.0, TextAlign::Right);
    assert_eq!(lines[0].x0, 80.0);
}

// ───────────────────────────── 真字体绘制 ─────────────────────────────

fn textarea(value: &str, sheet: &StyleSheet) -> RenderNode {
    let node = WxmlNode {
        node_type: WxmlNodeType::Element,
        tag_name: "textarea".to_string(),
        attributes: [("value".to_string(), value.to_string()), ("class".to_string(), "t".to_string())]
            .into_iter()
            .collect(),
        children: vec![],
        text_content: String::new(),
    };
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
    InputComponent::build(&node, &mut ctx).expect("textarea build")
}

/// 画一个 300×`h` 逻辑像素的 textarea（2x），返回 (画布, 排版, 光标矩形)
fn draw(value: &str, h: f32, focused: bool, cursor: usize) -> (Canvas, TextLayout, Option<(f32, f32, f32, f32)>) {
    let sheet = WxssParser::new(".t{ padding: 8px 12px; }").parse().unwrap_or_default();
    let rn = textarea(value, &sheet);
    let tr = crate::text::shared_fonts();
    let mut canvas = Canvas::new(700, (h * 2.0) as u32 + 40);
    canvas.clear(Color::WHITE);
    let layout = InputComponent::draw_textarea(
        &rn, &mut canvas, tr.as_deref(), 20.0, 20.0, 600.0, h * 2.0, 2.0, focused, cursor, None, false,
    )
    .expect("有字体就有排版");
    (canvas, layout, if focused { last_caret_rect() } else { None })
}

#[test]
fn a_newline_draws_nothing_by_itself() {
    // 豆腐块回归：「ab\n」与「ab」在第一行上逐像素一致 —— `\n` 不画任何字形
    let (with_nl, layout, _) = draw("ab\n", 100.0, false, 0);
    let (plain, _, _) = draw("ab", 100.0, false, 0);
    assert_eq!(layout.lines.len(), 2);
    assert!(with_nl.to_rgba() == plain.to_rgba(), "换行符不该画出豆腐块");
}

#[test]
fn caret_follows_the_line_it_is_on() {
    let v = "第一行\n第二行";
    let (_, layout, at_end) = draw(v, 100.0, true, 7);
    let (_, _, at_line2_start) = draw(v, 100.0, true, 4);
    let (_, _, at_line1_end) = draw(v, 100.0, true, 3);
    let (end, l2, l1) = (at_end.unwrap(), at_line2_start.unwrap(), at_line1_end.unwrap());
    assert!((l2.1 - l1.1 - layout.line_height).abs() < 0.01, "第二行光标比第一行低一个行高：{l1:?} -> {l2:?}");
    assert!((l2.0 + l2.2 / 2.0 - (20.0 + 24.0)).abs() < 0.01, "第二行行首光标贴着内容区左边：{l2:?}");
    assert_eq!(end.1, l2.1, "同一行的光标同高");
    assert!(end.0 > l2.0, "行内往右走");
}

#[test]
fn the_caret_line_scrolls_into_view() {
    // 40 逻辑像素高的框只放得下一两行：光标在第 6 行时整体上推，光标仍在盒子里
    let v = "1\n2\n3\n4\n5\n6";
    let (_, layout, caret) = draw(v, 40.0, true, v.chars().count());
    let c = caret.unwrap();
    assert!(layout.scroll_y > 0.0, "应当纵向滚动");
    assert!(c.1 >= 20.0 && c.1 + c.3 <= 20.0 + 80.0, "光标应在盒子里：{c:?}");
    // 点击定位按滚动后的行算：点盒子底部那一行，落在最后一行
    let (pos, _) = layout.hit(30.0, 80.0 - 4.0);
    assert_eq!(text_lines::caret_line(&layout.lines, pos, false), 5);
}

#[test]
fn unfocused_textarea_starts_at_the_top() {
    // 不再垂直居中：首行基线 = 内容区顶 + 字号（与 `<text>` 多行同一个算式）
    let (_, layout, _) = draw("abc", 100.0, false, 0);
    assert_eq!(layout.scroll_y, 0.0);
    assert_eq!(layout.origin, (24.0, 16.0), "内容区原点 = padding（设备像素）");
}
