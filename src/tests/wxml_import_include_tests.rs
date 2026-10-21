//! `<import>` / `<include>` 的文件级语义：在临时目录里搭一个小程序，走真实的加载函数。
//!
//! 期望值按官方 wxml/import.html：import 只引入目标文件**自己定义**的模板（不传递）；
//! include 嵌入目标文件除 `<template>` / `<wxs>` 外的内容；glass-easel（Skyline 只支持它）
//! 下 include 的内容看不见 `wx:for` 的临时变量。路径相对写它的文件，`/` 开头相对小程序根。
//! 缺文件、成环、越出小程序目录都只是「那一处什么都不渲染」，不 panic、不拖垮整页。

use crate::parser::template::TemplateEngine;
use crate::parser::wxml::{WxmlNode, WxmlNodeType, WxmlParser};
use crate::parser::wxml_file::{expand_source, read_expanded};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// 临时小程序目录（drop 时删掉）
pub(crate) struct Fixture {
    pub(crate) root: PathBuf,
}

impl Fixture {
    pub(crate) fn new(tag: &str) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("mini-tpl-{tag}-{}-{n}", std::process::id()));
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).expect("建临时目录");
        Self { root }
    }

    pub(crate) fn write(&self, rel: &str, text: &str) -> &Self {
        let p = self.root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
        self
    }

    pub(crate) fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    /// 经加载器读入并解析成节点树（与页面加载走同一个函数）
    pub(crate) fn load(&self, rel: &str) -> Vec<WxmlNode> {
        let src = read_expanded(&self.root, &self.path(rel)).expect("读页面");
        WxmlParser::new(&src).parse().expect("展开结果应能重新解析")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

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
    nodes.iter().for_each(|n| walk(n, &mut out));
    out
}

fn render(fx: &Fixture, rel: &str, data: &Value) -> Vec<String> {
    texts(&TemplateEngine::render(&fx.load(rel), data))
}

#[test]
fn import_引入目标文件定义的模板() {
    let fx = Fixture::new("import");
    fx.write("common/item.wxml", r#"<template name="item"><text>item:{{text}}</text></template><view>正文不该出现</view>"#)
        .write("pages/a/a.wxml", r#"<import src="../../common/item.wxml"/><template is="item" data="{{text: t}}"/>"#);
    assert_eq!(render(&fx, "pages/a/a.wxml", &json!({"t": "x"})), ["item:x"], "import 只带模板，不带正文");
}

#[test]
fn import_不传递_但被引入模板自己的依赖照常可用() {
    let fx = Fixture::new("transitive");
    fx.write("a.wxml", r#"<template name="A"><text>A{{v}}</text></template>"#)
        .write("b.wxml", r#"<import src="a.wxml"/><template name="B"><text>B</text><template is="A" data="{{v: 1}}"/></template>"#)
        .write("c.wxml", r#"<import src="b.wxml"/><template is="A" data="{{v: 2}}"/><template is="B"/>"#);
    // C 里直接用 A：看不见（官方：不会间接引入目标文件 import 的模板）；
    // C 用 B，B 的模板体里用 A：按 B 的作用域查，查得到
    assert_eq!(render(&fx, "c.wxml", &json!({})), ["B", "A1"]);
}

#[test]
fn 本文件定义优先于_import_后写的_import_优先于先写的() {
    let fx = Fixture::new("override");
    fx.write("x.wxml", r#"<template name="t"><text>x</text></template><template name="u"><text>x-u</text></template>"#)
        .write("y.wxml", r#"<template name="u"><text>y-u</text></template>"#)
        .write("p.wxml", r#"<import src="x"/><import src="y"/><template name="t"><text>local</text></template><template is="t"/><template is="u"/>"#);
    assert_eq!(render(&fx, "p.wxml", &json!({})), ["local", "y-u"]);
}

#[test]
fn include_嵌入正文_不带模板与_wxs() {
    let fx = Fixture::new("include");
    fx.write(
        "parts/header.wxml",
        r#"<template name="h"><text>h</text></template><wxs module="m">var a = i < 1;</wxs><view class="hd">{{title}}</view>"#,
    )
    .write("pages/p.wxml", r#"<include src="../parts/header.wxml"/><template is="h"/><view>body</view>"#);
    let nodes = TemplateEngine::render(&fx.load("pages/p.wxml"), &json!({"title": "标题"}));
    assert_eq!(texts(&nodes), ["标题", "body"], "include 进来的模板定义不在引入方作用域里，wxs 不渲染");
    assert_eq!(nodes[0].attributes.get("class").map(String::as_str), Some("hd"), "include 不留包裹节点");
}

#[test]
fn include_的内容可以用被引入文件自己的模板与_import() {
    let fx = Fixture::new("include-own");
    fx.write("lib.wxml", r#"<template name="L"><text>L</text></template>"#)
        .write("part.wxml", r#"<import src="lib.wxml"/><template name="P"><text>P</text></template><template is="P"/><template is="L"/>"#)
        .write("page.wxml", r#"<include src="part.wxml"/>"#);
    assert_eq!(render(&fx, "page.wxml", &json!({})), ["P", "L"]);
}

#[test]
fn include_可以嵌套_路径支持相对_根路径_省略后缀() {
    let fx = Fixture::new("paths");
    fx.write("common/a.wxml", r#"<text>a</text><include src="./b"/>"#)
        .write("common/b.wxml", r#"<text>b</text><include src="/common/sub/c.wxml"/>"#)
        .write("common/sub/c.wxml", r#"<text>c</text>"#)
        .write("pages/x/x.wxml", r#"<include src="/common/a.wxml"/><include src="../../common/sub/c"/>"#);
    assert_eq!(render(&fx, "pages/x/x.wxml", &json!({})), ["a", "b", "c", "c"]);
}

#[test]
fn 缺文件_成环_越界都只是那一处不渲染() {
    let fx = Fixture::new("guards");
    fx.write("loop1.wxml", r#"<text>1</text><include src="loop2.wxml"/>"#)
        .write("loop2.wxml", r#"<text>2</text><include src="loop1.wxml"/>"#)
        .write("imp1.wxml", r#"<import src="imp2.wxml"/><template name="i1"><text>i1</text></template>"#)
        .write("imp2.wxml", r#"<import src="imp1.wxml"/><template name="i2"><text>i2</text></template>"#)
        .write("bad.wxml", r#"<view><<<</view>"#)
        .write(
            "p.wxml",
            r#"<view>a</view><include src="nope.wxml"/><import src="nope2"/><include src="loop1.wxml"/>
               <import src="imp1.wxml"/><template is="i1"/><include src="../../../../etc/hosts"/>
               <include src="bad.wxml"/><include src="{{dyn}}"/><view>z</view>"#,
        );
    // loop1 → loop2 → (loop1 成环，跳过)：1、2 各出现一次
    assert_eq!(render(&fx, "p.wxml", &json!({})), ["a", "1", "2", "i1", "z"]);
}

#[test]
fn include_看不见_wx_for_的临时变量() {
    // glass-easel 迁移指引：wx:for 中 include 的内容里 item/index 不再有效
    let fx = Fixture::new("include-scope");
    fx.write("row.wxml", r#"<text>[{{item}}][{{title}}]</text>"#)
        .write("p.wxml", r#"<block wx:for="{{list}}"><include src="row.wxml"/></block>"#);
    assert_eq!(render(&fx, "p.wxml", &json!({"list": ["a", "b"], "title": "T"})), ["[][T]", "[][T]"]);
}

#[test]
fn 嵌套定义上提到文件级() {
    let fx = Fixture::new("hoist");
    fx.write("p.wxml", r#"<view><template name="deep"><text>deep</text></template></view><template is="deep"/>"#);
    let out = TemplateEngine::render(&fx.load("p.wxml"), &json!({}));
    assert_eq!(texts(&out), ["deep"]);
    assert!(out[0].children.is_empty(), "定义原位置不留东西");
}

#[test]
fn 没有引用的文件原样返回() {
    let src = "<view class=\"a\">\n  <text>{{x}}</text>\n</view>\n<!-- c -->";
    assert_eq!(expand_source(Path::new("/app"), Path::new("/app/p.wxml"), src), src);
    // 顶层自己解析不了：原样交回，让调用方报原来那条错误
    let broken = "<view><template is=\"x\"/></text>";
    assert_eq!(expand_source(Path::new("/app"), Path::new("/app/p.wxml"), broken), broken);
}

/// 真实页面加载链路：`page_loader::load_page` → `PageInfo{wxml, components}`。
/// 组件里的 import 按**组件自己的目录**解析，与页面无关。
#[test]
fn 页面与组件走真实加载器() {
    let fx = Fixture::new("loader");
    fx.write("app.json", r#"{"pages":["pages/index/index"]}"#)
        .write("pages/index/index.json", r#"{"enablePullDownRefresh":false,"usingComponents":{"x-card":"/components/card/card"}}"#)
        .write("pages/index/index.wxml", r#"<import src="tpl.wxml"/><template is="title" data="{{t}}"/><x-card/>"#)
        .write("pages/index/tpl.wxml", r#"<template name="title"><text>page:{{t}}</text></template>"#)
        .write("components/card/card.json", r#"{"component":true}"#)
        .write("components/card/card.js", "Component({})")
        .write("components/card/card.wxml", r#"<import src="./parts.wxml"/><view class="card"><template is="title" data="{{t: label}}"/></view>"#)
        .write("components/card/parts.wxml", r#"<template name="title"><text>comp:{{t}}</text></template>"#);
    let info = crate::host::page_loader::load_page(&fx.root, "pages/index/index").expect("页面应能加载");
    let page = WxmlParser::new(&info.wxml).parse().unwrap();
    let comps: crate::parser::template::ComponentTemplates = info
        .components
        .iter()
        .map(|c| (c.tag.clone(), WxmlParser::new(&c.wxml).parse().unwrap()))
        .collect();
    let data = json!({"t": "P", "$comp": {"x-card": {"label": "C"}}});
    let out = TemplateEngine::render_with_components(&page, &data, &comps);
    assert_eq!(texts(&out), ["page:P", "comp:C"], "页面与组件各自的同名模板互不串");
    assert_eq!(out[1].tag_name, "x-card");
}
