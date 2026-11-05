//! C1 / C2：手势组件（虚组件、状态、仲裁、速度、native-view）与 `worklet:` 前缀
use super::*;
use crate::host::gesture::{Axis, DragGesture, GestureAction, GestureTarget, SheetHit};

const LOG_FN: &str = "function L(s){ getApp().log.push(s); }";

#[test]
fn 手势组件是虚组件_子节点直接是_flex_项() {
    // 父节点 flex 横排，左边一个 100px 的块，右边是手势组件包着的 flex:1 红块：
    // 包装层不进树时红块铺满剩下的 275px（从前包装层成了 flex 项，红块宽度按内容算 = 0）
    let root = fixture(
        "gh-virtual",
        r#"<view style="display:flex;flex-direction:row;height:50px"><view style="width:100px;background-color:#0000ff"></view>
             <pan-gesture-handler on-gesture-event="g"><view style="flex:1;background-color:#ff0000"></view></pan-gesture-handler></view>"#,
        "Page({ g(){} });",
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    pump_until(&mut e, 300, |_| false);
    assert_eq!(rgb_at(&mut e, 300, 25), [255, 0, 0], "flex:1 的子节点铺满剩余宽度");
    assert_eq!(rgb_at(&mut e, 50, 25), [0, 0, 255]);
}

#[test]
fn pan_状态序列_平移从按下点算_抬手速度() {
    let root = fixture(
        "gh-pan",
        r#"<pan-gesture-handler worklet:ongesture="g"><view style="height:200px;background-color:#eeeeee"></view></pan-gesture-handler>"#,
        &format!("{LOG_FN} Page({{ g(e){{ L(e.state + ' ' + Math.round(e.translationX) + ' ' + (typeof e.velocityX)); }} }});"),
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    // 从 x=0 按下（从前 `g.x0 || x` 把 0 当成没有，平移恒为 0）
    drag(&mut e, (0.0, 100.0), (120.0, 100.0), 6);
    let l = log(&e);
    let states: Vec<&str> = l.iter().map(|s| &s[..1]).collect();
    assert_eq!(states.first(), Some(&"0"), "按下 POSSIBLE：{l:?}");
    assert_eq!(states.get(1), Some(&"1"), "认出来 BEGIN：{l:?}");
    assert!(states.contains(&"2"), "之后 ACTIVE：{l:?}");
    assert_eq!(l.last().map(|s| s.as_str()), Some("3 120 number"), "END，平移 120（起点 x=0），速度是数字：{l:?}");
}

#[test]
fn 嵌套手势_纵向滑动归_vertical_drag_pan_取消() {
    // 官方：vertical-drag 包 pan 时纵向滑动 vertical-drag 响应、pan 失效
    let root = fixture(
        "gh-nest",
        r#"<vertical-drag-gesture-handler on-gesture-event="v"><pan-gesture-handler on-gesture-event="p">
             <view style="height:300px;background-color:#eeeeee"></view></pan-gesture-handler></vertical-drag-gesture-handler>"#,
        &format!("{LOG_FN} Page({{ v(e){{ L('v' + e.state); }}, p(e){{ L('p' + e.state); }} }});"),
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    drag(&mut e, (100.0, 50.0), (100.0, 200.0), 6);
    let l = log(&e);
    assert!(l.contains(&"v1".to_string()) && l.contains(&"v3".to_string()), "vertical-drag 认出来并结束：{l:?}");
    assert!(l.contains(&"p4".to_string()), "pan 被抢走：CANCELLED：{l:?}");
    assert!(!l.contains(&"p1".to_string()), "pan 不该 BEGIN：{l:?}");
}

#[test]
fn should_accept_gesture_返回_false_报_cancelled_tap_状态恒为_1() {
    let root = fixture(
        "gh-accept",
        r#"<pan-gesture-handler on-gesture-event="g" worklet:should-accept-gesture="no"><view style="height:150px;background-color:#eeeeee"></view></pan-gesture-handler>
           <tap-gesture-handler on-gesture-event="t"><view style="height:150px;background-color:#dddddd"></view></tap-gesture-handler>"#,
        &format!("{LOG_FN} Page({{ no(){{ L('ask'); return false; }}, g(e){{ L('g' + e.state); }}, t(e){{ L('t' + e.state + ' ' + Math.round(e.absoluteY)); }} }});"),
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    drag(&mut e, (100.0, 50.0), (200.0, 60.0), 5);
    let l = log(&e);
    assert_eq!(l.iter().filter(|s| s.starts_with('g')).cloned().collect::<Vec<_>>(), ["g0", "g4"], "POSSIBLE → CANCELLED：{l:?}");
    assert!(l.contains(&"ask".to_string()));
    clear_log(&e);
    e.pointer_down(100.0, 220.0);
    e.pump(3000);
    e.pointer_up(100.0, 220.0);
    e.pump(3016);
    assert_eq!(log(&e), ["t1 220"], "tap 只报一次、state 1");
}

#[test]
fn simultaneous_handlers_互相声明后两层都认() {
    let root = fixture(
        "gh-simul",
        r#"<vertical-drag-gesture-handler tag="outer" simultaneous-handlers="{{['inner']}}" on-gesture-event="o">
             <vertical-drag-gesture-handler tag="inner" simultaneous-handlers="{{['outer']}}" on-gesture-event="i">
               <view style="height:300px;background-color:#eeeeee"></view></vertical-drag-gesture-handler></vertical-drag-gesture-handler>"#,
        &format!("{LOG_FN} Page({{ o(e){{ L('o' + e.state); }}, i(e){{ L('i' + e.state); }} }});"),
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    drag(&mut e, (100.0, 50.0), (100.0, 200.0), 6);
    let l = log(&e);
    assert!(l.contains(&"i1".to_string()) && l.contains(&"o1".to_string()), "两层都 BEGIN：{l:?}");
    assert!(l.iter().position(|s| s == "i1") < l.iter().position(|s| s == "o1"), "内层先：{l:?}");
}

#[test]
fn native_view_代理_scroll_view_列表照常滚_手势跟到松手() {
    let items: String = (0..30).map(|i| format!(r#"<view style="height:50px">{i}</view>"#)).collect();
    let root = fixture(
        "gh-native",
        &format!(
            r#"<vertical-drag-gesture-handler native-view="scroll-view" on-gesture-event="g">
                 <scroll-view id="sv" scroll-y style="height:400px" bindscroll="s">{items}</scroll-view></vertical-drag-gesture-handler>"#
        ),
        &format!("{LOG_FN} Page({{ g(e){{ L('g' + e.state); }}, s(){{ L('scroll'); }} }});"),
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    pump_until(&mut e, 200, |_| false);
    drag(&mut e, (100.0, 350.0), (100.0, 100.0), 10);
    pump_until(&mut e, 200, |_| false);
    let l = log(&e);
    assert!(e.scroll_view_offset("sv").unwrap_or(0.0) > 100.0, "列表滚了：{:?} {l:?}", e.scroll_view_offset("sv"));
    assert!(l.contains(&"g3".to_string()), "手势回调跟到松手（END），没被滚动接管的 touchcancel 打断：{l:?}");
    assert!(l.iter().filter(|s| *s == "g2").count() >= 5, "滚的时候 ACTIVE 一直在报：{l:?}");
}

#[test]
fn worklet_前缀_scroll_view_的_onscrollupdate_参数是事件对象本身() {
    let items: String = (0..30).map(|i| format!(r#"<view style="height:50px">{i}</view>"#)).collect();
    let root = fixture(
        "worklet-scroll",
        &format!(r#"<scroll-view scroll-y style="height:400px" worklet:onscrollstart="a" worklet:onscrollupdate="u" worklet:onscrollend="z">{items}</scroll-view>"#),
        &format!("{LOG_FN} Page({{ a(){{ L('start'); }}, u(e){{ L('u ' + (typeof e.scrollTop) + ' ' + (e.detail && typeof e.detail.scrollTop)); }}, z(){{ L('end'); }} }});"),
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    pump_until(&mut e, 200, |_| false);
    drag(&mut e, (100.0, 350.0), (100.0, 150.0), 8);
    pump_until(&mut e, 3000, |e| log(e).contains(&"end".to_string()));
    let l = log(&e);
    assert_eq!(l.first().map(|s| s.as_str()), Some("start"), "{l:?}");
    assert!(l.contains(&"u number number".to_string()), "字段在顶层、detail 也有：{l:?}");
    assert_eq!(l.last().map(|s| s.as_str()), Some("end"), "{l:?}");
}

#[test]
fn worklet_属性名换成组件_properties() {
    use crate::parser::template::worklet_prop;
    assert_eq!(worklet_prop("worklet:ongesture").as_deref(), Some("onGestureEvent"));
    assert_eq!(worklet_prop("worklet:should-accept-gesture").as_deref(), Some("shouldAcceptGesture"));
    assert_eq!(worklet_prop("worklet:onsizeupdate").as_deref(), Some("onsizeupdate"));
    assert_eq!(worklet_prop("bindtap"), None);
}

#[test]
fn draggable_sheet_面板与列表的仲裁() {
    let area = || Some(("list".to_string(), Axis::Vertical));
    let sheet = |at_max, at_top| Some(SheetHit { at_max, area_inside: true, area_at_top: at_top });
    // 面板没到最大、上推：归面板（谁也不滚）
    let mut g = DragGesture::new((100.0, 300.0), area(), false, true).with_sheet(sheet(false, false));
    assert_eq!(g.on_move(100.0, 280.0), GestureAction::None);
    assert_eq!(*g.target(), GestureTarget::Blocked);
    // 面板到最大、上推：列表滚
    let mut g = DragGesture::new((100.0, 300.0), area(), false, true).with_sheet(sheet(true, false));
    assert_eq!(g.on_move(100.0, 280.0), GestureAction::BeginArea { id: "list".into(), axis: Axis::Vertical });
    // 列表在顶、下拉：归面板
    let mut g = DragGesture::new((100.0, 300.0), area(), false, true).with_sheet(sheet(true, true));
    assert_eq!(g.on_move(100.0, 320.0), GestureAction::None);
    // 列表不在顶、下拉：列表滚
    let mut g = DragGesture::new((100.0, 300.0), area(), false, true).with_sheet(sheet(true, false));
    assert!(matches!(g.on_move(100.0, 320.0), GestureAction::BeginArea { .. }));
    // 面板里没有列表：竖着划都归面板，页面不滚
    let mut g = DragGesture::new((100.0, 300.0), None, false, true).with_sheet(Some(SheetHit { at_max: false, area_inside: false, area_at_top: true }));
    assert_eq!(g.on_move(100.0, 330.0), GestureAction::None);
}
