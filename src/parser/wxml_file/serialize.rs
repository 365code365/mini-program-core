//! 节点树 → WXML 源码。
//!
//! 只为一件事服务：展开 include/import 之后，把结果按**源码字符串**交还给下游
//! （下游都是 `WxmlParser::new(&src).parse()`）。所以判据只有一条：
//! `parse(to_source(parse(x)))` 与 `parse(x)` 的节点树完全相同（有测试守着）。
//!
//! 几个依赖解析器行为的点：
//! - 属性按名字排序输出（`attrs_sorted`）：`HashMap` 的遍历顺序每个进程不同，
//!   不排序的话同一份工程两次导出的字符串不一样；
//! - 解析器不认实体转义，属性值里有 `"` 时只能换单引号包（它本来就是从单引号里读出来的）；
//! - 文本里不可能有 `<`（解析器读到 `<` 就切断了），原样输出即可；
//! - `<wxs>` 的子文本是脚本原文，解析器对它走原始文本模式，原样输出即可；
//! - 注释在解析时已经丢了，这里也不输出。

use crate::parser::wxml::{WxmlNode, WxmlNodeType};

/// 把一组节点写回 WXML 源码
pub fn to_source(nodes: &[WxmlNode]) -> String {
    let mut out = String::new();
    for n in nodes {
        write_node(n, &mut out);
    }
    out
}

fn write_node(n: &WxmlNode, out: &mut String) {
    match n.node_type {
        WxmlNodeType::Comment => {}
        WxmlNodeType::Text => out.push_str(&n.text_content),
        WxmlNodeType::Element => {
            out.push('<');
            out.push_str(&n.tag_name);
            for (k, v) in n.attrs_sorted() {
                let q = if v.contains('"') { '\'' } else { '"' };
                out.push(' ');
                out.push_str(k);
                out.push('=');
                out.push(q);
                out.push_str(v);
                out.push(q);
            }
            if n.children.is_empty() {
                out.push_str("/>");
                return;
            }
            out.push('>');
            for c in &n.children {
                write_node(c, out);
            }
            out.push_str("</");
            out.push_str(&n.tag_name);
            out.push('>');
        }
    }
}
