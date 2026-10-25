//! `wx:key`：列表项的身份（组件实例键、元素的交互状态键）跟着 key 走，而不是下标。
//! 端到端（真指针、重排后开关状态跟着项走）见集成测试 `tests/wx_key_e2e.rs`。
//!
//! 依据：官方 [wxml/list.html](https://developers.weixin.qq.com/miniprogram/dev/reference/wxml/list.html)
//! 的 `wx:key`：字段名或 `*this`；列表项位置变化时按 key 复用、保持状态。

use crate::parser::template::{ComponentTemplates, TemplateEngine, KEYED_ID_ATTR};
use crate::parser::wxml::{WxmlNode, WxmlParser};
use serde_json::json;

fn parse(src: &str) -> Vec<WxmlNode> {
    WxmlParser::new(src).parse().unwrap()
}

fn comps() -> ComponentTemplates {
    let mut c = ComponentTemplates::new();
    c.insert("my-row".into(), parse("<text>{{n}}</text>"));
    c
}

fn keys(page: &str, data: serde_json::Value) -> Vec<String> {
    TemplateEngine::collect_components(&parse(page), &data, &comps()).into_iter().map(|o| o.key).collect()
}

#[test]
fn 组件实例键用_key_重排后同一项还是同一个键() {
    let page = r#"<view><my-row wx:for="{{list}}" wx:key="id" n="{{item.id}}"/></view>"#;
    let a = keys(page, json!({ "list": [{ "id": "x" }, { "id": 7 }] }));
    assert_eq!(a, vec!["/my-row@0.0[#x]", "/my-row@0.0[#7]"], "字符串与数字 key 都认");
    let b = keys(page, json!({ "list": [{ "id": 7 }, { "id": "x" }] }));
    assert_eq!(b, vec!["/my-row@0.0[#7]", "/my-row@0.0[#x]"], "重排：键跟着项走");
}

#[test]
fn star_this_缺字段退回下标_重复值加序号() {
    let page = r#"<view><my-row wx:for="{{list}}" wx:key="*this"/></view>"#;
    assert_eq!(keys(page, json!({ "list": ["p", "q"] })), vec!["/my-row@0.0[#p]", "/my-row@0.0[#q]"]);
    let page = r#"<view><my-row wx:for="{{list}}" wx:key="id"/></view>"#;
    assert_eq!(
        keys(page, json!({ "list": [{ "id": 1 }, {}, { "id": 1 }] })),
        vec!["/my-row@0.0[#1]", "/my-row@0.0[1]", "/my-row@0.0[#1~2]"]
    );
    // 没写 wx:key：按下标（与从前一致）
    let page = r#"<view><my-row wx:for="{{list}}"/></view>"#;
    assert_eq!(keys(page, json!({ "list": [1, 2] })), vec!["/my-row@0.0[0]", "/my-row@0.0[1]"]);
}

/// 带 key 的列表项里的元素有跟着 key 走的身份；不带 key 的列表、列表外的元素没有（仍按坐标）
#[test]
fn 元素身份跟着_key_走() {
    let page = parse(r#"<view class="top"><view wx:for="{{list}}" wx:key="id"><switch/><input/></view></view><view wx:for="{{list}}"><switch/></view>"#);
    let render = |list: serde_json::Value| TemplateEngine::render(&page, &json!({ "list": list }));
    let id = |n: &WxmlNode| n.get_attr(KEYED_ID_ATTR).map(str::to_string);
    let a = render(json!([{ "id": "a" }, { "id": "b" }]));
    let b = render(json!([{ "id": "b" }, { "id": "a" }]));
    let item_b_before = &a[0].children[1];
    let item_b_after = &b[0].children[0];
    assert!(id(item_b_before).is_some());
    assert_eq!(id(item_b_before), id(item_b_after), "同一项挪了位置，身份不变");
    assert_eq!(id(&item_b_before.children[0]), id(&item_b_after.children[0]), "项里的开关也跟着走");
    assert_ne!(id(&item_b_before.children[0]), id(&item_b_before.children[1]), "同一项里的两个元素身份不同");
    assert_ne!(id(&a[0].children[0]), id(&a[0].children[1]), "不同项身份不同");
    assert_eq!(id(&a[0]), None, "列表外的元素没有");
    assert_eq!(id(&a[1].children[0]), None, "没写 wx:key 的列表照旧按坐标");
}

/// 组件实例本身在带 key 的列表里时，组件模板里的元素也跟着实例走
#[test]
fn 带_key_的组件实例里的元素也有身份() {
    let mut c = ComponentTemplates::new();
    c.insert("my-sw".into(), parse("<switch/>"));
    let page = parse(r#"<view><my-sw wx:for="{{list}}" wx:key="*this"/></view>"#);
    let data = json!({ "list": ["u"], "$comp": { "/my-sw@0.0[#u]": {} } });
    let out = TemplateEngine::render_with_components(&page, &data, &c);
    let sw = &out[0].children[0].children[0];
    assert_eq!(sw.tag_name, "switch");
    assert_eq!(sw.get_attr(KEYED_ID_ATTR), Some("/my-sw@0.0[#u]|0"));
}

/// 输入框不在带 key 的列表里也有身份：按模板结构位置给，不随坐标、不随前面 wx:if 兄弟的显隐变。
/// 从前退回「input_x_y」：聊天页键盘弹起、列表变矮、输入条上移，焦点就对不上任何节点（tests/chat_keyboard_e2e.rs）
#[test]
fn 输入框按结构位置有稳定身份() {
    let page = parse(r#"<view><view wx:if="{{tip}}">提示</view><view class="bar" style="height:{{h}}px"><input/><textarea/></view></view>"#);
    let render = |d: serde_json::Value| TemplateEngine::render(&page, &d);
    let id = |n: &WxmlNode| n.get_attr(KEYED_ID_ATTR).map(str::to_string);
    let a = render(json!({ "tip": true, "h": 50 }));
    let b = render(json!({ "tip": false, "h": 80 }));
    let (bar_a, bar_b) = (&a[0].children[1], &b[0].children[0]);
    assert!(id(&bar_a.children[0]).is_some(), "输入框该有身份");
    assert_eq!(id(&bar_a.children[0]), id(&bar_b.children[0]), "前面的兄弟没了、位置变了，身份不变");
    assert_ne!(id(&bar_a.children[0]), id(&bar_a.children[1]), "两个输入框身份不同");
    assert_eq!(id(bar_a), None, "普通元素不加（仍按坐标，与从前一致）");
}
