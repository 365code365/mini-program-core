//! 滑动/滚动测试
//! 覆盖 ScrollController 拖动、惯性、边界回弹、触底事件，
//! 以及通过 InteractionManager 注册的 scroll-view 滚动，横向滚动，
//! 和渲染层 render_with_scroll。

use crate::ui::interaction::{InteractionManager, InteractiveElement, InteractionType};
use crate::ui::scroll_controller::{ScrollController, ScrollEvent, ScrollDirection};
use crate::renderer::wxml_renderer::WxmlRenderer;
use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::WxssParser;
use crate::{Canvas, Rect};
use serde_json::json;

// ============ ScrollController 直接测试 ============

#[test]
fn test_controller_basic_drag() {
    // 内容 300，视口 100 -> 最大滚动 200
    let mut c = ScrollController::new(300.0, 100.0);
    assert_eq!(c.get_max_scroll(), 200.0);
    assert!(c.is_at_top());

    c.begin_drag(100.0, 0);
    // 手指从 100 移到 60（上移 40）-> 内容向上滚动 40
    c.update_drag(60.0, 16);
    assert!((c.get_position() - 40.0).abs() < 0.5, "位置应约 40，实际 {}", c.get_position());
    c.end_drag();
}

#[test]
fn test_controller_drag_down_stays_top() {
    let mut c = ScrollController::new(300.0, 100.0);
    c.begin_drag(100.0, 0);
    // 顶部继续下拉（手指下移），橡皮筋效果 -> 位置为负但被阻尼
    c.update_drag(160.0, 16);
    assert!(c.get_position() < 0.0, "顶部下拉应产生负偏移(橡皮筋)");
    c.end_drag();
    // 回弹后应回到 0
    for _ in 0..60 {
        c.update(0.016);
    }
    assert!(c.get_position().abs() < 1.0, "回弹后应归零，实际 {}", c.get_position());
}

#[test]
fn test_controller_inertia() {
    let mut c = ScrollController::new(500.0, 100.0);
    c.begin_drag(200.0, 0);
    c.update_drag(100.0, 16); // 快速上移 100px
    let animating = c.end_drag();
    assert!(animating, "快速滑动后应进入惯性动画");
    assert!(c.is_animating());

    let p0 = c.get_position();
    c.update(0.016);
    let p1 = c.get_position();
    assert!(p1 > p0, "惯性应继续增加位置: {} -> {}", p0, p1);
}

#[test]
fn test_controller_bottom_bounce() {
    let mut c = ScrollController::new(200.0, 100.0); // max 100
    c.begin_drag(300.0, 0);
    // 大幅上移，越过底部
    c.update_drag(50.0, 16);
    assert!(c.get_position() > 100.0, "应可越界(橡皮筋)");
    assert!(c.get_position() < 250.0, "越界应被阻尼");
    c.end_drag();
    for _ in 0..60 {
        c.update(0.016);
    }
    assert!((c.get_position() - 100.0).abs() < 1.0, "应回弹到底部 100，实际 {}", c.get_position());
    assert!(c.is_at_bottom());
}

#[test]
fn test_controller_reach_bottom_event() {
    let mut c = ScrollController::new(1000.0, 100.0); // max 900
    // 直接滚动到接近底部
    c.handle_scroll(900.0, false);
    assert!(c.check_reach_bottom(), "接近底部应触发触底");
    // 已触发后不应重复
    assert!(!c.check_reach_bottom());
    // 重置后可再次触发
    c.reset_reach_bottom();
    assert!(c.check_reach_bottom());
}

#[test]
fn test_controller_wheel_scroll_clamped() {
    let mut c = ScrollController::new(300.0, 100.0); // max 200
    c.handle_scroll(50.0, false); // 鼠标滚轮 delta*2 = 100
    assert!(c.get_position() > 0.0 && c.get_position() <= 200.0);
    // 大量向下滚动，应被 clamp
    c.handle_scroll(9999.0, false);
    assert_eq!(c.get_position(), 200.0);
    // 向上滚回顶部
    c.handle_scroll(-9999.0, false);
    assert_eq!(c.get_position(), 0.0);
}

#[test]
fn test_controller_horizontal() {
    let mut c = ScrollController::new_horizontal(600.0, 200.0); // max 400
    assert_eq!(c.get_direction(), ScrollDirection::Horizontal);
    c.begin_drag(300.0, 0);
    c.update_drag(200.0, 16); // 左移 100
    assert!((c.get_position() - 100.0).abs() < 0.5);
    c.end_drag();
}

#[test]
fn test_controller_content_resize() {
    let mut c = ScrollController::new(300.0, 100.0); // max 200
    c.handle_scroll(999.0, false);
    assert_eq!(c.get_position(), 200.0);
    // 内容变短 -> 位置应被约束
    c.update_content_height(150.0, 100.0); // max 50
    assert!(c.get_position() <= 50.0, "内容变短后位置应被约束到新 max");
}

#[test]
fn test_controller_update_with_events_reach_bottom() {
    let mut c = ScrollController::new(300.0, 100.0); // max 200
    c.begin_drag(500.0, 0);
    c.update_drag(100.0, 16); // 快速上移越界
    c.end_drag();
    let mut got_bottom = false;
    for _ in 0..120 {
        let (_animating, ev) = c.update_with_events(0.016);
        if ev == Some(ScrollEvent::ReachBottom) {
            got_bottom = true;
            break;
        }
    }
    assert!(got_bottom, "滚动到底部应触发 ReachBottom 事件");
}

// ============ 通过 InteractionManager 的 scroll-view 测试 ============

fn scroll_area(id: &str, content_h: f32, viewport_h: f32, horizontal: bool) -> InteractiveElement {
    InteractiveElement {
        interaction_type: InteractionType::ScrollArea,
        id: id.to_string(),
        bounds: Rect::new(0.0, 0.0, if horizontal { viewport_h } else { 375.0 }, viewport_h),
        checked: false,
        value: String::new(),
        disabled: false,
        min: 0.0,
        max: 0.0,
        content_height: if horizontal { 0.0 } else { content_h },
        viewport_height: if horizontal { 0.0 } else { viewport_h },
        content_width: if horizontal { content_h } else { 0.0 },
        viewport_width: if horizontal { viewport_h } else { 0.0 },
        is_horizontal: horizontal,
        is_fixed: false,
    }
}

#[test]
fn test_scroll_area_registration_creates_controller() {
    let mut im = InteractionManager::new();
    im.register_element(scroll_area("sv", 800.0, 200.0, false));
    let c = im.get_scroll_controller("sv");
    assert!(c.is_some(), "注册 scroll-view 应创建滚动控制器");
    assert_eq!(c.unwrap().get_max_scroll(), 600.0);
}

#[test]
fn test_scroll_area_drag_via_manager() {
    let mut im = InteractionManager::new();
    im.register_element(scroll_area("sv", 800.0, 200.0, false));

    let c = im.get_scroll_controller_mut("sv").unwrap();
    c.begin_drag(150.0, 0);
    c.update_drag(100.0, 16); // 上移 50
    assert!((c.get_position() - 50.0).abs() < 0.5);
    c.end_drag();
}

#[test]
fn test_horizontal_scroll_area() {
    let mut im = InteractionManager::new();
    im.register_element(scroll_area("svx", 900.0, 300.0, true));
    let c = im.get_scroll_controller("svx").unwrap();
    assert_eq!(c.get_direction(), ScrollDirection::Horizontal);
    assert_eq!(c.get_max_scroll(), 600.0);
}

// ============ 渲染层滚动 ============

#[test]
fn test_render_with_scroll_offsets() {
    let css = r#"
        .list-item { height: 100rpx; padding: 20rpx; }
        .fixed-bar { position: fixed; bottom: 0; left: 0; right: 0; height: 100rpx; }
    "#;
    let wxml = r#"
        <view>
            <view class="list-item" wx:for="{{items}}" wx:key="*this">
                <text>Row {{item}}</text>
            </view>
            <view class="fixed-bar"><text>Bar</text></view>
        </view>
    "#;
    let ss = WxssParser::new(css).parse().unwrap_or_default();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 2.0);
    let mut c = Canvas::new(750, 1334);
    let mut im = InteractionManager::new();
    let ns = WxmlParser::new(wxml).parse().unwrap();
    let data = json!({ "items": (1..=30).collect::<Vec<i32>>() });

    // 不同滚动偏移下渲染都不应 panic，并返回内容高度
    let h0 = r.render_with_scroll_and_viewport(&mut c, &ns, &data, &mut im, 0.0, 667.0);
    let h1 = r.render_with_scroll_and_viewport(&mut c, &ns, &data, &mut im, 300.0, 667.0);
    let h2 = r.render_with_scroll_and_viewport(&mut c, &ns, &data, &mut im, 600.0, 667.0);
    assert!(h0 > 0.0 && h1 > 0.0 && h2 > 0.0);
}
