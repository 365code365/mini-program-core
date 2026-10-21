//! `<template is>`（模板片段）与 `<include>`（嵌入文件）的展开。
//!
//! 两者都不产生真实节点，内容原地展开；区别在于看得见哪些数据：
//!
//! - **模板片段**只看得见 `data` 传进来的那个对象（官方 wxml/template.html：
//!   「模板片段内部只能使用 data 传入的数据，不能使用其他数据和临时变量」）。
//!   `data` 必须是对象绑定：`{{...item}}`、`{{...a, b: 1}}`、`{{a: 1, b: x}}`、
//!   简写 `{{foo, bar}}`（= `{foo: foo, bar: bar}`）。`is` 可以是字面量，也可以带插值
//!   （`is="type{{t}}"`）。
//! - **include 的内容**用的是「数据根」（页面/组件 data，或所在模板的 data），**不认**
//!   `wx:for` 产生的 `item` / `index`。这是 glass-easel 的行为（wxml/import.html 与
//!   glass-easel 迁移指引：「在 wx:for 中使用 include 时，被引入的模板中的 item、index
//!   等新变量不再有效」），而 Skyline 只支持 glass-easel —— 旧 WebView/exparser 下
//!   include 是纯文本拷贝、看得见循环变量，这里**有意**不跟那个旧行为。

use super::scope::{warn_once, Cx, Frame, MAX_TEMPLATE_DEPTH};
use super::*;

impl TemplateEngine {
    /// `<template is="…" data="{{…}}"/>`：在当前作用域里找定义，用 `data` 作为**全部**作用域展开
    pub(super) fn emit_template<'a>(
        node: &'a WxmlNode,
        data: &JsonValue,
        cx: &Cx<'a>,
        f: &Frame<'a, '_>,
        out: &mut Vec<WxmlNode>,
    ) {
        let Some(is) = node.attributes.get("is") else { return };
        let name = Self::interpolate(is, data);
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let Some(def) = f.table.get(name).copied() else {
            warn_once(format!("<template is=\"{name}\">：当前文件里没有这个模板（也不在 import 的文件里），什么都不渲染"));
            return;
        };
        if f.depth >= MAX_TEMPLATE_DEPTH {
            warn_once(format!("<template is=\"{name}\">：模板嵌套超过 {MAX_TEMPLATE_DEPTH} 层，疑似无终止的递归，已截断"));
            return;
        }
        if !cx.take_budget() {
            warn_once(format!("<template is=\"{name}\">：单次渲染展开的模板过多，已截断"));
            return;
        }
        let scope = match node.attributes.get("data") {
            Some(raw) => Self::template_data(raw, data),
            None => JsonValue::Object(Default::default()),
        };
        // 模板里也可以用自定义组件：组件实例数据由 `Cx` 统一带着（不再往作用域里塞 `$comp`）。
        // 归属与 slot 沿用调用处 —— 模板片段只是「原地展开」，不改变这些节点属于谁
        let inner = Frame {
            table: cx.table_of(def.file),
            root: &scope,
            depth: f.depth + 1,
            owner: f.owner,
            slot: f.slot,
            cdepth: f.cdepth,
            // 模板里只能用定义该模板的文件里的 wxs 模块（官方 wxs 模块文档）
            wxs: cx.wxs_of(def.file),
        };
        let kids = Self::render_nodes(def.body, &scope, cx, &inner);
        out.extend(kids);
    }

    /// `<include>`：子节点是加载时嵌进来的文件内容，用数据根渲染（见模块注释）
    pub(super) fn emit_include<'a>(
        node: &'a WxmlNode,
        cx: &Cx<'a>,
        f: &Frame<'a, '_>,
        out: &mut Vec<WxmlNode>,
    ) {
        if node.children.is_empty() {
            return;
        }
        let inner = Frame {
            table: cx.include_table(&f.table, &node.children),
            root: f.root,
            depth: f.depth,
            owner: f.owner,
            slot: f.slot,
            cdepth: f.cdepth,
            // 被引入文件的内容用它自己文件里的模块；引入方的模块带不进去（include 不传递 wxs）
            wxs: cx.wxs_of(&node.children),
        };
        let kids = Self::render_nodes(&node.children, f.root, cx, &inner);
        out.extend(kids);
    }

    /// `data="{{ … }}"` 的花括号里是**对象字面量的内容**（不带外层 `{}`）。
    /// 不是 `{{}}` 绑定的写法微信不认，这里给空对象。
    fn template_data(raw: &str, data: &JsonValue) -> JsonValue {
        let t = raw.trim();
        match t.strip_prefix("{{").and_then(|s| s.strip_suffix("}}")) {
            Some(inner) => expr::eval_object_body(inner, data),
            None => JsonValue::Object(Default::default()),
        }
    }

    /// 只在编译期起作用、渲染时什么都不产生的节点：模板定义、import、wxs。
    /// 它们夹在 `wx:if` / `wx:else` 之间时**不打断**条件链（在微信里它们根本不在节点树上）。
    pub(super) fn is_declaration(node: &WxmlNode) -> bool {
        match node.tag_name.as_str() {
            "template" => node.attributes.contains_key("name"),
            "import" | "wxs" => true,
            _ => false,
        }
    }
}
