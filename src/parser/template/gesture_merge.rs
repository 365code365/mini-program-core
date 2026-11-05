//! 手势组件（`builtin_components/gesture.js`）是**虚组件**：官方说它「不会进行布局，真正响应事件的是其直接子节点」。
//!
//! 组件模板只有一层 `<view __mergechild="1" capture-bind:touchstart="__gh:<uid>:ts" …><slot/></view>`：
//! 展开时这一层不进树，它身上的绑定并到唯一的子节点（使用方写的那个）上 —— 父节点的 flex / 宽高百分比直接
//! 作用在子节点上，与微信一致。从前这一层留在树里，`<view style="display:flex">` 里包一个手势组件，
//! flex 项变成了包装层，子节点的 `flex:1` / `width:50%` 按包装层算。
//!
//! 绑定的处理函数名带实例（`__gh:<uid>:<方法>`），派发时由逻辑层按名字找手势实例（`page-components.js`
//! `__dispatchHandler`），不靠节点的组件归属 —— 子节点归使用方，它自己的 `bindtouchstart` 照常派给使用方。
//! 两层手势组件嵌在同一个子节点上时（官方示例二：horizontal-drag 包 vertical-drag 包 view），同一个键的
//! 处理函数用 `|` 连起来（内层在前），逻辑层按这个顺序做仲裁（`gesture.js`）。
//! 子节点自己已经写了同一个键（使用方的 `capture-bind:touchstart`）时换等价的写法（`capture-bindtouchstart`），
//! 两种写法都占了就丢掉手势这一条并告警。
use crate::parser::wxml::{WxmlNode, WxmlNodeType};

/// 模板根节点上的标记：带它的根节点并入子节点
pub(super) const MERGE_CHILD_ATTR: &str = "__mergechild";
/// 手势组件的处理函数名前缀（`page-components.js` 认它）
const HANDLER_PREFIX: &str = "__gh:";

/// 带 [`MERGE_CHILD_ATTR`] 的根节点 → 进树的节点。没有元素子节点时原样留下根节点（总得有个节点接触摸）
pub(super) fn merge_into_child(mut root: WxmlNode) -> Vec<WxmlNode> {
    root.attributes.remove(MERGE_CHILD_ATTR);
    let Some(i) = root.children.iter().position(|c| c.node_type == WxmlNodeType::Element) else {
        return vec![root];
    };
    let mut kids = std::mem::take(&mut root.children);
    let mut attrs: Vec<(&String, &String)> = root.attributes.iter().collect();
    attrs.sort();
    for (k, v) in attrs {
        if v.trim().is_empty() || !is_event_attr(k) {
            continue;
        }
        merge_attr(&mut kids[i], k, v);
    }
    // 官方：手势组件只能有一个直接子节点；空白文本不进树
    kids.retain(|c| c.node_type == WxmlNodeType::Element || !c.text_content.trim().is_empty());
    kids
}

fn is_event_attr(k: &str) -> bool {
    crate::renderer::components::parse_event_attr(k).is_some()
}

fn merge_attr(child: &mut WxmlNode, key: &str, val: &str) {
    // `capture-bind:touchstart` 与 `capture-bindtouchstart` 是同一条绑定的两种写法
    let alt = key.replacen(':', "", 1);
    for k in [key.to_string(), alt] {
        match child.attributes.get_mut(&k) {
            None => {
                child.attributes.insert(k, val.to_string());
                return;
            }
            Some(cur) if cur.starts_with(HANDLER_PREFIX) && val.starts_with(HANDLER_PREFIX) => {
                cur.push('|');
                cur.push_str(val);
                return;
            }
            Some(_) => continue,
        }
    }
    eprintln!("⚠️ 手势组件：子节点 <{}> 已经写了 {key}，手势的这条绑定放不上去", child.tag_name);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn el(tag: &str, attrs: &[(&str, &str)], children: Vec<WxmlNode>) -> WxmlNode {
        let mut n = WxmlNode::new_element(tag);
        for (k, v) in attrs {
            n.attributes.insert(k.to_string(), v.to_string());
        }
        n.children = children;
        n
    }

    #[test]
    fn 根节点并进唯一子节点_嵌套的连起来_使用方的绑定不动() {
        let user = el("view", &[("bindtouchstart", "mine"), ("class", "box")], vec![]);
        let inner = el("view", &[(MERGE_CHILD_ATTR, "1"), ("capture-bind:touchstart", "__gh:2:ts"), ("catch:__lockscroll", "")], vec![user]);
        let out = merge_into_child(inner);
        assert_eq!(out.len(), 1);
        let n = &out[0];
        assert_eq!(n.attributes.get("class").map(|s| s.as_str()), Some("box"));
        assert_eq!(n.attributes.get("bindtouchstart").map(|s| s.as_str()), Some("mine"));
        assert_eq!(n.attributes.get("capture-bind:touchstart").map(|s| s.as_str()), Some("__gh:2:ts"));
        assert!(!n.attributes.contains_key("catch:__lockscroll"), "空处理函数不并");
        let outer = el("view", &[(MERGE_CHILD_ATTR, "1"), ("capture-bind:touchstart", "__gh:1:ts")], out);
        let out = merge_into_child(outer);
        assert_eq!(out[0].attributes.get("capture-bind:touchstart").map(|s| s.as_str()), Some("__gh:2:ts|__gh:1:ts"), "内层在前");
        // 使用方占了这个键：换写法
        let user = el("view", &[("capture-bind:touchstart", "mine")], vec![]);
        let out = merge_into_child(el("view", &[(MERGE_CHILD_ATTR, "1"), ("capture-bind:touchstart", "__gh:3:ts")], vec![user]));
        assert_eq!(out[0].attributes.get("capture-bindtouchstart").map(|s| s.as_str()), Some("__gh:3:ts"));
        // 没有元素子节点：根节点留下
        let out = merge_into_child(el("view", &[(MERGE_CHILD_ATTR, "1")], vec![]));
        assert_eq!(out[0].tag_name, "view");
        assert!(!out[0].attributes.contains_key(MERGE_CHILD_ATTR));
    }
}
