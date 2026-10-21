//! wxs 模块：`{{m.f(x)}}` / `{{m.PI}}` 的求值、模块可见性（只在定义它的文件里）、
//! `<wxs src>` 与 `require` 的加载期打包、没有逻辑层时的退化、像素等价。
//!
//! 依据官方 [wxs 模块](https://developers.weixin.qq.com/miniprogram/dev/reference/wxs/01wxs-module.html)：
//! 模块只在定义它的 WXML 文件里可见；`<template>` 里只能用定义该模板的文件里的模块；
//! include / import 不会把被引入文件的模块带进来；`require` 只能相对引 `.wxs`。

use crate::parser::template::{ComponentTemplates, TemplateEngine};
use crate::parser::wxml::{WxmlNode, WxmlNodeType, WxmlParser};
use crate::renderer::wxml_renderer::WxmlRenderer;
use crate::runtime::MiniApp;
use crate::Canvas;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// 建一个逻辑层（`init` 会把本线程的 wxs 宿主装上）。返回值要活着：宿主只持有弱引用
fn app() -> MiniApp {
    let mut a = MiniApp::new(375, 667).expect("create MiniApp");
    a.init().expect("init");
    a
}

fn parse(src: &str) -> Vec<WxmlNode> {
    WxmlParser::new(src).parse().expect("parse")
}

fn texts(nodes: &[WxmlNode]) -> Vec<String> {
    fn walk(n: &WxmlNode, out: &mut Vec<String>) {
        if n.node_type == WxmlNodeType::Text {
            if !n.text_content.trim().is_empty() {
                out.push(n.text_content.trim().to_string());
            }
            return;
        }
        n.children.iter().for_each(|c| walk(c, out));
    }
    let mut out = Vec::new();
    nodes.iter().for_each(|n| walk(n, &mut out));
    out
}

fn render(src: &str, data: Value) -> Vec<String> {
    texts(&TemplateEngine::render(&parse(src), &data))
}

const M: &str = r#"<wxs module="m">
var PI = 3.14;
function add(a, b) { return a + b; }
function upper(s) { return s.toUpperCase(); }
function pick(o) { return { k: o.k * 2, list: [o.k, 'x'] }; }
function bad() { throw new Error('boom'); }
function lt(i, n) { var c = 0; for (var j = 0; j < n; j++) { if (j < i) c++; } return c; }
function day(t) { return getDate(t).getUTCDate(); }
function digits(s) { return s.replace(getRegExp('[^0-9]', 'g'), ''); }
module.exports = { PI: PI, add: add, upper: upper, pick: pick, bad: bad, lt: lt, day: day, digits: digits, nested: { twice: function (x) { return x * 2; } } };
</wxs>"#;

#[test]
fn 调用模块函数_参数与返回值保留类型() {
    let _a = app();
    let page = format!(
        "{M}<view><text>{{{{m.add(a, 1)}}}}</text><text>{{{{m.upper(name)}}}}</text><text>{{{{m.pick(o).k}}}}-{{{{m.pick(o).list[1]}}}}</text><text>{{{{m.nested.twice(4)}}}}</text><text>{{{{m['add'](1, 2) + 1}}}}</text><text>{{{{m.lt(3, 10)}}}}</text></view>"
    );
    let out = render(&page, json!({ "a": 2, "name": "abc", "o": { "k": 5 } }));
    assert_eq!(out, vec!["3", "ABC", "10-x", "8", "4", "3"]);
}

#[test]
fn 模块常量_get_date_get_reg_exp() {
    let _a = app();
    let page = format!("{M}<text>{{{{m.PI}}}}|{{{{m.day(86400000 * 2)}}}}|{{{{m.digits('a1b2c3')}}}}</text>");
    assert_eq!(render(&page, json!({})), vec!["3.14|3|123"]);
}

#[test]
fn 模块名压过同名数据字段_出错与不存在的函数得_null() {
    let _a = app();
    let page = format!("{M}<text>[{{{{m.PI}}}}][{{{{m.bad()}}}}][{{{{m.nope(1)}}}}][{{{{other.f(1)}}}}]</text>");
    assert_eq!(render(&page, json!({ "m": { "PI": 1 }, "other": { "f": 1 } })), vec!["[3.14][][][]"]);
}

#[test]
fn wx_for_里每项各调一次_条件与属性也能用() {
    let _a = app();
    let page = format!(
        r#"{M}<view wx:for="{{{{list}}}}" class="c{{{{m.add(index, 1)}}}}"><text wx:if="{{{{m.add(item, 0) > 1}}}}">{{{{m.add(item, 10)}}}}</text></view>"#
    );
    let out = TemplateEngine::render(&parse(&page), &json!({ "list": [1, 2, 3] }));
    assert_eq!(texts(&out), vec!["12", "13"]);
    assert_eq!(out[2].get_attr("class"), Some("c3"));
}

#[test]
fn 同一文件里的模板片段能用_组件模板只用自己文件的模块_slot内容用使用方的() {
    let _a = app();
    // 模板定义在页面文件里：用页面的模块
    let page = format!(r#"{M}<template name="t"><text>{{{{m.add(x, 100)}}}}</text></template><template is="t" data="{{{{x: 1}}}}"/>"#);
    assert_eq!(render(&page, json!({})), vec!["101"]);

    // 组件文件里有自己的 m（另一份代码）；页面的 m 进不去，slot 内容（页面写的）用页面的 m
    let comp = r#"<wxs module="m">module.exports = { add: function (a, b) { return 'C' + (a + b); } };</wxs><view><text>{{m.add(1, 1)}}</text><slot/></view>"#;
    let mut comps = ComponentTemplates::new();
    comps.insert("my-c".into(), parse(comp));
    let page = format!(r#"{M}<my-c><text>{{{{m.add(1, 1)}}}}</text></my-c>"#);
    let out = TemplateEngine::render_with_components(&parse(&page), &json!({ "$comp": { "/my-c@1": {} } }), &comps);
    assert_eq!(texts(&out), vec!["C2", "2"]);

    // 组件文件里没有模块：组件模板里的 m 不是页面的那个（求值为空）
    comps.insert("my-c".into(), parse(r#"<view><text>[{{m.add(1, 1)}}]</text></view>"#));
    let out = TemplateEngine::render_with_components(&parse(&page), &json!({ "$comp": { "/my-c@1": {} } }), &comps);
    assert_eq!(texts(&out), vec!["[]"]);
}

#[test]
fn 没有逻辑层的线程上调用退化为空() {
    // 画廊、离屏渲染只建渲染器不建逻辑层：与从前一样得 null，不崩
    let out = std::thread::spawn(|| render(&format!("{M}<text>[{{{{m.add(1, 2)}}}}][{{{{m.PI}}}}]</text>"), json!({})))
        .join()
        .unwrap();
    assert_eq!(out, vec!["[][]"]);
}

#[test]
fn 用模块渲染与直接写结果逐像素相同() {
    let _a = app();
    let draw = |src: &str| {
        let mut r = WxmlRenderer::new_with_scale(Default::default(), 375.0, 120.0, 2.0);
        let mut c = Canvas::new(750, 240);
        r.render(&mut c, &parse(src), &json!({ "price": 1999 }));
        c.to_rgba()
    };
    let fmt = r#"<wxs module="f">module.exports.yuan = function (c) { return '¥' + (c / 100).toFixed(2); };</wxs>"#;
    let a = draw(&format!(r#"{fmt}<view style="padding:10px;font-size:20px;color:#c00"><text>{{{{f.yuan(price)}}}}</text></view>"#));
    let b = draw(r#"<view style="padding:10px;font-size:20px;color:#c00"><text>¥19.99</text></view>"#);
    assert!(a.chunks(4).any(|p| p[0] > 150 && p[1] < 80), "文字应该画出来了");
    assert_eq!(a.iter().zip(&b).filter(|(x, y)| x != y).count(), 0, "wxs 渲染应与手写结果逐字节一致");
}

// ============ 文件层：<wxs src>、require、include / import 的可见性 ============

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn tmp(name: &str, files: &[(&str, &str)]) -> Tmp {
    let root = std::env::temp_dir().join(format!("mini-wxs-{}-{name}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    Tmp(root)
}

fn load(root: &Path, file: &str) -> Vec<WxmlNode> {
    let src = crate::parser::wxml_file::read_expanded(root, &root.join(file)).expect("read");
    parse(&src)
}

#[test]
fn src_与_require_在加载期打包_相对路径与单例() {
    let _a = app();
    let fx = tmp(
        "src",
        &[
            ("utils/fmt.wxs", "var base = require('./base.wxs'); var again = require('./base.wxs'); module.exports = { tag: function (s) { return base.wrap(s) + (base === again ? '!' : '?'); } };"),
            ("utils/base.wxs", "var n = require('../lib/num.wxs'); module.exports = { wrap: function (s) { return '[' + s + n.ONE + ']'; } };"),
            ("lib/num.wxs", "module.exports = { ONE: 1 };"),
            (
                "pages/a/a.wxml",
                r#"<wxs module="f" src="../../utils/fmt.wxs"/><wxs module="g">var b = require('../../utils/base.wxs'); module.exports.w = function (s) { return b.wrap(s); };</wxs><text>{{f.tag('x')}}|{{g.w('y')}}</text>"#,
            ),
        ],
    );
    let nodes = load(&fx.0, "pages/a/a.wxml");
    assert_eq!(texts(&TemplateEngine::render(&nodes, &json!({})))
        .iter()
        .filter(|t| t.contains('|'))
        .cloned()
        .collect::<Vec<_>>(), vec!["[x1]!|[y1]"]);
}

#[test]
fn include_与_import_不带模块_被引入文件里用它自己的() {
    let _a = app();
    let fx = tmp(
        "vis",
        &[
            (
                "part.wxml",
                r#"<wxs module="p">module.exports.v = function () { return 'P'; };</wxs><text>inc[{{p.v()}}][{{m.v()}}]</text><template name="tp"><text>tpl[{{p.v()}}][{{m.v()}}]</text></template>"#,
            ),
            (
                "index.wxml",
                r#"<wxs module="m">module.exports.v = function () { return 'M'; };</wxs><import src="part"/><include src="part"/><template is="tp"/><text>page[{{m.v()}}][{{p.v()}}]</text>"#,
            ),
        ],
    );
    let out = texts(&TemplateEngine::render(&load(&fx.0, "index.wxml"), &json!({})));
    assert_eq!(out, vec!["inc[P][]", "tpl[P][]", "page[M][]"]);
}

#[test]
fn 坏引用只让那一个模块为空() {
    let _a = app();
    let fx = tmp(
        "bad",
        &[(
            "index.wxml",
            r#"<wxs module="a" src="./missing.wxs"/><wxs module="b">var x = require('./nope.wxs'); module.exports.v = function () { return 1; };</wxs><wxs module="c">module.exports.v = function () { return 'ok'; };</wxs><text>[{{a.v()}}][{{b.v()}}][{{c.v()}}]</text>"#,
        )],
    );
    assert_eq!(texts(&TemplateEngine::render(&load(&fx.0, "index.wxml"), &json!({}))), vec!["[][][ok]"]);
}

#[test]
fn require_扫描只认相对字符串参数() {
    use crate::parser::wxml_file::wxs_requires;
    assert_eq!(wxs_requires("var a = require('./a.wxs'); var b = require ( \"../b.wxs\" );"), vec!["./a.wxs", "../b.wxs"]);
    assert!(wxs_requires("myrequire('./x.wxs'); o.require('./y.wxs'); require(name);").is_empty());
}

/// H5 导出：`<wxs src>` 与 `require` 编译期打进 `__WXML__`（带打包标记），浏览器运行时认得 wxs 节点。
/// 浏览器里真跑出来的结果（headless Chrome `--dump-dom`）在 doc/引擎测试说明.md T12 里记了手工核对步骤
#[test]
fn h5_导出带上_wxs_模块包() {
    let fx = tmp(
        "h5",
        &[
            ("app.json", r#"{"pages":["pages/index/index"]}"#),
            ("app.js", "App({})"),
            ("pages/index/index.json", "{}"),
            ("pages/index/index.js", "Page({data:{price:1999}})"),
            ("utils/fmt.wxs", "var b = require('./base.wxs'); module.exports.yuan = function (c) { return b.sym + (c / 100).toFixed(2); };"),
            ("utils/base.wxs", "module.exports = { sym: 'Y' };"),
            ("pages/index/index.wxml", r#"<wxs module="f" src="../../utils/fmt.wxs"/><text>{{f.yuan(price)}}</text>"#),
        ],
    );
    let app = crate::compiler::load_app_source(&fx.0.to_string_lossy()).expect("加载小程序");
    let files = crate::compiler::CompileTarget::compile(&crate::compiler::html::HtmlTarget::new(), &app).expect("编译");
    let page = files
        .iter()
        .find(|f| f.path == "pages/index/index.html")
        .map(|f| String::from_utf8_lossy(&f.contents).to_string())
        .expect("页面 HTML");
    let (_, ast) = page.split_once("window.__WXML__ = ").expect("内嵌 AST");
    assert!(ast.contains("__wxs_bundle"), "wxs 节点应带打包标记");
    assert!(ast.contains("/utils/base.wxs") && ast.contains("/utils/fmt.wxs"), "src 与 require 的文件都应打进包里");
    assert!(!ast.contains("</wxs"), "包里不能出现会截断原始文本的 </");
    let js = crate::compiler::html::RUNTIME_JS;
    for needle in ["wxsOf", "wxsLoad", "with($w)"] {
        assert!(js.contains(needle), "运行时缺少 {needle}");
    }
}
