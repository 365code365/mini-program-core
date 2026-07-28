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

/// 覆盖层里的输入框，即使**正常流有元素压在同一个坐标上**也必须能聚焦。
///
/// 元素表的顺序是「覆盖层的在前、正常流的在后」（正常流每帧重建并追加），
/// 而命中是从后往前找。从前 `handle_click_scoped(fixed_only)` 写成
/// 「全局 hit_test 之后再判 is_fixed」，于是先撞上正常流那个元素、再被过滤掉，
/// 返回 None —— 底部固定输入条点不出焦点、固定层里的开关拨不动。
#[test]
fn fixed_scope_click_ignores_flow_element_on_top() {
    let mut im = InteractionManager::new();
    // 覆盖层里的输入框（视口坐标）先注册
    let mut input = elem(InteractionType::Input, "input_25_520", 25.0, 520.0, 272.0, 36.0);
    input.is_fixed = true;
    im.register_element(input);
    // 正常流里恰好压在同一片区域的可点元素后注册
    im.register_element(elem(InteractionType::Checkbox, "flow_card", 0.0, 500.0, 375.0, 80.0));

    let r = im.handle_click_scoped(100.0, 537.0, true);
    match r {
        Some(InteractionResult::Focus { ref id, .. }) => assert_eq!(id, "input_25_520"),
        other => panic!("覆盖层输入框应当获得焦点，实际是 {other:?}"),
    }
    assert!(im.has_focused_input());
}

/// 同一坐标上「正常流」的作用域不该被覆盖层元素抢走
#[test]
fn flow_scope_click_ignores_fixed_element() {
    let mut im = InteractionManager::new();
    let mut fixed = elem(InteractionType::Checkbox, "fixed_cb", 0.0, 500.0, 375.0, 80.0);
    fixed.is_fixed = true;
    im.register_element(fixed);
    im.register_element(elem(InteractionType::Switch, "flow_sw", 0.0, 500.0, 375.0, 80.0));

    let r = im.handle_click(100.0, 537.0);
    match r {
        Some(InteractionResult::Toggle { ref id, .. }) => assert_eq!(id, "flow_sw"),
        other => panic!("正常流的开关应当被拨动，实际是 {other:?}"),
    }
}

/// 滚动区里盖着一张卡片时，滚轮仍应交给滚动区（而不是被卡片挡掉）
#[test]
fn scroll_area_hit_ignores_cards_on_top() {
    let mut im = InteractionManager::new();
    let mut area = elem(InteractionType::ScrollArea, "sv", 0.0, 100.0, 375.0, 200.0);
    area.is_horizontal = true;
    area.content_width = 900.0;
    area.viewport_width = 375.0;
    im.register_element(area);
    // 卡片压在滚动区上面
    im.register_element(elem(InteractionType::Checkbox, "card", 10.0, 120.0, 150.0, 150.0));

    let hit = im.hit_test_scroll_area(60.0, 180.0, false).expect("应命中滚动区");
    assert_eq!(hit.id, "sv");
    assert!(hit.is_horizontal);
}

/// `checkbox-group` 的 `change` 事件要发**组内选中项的 value 数组**（微信语义），
/// 不是被点那一个的开关状态。页面里 `e.detail.value.indexOf(...)` 就是这么用的，
/// 发个布尔值过去直接 `TypeError: not a function`。
#[test]
fn checked_values_in_group_are_collected_in_document_order() {
    let mut im = InteractionManager::new();
    let mut mk = |id: &str, value: &str, x: f32, checked: bool| {
        let mut e = elem(InteractionType::Checkbox, id, x, 100.0, 40.0, 40.0);
        e.value = value.to_string();
        e.checked = checked;
        im.register_element(e);
    };
    mk("cb_read", "reading", 10.0, true);
    mk("cb_sport", "sport", 120.0, false);
    mk("cb_music", "music", 230.0, true);
    // 组外的那个不该被算进来
    let mut outside = elem(InteractionType::Checkbox, "cb_other", 10.0, 400.0, 40.0, 40.0);
    outside.value = "other".into();
    outside.checked = true;
    im.register_element(outside);

    let group = Rect::new(0.0, 90.0, 375.0, 60.0);
    let vals = im.checked_values_in(&group, InteractionType::Checkbox);
    assert_eq!(vals, vec!["reading".to_string(), "music".to_string()]);

    // 点一下 sport：状态以 states 为准，顺序仍按文档顺序
    im.handle_click(140.0, 120.0);
    let vals = im.checked_values_in(&group, InteractionType::Checkbox);
    assert_eq!(
        vals,
        vec!["reading".to_string(), "sport".to_string(), "music".to_string()]
    );
}
