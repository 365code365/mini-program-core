//! 模板引擎单测（控制流）。从 `template.rs` 的 `mod tests` 原样切出；
//! template / import / include 的用例在 `src/tests/wxml_template_tests.rs` 等处。

use super::*;
use crate::parser::wxml::WxmlParser;
use serde_json::json;

fn render_wxml(wxml: &str, data: &JsonValue) -> Vec<WxmlNode> {
    let mut parser = WxmlParser::new(wxml);
    let nodes = parser.parse().unwrap();
    TemplateEngine::render(&nodes, data)
}

#[test]
fn test_if_elif_else() {
    let wxml = r#"
        <view wx:if="{{n === 1}}">one</view>
        <view wx:elif="{{n === 2}}">two</view>
        <view wx:else>other</view>
    "#;
    let out = render_wxml(wxml, &json!({"n": 2}));
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].children[0].text_content, "two");
}

#[test]
fn test_for_loop() {
    let wxml = r#"<view wx:for="{{list}}">{{item}}-{{index}}</view>"#;
    let out = render_wxml(wxml, &json!({"list": ["a", "b"]}));
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].children[0].text_content, "a-0");
    assert_eq!(out[1].children[0].text_content, "b-1");
}

#[test]
fn test_block_unwrap() {
    let wxml = r#"<block wx:if="{{show}}"><text>x</text><text>y</text></block>"#;
    let out = render_wxml(wxml, &json!({"show": true}));
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].tag_name, "text");
    assert_eq!(out[1].tag_name, "text");
}
