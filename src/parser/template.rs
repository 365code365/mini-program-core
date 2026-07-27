//! 模板引擎 - 处理数据绑定、条件渲染、列表渲染
//!
//! 表达式求值统一委托给 `super::expr`（完整的 `{{}}` 表达式引擎），
//! 本模块只负责 WXML 控制流：`wx:if / wx:elif / wx:else`、`wx:for`、
//! `<block>` 透传，以及文本/属性插值。

use super::wxml::{WxmlNode, WxmlNodeType};
use super::expr;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// 自定义组件模板表：标签名 → 组件 WXML 根节点
pub type ComponentTemplates = HashMap<String, Vec<WxmlNode>>;

/// 组件实例数据在渲染数据里的存放位置：`data["$comp"]["<标签名>"]`。
/// 用 `$` 开头是因为 WXML 表达式里不会出现这样的标识符，不会和页面字段撞名。
pub const COMPONENT_DATA_KEY: &str = "$comp";

/// 模板引擎
pub struct TemplateEngine;

impl TemplateEngine {
    /// 渲染模板，替换 {{}} 表达式
    pub fn render(nodes: &[WxmlNode], data: &JsonValue) -> Vec<WxmlNode> {
        Self::render_nodes(nodes, data, &ComponentTemplates::new())
    }
    
    /// 渲染模板（保留 viewport 形参以兼容旧调用；虚拟列表裁剪在绘制阶段处理）
    pub fn render_with_virtual_list(
        nodes: &[WxmlNode],
        data: &JsonValue,
        _viewport: Option<(f32, f32)>,
    ) -> Vec<WxmlNode> {
        Self::render_nodes(nodes, data, &ComponentTemplates::new())
    }

    /// 渲染模板，并把 `usingComponents` 声明的自定义组件标签展开成组件自己的模板。
    ///
    /// 组件子树用**组件实例自己的 data** 求值（放在 `data["$comp"][标签名]`）——
    /// 组件的 `{{a}}` 与页面的 `{{a}}` 是两套东西，共用一个作用域会互相串味。
    pub fn render_with_components(
        nodes: &[WxmlNode],
        data: &JsonValue,
        components: &ComponentTemplates,
    ) -> Vec<WxmlNode> {
        Self::render_nodes(nodes, data, components)
    }
    
    /// 渲染一组兄弟节点，处理 if/elif/else 链
    fn render_nodes(nodes: &[WxmlNode], data: &JsonValue, comps: &ComponentTemplates) -> Vec<WxmlNode> {
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
                        Self::render_for(node, data, comps, &mut out);
                        continue;
                    }
                    
                    if let Some(cond) = node.attributes.get("wx:if") {
                        chain_active = true;
                        chain_taken = Self::eval_condition(cond, data);
                        if chain_taken {
                            Self::emit_element(node, data, comps, &mut out);
                        }
                    } else if let Some(cond) = node.attributes.get("wx:elif") {
                        if chain_active && !chain_taken && Self::eval_condition(cond, data) {
                            chain_taken = true;
                            Self::emit_element(node, data, comps, &mut out);
                        }
                    } else if node.attributes.contains_key("wx:else") {
                        if chain_active && !chain_taken {
                            Self::emit_element(node, data, comps, &mut out);
                        }
                        chain_active = false;
                        chain_taken = false;
                    } else {
                        chain_active = false;
                        Self::emit_element(node, data, comps, &mut out);
                    }
                }
            }
        }
        
        out
    }
    
    /// 输出一个已经通过条件判断的元素。
    /// `<block>` 不产生真实节点，只展开其子节点。
    fn emit_element(node: &WxmlNode, data: &JsonValue, comps: &ComponentTemplates, out: &mut Vec<WxmlNode>) {
        if node.tag_name == "block" {
            let children = Self::render_nodes(&node.children, data, comps);
            out.extend(children);
            return;
        }
        if let Some(tpl) = comps.get(&node.tag_name) {
            Self::emit_component(node, tpl, data, comps, out);
            return;
        }
        
        let mut new_node = WxmlNode::new_element(&node.tag_name);
        for (key, value) in &node.attributes {
            if Self::is_directive(key) {
                continue;
            }
            new_node.attributes.insert(key.clone(), Self::interpolate_attr(key, value, data));
        }
        new_node.children = Self::render_nodes(&node.children, data, comps);
        out.push(new_node);
    }

    /// 属性插值。`class` / `style` 绑定到数组或对象时按 CSS 语义拼接，
    /// 而不是把 JSON 直接塞进属性值。
    ///
    /// `class="{{['tabbar', d]}}"` 是 uni-app 每个组件/页面根节点的固定写法
    /// （`d` 是虚拟宿主类名）。原来走通用插值会得到字面量 `['tabbar','']`，
    /// 于是 `.tabbar` / `.root` / `.page` 这些**根节点样式全部失效** ——
    /// 底部导航因此既没有 `position:fixed` 也没有背景，整页配色也对不上。
    fn interpolate_attr(key: &str, value: &str, data: &JsonValue) -> String {
        if key != "class" && key != "style" {
            return Self::interpolate(value, data);
        }
        let trimmed = value.trim();
        // 只有「整个属性值就是一个表达式」时才需要特殊拼接
        let inner = match trimmed.strip_prefix("{{").and_then(|s| s.strip_suffix("}}")) {
            Some(i) if !i.contains("{{") => i.trim(),
            _ => return Self::interpolate(value, data),
        };
        let sep = if key == "class" { " " } else { ";" };
        match expr::eval_str(inner, data) {
            JsonValue::Array(items) => items
                .iter()
                .map(expr::render_value)
                .filter(|s| !s.trim().is_empty())
                .collect::<Vec<_>>()
                .join(sep),
            // `class="{{ {active: on} }}"`：取值为真的键；style 取 `k:v`
            JsonValue::Object(map) => map
                .iter()
                .filter(|(_, v)| expr::is_truthy(v))
                .map(|(k, v)| {
                    if key == "class" {
                        k.clone()
                    } else {
                        format!("{k}:{}", expr::render_value(v))
                    }
                })
                .collect::<Vec<_>>()
                .join(sep),
            other => expr::render_value(&other),
        }
    }

    /// 展开一个自定义组件：保留一个以组件标签命名的**宿主节点**（承载使用方写在标签上的
    /// class/style，并让组件 WXSS 里的 `:host` 与作用域前缀能命中），
    /// 其内容用组件实例自己的 data 渲染。
    fn emit_component(
        node: &WxmlNode,
        tpl: &[WxmlNode],
        data: &JsonValue,
        comps: &ComponentTemplates,
        out: &mut Vec<WxmlNode>,
    ) {
        let mut host = WxmlNode::new_element(&node.tag_name);
        for (key, value) in &node.attributes {
            if Self::is_directive(key) {
                continue;
            }
            host.attributes.insert(key.clone(), Self::interpolate_attr(key, value, data));
        }
        let comp_data = data
            .get(COMPONENT_DATA_KEY)
            .and_then(|m| m.get(&node.tag_name))
            .cloned()
            .unwrap_or_else(|| JsonValue::Object(Default::default()));
        // 组件内部再用别的组件时，`$comp` 要继续可见
        let mut scope = comp_data;
        if let (Some(obj), Some(all)) = (scope.as_object_mut(), data.get(COMPONENT_DATA_KEY)) {
            obj.insert(COMPONENT_DATA_KEY.to_string(), all.clone());
        }
        host.children = Self::render_nodes(tpl, &scope, comps);
        out.push(host);
    }
    
    /// 渲染 wx:for 循环
    fn render_for(node: &WxmlNode, data: &JsonValue, comps: &ComponentTemplates, out: &mut Vec<WxmlNode>) {
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
            
            Self::emit_element(node, &loop_data, comps, out);
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
