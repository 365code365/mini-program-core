//! 事件系统对齐微信：绑定语法、捕获/冒泡/catch/mut-bind 的传播顺序。
//!
//! 之前引擎是**按固定属性名白名单**认事件的（只有 bindtap/catchtap/bindchange…），
//! 于是 `bindtouchstart`、`bind:tap`、`catchtouchmove`（遮罩阻止页面滚动的标准写法）、
//! `capture-bind:tap`、`mut-bind:tap` 全部被无声忽略 —— 自定义手势、事件捕获、
//! 互斥绑定统统没有。这里锁住语法与传播顺序两件事。

use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::WxssParser;
use crate::renderer::components::{extract_events, parse_event_attr, EventPhase};
use crate::renderer::wxml_renderer::WxmlRenderer;
use crate::Canvas;
use serde_json::json;

#[test]
fn parses_all_six_binding_prefixes() {
    // (属性名, 事件名, catch, 阶段, 互斥)
    let cases = [
        ("bindtap", "tap", false, EventPhase::Bubble, false),
        ("bind:tap", "tap", false, EventPhase::Bubble, false),
        ("bindtouchstart", "touchstart", false, EventPhase::Bubble, false),
        ("catchtouchmove", "touchmove", true, EventPhase::Bubble, false),
        ("catch:longpress", "longpress", true, EventPhase::Bubble, false),
        ("mut-bind:tap", "tap", false, EventPhase::Bubble, true),
        ("capture-bind:tap", "tap", false, EventPhase::Capture, false),
        ("capture-catch:touchstart", "touchstart", true, EventPhase::Capture, false),
        // 自定义组件事件（uni-app 的 `bind:__l` 也走这条）
        ("bind:myEvent", "myEvent", false, EventPhase::Bubble, false),
    ];
    for (attr, name, catch, phase, mutb) in cases {
        let got = parse_event_attr(attr).unwrap_or_else(|| panic!("{attr} 应被识别为事件"));
        assert_eq!(got, (name, catch, phase, mutb), "解析 {attr}");
    }
}

/// 不是事件的属性一个都不能误认 —— `change:eS` 是 wxs 的属性观察器，
/// 误当事件会凭空多出一堆绑定。
#[test]
fn rejects_non_event_attributes() {
    for attr in ["class", "style", "data-id", "change:eS", "id", "src", "bind", "catch:"] {
        assert!(parse_event_attr(attr).is_none(), "{attr} 不该被当成事件");
    }
}

#[test]
fn extract_events_collects_dataset_for_every_binding() {
    let nodes = WxmlParser::new(
        r#"<view bindtouchstart="onStart" catchtouchmove="onMove" data-id="7" data-name="x"/>"#,
    )
    .parse()
    .unwrap();
    let events = extract_events(&nodes[0]);
    assert_eq!(events.len(), 2, "两条绑定都要收进来: {events:?}");
    for e in &events {
        assert_eq!(e.data.get("id"), Some(&"7".to_string()));
        assert_eq!(e.data.get("name"), Some(&"x".to_string()));
    }
    let mv = events.iter().find(|e| e.event_type == "touchmove").unwrap();
    assert!(mv.is_catch, "catchtouchmove 必须是 catch");
}

// ============ 传播顺序 ============

fn chain_handlers(css: &str, wxml: &str, x: f32, y: f32, event: &str) -> Vec<String> {
    let ss = WxssParser::new(css).parse().unwrap_or_default();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 1.0);
    let mut c = Canvas::new(375, 667);
    let nodes = WxmlParser::new(wxml).parse().unwrap();
    r.render(&mut c, &nodes, &json!({}));
    r.dispatch_chain(x, y, event, Some(false))
        .iter()
        .map(|b| b.handler.clone())
        .collect()
}

/// 捕获（外→内）先于冒泡（内→外）
#[test]
fn capture_runs_outside_in_then_bubble_inside_out() {
    let css = ".outer{width:300px;height:300px;}.inner{width:100px;height:100px;}";
    let wxml = r#"
        <view class="outer" capture-bind:tap="capOuter" bindtap="bubOuter">
            <view class="inner" capture-bind:tap="capInner" bindtap="bubInner"></view>
        </view>"#;
    let got = chain_handlers(css, wxml, 10.0, 10.0, "tap");
    assert_eq!(got, vec!["capOuter", "capInner", "bubInner", "bubOuter"], "{got:?}");
}

/// `capture-catch` 在捕获阶段就掐断，内层与冒泡一个都不该跑
#[test]
fn capture_catch_stops_everything() {
    let css = ".outer{width:300px;height:300px;}.inner{width:100px;height:100px;}";
    let wxml = r#"
        <view class="outer" capture-catch:tap="capOuter" bindtap="bubOuter">
            <view class="inner" bindtap="bubInner"></view>
        </view>"#;
    let got = chain_handlers(css, wxml, 10.0, 10.0, "tap");
    assert_eq!(got, vec!["capOuter"], "{got:?}");
}

/// `catch` 在冒泡阶段截断：自己触发，外层不再触发
#[test]
fn catch_stops_bubbling_at_itself() {
    let css = ".outer{width:300px;height:300px;}.mid{width:200px;height:200px;}.inner{width:100px;height:100px;}";
    let wxml = r#"
        <view class="outer" bindtap="outer">
            <view class="mid" catchtap="mid">
                <view class="inner" bindtap="inner"></view>
            </view>
        </view>"#;
    let got = chain_handlers(css, wxml, 10.0, 10.0, "tap");
    assert_eq!(got, vec!["inner", "mid"], "{got:?}");
}

/// `mut-bind` 互斥：同一次传播只触发最内层那条，但普通 `bind` 照常触发
#[test]
fn mut_bind_only_fires_innermost() {
    let css = ".outer{width:300px;height:300px;}.mid{width:200px;height:200px;}.inner{width:100px;height:100px;}";
    let wxml = r#"
        <view class="outer" mut-bind:tap="mutOuter" bindtap="bindOuter">
            <view class="mid" mut-bind:tap="mutMid">
                <view class="inner" bindtap="bindInner"></view>
            </view>
        </view>"#;
    let got = chain_handlers(css, wxml, 10.0, 10.0, "tap");
    assert_eq!(got, vec!["bindInner", "mutMid", "bindOuter"], "{got:?}");
}

/// 触摸事件也走同一套传播链（不再只有 tap 能被派发）
#[test]
fn touch_events_use_the_same_chain() {
    let css = ".outer{width:300px;height:300px;}.inner{width:100px;height:100px;}";
    let wxml = r#"
        <view class="outer" bindtouchmove="outerMove">
            <view class="inner" catchtouchmove="innerMove"></view>
        </view>"#;
    let got = chain_handlers(css, wxml, 10.0, 10.0, "touchmove");
    assert_eq!(got, vec!["innerMove"], "catchtouchmove 应截断: {got:?}");
}
