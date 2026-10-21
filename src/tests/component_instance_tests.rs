//! 自定义组件「每处使用一个实例」与 `<slot>` 的模板层用例（不跑逻辑层）。
//!
//! 实例键、带类型的属性、归属标记、slot 分配都在模板展开时决定；逻辑层对账与事件的
//! 端到端用例在 `component_runtime_tests.rs`。

use crate::parser::template::{
    ComponentTemplates, TemplateEngine, COMPONENT_INSTANCE_ATTR, COMPONENT_OWNER_ATTR,
};
use crate::parser::wxml::{WxmlNode, WxmlNodeType, WxmlParser};
use crate::parser::wxss::WxssParser;
use crate::renderer::wxml_renderer::WxmlRenderer;
use crate::Canvas;
use serde_json::{json, Value};

fn parse(src: &str) -> Vec<WxmlNode> {
    WxmlParser::new(src).parse().unwrap_or_default()
}

fn templates(pairs: &[(&str, &str)]) -> ComponentTemplates {
    pairs.iter().map(|(t, w)| (t.to_string(), parse(w))).collect()
}

/// 子树里全部文本（按文档顺序拼起来）
fn texts(nodes: &[WxmlNode]) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(n: &WxmlNode, out: &mut Vec<String>) {
        if n.node_type == WxmlNodeType::Text {
            if !n.text_content.trim().is_empty() {
                out.push(n.text_content.trim().to_string());
            }
            return;
        }
        for c in &n.children {
            walk(c, out);
        }
    }
    for n in nodes {
        walk(n, &mut out);
    }
    out
}

/// 按 class 找第一个元素
fn find_class<'a>(nodes: &'a [WxmlNode], cls: &str) -> Option<&'a WxmlNode> {
    for n in nodes {
        if n.get_attr("class").map_or(false, |c| c.split_whitespace().any(|x| x == cls)) {
            return Some(n);
        }
        if let Some(f) = find_class(&n.children, cls) {
            return Some(f);
        }
    }
    None
}

#[test]
fn 两处使用各有实例键和各自的属性() {
    let comps = templates(&[("my-card", r#"<text>{{title}}</text>"#)]);
    let page = parse(r#"<view><my-card title="A"/><my-card title="B" count="{{n}}"/></view>"#);
    let occ = TemplateEngine::collect_components(&page, &json!({ "n": 3 }), &comps);
    assert_eq!(occ.len(), 2);
    assert_eq!(occ[0].key, "/my-card@0.0");
    assert_eq!(occ[1].key, "/my-card@0.1");
    assert_eq!(occ[0].props["title"], json!("A"));
    assert_eq!(occ[1].props["title"], json!("B"));
    assert_eq!(occ[1].props["count"], json!(3), "{{n}} 应保留数字类型");

    let data = json!({ "$comp": { "/my-card@0.0": { "title": "A" }, "/my-card@0.1": { "title": "B" } } });
    let out = TemplateEngine::render_with_components(&page, &data, &comps);
    assert_eq!(texts(&out), vec!["A", "B"], "两处使用应渲染各自实例的 data");
    let host = &out[0].children[1];
    assert_eq!(host.get_attr(COMPONENT_INSTANCE_ATTR), Some("/my-card@0.1"));
}

#[test]
fn wx_for里的组件按下标各一个实例且属性带类型() {
    let comps = templates(&[("my-item", r#"<text>{{n}}</text>"#)]);
    let page = parse(
        r#"<view><my-item wx:for="{{list}}" n="{{item}}" idx="{{index}}" label="第{{index}}项" flag="{{item > 15}}" pair="{{[item, index]}}" data-pos="{{index}}" bind:pick="onPick"/></view>"#,
    );
    let occ = TemplateEngine::collect_components(&page, &json!({ "list": [10, 20, 30] }), &comps);
    assert_eq!(occ.len(), 3);
    for (i, o) in occ.iter().enumerate() {
        assert_eq!(o.key, format!("/my-item@0.0[{i}]"));
        assert!(o.props["n"].is_number(), "item 应是数字：{:?}", o.props["n"]);
        assert_eq!(o.props["idx"], json!(i));
        assert_eq!(o.props["label"], json!(format!("第{i}项")), "混着文本的插值是字符串");
        assert!(o.props["pair"].is_array());
        assert_eq!(o.dataset["pos"], json!(i));
        assert_eq!(o.events["pick"], json!(["onPick"]));
        assert!(!o.props.contains_key("bind:pick") && !o.props.contains_key("data-pos"));
    }
    assert_eq!(occ[0].props["flag"], json!(false));
    assert_eq!(occ[1].props["flag"], json!(true));
}

/// 兄弟节点的 wx:if 翻转不让别的组件换实例键（键是结构位置，不是「第几个同名组件」）
#[test]
fn 前面的wx_if翻转不改变后面组件的键() {
    let comps = templates(&[("my-card", r#"<text>x</text>"#)]);
    let page = parse(r#"<view><my-card wx:if="{{a}}" title="1"/><my-card title="2"/></view>"#);
    let on = TemplateEngine::collect_components(&page, &json!({ "a": true }), &comps);
    let off = TemplateEngine::collect_components(&page, &json!({ "a": false }), &comps);
    assert_eq!(on.len(), 2);
    assert_eq!(off.len(), 1);
    assert_eq!(on[1].key, off[0].key);
    assert_eq!(off[0].props["title"], json!("2"));
}

/// 组件里套组件：外层没有实例 data 之前不收内层；有了之后内层的归属是外层实例
#[test]
fn 嵌套组件等外层有了实例再收集() {
    let comps = templates(&[
        ("outer-x", r#"<view class="o"><inner-x v="{{a}}"/></view>"#),
        ("inner-x", r#"<text class="i" bindtap="onInner">{{v}}</text>"#),
    ]);
    let page = parse(r#"<view><outer-x/></view>"#);
    let first = TemplateEngine::collect_components(&page, &json!({}), &comps);
    assert_eq!(first.len(), 1, "外层还没实例时不展开它的模板");
    let data = json!({ "$comp": { "/outer-x@0.0": { "a": 5 }, "/outer-x@0.0/inner-x@0.0": { "v": 5 } } });
    let occ = TemplateEngine::collect_components(&page, &data, &comps);
    assert_eq!(occ.len(), 2);
    assert_eq!(occ[1].key, "/outer-x@0.0/inner-x@0.0");
    assert_eq!(occ[1].owner, "/outer-x@0.0");
    assert_eq!(occ[1].props["v"], json!(5));

    // 归属：外层模板的节点归外层实例，内层宿主节点也归外层（它身上的绑定是外层的），内层模板归内层
    let out = TemplateEngine::render_with_components(&page, &data, &comps);
    let o = find_class(&out, "o").unwrap();
    assert_eq!(o.get_attr(COMPONENT_OWNER_ATTR), Some("/outer-x@0.0"));
    let inner_host = &o.children[0];
    assert_eq!(inner_host.get_attr(COMPONENT_OWNER_ATTR), Some("/outer-x@0.0"));
    let i = find_class(&out, "i").unwrap();
    assert_eq!(i.get_attr(COMPONENT_OWNER_ATTR), Some("/outer-x@0.0/inner-x@0.0"));
    assert_eq!(out[0].get_attr(COMPONENT_OWNER_ATTR), None, "页面节点没有归属标记");
}

#[test]
fn 模板片段里的组件也是一处使用() {
    let comps = templates(&[("my-card", r#"<text>{{title}}</text>"#)]);
    let page = parse(r#"<template name="row"><my-card title="{{t}}"/></template><view><template is="row" data="{{t: 'X'}}"/></view>"#);
    let occ = TemplateEngine::collect_components(&page, &json!({}), &comps);
    assert_eq!(occ.len(), 1);
    assert_eq!(occ[0].props["title"], json!("X"));
    assert!(occ[0].key.starts_with("/my-card@1."), "{}", occ[0].key);
    // 渲染时按同一个键取实例 data（收集与渲染共用展开代码，键必然一致）
    let data = json!({ "$comp": { occ[0].key.clone(): { "title": "X" } } });
    assert_eq!(texts(&TemplateEngine::render_with_components(&page, &data, &comps)), vec!["X"]);
}

/// `<include>` 的内容（加载期已嵌成 include 的子节点）里的组件同样是一处使用，属性用数据根求值
#[test]
fn include内容里的组件也是一处使用() {
    let comps = templates(&[("my-card", r#"<text>{{title}}</text>"#)]);
    let page = parse(r#"<view><include src="part"><my-card title="{{t}}"/></include></view>"#);
    let occ = TemplateEngine::collect_components(&page, &json!({ "t": "I" }), &comps);
    assert_eq!(occ.len(), 1);
    assert_eq!(occ[0].props["title"], json!("I"));
    let data = json!({ "t": "I", "$comp": { occ[0].key.clone(): { "title": "I" } } });
    assert_eq!(texts(&TemplateEngine::render_with_components(&page, &data, &comps)), vec!["I"]);
}

// ============ slot ============

#[test]
fn 默认slot放进使用方的子节点且归属使用方() {
    let comps = templates(&[("my-panel", r#"<view class="box"><text>head</text><slot/></view>"#)]);
    let page = parse(r#"<view><my-panel><text class="c" bindtap="onTap">正文{{msg}}</text></my-panel></view>"#);
    let data = json!({ "msg": "!", "$comp": { "/my-panel@0.0": { "msg": "组件的" } } });
    let out = TemplateEngine::render_with_components(&page, &data, &comps);
    assert_eq!(texts(&out), vec!["head", "正文!"], "slot 内容用页面数据求值");
    let box_ = find_class(&out, "box").unwrap();
    assert_eq!(box_.get_attr(COMPONENT_OWNER_ATTR), Some("/my-panel@0.0"));
    let c = find_class(&out, "c").unwrap();
    assert_eq!(c.get_attr(COMPONENT_OWNER_ATTR), None, "页面写的 slot 内容归页面");
}

#[test]
fn 具名slot要multiple_slots_否则全进第一个slot() {
    let comps = templates(&[(
        "my-two",
        r#"<view><view class="a"><slot name="a"/></view><view class="b"><slot/></view><view class="c"><slot name="c"/></view></view>"#,
    )]);
    let page = parse(r#"<view><my-two><text slot="a">A</text><text>D</text><text slot="z">Z</text></my-two></view>"#);
    let key = "/my-two@0.0";
    let multi = json!({ "$comp": { key: {} }, "$copt": { "my-two": { "multipleSlots": true } } });
    let out = TemplateEngine::render_with_components(&page, &multi, &comps);
    assert_eq!(texts(&find_class(&out, "a").unwrap().children), vec!["A"]);
    assert_eq!(texts(&find_class(&out, "b").unwrap().children), vec!["D"], "没写 slot 的进默认 slot");
    assert!(texts(&find_class(&out, "c").unwrap().children).is_empty(), "没有内容分到 c");
    assert!(!texts(&out).contains(&"Z".to_string()), "没有对应 slot 的内容不渲染");

    // 单一 slot 模式：第一个 <slot>（不论 name）吃下全部内容，slot 属性不起作用
    let single = json!({ "$comp": { key: {} } });
    let out = TemplateEngine::render_with_components(&page, &single, &comps);
    assert_eq!(texts(&find_class(&out, "a").unwrap().children), vec!["A", "D", "Z"]);
    assert!(texts(&find_class(&out, "b").unwrap().children).is_empty());
}

#[test]
fn slot内容带着使用方的wx_for变量() {
    let comps = templates(&[("my-panel", r#"<view class="box"><slot/></view>"#)]);
    let page = parse(r#"<view><my-panel wx:for="{{list}}"><text>{{item.name}}-{{index}}-{{msg}}</text></my-panel></view>"#);
    let data = json!({
        "list": [{ "name": "x" }, { "name": "y" }], "msg": "m",
        "$comp": { "/my-panel@0.0[0]": { "msg": "C", "item": "C" }, "/my-panel@0.0[1]": { "msg": "C" } }
    });
    let out = TemplateEngine::render_with_components(&page, &data, &comps);
    assert_eq!(texts(&out), vec!["x-0-m", "y-1-m"]);
}

#[test]
fn slot可以写在wx_if里_写在wx_for里内容只出现一次() {
    let comps = templates(&[
        ("my-fold", r#"<view class="f"><slot wx:if="{{open}}"/></view>"#),
        ("my-rep", r#"<view><view wx:for="{{[1,2,3]}}" class="r{{item}}"><slot/></view></view>"#),
    ]);
    let page = parse(r#"<view><my-fold><text>内容</text></my-fold><my-rep><text>一次</text></my-rep></view>"#);
    let data = |open: bool| json!({ "$comp": { "/my-fold@0.0": { "open": open }, "/my-rep@0.1": {} } });
    let shown = TemplateEngine::render_with_components(&page, &data(true), &comps);
    assert_eq!(texts(&find_class(&shown, "f").unwrap().children), vec!["内容"]);
    let hidden = TemplateEngine::render_with_components(&page, &data(false), &comps);
    assert!(texts(&find_class(&hidden, "f").unwrap().children).is_empty(), "slot 在假分支里时内容不渲染");
    assert_eq!(texts(&shown).iter().filter(|t| *t == "一次").count(), 1, "静态 slot 的内容只出现一次");
    assert_eq!(texts(&find_class(&shown, "r1").unwrap().children), vec!["一次"], "重复的 slot 只有第一个生效");
}

/// slot 里的组件属于使用方：按使用方的位置算键、归属使用方
#[test]
fn slot里的组件归使用方() {
    let comps = templates(&[
        ("my-panel", r#"<view class="box"><slot/></view>"#),
        ("my-tag", r#"<text>{{t}}</text>"#),
    ]);
    let page = parse(r#"<view><my-panel><my-tag t="{{v}}"/></my-panel></view>"#);
    let data = json!({ "v": 7, "$comp": { "/my-panel@0.0": {} } });
    let occ = TemplateEngine::collect_components(&page, &data, &comps);
    assert_eq!(occ.len(), 2);
    assert_eq!(occ[1].key, "/my-tag@0.0.s.0");
    assert_eq!(occ[1].owner, "", "页面写进 slot 的组件归页面");
    assert_eq!(occ[1].props["t"], json!(7));
}

/// 同名处理函数：组件模板里的绑定归组件实例，slot 里的归页面
#[test]
fn slot内容的事件绑定归使用方() {
    let comps = templates(&[("my-box", r#"<view class="bx" bindtap="onTap" style="height:50px"><slot/></view>"#)]);
    let page = parse(r#"<view><my-box><view class="in" bindtap="onTap" style="height:20px"/></my-box></view>"#);
    let mut r = WxmlRenderer::new_with_scale(Default::default(), 375.0, 667.0, 1.0);
    r.set_component_templates(comps);
    let mut canvas = Canvas::new(375, 200);
    r.render(&mut canvas, &page, &json!({ "$comp": { "/my-box@0.0": {} } }));
    let mut owners: Vec<String> = r.get_event_bindings().iter().filter(|b| b.handler == "onTap").map(|b| b.owner.clone()).collect();
    owners.sort();
    assert_eq!(owners, vec!["".to_string(), "/my-box@0.0".to_string()]);
}

/// 像素：组件 + slot 与「手工展开」的等价页面逐字节一致（slot 内容只用页面样式）
#[test]
fn 组件加slot与手工展开逐字节一致() {
    let comp_css = crate::using_components::scope_component_wxss(
        ":host{display:block}.box{background-color:#eeeeee;padding:8px}.head{height:20px;background-color:#0088ff}",
        "my-panel",
    );
    let css = format!(".page{{padding:10px}}.msg{{color:#cc0000;font-size:16px}}\n{comp_css}");
    let render = |page: &str, comps: ComponentTemplates, data: Value| {
        let ss = WxssParser::new(&css).parse().unwrap_or_default();
        let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 200.0, 1.0);
        r.set_component_templates(comps);
        let mut c = Canvas::new(375, 200);
        r.render(&mut c, &parse(page), &data);
        c.to_rgba()
    };
    let comps = templates(&[("my-panel", r#"<view class="box"><view class="head"></view><slot/></view>"#)]);
    let a = render(
        r#"<view class="page"><my-panel class="pp"><text class="msg">你好 {{who}}</text></my-panel></view>"#,
        comps,
        json!({ "who": "世界", "$comp": { "/my-panel@0.0": {} } }),
    );
    let b = render(
        r#"<view class="page"><my-panel class="pp"><view class="box"><view class="head"></view><text class="msg">你好 世界</text></view></my-panel></view>"#,
        ComponentTemplates::new(),
        json!({}),
    );
    let diff = a.iter().zip(&b).filter(|(x, y)| x != y).count();
    assert_eq!(diff, 0, "组件 + slot 应与手工展开逐字节一致，差 {diff} 字节");
    assert!(a.chunks(4).any(|p| p[0] > 150 && p[1] < 60 && p[2] < 60), "slot 里的红字应该画出来了");
}

/// 样式隔离：组件的 `.t` 只管组件自己模板里的 `.t`，不命中页面写进 slot 的 `.t`；
/// 页面用「宿主的 class + 后代」选 slot 内容仍然有效
#[test]
fn 组件样式不命中slot内容() {
    let comp_css = crate::using_components::scope_component_wxss(".t{height:40px}", "my-panel");
    let css = format!(".pp .s{{height:15px}}\n{comp_css}");
    let ss = WxssParser::new(&css).parse().unwrap_or_default();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 1.0);
    r.set_component_templates(templates(&[(
        "my-panel",
        r#"<view><view class="t" bindtap="inner"></view><slot/></view>"#,
    )]));
    let page = parse(r#"<view><my-panel class="pp"><view class="t" bindtap="slotted"><view style="height:5px"></view></view><view class="s" bindtap="bycls"></view></my-panel></view>"#);
    let mut canvas = Canvas::new(375, 300);
    r.render(&mut canvas, &page, &json!({ "$comp": { "/my-panel@0.0": {} } }));
    let h = |name: &str| r.get_event_bindings().iter().find(|b| b.handler == name).map(|b| b.bounds.height).unwrap_or(-1.0);
    assert!((h("inner") - 40.0).abs() < 0.5, "组件自己的 .t 应是 40px，实际 {}", h("inner"));
    assert!((h("slotted") - 5.0).abs() < 0.5, "页面写进 slot 的 .t 不该吃组件样式（应是内容高 5px），实际 {}", h("slotted"));
    assert!((h("bycls") - 15.0).abs() < 0.5, "页面按宿主 class 选 slot 内容应有效，实际 {}", h("bycls"));
}
