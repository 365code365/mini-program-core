//! template / import / include 的两条「出图」判据：
//!
//! 1. **像素等价**：用 include + import + template 写的页面，与把内容手抄进来的同一页，
//!    原生渲染出的 RGBA 逐字节相同（这两种写法在微信里本来就是同一棵节点树）；
//! 2. **H5 导出**：编译器走同一个加载函数 —— 首屏静态 HTML 里有模板展开结果，
//!    `window.__WXML__` 里带着 include 进来的内容与模板定义（浏览器端运行时按它重渲染），
//!    custom-tab-bar 里的模板同样展开。

use super::wxml_import_include_tests::Fixture;
use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::WxssParser;
use crate::renderer::wxml_renderer::WxmlRenderer;
use crate::Canvas;
use serde_json::{json, Value};

const CSS: &str = ".box{height:40px;margin:6px;background-color:#ff6b35}
.row{display:flex;flex-direction:row;height:30px;background-color:#e8f4ff}
.cell{flex:1;margin:2px;background-color:#3a7bd5}
.t{font-size:16px;color:#222}";

fn rgba(nodes_src: &str, data: &Value) -> Vec<u8> {
    let ss = WxssParser::new(CSS).parse().unwrap_or_default();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 1.0);
    let nodes = WxmlParser::new(nodes_src).parse().expect("解析");
    let mut c = Canvas::new(375, 667);
    c.clear(crate::Color::WHITE);
    r.render(&mut c, &nodes, data);
    c.to_rgba()
}

#[test]
fn include_与模板写法和手抄写法逐像素相同() {
    let fx = Fixture::new("pixel");
    fx.write("common/row.wxml", r#"<template name="row"><view class="row"><view class="cell" wx:for="{{cells}}"><text class="t">{{item}}</text></view></view></template>"#)
        .write("common/header.wxml", r#"<view class="box"><text class="t">{{title}}</text></view>"#)
        .write(
            "pages/p/p.wxml",
            r#"<import src="/common/row.wxml"/>
               <include src="../../common/header"/>
               <block wx:for="{{rows}}" wx:for-item="r"><template is="row" data="{{...r}}"/></block>
               <template is="badge" data="{{n: rows.length}}"/>
               <template name="badge"><view class="box"><text class="t">共 {{n}} 行</text></view></template>"#,
        );
    let inline = r#"<view class="box"><text class="t">{{title}}</text></view>
        <block wx:for="{{rows}}" wx:for-item="r"><view class="row"><view class="cell" wx:for="{{r.cells}}"><text class="t">{{item}}</text></view></view></block>
        <view class="box"><text class="t">共 {{rows.length}} 行</text></view>"#;
    let data = json!({"title": "标题", "rows": [{"cells": ["a", "b", "c"]}, {"cells": ["d"]}]});

    let expanded = crate::parser::wxml_file::read_expanded(&fx.root, &fx.path("pages/p/p.wxml")).unwrap();
    let a = rgba(&expanded, &data);
    let b = rgba(inline, &data);
    let blank = rgba("<view/>", &data);
    assert_ne!(b, blank, "对照页不能是空白（否则「相同」没有意义）");
    let diff = a.chunks(4).zip(b.chunks(4)).filter(|(x, y)| x != y).count();
    assert_eq!(diff, 0, "include/template 写法与手抄写法有 {diff} 个像素不同");
}

/// H5：编译一个带 include / import / template 的小程序，检查三处产物
#[test]
fn h5_导出带上_include_内容与模板表_首屏有模板输出() {
    let fx = Fixture::new("h5");
    fx.write(
        "app.json",
        r#"{"pages":["pages/index/index"],"tabBar":{"custom":true,"list":[{"pagePath":"pages/index/index","text":"首页"}]}}"#,
    )
    .write("app.js", "App({})")
    .write("pages/index/index.js", "Page({data:{user:{name:'张三'},list:[1,2]}})")
    .write("pages/index/index.json", r#"{"enablePullDownRefresh":false}"#)
    .write("pages/index/card.wxml", r#"<template name="card"><view class="card"><text>卡片:{{name}}</text></view></template>"#)
    .write("pages/index/foot.wxml", r#"<view class="foot"><text>页脚INCLUDE</text></view>"#)
    .write(
        "pages/index/index.wxml",
        r#"<import src="card"/><template is="card" data="{{...user}}"/><include src="foot.wxml"/>"#,
    )
    .write("custom-tab-bar/index.js", "Component({data:{label:'TAB'}})")
    .write("custom-tab-bar/index.wxml", r#"<template name="tab"><text>{{x}}-tab</text></template><view class="bar"><template is="tab" data="{{x: label}}"/></view>"#);

    let root = fx.root.to_string_lossy().to_string();
    let app = crate::compiler::load_app_source(&root).expect("加载小程序");
    let files = crate::compiler::CompileTarget::compile(&crate::compiler::html::HtmlTarget::new(), &app).expect("编译");
    let page = files
        .iter()
        .find(|f| f.path == "pages/index/index.html")
        .map(|f| String::from_utf8_lossy(&f.contents).to_string())
        .expect("页面 HTML");

    let (body, ast) = page.split_once("window.__WXML__ = ").expect("内嵌 AST");
    let ast = ast.split_once(";\n</script>").map(|(a, _)| a).unwrap_or(ast);
    // 首屏静态 HTML：模板展开 + include 内容
    assert!(body.contains("卡片:张三"), "首屏应有模板输出\n{body}");
    assert!(body.contains("页脚INCLUDE"), "首屏应有 include 内容");
    // 运行时 AST：include 已在编译期内联，模板定义随 AST 下发（import 节点的子节点）
    let ast: Value = serde_json::from_str(ast).expect("AST 是 JSON");
    let flat = ast.to_string();
    assert!(flat.contains("页脚INCLUDE"), "include 内容应内联进 __WXML__");
    assert!(flat.contains(r#""name":"card""#), "模板定义应随 __WXML__ 下发");
    assert!(flat.contains(r#""is":"card""#));
    // custom-tab-bar 里的模板
    let bar = app.custom_tab_bar.as_ref().expect("custom-tab-bar");
    let bar_html = crate::compiler::html::wxml_to_html(&bar.wxml, &bar.data);
    assert!(bar_html.contains("TAB-tab"), "custom-tab-bar 的模板应展开\n{bar_html}");
}

/// 浏览器端运行时得认识这几种节点：定义不渲染、`is` 查表展开、include 透传、import/wxs 跳过
#[test]
fn h5_运行时处理_template_include() {
    let js = crate::compiler::html::RUNTIME_JS;
    for needle in ["renderTemplateV", "renderIncludeV", "tplTableOf", "isDeclV"] {
        assert!(js.contains(needle), "运行时缺少 {needle}");
    }
    assert!(
        !js.contains(r#"tag === "template" || tag === "import" || tag === "include""#),
        "template/import/include 不能再当 block 透传"
    );
}
