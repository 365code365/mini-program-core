//! `<template name>` / `<template is data>` 在模板展开阶段的语义（不碰文件）。
//!
//! 期望值都按微信官方文档（wxml/template.html、framework/view/wxml/data.html）：
//! 定义不渲染；`is` 可字面量、可表达式、可带插值；`data` 是对象（展开 / 简写 / 显式键）
//! 且是模板的**全部**作用域；模板可以先用后定义；glass-easel 下允许递归。
//! 文件级的 import / include 在 `wxml_import_include_tests.rs`。

use crate::parser::template::{ComponentTemplates, TemplateEngine};
use crate::parser::wxml::{WxmlNode, WxmlNodeType, WxmlParser};
use serde_json::{json, Value};

fn render(src: &str, data: &Value) -> Vec<WxmlNode> {
    let nodes = WxmlParser::new(src).parse().expect("WXML 应能解析");
    TemplateEngine::render(&nodes, data)
}

/// 渲染结果拍平成文本（按文档序），方便一眼断言
fn texts(nodes: &[WxmlNode]) -> Vec<String> {
    fn walk(n: &WxmlNode, out: &mut Vec<String>) {
        if n.node_type == WxmlNodeType::Text {
            out.push(n.text_content.trim().to_string());
        }
        for c in &n.children {
            walk(c, out);
        }
    }
    let mut out = Vec::new();
    for n in nodes {
        walk(n, &mut out);
    }
    out
}

fn tags(nodes: &[WxmlNode]) -> Vec<&str> {
    nodes.iter().map(|n| n.tag_name.as_str()).collect()
}

#[test]
fn 定义本身不渲染() {
    let out = render(r#"<template name="a"><view>A</view></template><view>page</view>"#, &json!({}));
    assert_eq!(tags(&out), ["view"]);
    assert_eq!(texts(&out), ["page"]);
}

#[test]
fn 字面量_is_展开模板体且不留包裹节点() {
    let out = render(
        r#"<template name="card"><view class="c">{{t}}</view><text>2</text></template>
           <template is="card" data="{{t: 'hi'}}"/>"#,
        &json!({}),
    );
    assert_eq!(tags(&out), ["view", "text"], "模板内容原地展开，不产生 template 节点");
    assert_eq!(texts(&out), ["hi", "2"]);
}

#[test]
fn 动态_is_表达式与插值() {
    let src = r#"<template name="odd"><text>odd {{n}}</text></template>
                 <template name="even"><text>even {{n}}</text></template>
                 <template name="typeA"><text>A</text></template>
                 <template is="{{n % 2 == 0 ? 'even' : 'odd'}}" data="{{n}}"/>
                 <template is="type{{kind}}"/>"#;
    let out = render(src, &json!({"n": 3, "kind": "A"}));
    assert_eq!(texts(&out), ["odd 3", "A"]);
    let out = render(src, &json!({"n": 4, "kind": "A"}));
    assert_eq!(texts(&out), ["even 4", "A"]);
}

#[test]
fn data_对象展开() {
    let out = render(
        r#"<template name="u"><text>{{name}}/{{age}}</text></template>
           <template is="u" data="{{...user}}"/>"#,
        &json!({"user": {"name": "张三", "age": 20}}),
    );
    assert_eq!(texts(&out), ["张三/20"]);
}

#[test]
fn data_展开加额外键_后写的覆盖先写的() {
    let out = render(
        r#"<template name="u"><text>{{name}}-{{extra}}-{{age}}</text></template>
           <template is="u" data="{{...user, extra: 1, age: 99}}"/>"#,
        &json!({"user": {"name": "a", "age": 20}}),
    );
    assert_eq!(texts(&out), ["a-1-99"]);
    // 多个展开叠加：同名键后面的赢（官方 data.html 的 objectCombine 例子）
    let out = render(
        r#"<template name="o"><text>{{a}}{{b}}{{c}}{{d}}{{e}}</text></template>
           <template is="o" data="{{...obj1, ...obj2, e: 5}}"/>"#,
        &json!({"obj1": {"a": 1, "b": 2}, "obj2": {"b": 3, "c": 3, "d": 4}}),
    );
    assert_eq!(texts(&out), ["13345"]);
}

#[test]
fn data_显式键与简写() {
    let out = render(
        r#"<template name="t"><text>{{a}}|{{b}}</text></template>
           <template is="t" data="{{a: 1, b: x}}"/>
           <template is="t" data="{{ a, b }}"/>"#,
        &json!({"x": "X", "a": "A", "b": "B"}),
    );
    assert_eq!(texts(&out), ["1|X", "A|B"]);
}

#[test]
fn 模板只看得见_data_传入的数据() {
    let out = render(
        r#"<template name="t"><text>[{{secret}}][{{v}}]</text></template>
           <template is="t" data="{{v: 1}}"/>
           <template is="t"/>"#,
        &json!({"secret": "页面字段"}),
    );
    assert_eq!(texts(&out), ["[][1]", "[][]"], "页面字段不能漏进模板作用域");
}

#[test]
fn 模板也看不见外层_wx_for_的临时变量() {
    let out = render(
        r#"<template name="t"><text>{{item}}:{{v}}</text></template>
           <block wx:for="{{list}}"><template is="t" data="{{v: index}}"/></block>"#,
        &json!({"list": ["a", "b"]}),
    );
    assert_eq!(texts(&out), [":0", ":1"]);
}

#[test]
fn 可以先引用后定义() {
    let out = render(
        r#"<template is="late" data="{{x: 7}}"/><template name="late"><text>{{x}}</text></template>"#,
        &json!({}),
    );
    assert_eq!(texts(&out), ["7"]);
}

#[test]
fn 模板里再用模板() {
    let out = render(
        r#"<template name="inner"><text>in:{{v}}</text></template>
           <template name="outer"><view><template is="inner" data="{{v: w + 1}}"/></view></template>
           <template is="outer" data="{{w: 1}}"/>"#,
        &json!({}),
    );
    assert_eq!(tags(&out), ["view"]);
    assert_eq!(texts(&out), ["in:2"]);
}

#[test]
fn 有终止条件的递归照常展开() {
    // glass-easel 允许模板递归：画一棵树
    let out = render(
        r#"<template name="node"><text>{{name}}</text><template wx:for="{{kids}}" is="node" data="{{...item}}"/></template>
           <template is="node" data="{{...tree}}"/>"#,
        &json!({"tree": {"name": "root", "kids": [{"name": "a", "kids": [{"name": "a1"}]}, {"name": "b"}]}}),
    );
    assert_eq!(texts(&out), ["root", "a", "a1", "b"]);
}

#[test]
fn 无终止的自引用被截断而不是卡死() {
    let t = std::time::Instant::now();
    let out = render(r#"<template name="x"><text>x</text><template is="x"/></template><template is="x"/>"#, &json!({}));
    // 深度上限 16：恰好展开 16 层
    assert_eq!(out.len(), 16, "{:?}", tags(&out));
    // 一层里引用自己三次是指数爆炸（3^16），靠总量封顶兜住
    let out = render(
        r#"<template name="y"><template is="y"/><template is="y"/><template is="y"/><text>y</text></template><template is="y"/>"#,
        &json!({}),
    );
    assert!(!out.is_empty() && out.len() <= 50_000, "{}", out.len());
    assert!(t.elapsed().as_secs() < 20, "截断应当很快，实际 {:?}", t.elapsed());
}

#[test]
fn wx_for_与_wx_if_作用在_template_is_上() {
    let out = render(
        r#"<template name="row"><text>{{i}}={{label}}</text></template>
           <template wx:for="{{rows}}" wx:for-item="r" wx:for-index="i" is="row" data="{{...r, i}}"/>
           <template wx:if="{{false}}" is="row" data="{{label: 'no'}}"/>
           <template wx:else is="row" data="{{label: 'else', i: 9}}"/>"#,
        &json!({"rows": [{"label": "a"}, {"label": "b"}]}),
    );
    assert_eq!(texts(&out), ["0=a", "1=b", "9=else"]);
}

#[test]
fn 未定义的模板名什么都不渲染() {
    let out = render(r#"<view>a</view><template is="nope"/><template is="{{missing}}"/><view>b</view>"#, &json!({}));
    assert_eq!(texts(&out), ["a", "b"]);
}

#[test]
fn 定义与_wxs_夹在条件链中间不打断_if_else() {
    let out = render(
        r#"<view wx:if="{{false}}">if</view>
           <template name="t"><text>t</text></template>
           <wxs module="m">var a = 1;</wxs>
           <view wx:else>else</view>"#,
        &json!({}),
    );
    assert_eq!(texts(&out), ["else"]);
}

#[test]
fn wxs_的脚本按原文解析且不渲染() {
    // `i < n` 里的 `<` 以前会让整页解析失败（Empty tag name）
    let src = r#"<wxs module="m">
        function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += i; } return s; }
        module.exports = { sum: sum };
    </wxs><view>ok</view><wxs src="./a.wxs" module="b"/>"#;
    let nodes = WxmlParser::new(src).parse().expect("wxs 里的 < 不应打断解析");
    assert_eq!(tags(&nodes), ["wxs", "view", "wxs"]);
    assert!(nodes[0].children[0].text_content.contains("i < n"), "脚本原文要保留给后续的 wxs 支持");
    let out = TemplateEngine::render(&nodes, &json!({}));
    assert_eq!(tags(&out), ["view"]);
    assert_eq!(texts(&out), ["ok"], "脚本源码不能当文字画出来");
}

#[test]
fn 没解析过的_include_import_什么都不渲染() {
    // 没走文件加载器（节点里没有被引入的内容）时，include/import 只是空标签
    let out = render(r#"<include src="x.wxml"/><import src="y.wxml"/><view>v</view>"#, &json!({}));
    assert_eq!(tags(&out), ["view"]);
}

#[test]
fn 模板里的自定义组件拿得到实例数据() {
    let comps: ComponentTemplates = [(
        "my-tag".to_string(),
        WxmlParser::new(r#"<text>{{label}}</text>"#).parse().unwrap(),
    )]
    .into_iter()
    .collect();
    let page = WxmlParser::new(
        r#"<template name="t"><view><my-tag/></view></template><template is="t"/>"#,
    )
    .parse()
    .unwrap();
    let data = json!({"$comp": {"my-tag": {"label": "组件"}}});
    let out = TemplateEngine::render_with_components(&page, &data, &comps);
    assert_eq!(out[0].children[0].tag_name, "my-tag");
    assert_eq!(texts(&out), ["组件"]);
}

#[test]
fn 模板内_class_数组绑定照常拼接() {
    let out = render(
        r#"<template name="t"><view class="{{['a', on ? 'on' : '']}}"/></template><template is="t" data="{{on: true}}"/>"#,
        &json!({}),
    );
    assert_eq!(out[0].attributes.get("class").map(String::as_str), Some("a on"));
}
