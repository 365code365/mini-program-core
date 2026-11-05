//! 模板阶段生成孩子的两个组件：
//!
//! - Skyline 的 `<list-builder>` / `<grid-builder>`（官方 component/list-builder、grid-builder）：
//!   `<list-builder list="{{list}}"><view slot:item="item" slot:index="index">…</view></list-builder>` ——
//!   写了 `slot:item` / `slot:index` 的子节点是**每一项的模板**，按 `list` 逐项展开（`child-count` 给了就展开这么多项，
//!   超出 `list` 的项 item 为 null）。`slot:item` 不写值时变量名就是 `item` / `index`（官方示例 `<view slot:item slot:index>`）。
//!   一个子节点都没写 `slot:` 时，全部元素子节点一起当模板。每一项进结构位置（`Seg::Iter`），项里的组件各有各的实例；
//! - `<rich-text>`：`nodes` 转成真正的子树（components/rich_text/tree.rs）。`nodes="{{expr}}"` 直接求出 JSON 值
//!   （不经过字符串插值：插值把对象写成单引号 JSON，文字里带撇号就解析不回来）。
use super::scope::Seg;
use super::*;

impl TemplateEngine {
    /// 构造器的孩子：按 list 逐项展开模板。返回 (孩子, 每项几个根节点)
    pub(super) fn render_builder<'a>(node: &'a WxmlNode, data: &JsonValue, cx: &Cx<'a>, f: &Frame<'a, '_>) -> (Vec<WxmlNode>, usize) {
        let list = node.attributes.get("list").map(|raw| Self::eval_value(raw, data)).unwrap_or(JsonValue::Null);
        let items: &[JsonValue] = list.as_array().map(|a| a.as_slice()).unwrap_or(&[]);
        let count = node
            .attributes
            .get("child-count")
            .map(|raw| Self::eval_value(raw, data))
            .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok())))
            .map(|n| n.max(0.0) as usize)
            .unwrap_or(items.len());
        let slotted: Vec<&WxmlNode> = node
            .children
            .iter()
            .filter(|c| c.node_type == WxmlNodeType::Element && c.attributes.keys().any(|k| k.starts_with("slot:")))
            .collect();
        let templates: Vec<&WxmlNode> = if slotted.is_empty() {
            node.children.iter().filter(|c| c.node_type == WxmlNodeType::Element).collect()
        } else {
            slotted
        };
        if templates.is_empty() {
            return (Vec::new(), 1);
        }
        // 每个模板的变量名（`slot:item="x"` → x；没写值 → item）
        let names: Vec<(String, String)> = templates
            .iter()
            .map(|t| {
                let alias = |k: &str, dft: &str| {
                    t.attributes
                        .get(k)
                        .map(|v| v.trim())
                        .filter(|v| !v.is_empty() && *v != "true")
                        .unwrap_or(dft)
                        .to_string()
                };
                (alias("slot:item", "item"), alias("slot:index", "index"))
            })
            .collect();
        // 作用域只拷一次，每项只改写 item / index 两个键。从前每项 `data.clone()` 一份，
        // 而 data 里就装着 list 本身：200 项是 4 万个对象的深拷贝，真机（jemalloc 带锁）一次 setData 展开 ~32ms。
        // 几个模板的变量名不同时，每个模板渲染完把它的两个键还原，与「每个模板一份新作用域」的结果一致
        let mut scoped = data.clone();
        let originals: Vec<(Option<JsonValue>, Option<JsonValue>)> = match (templates.len() > 1, data.as_object()) {
            (true, Some(o)) => names.iter().map(|(a, b)| (o.get(a).cloned(), o.get(b).cloned())).collect(),
            _ => Vec::new(),
        };
        let mut out = Vec::new();
        for i in 0..count {
            let item = items.get(i);
            let _at = cx.enter(Seg::Iter(i as u32));
            for (k, t) in templates.iter().enumerate() {
                if let Some(o) = scoped.as_object_mut() {
                    o.insert(names[k].0.clone(), item.cloned().unwrap_or(JsonValue::Null));
                    o.insert(names[k].1.clone(), JsonValue::Number(i.into()));
                }
                let _t = cx.enter(Seg::Child(k as u32));
                out.extend(Self::render_nodes(std::slice::from_ref(*t), &scoped, cx, f));
                if let (Some((a0, b0)), Some(o)) = (originals.get(k), scoped.as_object_mut()) {
                    for (name, orig) in [(&names[k].0, a0), (&names[k].1, b0)] {
                        match orig {
                            Some(v) => o.insert(name.clone(), v.clone()),
                            None => o.remove(name),
                        };
                    }
                }
            }
        }
        (out, templates.len())
    }

    /// rich-text：`nodes` → 子树（挂在 `new_node` 下，打上与 rich-text 同样的组件归属：
    /// 自定义组件里的 rich-text 只吃这个组件的 WXSS，官方 rich-text tip）
    pub(super) fn expand_rich_text(node: &WxmlNode, data: &JsonValue, owner: &str, new_node: &mut WxmlNode) {
        let Some(raw) = node.attributes.get("nodes") else { return };
        let t = raw.trim();
        let value = match t.strip_prefix("{{").and_then(|s| s.strip_suffix("}}")).filter(|i| !i.contains("{{")) {
            Some(inner) => expr::eval_str(inner.trim(), data),
            None => JsonValue::String(Self::interpolate(raw, data)),
        };
        let space = new_node.attributes.get("space").cloned();
        let mut kids = crate::renderer::components::rich_text_expand(&value, space.as_deref());
        if kids.is_empty() {
            return;
        }
        fn stamp(n: &mut WxmlNode, owner: &str) {
            if n.node_type == WxmlNodeType::Element {
                TemplateEngine::stamp_owner(n, owner);
                for c in n.children.iter_mut() {
                    stamp(c, owner);
                }
            }
        }
        for k in kids.iter_mut() {
            stamp(k, owner);
        }
        // `nodes` 原串留在节点上：编译 H5 的那条路（compiler/html/transpile/inner.rs）按它出 HTML。
        // 增量布局比较节点时会多比一次这个串（memcmp，几万字也是微秒级）
        new_node.children = kids;
    }
}
