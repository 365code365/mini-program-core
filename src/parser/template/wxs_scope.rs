//! 一个 WXML 文件里定义了哪些 wxs 模块（模块名 → 模块 id），以及模块代码的登记。
//!
//! 可见性按官方 wxs/01wxs-module.html：模块只在**定义它的那个 WXML 文件**里可见；
//! `<template>` 里只能用定义该模板的文件里的模块；include / import 都**不**把被引入文件的模块
//! 带进来。所以扫描时不进 `<include>` / `<import>` 的子节点 —— 那是别的文件的内容，
//! 它们各自作为作用域根时再扫。
//!
//! 模块代码有两种形状：
//! - 加载期打好的包（`<wxs src>`，或内联代码里 `require` 了别的 `.wxs`）：节点带
//!   [`WXS_BUNDLE_ATTR`]，子文本就是 `{"entry":…,"files":{路径: 代码}}`（见 `wxml_file::loader`）；
//! - 没经过加载器的内联代码（单测直接解析的 WXML、不 require 别人的内联模块）：这里现包一个。

use super::*;
use crate::parser::expr::wxs::{self, WxsScope};
use std::rc::Rc;

/// `<wxs>` 节点上「子文本是加载期打好的模块包」的隐藏标记
pub const WXS_BUNDLE_ATTR: &str = "__wxs_bundle";

/// 扫一个作用域根；一个模块都没有返回 None（绝大多数页面）
pub(super) fn scan(list: &[WxmlNode]) -> Option<Rc<WxsScope>> {
    let mut sc = WxsScope::default();
    collect(list, &mut sc);
    if sc.modules.is_empty() {
        None
    } else {
        Some(Rc::new(sc))
    }
}

fn collect(list: &[WxmlNode], sc: &mut WxsScope) {
    for n in list {
        if n.node_type != WxmlNodeType::Element {
            continue;
        }
        match n.tag_name.as_str() {
            "wxs" => {
                let Some(name) = n.get_attr("module").map(str::trim).filter(|s| !s.is_empty()) else {
                    continue;
                };
                // 同名模块微信编译期就报错；这里保留先写的那个
                if sc.modules.contains_key(name) {
                    continue;
                }
                let bundle = bundle_of(n, name);
                let id = wxs::module_id_of(&bundle);
                wxs::register(&id, &bundle);
                sc.modules.insert(name.to_string(), id);
            }
            "include" | "import" => {}
            _ => collect(&n.children, sc),
        }
    }
}

/// 模块的打包源码
pub fn bundle_of(n: &WxmlNode, name: &str) -> String {
    let text: String = n
        .children
        .iter()
        .filter(|c| c.node_type == WxmlNodeType::Text)
        .map(|c| c.text_content.as_str())
        .collect();
    if n.attributes.contains_key(WXS_BUNDLE_ATTR) {
        return text;
    }
    let entry = format!("inline:{name}");
    serde_json::json!({ "entry": entry, "files": { entry.clone(): text } }).to_string()
}
