//! `wx:for` 列表渲染（数组 / 对象取值 / 字符串逐字符），以及 `wx:key` 给出的项身份。
//!
//! ## `wx:key`
//! 官方 wxml/list.html：列表项的位置会动态改变或有新项加入、又希望项保持自己的特征与状态
//! （`<input>` 里的输入、`<switch>` 的选中、组件实例的 data）时，用 `wx:key` 指定唯一标识：
//! - `wx:key="id"`：项的 `id` 字段（字符串或数字）；
//! - `wx:key="*this"`：项本身（项是字符串或数字时）。
//!
//! 这里的实现是把项身份写进**结构位置**（`Seg::Key`）：组件实例键、以及带 key 的列表项里
//! 每个元素的稳定身份（隐藏属性 [`KEYED_ID_ATTR`]，渲染器拿它代替「按坐标」的交互状态键）
//! 都跟着 key 走 —— 列表重排、前面插入一项时，状态跟着项走，而不是留在原来的下标 / 坐标上。
//! 没写 `wx:key` 时仍按下标（微信同样如此，只是会在控制台提示）。
//!
//! key 取不到（字段缺失、是对象）的项退回下标；重复的 key 微信会警告，这里给重复者加序号，
//! 至少保证身份互不相撞。

use super::scope::{warn_once, Seg};
use super::*;
use std::collections::HashMap;
use std::rc::Rc;

/// 带 `wx:key` 的列表项里元素的稳定身份（隐藏属性，不进 dataset）
pub const KEYED_ID_ATTR: &str = "__key";

impl TemplateEngine {
    /// 渲染 wx:for 循环
    pub(super) fn render_for<'a>(
        node: &'a WxmlNode,
        data: &JsonValue,
        cx: &Cx<'a>,
        f: &Frame<'a, '_>,
        out: &mut Vec<WxmlNode>,
    ) {
        let for_expr = match node.attributes.get("wx:for") {
            Some(e) => e,
            None => return,
        };
        let item_name = node.attributes.get("wx:for-item").map(|s| s.as_str()).unwrap_or("item");
        let index_name = node.attributes.get("wx:for-index").map(|s| s.as_str()).unwrap_or("index");
        let key_attr = node.attributes.get("wx:key").map(|s| s.trim()).filter(|s| !s.is_empty());
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
        let mut seen: HashMap<String, u32> = HashMap::new();
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
            // 第几项进结构位置：列表里的组件按 key（没写 key 时按下标）各有各的实例
            let seg = match key_attr.and_then(|k| item_key(k, item, &loop_data)) {
                Some(k) => {
                    let n = seen.entry(k.clone()).or_insert(0);
                    *n += 1;
                    if *n > 1 {
                        warn_once(format!("wx:key=\"{}\" 出现重复值 {k:?}：列表项身份会按出现次序区分", key_attr.unwrap_or("")));
                        Seg::Key(Rc::from(format!("{k}~{n}")))
                    } else {
                        Seg::Key(Rc::from(k))
                    }
                }
                None => Seg::Iter(index as u32),
            };
            let _at = cx.enter(seg);
            Self::emit_element(node, &loop_data, cx, f, out);
        }
    }
}

/// 一项的 key：`*this` 取项本身，否则取项的同名字段；写成 `{{表达式}}` 的按表达式求值
/// （不是官方写法，但框架产物里见得到）。只认字符串与数字
fn item_key(k: &str, item: &JsonValue, scope: &JsonValue) -> Option<String> {
    let v = if k == "*this" {
        item.clone()
    } else if k.contains("{{") {
        TemplateEngine::eval_value(k, scope)
    } else {
        item.get(k).cloned().unwrap_or(JsonValue::Null)
    };
    match v {
        JsonValue::String(s) => Some(s),
        JsonValue::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// 树里有没有写 `wx:key` 的节点（决定这次渲染要不要记结构位置）
pub(super) fn has_wx_key(nodes: &[WxmlNode]) -> bool {
    nodes.iter().any(|n| {
        n.node_type == WxmlNodeType::Element
            && n.tag_name != "wxs"
            && (n.attributes.contains_key("wx:key") || has_wx_key(&n.children))
    })
}
