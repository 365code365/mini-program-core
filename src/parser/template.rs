//! 模板引擎 - 处理数据绑定、条件渲染、列表渲染
//!
//! 表达式求值统一委托给 `super::expr`（完整的 `{{}}` 表达式引擎），
//! 本模块只负责 WXML 控制流：`wx:if / wx:elif / wx:else`、`wx:for`、
//! `<block>` 透传，以及文本/属性插值。

use super::wxml::{WxmlNode, WxmlNodeType};
use super::expr;
use serde_json::Value as JsonValue;

/// 模板引擎
pub struct TemplateEngine;

impl TemplateEngine {
    /// 渲染模板，替换 {{}} 表达式
    pub fn render(nodes: &[WxmlNode], data: &JsonValue) -> Vec<WxmlNode> {
        Self::render_nodes(nodes, data)
    }
    
    /// 渲染模板（保留 viewport 形参以兼容旧调用；虚拟列表裁剪在绘制阶段处理）
    pub fn render_with_virtual_list(
        nodes: &[WxmlNode],
        data: &JsonValue,
        _viewport: Option<(f32, f32)>,
    ) -> Vec<WxmlNode> {
        Self::render_nodes(nodes, data)
    }
    
    /// 渲染一组兄弟节点，处理 if/elif/else 链
    fn render_nodes(nodes: &[WxmlNode], data: &JsonValue) -> Vec<WxmlNode> {
        let mut out = Vec::new();
        // if/elif/else 链状态
        let mut chain_active = false; // 当前是否处于一个 wx:if 链中
        let mut chain_taken = false;  // 链中是否已有分支命中
        
        for node in nodes {
            match node.node_type {
                WxmlNodeType::Comment => { /* 丢弃注释 */ }
                WxmlNodeType::Text => {
                    chain_active = false;
                    let text = Self::interpolate(&node.text_content, data);
                    out.push(WxmlNode::new_text(&text));
                }
                WxmlNodeType::Element => {
                    // wx:for 优先级最高
                    if node.attributes.contains_key("wx:for") {
                        chain_active = false;
                        Self::render_for(node, data, &mut out);
                        continue;
                    }
                    
                    if let Some(cond) = node.attributes.get("wx:if") {
                        chain_active = true;
                        chain_taken = Self::eval_condition(cond, data);
                        if chain_taken {
                            Self::emit_element(node, data, &mut out);
                        }
                    } else if let Some(cond) = node.attributes.get("wx:elif") {
                        if chain_active && !chain_taken && Self::eval_condition(cond, data) {
                            chain_taken = true;
                            Self::emit_element(node, data, &mut out);
                        }
                    } else if node.attributes.contains_key("wx:else") {
                        if chain_active && !chain_taken {
                            Self::emit_element(node, data, &mut out);
                        }
                        chain_active = false;
                        chain_taken = false;
                    } else {
                        chain_active = false;
                        Self::emit_element(node, data, &mut out);
                    }
                }
            }
        }
        
        out
    }
    
    /// 输出一个已经通过条件判断的元素。
    /// `<block>` 不产生真实节点，只展开其子节点。
    fn emit_element(node: &WxmlNode, data: &JsonValue, out: &mut Vec<WxmlNode>) {
        if node.tag_name == "block" {
            let children = Self::render_nodes(&node.children, data);
            out.extend(children);
            return;
        }
        
        let mut new_node = WxmlNode::new_element(&node.tag_name);
        for (key, value) in &node.attributes {
            if Self::is_directive(key) {
                continue;
            }
            new_node.attributes.insert(key.clone(), Self::interpolate(value, data));
        }
        new_node.children = Self::render_nodes(&node.children, data);
        out.push(new_node);
    }
    
    /// 渲染 wx:for 循环
    fn render_for(node: &WxmlNode, data: &JsonValue, out: &mut Vec<WxmlNode>) {
        let for_expr = match node.attributes.get("wx:for") {
            Some(e) => e,
            None => return,
        };
        let item_name = node.attributes.get("wx:for-item").map(|s| s.as_str()).unwrap_or("item");
        let index_name = node.attributes.get("wx:for-index").map(|s| s.as_str()).unwrap_or("index");
        
        // 计算被遍历的集合
        let arr_val = Self::eval_value(for_expr, data);
        let items: Vec<JsonValue> = match arr_val {
            JsonValue::Array(a) => a,
            // wx:for 作用于对象时遍历其值
            JsonValue::Object(m) => m.into_iter().map(|(_, v)| v).collect(),
            // 作用于字符串时按字符遍历
            JsonValue::String(s) => s.chars().map(|c| JsonValue::String(c.to_string())).collect(),
            _ => return,
        };
        
        for (index, item) in items.iter().enumerate() {
            let mut loop_data = data.clone();
            if let Some(obj) = loop_data.as_object_mut() {
                obj.insert(item_name.to_string(), item.clone());
                obj.insert(index_name.to_string(), JsonValue::Number(index.into()));
            }
            
            // wx:for 与 wx:if 同时存在时，wx:if 对每个 item 求值
            if let Some(cond) = node.attributes.get("wx:if") {
                if !Self::eval_condition(cond, &loop_data) {
                    continue;
                }
            }
            
            Self::emit_element(node, &loop_data, out);
        }
    }
    
    /// 文本/属性插值：替换所有 {{ ... }} 片段
    fn interpolate(template: &str, data: &JsonValue) -> String {
        if !template.contains("{{") {
            return template.to_string();
        }
        
        let mut result = String::new();
        let mut rest = template;
        
        while let Some(open) = rest.find("{{") {
            result.push_str(&rest[..open]);
            rest = &rest[open + 2..];
            if let Some(close) = rest.find("}}") {
                let expr_src = &rest[..close];
                result.push_str(&expr::eval_to_display(expr_src, data));
                rest = &rest[close + 2..];
            } else {
                // 没有闭合，原样输出
                result.push_str("{{");
                result.push_str(rest);
                rest = "";
                break;
            }
        }
        result.push_str(rest);
        result
    }
    
    /// 求值一个可能被 {{ }} 包裹的表达式，返回 JSON 值（用于 wx:for）
    fn eval_value(raw: &str, data: &JsonValue) -> JsonValue {
        expr::eval_str(&Self::extract_expression(raw), data)
    }
    
    /// 求值条件表达式（用于 wx:if / wx:elif）
    fn eval_condition(raw: &str, data: &JsonValue) -> bool {
        expr::is_truthy(&Self::eval_value(raw, data))
    }
    
    /// 提取 {{ }} 中的表达式；若没有大括号则原样返回
    fn extract_expression(s: &str) -> String {
        let s = s.trim();
        if s.starts_with("{{") && s.ends_with("}}") && s.len() >= 4 {
            s[2..s.len() - 2].trim().to_string()
        } else {
            s.to_string()
        }
    }
    
    /// 是否为 wx: 指令属性（渲染时不输出到最终节点）
    fn is_directive(key: &str) -> bool {
        key.starts_with("wx:")
    }
}

#[cfg(test)]
mod tests {
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
}
