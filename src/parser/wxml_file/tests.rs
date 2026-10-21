//! 序列化往返：`parse(to_source(parse(x)))` 必须与 `parse(x)` 是同一棵树。
//! 加载器把展开结果按源码字符串交给下游重新解析，这一条不成立的话页面就会悄悄变样。

use super::to_source;
use crate::parser::wxml::{WxmlNode, WxmlParser};

fn same(a: &[WxmlNode], b: &[WxmlNode]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.node_type == y.node_type
                && x.tag_name == y.tag_name
                && x.text_content == y.text_content
                && x.attributes == y.attributes
                && same(&x.children, &y.children)
        })
}

fn round_trip(src: &str) {
    let first = WxmlParser::new(src).parse().expect("原文应能解析");
    let text = to_source(&first);
    let second = WxmlParser::new(&text).parse().unwrap_or_else(|e| panic!("序列化结果解析失败：{e}\n{text}"));
    assert!(same(&first, &second), "往返后节点树变了\n原文：{src}\n序列化：{text}");
    // 序列化本身也要稳定（属性排序），同一棵树两次输出逐字节相同
    assert_eq!(text, to_source(&second));
}

#[test]
fn 常见写法往返不变() {
    round_trip(r#"<view class="a {{b}}" wx:if="{{x > 1 && y < 2}}" data-id="7"><text>hi {{name}} </text>tail</view>"#);
    round_trip(r#"<view wx:else/><input disabled value='{{"q"}}'/><block wx:for="{{list}}" wx:key="id">{{item}}</block>"#);
    round_trip("<view>\n  多行\n  文本 <text>a</text> 中间 <text>b</text>\n</view><!-- 注释丢掉 -->");
    round_trip(r#"<template name="t"><view>{{a}}</view></template><template is="t" data="{{...o, k: 'v'}}"/>"#);
}

#[test]
fn wxs_脚本原文往返不变() {
    round_trip("<wxs module=\"m\">var f = function (a, b) { return a < b && b > 0; };\nmodule.exports.f = f;</wxs><view/>");
    round_trip(r#"<wxs src="./x.wxs" module="x"/><wxs module="e"></wxs>"#);
}
