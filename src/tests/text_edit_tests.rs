//! 输入框编辑：换行、快捷键、撤销、textarea 的按行移动（只走 `InteractionManager`，不开渲染器）。
//!
//! 起因是 textarea 里按回车：值里多出 `\n`、画成豆腐块（「212121□□assa□」），
//! 而且回车只会「确认并失焦」，↑↓ / 按词移动 / 撤销这些常用快捷键一个都没有。
//! 期望值都按微信 textarea / 浏览器的语义写死，判据可复现。

use crate::ui::interaction::{
    text_lines, InteractionManager, InteractionResult, InteractionType, InteractiveElement, KeyInput, KEY_NAMES,
};
use crate::Rect;

fn focused(multiline: bool) -> InteractionManager {
    let mut im = InteractionManager::new();
    im.register_element(InteractiveElement {
        interaction_type: InteractionType::Input,
        id: "ta".to_string(),
        bounds: Rect::new(0.0, 0.0, 300.0, 120.0),
        checked: false,
        value: String::new(),
        disabled: false,
        min: 0.0,
        max: 100.0,
        content_height: 0.0,
        viewport_height: 0.0,
        content_width: 0.0,
        viewport_width: 0.0,
        is_horizontal: false,
        is_fixed: false,
    });
    assert!(matches!(im.handle_click(10.0, 10.0), Some(InteractionResult::Focus { .. })));
    let f = im.focused_input.as_mut().unwrap();
    f.multiline = multiline;
    f.confirm_type = if multiline { "return" } else { "done" }.to_string();
    im
}

fn value(im: &InteractionManager) -> String {
    im.focused_input.as_ref().map(|f| f.value.clone()).unwrap_or_else(|| im.get_state("ta").unwrap().value.clone())
}

fn cursor(im: &InteractionManager) -> usize {
    im.focused_input.as_ref().unwrap().cursor_pos
}

fn typed(im: &mut InteractionManager, s: &str) {
    for c in s.chars() {
        im.handle_key_input(KeyInput::Char(c));
    }
}

// ───────────────────────────── 回车与换行 ─────────────────────────────

#[test]
fn textarea_enter_inserts_a_newline_and_keeps_focus() {
    let mut im = focused(true);
    typed(&mut im, "ab");
    let r = im.handle_key_input(KeyInput::Enter);
    assert!(matches!(r, Some(InteractionResult::InputChange { ref value, .. }) if value == "ab\n"), "{r:?}");
    typed(&mut im, "cd");
    assert_eq!(value(&im), "ab\ncd");
    assert!(im.has_focused_input(), "textarea 回车只换行，不失焦");
    assert_eq!(im.get_state("ta").unwrap().value, "ab\ncd", "渲染读的状态要同步");
}

#[test]
fn textarea_with_send_confirm_type_confirms_on_enter_but_shift_enter_still_breaks() {
    let mut im = focused(true);
    im.focused_input.as_mut().unwrap().confirm_type = "send".into();
    typed(&mut im, "hi");
    im.handle_key_input(KeyInput::Newline);
    assert_eq!(value(&im), "hi\n", "Shift+Enter 在 send 模式下照样换行");
    let r = im.handle_key_input(KeyInput::Enter);
    assert!(matches!(r, Some(InteractionResult::InputConfirm { keep_focus: false, .. })), "{r:?}");
    assert!(!im.has_focused_input());
}

#[test]
fn input_confirm_hold_keeps_focus() {
    let mut im = focused(false);
    im.focused_input.as_mut().unwrap().confirm_hold = true;
    typed(&mut im, "q");
    let r = im.handle_key_input(KeyInput::Enter);
    assert!(matches!(r, Some(InteractionResult::InputConfirm { keep_focus: true, .. })), "{r:?}");
    assert!(im.has_focused_input(), "confirm-hold 时确认后仍聚焦");
}

#[test]
fn single_line_input_never_stores_a_newline() {
    let mut im = focused(false);
    typed(&mut im, "a");
    assert!(im.handle_key_input(KeyInput::Char('\n')).is_none());
    im.handle_key_input(KeyInput::Paste("b\r\nc".into()));
    assert_eq!(value(&im), "abc", "单行粘贴多行文本：换行去掉");
}

#[test]
fn committed_newline_text_acts_as_the_return_key() {
    // iOS insertText("\n")、Android 硬件回车、Flutter 多行输入的回车都以整串 "\n" 送来
    let mut ta = focused(true);
    ta.insert_text("ab");
    ta.insert_text("\r\n");
    ta.insert_text("c\rd");
    assert_eq!(value(&ta), "ab\nc\nd", "\\r\\n 与 \\r 都归一成 \\n");

    let mut inp = focused(false);
    inp.insert_text("12");
    let r = inp.insert_text("\n");
    assert!(matches!(r.as_slice(), [InteractionResult::InputConfirm { .. }]), "{r:?}");
}

#[test]
fn control_characters_are_dropped_instead_of_drawn_as_tofu() {
    let mut im = focused(true);
    im.insert_text("a\tb\u{8}c\u{7f}d\u{0}");
    assert_eq!(value(&im), "abcd");
}

#[test]
fn newline_counts_against_maxlength() {
    let mut im = focused(true);
    im.focused_input.as_mut().unwrap().maxlength = 3;
    typed(&mut im, "ab");
    im.handle_key_input(KeyInput::Enter);
    assert!(im.handle_key_input(KeyInput::Enter).is_none(), "满了就不再换行");
    assert_eq!(value(&im), "ab\n");
}

// ───────────────────────────── 移动与选区 ─────────────────────────────

#[test]
fn up_down_walk_lines_and_keep_the_goal_column() {
    let mut im = focused(true);
    typed(&mut im, "abcde");
    im.handle_key_input(KeyInput::Enter);
    typed(&mut im, "x");
    im.handle_key_input(KeyInput::Enter);
    typed(&mut im, "fghij"); // 光标在第 3 行第 5 列（下标 13）
    assert_eq!(cursor(&im), 13);
    im.handle_key_input(KeyInput::Up);
    assert_eq!(cursor(&im), 7, "短行上停在行尾");
    im.handle_key_input(KeyInput::Up);
    assert_eq!(cursor(&im), 5, "回到长行时恢复原来那一列");
    im.handle_key_input(KeyInput::Up);
    assert_eq!(cursor(&im), 0, "第一行再往上到全文开头");
    im.handle_key_input(KeyInput::Down);
    im.handle_key_input(KeyInput::Down);
    im.handle_key_input(KeyInput::Down);
    assert_eq!(cursor(&im), 13, "最后一行再往下到结尾");
}

#[test]
fn home_end_are_per_line_in_a_textarea() {
    let mut im = focused(true);
    im.insert_text("one\ntwo\nthree");
    im.handle_key_input(KeyInput::Up); // 第 2 行
    im.handle_key_input(KeyInput::Home);
    assert_eq!(cursor(&im), 4);
    im.handle_key_input(KeyInput::End);
    assert_eq!(cursor(&im), 7);
    im.handle_key_input(KeyInput::DocStart);
    assert_eq!(cursor(&im), 0);
    im.handle_key_input(KeyInput::DocEnd);
    assert_eq!(cursor(&im), 13);
}

#[test]
fn word_motions_and_deletes() {
    let mut im = focused(false);
    typed(&mut im, "hello world, 你好");
    im.handle_key_input(KeyInput::WordLeft);
    assert_eq!(cursor(&im), 13, "CJK 连续一段算一个词");
    im.handle_key_input(KeyInput::WordLeft);
    assert_eq!(cursor(&im), 11, "标点自成一段");
    im.handle_key_input(KeyInput::WordLeft);
    assert_eq!(cursor(&im), 6);
    im.handle_key_input(KeyInput::WordRight);
    assert_eq!(cursor(&im), 11);
    im.handle_key_input(KeyInput::DocEnd);
    im.handle_key_input(KeyInput::DeleteWordBack);
    assert_eq!(value(&im), "hello world, ");
    im.handle_key_input(KeyInput::DocStart);
    im.handle_key_input(KeyInput::DeleteWordForward);
    assert_eq!(value(&im), " world, ");
}

#[test]
fn delete_line_back_removes_to_line_start_then_joins_lines() {
    let mut im = focused(true);
    im.insert_text("ab\ncd");
    im.handle_key_input(KeyInput::DeleteLineBack);
    assert_eq!(value(&im), "ab\n");
    im.handle_key_input(KeyInput::DeleteLineBack);
    assert_eq!(value(&im), "ab", "已在行首：删掉换行把两行接起来");
}

#[test]
fn left_right_collapse_a_selection_to_its_edge() {
    let mut im = focused(false);
    typed(&mut im, "abcdef");
    im.handle_key_input(KeyInput::Left);
    im.handle_key_input(KeyInput::ShiftLeft);
    im.handle_key_input(KeyInput::ShiftLeft); // 选中 [3, 5)
    assert_eq!(im.focused_input.as_ref().unwrap().get_selection_range(), Some((3, 5)));
    im.handle_key_input(KeyInput::Right);
    assert_eq!(cursor(&im), 5, "有选区时 → 收拢到选区右端，不再多走一格");
    im.handle_key_input(KeyInput::ShiftWordLeft);
    assert_eq!(im.focused_input.as_ref().unwrap().get_selection_range(), Some((0, 5)));
    im.handle_key_input(KeyInput::Left);
    assert_eq!(cursor(&im), 0);
}

#[test]
fn shift_up_extends_the_selection_across_lines() {
    let mut im = focused(true);
    im.insert_text("abc\ndef");
    im.handle_key_input(KeyInput::ShiftUp);
    assert_eq!(im.focused_input.as_ref().unwrap().get_selection_range(), Some((3, 7)));
    im.handle_key_input(KeyInput::Backspace);
    assert_eq!(value(&im), "abc");
}

#[test]
fn cut_reports_the_new_value() {
    let mut im = focused(false);
    typed(&mut im, "abc");
    im.handle_key_input(KeyInput::ShiftLeft);
    let r = im.handle_key_input(KeyInput::Cut);
    assert!(
        matches!(r, Some(InteractionResult::CutText { ref text, ref value, .. }) if text == "c" && value == "ab"),
        "{r:?}"
    );
    assert_eq!(im.get_state("ta").unwrap().value, "ab");
}

// ───────────────────────────── 撤销 / 重做 ─────────────────────────────

#[test]
fn undo_merges_consecutive_typing_and_redo_replays() {
    let mut im = focused(true);
    typed(&mut im, "abc");
    im.handle_key_input(KeyInput::Enter);
    typed(&mut im, "de");
    im.handle_key_input(KeyInput::Backspace);
    assert_eq!(value(&im), "abc\nd");
    im.handle_key_input(KeyInput::Undo);
    assert_eq!(value(&im), "abc\nde", "撤销退格");
    im.handle_key_input(KeyInput::Undo);
    assert_eq!(value(&im), "abc\n", "连续打的 de 一步撤掉");
    im.handle_key_input(KeyInput::Undo);
    assert_eq!(value(&im), "abc", "换行自成一步");
    im.handle_key_input(KeyInput::Undo);
    assert_eq!(value(&im), "");
    assert!(im.handle_key_input(KeyInput::Undo).is_none(), "栈空了不发 input");
    im.handle_key_input(KeyInput::Redo);
    im.handle_key_input(KeyInput::Redo);
    assert_eq!(value(&im), "abc\n");
    assert_eq!(cursor(&im), 4);
    typed(&mut im, "z");
    assert!(im.handle_key_input(KeyInput::Redo).is_none(), "有新编辑之后重做栈作废");
}

#[test]
fn moving_the_caret_splits_typing_into_separate_undo_steps() {
    let mut im = focused(false);
    typed(&mut im, "ab");
    im.handle_key_input(KeyInput::Left);
    typed(&mut im, "X");
    assert_eq!(value(&im), "aXb");
    im.handle_key_input(KeyInput::Undo);
    assert_eq!(value(&im), "ab");
    assert_eq!(cursor(&im), 1, "撤销后光标回到编辑处");
}

// ───────────────────────────── 按键名 ─────────────────────────────

#[test]
fn every_documented_key_name_resolves() {
    for name in KEY_NAMES {
        assert!(KeyInput::from_name(name).is_some(), "{name} 应该认得");
    }
    assert_eq!(KeyInput::from_name("Shift+Left"), Some(KeyInput::ShiftLeft), "不分大小写");
    assert_eq!(KeyInput::from_name("nope"), None);
}

// ───────────────────────────── 视觉行（与字体无关的部分） ─────────────────────────────

#[test]
fn soft_wrapped_line_end_uses_upstream_affinity() {
    // 「hello 」+「world」：第一行软换行结束于 6，第二行从 6 开始
    let lines = vec![
        text_lines::TextLine { start: 0, end: 6, x0: 0.0, xs: (0..=6).map(|i| i as f32 * 10.0).collect() },
        text_lines::TextLine { start: 6, end: 11, x0: 0.0, xs: (0..=5).map(|i| i as f32 * 10.0).collect() },
    ];
    assert!(text_lines::is_soft_end(&lines, 0));
    assert_eq!(text_lines::caret_line(&lines, 6, false), 1, "默认算下一行首");
    assert_eq!(text_lines::caret_line(&lines, 6, true), 0, "upstream 时画在上一行末");
    assert_eq!(text_lines::line_end(&lines, 2, false), (6, true), "折行那一行的 End 带 upstream");
    let layout = text_lines::TextLayout {
        text: "hello world".into(),
        lines,
        line_height: 20.0,
        origin: (5.0, 5.0),
        scroll_y: 0.0,
    };
    assert_eq!(layout.hit(5.0 + 23.0, 5.0 + 30.0), (8, false), "第二行第 2 个字符后面");
    assert_eq!(layout.hit(5.0 + 200.0, 5.0 + 1.0), (6, true), "点第一行右边空白：停在第一行末");
    assert_eq!(layout.hit(5.0 + 200.0, 500.0), (11, false), "点到最后一行下面：落在最后一行");
}
