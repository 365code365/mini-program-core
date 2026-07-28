//! 事件系统测试
//! 覆盖 bindtap 冒泡、catchtap 阻止冒泡、事件对象 dataset、多层嵌套

use crate::renderer::wxml_renderer::WxmlRenderer;
use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::WxssParser;
use crate::Canvas;
use serde_json::json;

/// scale=1.0，便于用逻辑坐标断言
fn render(css: &str, wxml: &str) -> WxmlRenderer {
    let ss = WxssParser::new(css).parse().unwrap_or_default();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 1.0);
    let mut c = Canvas::new(375, 667);
    let ns = WxmlParser::new(wxml).parse().unwrap();
    r.render(&mut c, &ns, &json!({}));
    r
}

#[test]
fn test_event_binding_and_dataset() {
    let r = render("", r#"<view bindtap="onTap" data-id="42" data-name="foo"><text>x</text></view>"#);
    let bindings = r.get_event_bindings();
    assert!(!bindings.is_empty());
    let b = bindings.iter().find(|b| b.handler == "onTap").expect("onTap binding");
    assert_eq!(b.event_type, "tap");
    assert_eq!(b.data.get("id"), Some(&"42".to_string()));
    assert_eq!(b.data.get("name"), Some(&"foo".to_string()));
    assert!(!b.is_catch);
}

#[test]
fn test_catchtap_is_catch_flag() {
    let r = render("", r#"<view catchtap="onCatch"><text>x</text></view>"#);
    let b = r.get_event_bindings().iter().find(|b| b.handler == "onCatch").cloned();
    assert!(b.is_some());
    assert!(b.unwrap().is_catch, "catchtap 应标记为 catch");
}

#[test]
fn test_tap_bubbles_inner_to_outer() {
    let css = ".outer{width:200px;height:200px;} .inner{width:100px;height:100px;}";
    let wxml = r#"
        <view class="outer" bindtap="onOuter">
            <view class="inner" bindtap="onInner"></view>
        </view>
    "#;
    let r = render(css, wxml);
    // 点击内层区域 (10,10)，应同时命中 inner 和 outer，冒泡顺序 内 -> 外
    let chain = r.hit_test_bubble(10.0, 10.0, "tap");
    let handlers: Vec<&str> = chain.iter().map(|b| b.handler.as_str()).collect();
    assert_eq!(handlers, vec!["onInner", "onOuter"], "应从内向外冒泡");
}

#[test]
fn test_catchtap_stops_bubbling() {
    let css = ".outer{width:200px;height:200px;} .inner{width:100px;height:100px;}";
    let wxml = r#"
        <view class="outer" bindtap="onOuter">
            <view class="inner" catchtap="onInner"></view>
        </view>
    "#;
    let r = render(css, wxml);
    let chain = r.hit_test_bubble(10.0, 10.0, "tap");
    let handlers: Vec<&str> = chain.iter().map(|b| b.handler.as_str()).collect();
    assert_eq!(handlers, vec!["onInner"], "catchtap 应阻止冒泡到外层");
}

#[test]
fn test_tap_outside_inner_only_hits_outer() {
    let css = ".outer{width:200px;height:200px;} .inner{width:100px;height:100px;}";
    let wxml = r#"
        <view class="outer" bindtap="onOuter">
            <view class="inner" bindtap="onInner"></view>
        </view>
    "#;
    let r = render(css, wxml);
    // 点击 (150,150)：在 outer 内、inner 外
    let chain = r.hit_test_bubble(150.0, 150.0, "tap");
    let handlers: Vec<&str> = chain.iter().map(|b| b.handler.as_str()).collect();
    assert_eq!(handlers, vec!["onOuter"]);
}

#[test]
fn test_three_level_bubbling() {
    let css = r#"
        .a{width:300px;height:300px;}
        .b{width:200px;height:200px;}
        .c{width:100px;height:100px;}
    "#;
    let wxml = r#"
        <view class="a" bindtap="onA">
            <view class="b" bindtap="onB">
                <view class="c" bindtap="onC"></view>
            </view>
        </view>
    "#;
    let r = render(css, wxml);
    let chain = r.hit_test_bubble(10.0, 10.0, "tap");
    let handlers: Vec<&str> = chain.iter().map(|b| b.handler.as_str()).collect();
    assert_eq!(handlers, vec!["onC", "onB", "onA"], "三层冒泡顺序");
}

#[test]
fn test_three_level_catch_middle_stops() {
    let css = r#"
        .a{width:300px;height:300px;}
        .b{width:200px;height:200px;}
        .c{width:100px;height:100px;}
    "#;
    let wxml = r#"
        <view class="a" bindtap="onA">
            <view class="b" catchtap="onB">
                <view class="c" bindtap="onC"></view>
            </view>
        </view>
    "#;
    let r = render(css, wxml);
    let chain = r.hit_test_bubble(10.0, 10.0, "tap");
    let handlers: Vec<&str> = chain.iter().map(|b| b.handler.as_str()).collect();
    assert_eq!(handlers, vec!["onC", "onB"], "中层 catch 应在自己这里停止");
}

#[test]
fn test_multiple_independent_handlers() {
    let css = ".item{width:100px;height:50px;}";
    let wxml = r#"
        <view>
            <view class="item" bindtap="onItem1" data-idx="0"></view>
            <view class="item" bindtap="onItem2" data-idx="1"></view>
            <view class="item" bindtap="onItem3" data-idx="2"></view>
        </view>
    "#;
    let r = render(css, wxml);
    assert_eq!(r.get_event_bindings().len(), 3);
}

#[test]
fn test_different_event_types_not_mixed() {
    let css = ".box{width:100px;height:100px;}";
    // 同一元素上 tap 与 confirm 两种事件，hit_test_bubble("tap") 只返回 tap
    let wxml = r#"
        <view class="box" bindtap="onTap" bindconfirm="onConfirm"></view>
    "#;
    let r = render(css, wxml);
    // 两种事件都被解析
    let all = r.get_event_bindings();
    assert!(all.iter().any(|b| b.event_type == "confirm"));
    // 但冒泡链只含 tap
    let chain = r.hit_test_bubble(10.0, 10.0, "tap");
    assert!(!chain.is_empty());
    assert!(chain.iter().all(|b| b.event_type == "tap"));
    assert!(chain.iter().any(|b| b.handler == "onTap"));
}

// ============ 输入框事件的 JS 语义 ============

/// `e.detail.value` **必须是字符串**（微信语义），哪怕内容全是数字。
///
/// 从前输入事件走 `__callPageMethod` + dataset：dataset 会按字面量还原类型
/// （`data-id="12"` → 数字 12），于是输入 `1212121` 到页面里变成 number，
/// `e.detail.value.trim()` 直接 `TypeError: not a function`。
#[test]
fn input_detail_value_stays_a_string() {
    let mut app = crate::runtime::MiniApp::new(375, 667).expect("create MiniApp");
    app.init().expect("init");
    app.load_script(
        r#"Page({
            data: {},
            onInput: function (e) {
                this.kind = typeof e.detail.value;
                this.text = e.detail.value;
                this.trimmed = e.detail.value.trim();
                this.cursor = e.detail.cursor;
                this.evType = e.type;
            }
        });"#,
    )
    .unwrap();
    app.eval(
        r#"__dispatchEvent('onInput', {
            type: 'input',
            target: { id: '', dataset: {}, offsetLeft: 25, offsetTop: 520 },
            currentTarget: { id: '', dataset: {}, offsetLeft: 25, offsetTop: 520 },
            detail: { value: '1212121', cursor: 7, keyCode: 0 }
        })"#,
    )
    .unwrap();
    assert_eq!(app.eval("__currentPage.kind").unwrap(), "string");
    assert_eq!(app.eval("__currentPage.text").unwrap(), "1212121");
    assert_eq!(app.eval("__currentPage.trimmed").unwrap(), "1212121");
    assert_eq!(app.eval("__currentPage.cursor").unwrap(), "7");
    assert_eq!(app.eval("__currentPage.evType").unwrap(), "input");
}

/// `data-*` 仍然要按字面量还原（列表项 `data-id="12"` 与数字比较）
#[test]
fn dataset_is_still_coerced_while_detail_is_not() {
    let mut app = crate::runtime::MiniApp::new(375, 667).expect("create MiniApp");
    app.init().expect("init");
    app.load_script(
        r#"Page({
            data: {},
            onTap: function (e) {
                this.idKind = typeof e.currentTarget.dataset.id;
                this.detailKind = typeof e.detail.value;
            }
        });"#,
    )
    .unwrap();
    app.eval(
        r#"__dispatchEvent('onTap', {
            type: 'tap',
            target: { id: 'a', dataset: { id: '12' } },
            currentTarget: { id: 'a', dataset: { id: '12' } },
            detail: { value: '12' }
        })"#,
    )
    .unwrap();
    assert_eq!(app.eval("__currentPage.idKind").unwrap(), "number", "dataset 该还原成数字");
    assert_eq!(app.eval("__currentPage.detailKind").unwrap(), "string", "detail 不该被还原");
}

/// 没给 `timeStamp` 时由 JS 侧补上（宿主的输入事件不带页面时钟）
#[test]
fn dispatch_event_fills_missing_timestamp() {
    let mut app = crate::runtime::MiniApp::new(375, 667).expect("create MiniApp");
    app.init().expect("init");
    app.load_script(r#"Page({ data: {}, onBlur: function (e) { this.ts = e.timeStamp; } });"#)
        .unwrap();
    app.eval(r#"__dispatchEvent('onBlur', { type: 'blur', detail: { value: 'x' } })"#).unwrap();
    assert_eq!(app.eval("typeof __currentPage.ts").unwrap(), "number");
    assert!(app.eval("__currentPage.ts > 0").unwrap() == "true");
}
