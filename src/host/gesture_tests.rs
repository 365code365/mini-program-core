//! `gesture.rs` 的单测（从那边原样搬出：主文件超过 500 行）
use super::*;

fn vertical_area() -> Option<(String, Axis)> {
    Some(("list".to_string(), Axis::Vertical))
}
fn horizontal_area() -> Option<(String, Axis)> {
    Some(("row".to_string(), Axis::Horizontal))
}

/// 抖动不该定方向
#[test]
fn tiny_move_stays_undecided() {
    let mut g = DragGesture::new((100.0, 100.0), vertical_area(), false, true);
    assert_eq!(g.on_move(101.0, 102.0), GestureAction::None);
    assert_eq!(*g.target(), GestureTarget::Undecided);
}

/// 纵向划 + 纵向 scroll-view → 交给它，并且是从按下点开始拖
#[test]
fn vertical_drag_goes_to_vertical_area() {
    let mut g = DragGesture::new((100.0, 100.0), vertical_area(), false, true);
    assert_eq!(
        g.on_move(101.0, 130.0),
        GestureAction::BeginArea { id: "list".into(), axis: Axis::Vertical }
    );
    assert_eq!(
        g.on_move(101.0, 160.0),
        GestureAction::UpdateArea { id: "list".into(), axis: Axis::Vertical }
    );
}

/// **横向 scroll-view 里纵向划要滚页面** —— 旧实现会被横向容器吃掉，手指竖划没反应
#[test]
fn vertical_drag_inside_horizontal_area_scrolls_page() {
    let mut g = DragGesture::new((100.0, 100.0), horizontal_area(), false, true);
    assert_eq!(g.on_move(102.0, 140.0), GestureAction::BeginPage);
    assert_eq!(*g.target(), GestureTarget::Page);
    assert_eq!(g.on_move(102.0, 180.0), GestureAction::UpdatePage);
}

/// 横向划 + 横向 scroll-view → 交给它；页面不该跟着上下动
#[test]
fn horizontal_drag_goes_to_horizontal_area() {
    let mut g = DragGesture::new((100.0, 100.0), horizontal_area(), false, true);
    assert_eq!(
        g.on_move(60.0, 101.0),
        GestureAction::BeginArea { id: "row".into(), axis: Axis::Horizontal }
    );
}

/// 纵向容器里横向划：谁都不滚（页面只能纵向滚）
#[test]
fn horizontal_drag_with_only_vertical_scrollables_does_nothing() {
    let mut g = DragGesture::new((100.0, 100.0), vertical_area(), false, true);
    assert_eq!(g.on_move(40.0, 101.0), GestureAction::None);
    assert_eq!(*g.target(), GestureTarget::Blocked);
}

/// `catchtouchmove`：一律不滚（遮罩锁住下层页面）
#[test]
fn catch_touchmove_blocks_scrolling() {
    let mut g = DragGesture::new((100.0, 100.0), vertical_area(), true, true);
    assert_eq!(g.on_move(100.0, 200.0), GestureAction::None);
    assert_eq!(*g.target(), GestureTarget::Blocked);
}

/// 页面不可滚且没有可滚区域时，纵向划也不该动
#[test]
fn nothing_scrollable_means_blocked() {
    let mut g = DragGesture::new((100.0, 100.0), None, false, false);
    assert_eq!(g.on_move(100.0, 200.0), GestureAction::None);
    assert_eq!(*g.target(), GestureTarget::Blocked);
}

/// 嵌套传递：内层到边界 → 页面接手，且从当前点开始
#[test]
fn handoff_switches_to_page_from_current_point() {
    let mut g = DragGesture::new((100.0, 100.0), vertical_area(), false, true);
    g.on_move(100.0, 140.0);
    assert_eq!(g.handoff_to_page(), GestureAction::BeginPage);
    assert_eq!(g.last(), (100.0, 140.0));
    assert_eq!(g.on_move(100.0, 180.0), GestureAction::UpdatePage);
}

/// 左边缘往右划：即使命中了横向 scroll-view，也要走侧滑返回（系统手势优先）
#[test]
fn edge_swipe_right_wins_over_horizontal_area() {
    let mut g = DragGesture::new((6.0, 300.0), horizontal_area(), false, true).allow_edge_back(true);
    assert_eq!(g.on_move(40.0, 301.0), GestureAction::BeginEdgeBack);
    assert_eq!(*g.target(), GestureTarget::EdgeBack);
    assert_eq!(g.on_move(80.0, 302.0), GestureAction::UpdateEdgeBack);
    // 已经锁成侧滑返回，就不再交给页面
    assert_eq!(g.handoff_to_page(), GestureAction::None);
}

/// `catchtouchmove` 拦不住系统级的侧滑返回
#[test]
fn edge_swipe_ignores_catch_touchmove() {
    let mut g = DragGesture::new((2.0, 300.0), None, true, true).allow_edge_back(true);
    assert_eq!(g.on_move(40.0, 300.0), GestureAction::BeginEdgeBack);
}

/// 边缘往**左**划、或纵向划，都不是侧滑返回
#[test]
fn edge_swipe_needs_rightward_horizontal_move() {
    let mut g = DragGesture::new((6.0, 300.0), vertical_area(), false, true).allow_edge_back(true);
    assert_eq!(
        g.on_move(6.0, 350.0),
        GestureAction::BeginArea { id: "list".into(), axis: Axis::Vertical }
    );

    let mut g2 = DragGesture::new((18.0, 300.0), horizontal_area(), false, true).allow_edge_back(true);
    assert_eq!(
        g2.on_move(2.0, 300.0),
        GestureAction::BeginArea { id: "row".into(), axis: Axis::Horizontal }
    );
}

/// 宿主没开这个开关（栈底页 / 弹窗弹着）时，边缘右划退回普通仲裁
#[test]
fn edge_swipe_disabled_falls_back_to_normal_arbitration() {
    let mut g = DragGesture::new((4.0, 300.0), horizontal_area(), false, true);
    assert_eq!(
        g.on_move(40.0, 300.0),
        GestureAction::BeginArea { id: "row".into(), axis: Axis::Horizontal }
    );
}

/// 纵向列表里的 swiper：横划给 swiper，竖划仍滚列表
#[test]
fn swiper_takes_its_axis_inside_vertical_list() {
    let sw = Some(("sw".to_string(), Axis::Horizontal));
    let mut g = DragGesture::new((100.0, 100.0), vertical_area(), false, true).with_swiper(sw.clone(), true);
    assert_eq!(g.on_move(60.0, 101.0), GestureAction::BeginSwiper { id: "sw".into(), axis: Axis::Horizontal });
    assert_eq!(g.on_move(40.0, 101.0), GestureAction::UpdateSwiper { id: "sw".into(), axis: Axis::Horizontal });
    let mut g = DragGesture::new((100.0, 100.0), vertical_area(), false, true).with_swiper(sw, true);
    assert_eq!(g.on_move(101.0, 140.0), GestureAction::BeginArea { id: "list".into(), axis: Axis::Vertical });
}

/// 侧滑返回仍然优先于 swiper；swiper 在横向 scroll-view **外层**时横划给 scroll-view
#[test]
fn edge_back_and_inner_area_win_over_swiper() {
    let sw = Some(("sw".to_string(), Axis::Horizontal));
    let mut g = DragGesture::new((6.0, 300.0), None, false, true).allow_edge_back(true).with_swiper(sw.clone(), true);
    assert_eq!(g.on_move(40.0, 301.0), GestureAction::BeginEdgeBack);
    let mut g = DragGesture::new((100.0, 100.0), horizontal_area(), false, true).with_swiper(sw, false);
    assert_eq!(g.on_move(60.0, 101.0), GestureAction::BeginArea { id: "row".into(), axis: Axis::Horizontal });
}

/// 横向区域不参与纵向嵌套传递；页面不可滚时也不传递
#[test]
fn handoff_requires_vertical_area_and_scrollable_page() {
    let mut g = DragGesture::new((100.0, 100.0), horizontal_area(), false, true);
    g.on_move(60.0, 100.0); // 锁成横向区域
    assert_eq!(g.handoff_to_page(), GestureAction::None);

    let mut g2 = DragGesture::new((100.0, 100.0), vertical_area(), false, false);
    g2.on_move(100.0, 140.0);
    assert_eq!(g2.handoff_to_page(), GestureAction::None);
}
/// slider 上横着拖 → 交给滑块（哪怕外面是横向 swiper、哪怕遮罩上写了 catchtouchmove）
#[test]
fn horizontal_drag_on_slider_goes_to_slider() {
    let mut g = DragGesture::new((100.0, 100.0), horizontal_area(), true, true)
        .with_swiper(Some(("sw".into(), Axis::Horizontal)), true)
        .with_slider(Some("sl".into()));
    assert_eq!(g.on_move(110.0, 101.0), GestureAction::BeginSlider { id: "sl".into() });
    assert_eq!(g.on_move(140.0, 103.0), GestureAction::UpdateSlider { id: "sl".into() });
}
/// slider 上竖着划 → 照样滚页面（从前一按到滑块整次触摸就被吃掉）
#[test]
fn vertical_drag_on_slider_scrolls_page() {
    let mut g = DragGesture::new((100.0, 100.0), None, false, true).with_slider(Some("sl".into()));
    assert_eq!(g.on_move(101.0, 140.0), GestureAction::BeginPage);
    assert_eq!(g.on_move(101.0, 180.0), GestureAction::UpdatePage);
}
/// 视频进度条：横着拖归它（哪怕外面是横向 scroll-view），竖着划滚页面
#[test]
fn video_seek_takes_horizontal_only() {
    let mut g = DragGesture::new((100.0, 100.0), horizontal_area(), false, true).with_video_seek(Some("v".into()));
    assert_eq!(g.on_move(110.0, 101.0), GestureAction::BeginVideoSeek { key: "v".into() });
    assert_eq!(g.on_move(150.0, 102.0), GestureAction::UpdateVideoSeek { key: "v".into() });
    let mut g = DragGesture::new((100.0, 100.0), None, false, true).with_video_seek(Some("v".into()));
    assert_eq!(g.on_move(101.0, 140.0), GestureAction::BeginPage);
}
