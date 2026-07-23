//! 组件交互测试
//! 覆盖 checkbox / radio / switch / slider / input / button 的交互行为

use crate::ui::interaction::{
    InteractionManager, InteractiveElement, InteractionType, InteractionResult, KeyInput,
};
use crate::Rect;

fn elem(t: InteractionType, id: &str, x: f32, y: f32, w: f32, h: f32) -> InteractiveElement {
    InteractiveElement {
        interaction_type: t,
        id: id.to_string(),
        bounds: Rect::new(x, y, w, h),
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
    }
}

#[test]
fn test_checkbox_toggle() {
    let mut im = InteractionManager::new();
    im.register_element(elem(InteractionType::Checkbox, "cb1", 0.0, 0.0, 40.0, 40.0));

    // 初始未选中
    assert!(im.get_state("cb1").map(|s| s.checked).unwrap_or(false) == false);

    // 点击 -> 选中
    let r = im.handle_click(20.0, 20.0);
    assert!(matches!(r, Some(InteractionResult::Toggle { checked: true, .. })));
    assert!(im.get_state("cb1").unwrap().checked);

    // 再点击 -> 取消
    let r = im.handle_click(20.0, 20.0);
    assert!(matches!(r, Some(InteractionResult::Toggle { checked: false, .. })));
    assert!(!im.get_state("cb1").unwrap().checked);
}

#[test]
fn test_radio_exclusive() {
    let mut im = InteractionManager::new();
    im.register_element(elem(InteractionType::Radio, "r1", 0.0, 0.0, 40.0, 40.0));
    im.register_element(elem(InteractionType::Radio, "r2", 0.0, 50.0, 40.0, 40.0));

    // 选中 r1
    im.handle_click(20.0, 20.0);
    assert!(im.get_state("r1").unwrap().checked);

    // 选中 r2 -> r1 应被取消
    im.handle_click(20.0, 70.0);
    assert!(im.get_state("r2").unwrap().checked);
    assert!(!im.get_state("r1").unwrap().checked, "radio 应互斥");
}

#[test]
fn test_switch_toggle() {
    let mut im = InteractionManager::new();
    im.register_element(elem(InteractionType::Switch, "sw", 0.0, 0.0, 60.0, 30.0));

    let r = im.handle_click(30.0, 15.0);
    assert!(matches!(r, Some(InteractionResult::Toggle { checked: true, .. })));
    assert!(im.get_state("sw").unwrap().checked);
}

#[test]
fn test_slider_drag() {
    let mut im = InteractionManager::new();
    // slider 宽 100，min 0，max 100
    im.register_element(elem(InteractionType::Slider, "sl", 0.0, 0.0, 100.0, 20.0));

    // 点击中点 -> ~50
    let r = im.handle_click(50.0, 10.0);
    match r {
        Some(InteractionResult::SliderChange { value, .. }) => {
            assert!((value - 50).abs() <= 1, "点击中点值应约 50，实际 {}", value);
        }
        _ => panic!("expected SliderChange"),
    }
    assert!(im.is_dragging_slider());

    // 拖动到 75%
    let r = im.handle_mouse_move(75.0, 10.0);
    match r {
        Some(InteractionResult::SliderChange { value, .. }) => {
            assert!((value - 75).abs() <= 1, "拖到 75%，实际 {}", value);
        }
        _ => panic!("expected SliderChange on move"),
    }

    // 释放
    let r = im.handle_mouse_release();
    assert!(matches!(r, Some(InteractionResult::SliderEnd { .. })));
    assert!(!im.is_dragging_slider());
}

#[test]
fn test_slider_clamp() {
    let mut im = InteractionManager::new();
    im.register_element(elem(InteractionType::Slider, "sl", 0.0, 0.0, 100.0, 20.0));
    im.handle_click(50.0, 10.0);
    // 拖到超出右边界 -> 应被 clamp 到 100
    let r = im.handle_mouse_move(999.0, 10.0);
    if let Some(InteractionResult::SliderChange { value, .. }) = r {
        assert_eq!(value, 100);
    } else {
        panic!("expected clamp to max");
    }
    // 拖到超出左边界 -> 0
    let r = im.handle_mouse_move(-50.0, 10.0);
    if let Some(InteractionResult::SliderChange { value, .. }) = r {
        assert_eq!(value, 0);
    } else {
        panic!("expected clamp to min");
    }
}

#[test]
fn test_input_typing_and_edit() {
    let mut im = InteractionManager::new();
    im.register_element(elem(InteractionType::Input, "inp", 0.0, 0.0, 200.0, 40.0));

    // 聚焦
    let r = im.handle_click(10.0, 20.0);
    assert!(matches!(r, Some(InteractionResult::Focus { .. })));
    assert!(im.has_focused_input());

    // 输入 "abc"
    for c in ['a', 'b', 'c'] {
        im.handle_key_input(KeyInput::Char(c));
    }
    assert_eq!(im.get_state("inp").unwrap().value, "abc");

    // 退格
    im.handle_key_input(KeyInput::Backspace);
    assert_eq!(im.get_state("inp").unwrap().value, "ab");

    // 全选后输入 -> 替换
    im.handle_key_input(KeyInput::SelectAll);
    im.handle_key_input(KeyInput::Char('X'));
    assert_eq!(im.get_state("inp").unwrap().value, "X");

    // 失焦
    let r = im.blur_input();
    assert!(matches!(r, Some(InteractionResult::InputBlur { .. })));
    assert!(!im.has_focused_input());
}

#[test]
fn test_input_paste_and_confirm() {
    let mut im = InteractionManager::new();
    im.register_element(elem(InteractionType::Input, "inp", 0.0, 0.0, 200.0, 40.0));
    im.handle_click(10.0, 20.0);

    im.handle_key_input(KeyInput::Paste("hello".to_string()));
    assert_eq!(im.get_state("inp").unwrap().value, "hello");

    // 回车确认 -> InputConfirm 且失焦
    let r = im.handle_key_input(KeyInput::Enter);
    assert!(matches!(r, Some(InteractionResult::InputConfirm { .. })));
    assert!(!im.has_focused_input());
}

#[test]
fn test_input_number_type_validation() {
    let mut im = InteractionManager::new();
    im.register_element(elem(InteractionType::Input, "num", 0.0, 0.0, 200.0, 40.0));
    im.handle_click(10.0, 20.0);
    // 设为 number 类型
    if let Some(inp) = im.focused_input.as_mut() {
        inp.input_type = "number".to_string();
    }
    im.handle_key_input(KeyInput::Char('1'));
    im.handle_key_input(KeyInput::Char('a')); // 应被拒绝
    im.handle_key_input(KeyInput::Char('2'));
    assert_eq!(im.get_state("num").unwrap().value, "12");
}

#[test]
fn test_input_maxlength() {
    let mut im = InteractionManager::new();
    im.register_element(elem(InteractionType::Input, "ml", 0.0, 0.0, 200.0, 40.0));
    im.handle_click(10.0, 20.0);
    if let Some(inp) = im.focused_input.as_mut() {
        inp.maxlength = 3;
    }
    for c in "abcdef".chars() {
        im.handle_key_input(KeyInput::Char(c));
    }
    assert_eq!(im.get_state("ml").unwrap().value, "abc", "应受 maxlength=3 限制");
}

#[test]
fn test_button_click() {
    let mut im = InteractionManager::new();
    im.register_element(elem(InteractionType::Button, "btn", 0.0, 0.0, 120.0, 44.0));
    let r = im.handle_click(60.0, 22.0);
    assert!(matches!(r, Some(InteractionResult::ButtonClick { .. })));
}

#[test]
fn test_disabled_element_not_clickable() {
    let mut im = InteractionManager::new();
    let mut e = elem(InteractionType::Checkbox, "cb", 0.0, 0.0, 40.0, 40.0);
    e.disabled = true;
    im.register_element(e);
    let r = im.handle_click(20.0, 20.0);
    assert!(r.is_none(), "禁用元素不应响应点击");
}

#[test]
fn test_hit_test_topmost() {
    let mut im = InteractionManager::new();
    im.register_element(elem(InteractionType::Button, "bottom", 0.0, 0.0, 100.0, 100.0));
    im.register_element(elem(InteractionType::Checkbox, "top", 0.0, 0.0, 50.0, 50.0));
    // 重叠区域应命中后注册的（top）
    let hit = im.hit_test(25.0, 25.0);
    assert_eq!(hit.map(|e| e.id.as_str()), Some("top"));
}

#[test]
fn test_page_switch_clears_state() {
    let mut im = InteractionManager::new();
    im.register_element(elem(InteractionType::Checkbox, "cb", 0.0, 0.0, 40.0, 40.0));
    im.handle_click(20.0, 20.0);
    assert!(im.get_state("cb").is_some());

    // 切换页面
    im.clear_page_state();
    assert!(im.get_state("cb").is_none());
    assert!(!im.has_focused_input());
    assert!(im.scroll_controllers.is_empty());
}
