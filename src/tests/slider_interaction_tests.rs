//! slider 的交互层：点按跳转、横向拖动（抓圆点保持偏移 / 抓轨道跳到手指）、step 吸附、
//! 受控值（页面 setData 改了 value 就以页面为准）。
//!
//! 几何与登记端（`renderer/wxml_renderer/interactions.rs`）同一份算式：
//! 盒子 x=20 宽 300、block-size 28、show-value → 轨道 34..256（左右各让 14，右侧再让 50）。
use crate::renderer::components::{slider_track_span, SliderAttrs, SLIDER_VALUE_WIDTH};
use crate::ui::interaction::{InteractionManager, InteractionResult, InteractionType, InteractiveElement, SliderSpec};
use crate::Rect;
use std::collections::HashMap;

/// 按渲染端的登记方式登记一个 slider（每帧都会这么做一次）
fn register(im: &mut InteractionManager, id: &str, pairs: &[(&str, &str)]) -> SliderSpec {
    let attrs: HashMap<String, String> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    let a = SliderAttrs::from_attrs(&attrs);
    let bounds = Rect::new(20.0, 100.0, 300.0, 28.0);
    let value_w = if a.show_value { SLIDER_VALUE_WIDTH } else { 0.0 };
    let (track_x, track_w) = slider_track_span(bounds.x, bounds.width, a.block_size, value_w);
    let spec = SliderSpec { attrs: a, bounds, track_x, track_w, is_fixed: false };
    im.note_slider(id.to_string(), spec);
    im.register_element(InteractiveElement {
        interaction_type: InteractionType::Slider,
        id: id.to_string(),
        bounds,
        checked: false,
        value: SliderAttrs::format(a.value),
        disabled: false,
        min: a.min as f32,
        max: a.max as f32,
        content_height: 0.0,
        viewport_height: 0.0,
        content_width: 0.0,
        viewport_width: 0.0,
        is_horizontal: false,
        is_fixed: false,
    });
    spec
}

#[test]
fn 点按跳到手指处且不进入拖动() {
    let mut im = InteractionManager::new();
    let s = register(&mut im, "sl", &[("min", "1"), ("max", "5"), ("value", "4"), ("show-value", "true")]);
    // 轨道 25% 处 → 2（min=1 max=5）。从前按整个盒子换算、再用 v/100 画，滑块直接跳到最左
    let r = im.handle_click(s.track_x + s.track_w * 0.25, 114.0);
    match r {
        Some(InteractionResult::SliderChange { value, .. }) => assert_eq!(value, 2.0),
        other => panic!("应该是 SliderChange，实际 {other:?}"),
    }
    assert!(!im.is_dragging_slider(), "点按之后不该处于拖动态（否则鼠标悬停也会带着滑块走）");
    assert_eq!(im.slider_value("sl", 4.0), Some(2.0));
}

#[test]
fn 抓圆点拖动保持偏移且按step吸附() {
    let mut im = InteractionManager::new();
    let s = register(&mut im, "sl", &[("value", "50"), ("step", "10")]);
    let thumb = s.thumb_x(50.0);
    // 在圆心右边 8px 处抓起、原地锁定：值不变（不会一抓就跳到手指下）
    assert_eq!(im.begin_slider_drag("sl", thumb + 8.0, thumb + 8.0), None);
    // 往右推 30% 轨道长：50 → 80
    let v = im.drag_slider_to(thumb + 8.0 + s.track_w * 0.3);
    assert_eq!(v, Some(80.0));
    // 吸附后没换格：不报变化（不发 changing、不重画）
    assert_eq!(im.drag_slider_to(thumb + 8.0 + s.track_w * 0.31), None);
    // 拖出轨道：夹到 max
    assert_eq!(im.drag_slider_to(5000.0), Some(100.0));
    assert_eq!(im.end_slider_drag(), Some(("sl".to_string(), 100.0, true)));
    assert!(!im.is_dragging_slider());
}

#[test]
fn 从轨道别处拖起直接跳到手指() {
    let mut im = InteractionManager::new();
    let s = register(&mut im, "sl", &[("value", "0")]);
    // 圆点在最左；从 75% 处按下横拖
    let x = s.track_x + s.track_w * 0.75;
    assert_eq!(im.begin_slider_drag("sl", x, x + 2.0), Some(76.0));
}

#[test]
fn 拖动不动就不算拖过() {
    let mut im = InteractionManager::new();
    let s = register(&mut im, "sl", &[("value", "50")]);
    let thumb = s.thumb_x(50.0);
    im.begin_slider_drag("sl", thumb, thumb);
    assert_eq!(im.end_slider_drag(), Some(("sl".to_string(), 50.0, false)));
}

#[test]
fn 页面改了value就以页面为准() {
    let mut im = InteractionManager::new();
    let s = register(&mut im, "sl", &[("value", "30")]);
    im.handle_click(s.track_x + s.track_w * 0.9, 114.0);
    assert_eq!(im.slider_value("sl", 30.0), Some(90.0));
    // 页面没改 value（没绑定 / 不回写）：下一帧照样是用户拖到的位置
    register(&mut im, "sl", &[("value", "30")]);
    assert_eq!(im.slider_value("sl", 30.0), Some(90.0));
    // 页面 setData 把 value 改成 10：交互状态作废，以页面为准
    register(&mut im, "sl", &[("value", "10")]);
    assert_eq!(im.slider_value("sl", 10.0), None);
    assert!(im.get_state("sl").is_none(), "过期的交互状态应在登记时清掉");
}

#[test]
fn 拖动中页面回写value不打断拖动() {
    // bindchanging 里 setData({value}) 很常见：拖动中 WXML 的 value 每一步都在变，不能因此丢状态
    let mut im = InteractionManager::new();
    let s = register(&mut im, "sl", &[("value", "0")]);
    let x0 = s.track_x + s.track_w * 0.2;
    assert_eq!(im.begin_slider_drag("sl", x0, x0), Some(20.0));
    register(&mut im, "sl", &[("value", "20")]);
    assert_eq!(im.drag_slider_to(s.track_x + s.track_w * 0.4), Some(40.0));
    assert_eq!(im.slider_value("sl", 20.0), Some(40.0));
}

#[test]
fn 命中只认最上层的可用slider() {
    let mut im = InteractionManager::new();
    let s = register(&mut im, "sl", &[]);
    assert_eq!(im.slider_at(s.track_x, 114.0, false).as_deref(), Some("sl"));
    assert_eq!(im.slider_at(s.track_x, 114.0, true), None, "覆盖层查询不该命中正常流的滑块");
    assert_eq!(im.slider_at(s.track_x, 300.0, false), None);
}
