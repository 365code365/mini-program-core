//! `<slot>`：组件模板里留给**使用方子节点**的位置。从前组件展开时直接丢掉使用方的子节点，
//! `<my-card><text>正文</text></my-card>` 里的正文永远不出现。
//!
//! 语义按官方文档（custom-component/wxml-wxss.html、glass-easel/dynamic-slots.html）与
//! glass-easel 运行时（`shadow_root.ts` 的 SlotMode）：
//!
//! - **默认（单一 slot）**：「一个组件的 wxml 中只能有一个 slot」。生效的是模板里**第一个**
//!   `<slot>`（不论它写没写 name），使用方的全部子节点都进它，子节点上的 `slot="x"` 不起作用
//!   —— glass-easel 单一模式下 `getContainingSlot` 对任何子节点都返回这唯一的 slot。
//! - **`options.multipleSlots: true`**：按 name 分配。`slot="x"` 的子节点进 `<slot name="x">`，
//!   没写 slot 的（含文本）进不带 name 的 `<slot>`；没有对应 slot 的内容不渲染。
//! - 两种都是「静态 slot」：同名 `<slot>` 重复时只有第一个生效，内容只出现一次 ——
//!   slot 写在 `wx:for` 里也不会被复制。
//! - **`options.dynamicSlots: true`**（glass-easel interaction/slot.md「动态 slot」）：`<slot>` 可以重复
//!   （写在 `wx:for` 里），每出现一次就把（同名的）内容再展开一次；`<slot>` 上除 `name` 外的属性是
//!   传给内容的数据（`list-index` → `listIndex`），内容节点用 `slot:item` 接收（变量名同字段名）、
//!   `slot:listIndex="index"` 改名接收。每一份里的子组件按「第几个 slot」各有各的实例。
//! - `<slot>` 自己的子节点不渲染：微信没有「slot 默认内容」，glass-easel 运行时也没有回退内容。
//! - 内容在**使用方**的作用域里求值（页面 data 或父组件 data，含使用方的 `wx:for` 变量），
//!   归属使用方（`bindtap` 派发给使用方，不是组件的同名方法），模板表也是使用方的。

use super::*;
use std::cell::RefCell;

/// 正在展开的组件实例给它的 `<slot>` 留的使用方信息
pub(super) struct SlotCx<'a, 'r> {
    /// 使用方写在组件标签里的子节点（未展开的源节点）
    children: &'a [WxmlNode],
    /// 使用方在组件标签处的作用域（含它的 `wx:for` 变量）
    data: &'r JsonValue,
    /// 使用方的作用域帧：模板表、数据根、归属、它自己的 slot（组件把 slot 再转交给子组件时要用）
    frame: &'r Frame<'a, 'r>,
    /// 使用方在组件标签处的结构位置：slot 里的子组件按使用方的位置算实例键
    path: Vec<Seg>,
    multiple: bool,
    /// 已经填过内容的 slot 名（单一模式下只用空串一个）
    used: RefCell<Vec<String>>,
    /// 组件标签：slot 内容的根节点记一下它是从哪个组件里穿过去的（样式隔离用）
    host_tag: &'r str,
    /// `options.dynamicSlots`
    dynamic: bool,
}

impl<'a, 'r> SlotCx<'a, 'r> {
    pub(super) fn new(
        children: &'a [WxmlNode],
        data: &'r JsonValue,
        frame: &'r Frame<'a, 'r>,
        path: Vec<Seg>,
        multiple: bool,
        host_tag: &'r str,
    ) -> Self {
        Self { children, data, frame, path, multiple, used: RefCell::new(Vec::new()), host_tag, dynamic: false }
    }

    pub(super) fn dynamic(mut self, on: bool) -> Self {
        self.dynamic = on;
        self
    }
}

/// slot 内容的根节点上的隐藏标记（值是穿过的组件标签）。布局据此让组件的作用域样式
/// （`标签 .x`）不命中 slot 内容 —— 微信的样式隔离里，组件 wxss 只管组件自己 wxml 的节点。
pub const SLOT_CONTENT_ATTR: &str = "__slot";

impl TemplateEngine {
    /// 输出一个 `<slot>`：在使用方的作用域里展开分到这里的子节点（slot 本身不产生节点）
    pub(super) fn emit_slot<'a>(
        node: &'a WxmlNode,
        data: &JsonValue,
        cx: &Cx<'a>,
        f: &Frame<'a, '_>,
        out: &mut Vec<WxmlNode>,
    ) {
        let Some(s) = f.slot else { return };
        if s.dynamic {
            return Self::emit_dynamic_slot(node, data, cx, s, out);
        }
        // 单一模式：第一个 <slot> 吃下全部内容，name 无所谓
        let name = if s.multiple {
            node.attributes
                .get("name")
                .map(|n| Self::interpolate(n, data).trim().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };
        {
            let mut used = s.used.borrow_mut();
            if used.contains(&name) {
                return; // 同名 slot 只有第一个生效
            }
            used.push(name.clone());
        }
        if s.children.is_empty() {
            return;
        }
        let mut p = s.path.clone();
        p.push(Seg::Slot);
        let saved = cx.swap_path(p);
        let kids = if s.multiple {
            let keep = |n: &WxmlNode| Self::slot_name_of(n, s.data) == name;
            Self::render_nodes_where(s.children, s.data, cx, s.frame, Some(&keep))
        } else {
            Self::render_nodes(s.children, s.data, cx, s.frame)
        };
        cx.swap_path(saved);
        for mut k in kids {
            if k.node_type == WxmlNodeType::Element && !cx.collecting() {
                k.attributes.insert(SLOT_CONTENT_ATTR.to_string(), s.host_tag.to_string());
            }
            out.push(k);
        }
    }

    /// 布局建祖先链时用：slot 内容的祖先链里，把它穿过的那个组件宿主的**标签**抹掉。
    ///
    /// 组件 wxss 被作用域化成 `标签 .x`（`using_components::scope_component_wxss`），而 slot
    /// 内容在节点树上正好在宿主节点里面 —— 不抹的话组件的 `.title` 会命中页面写进 slot 的
    /// `.title`。微信的样式隔离里组件 wxss 只管组件自己 wxml 里的节点（wxml-wxss.html）。
    /// 宿主的 id / class 保留：使用方用 `.宿主类 .x` 选 slot 内容是合法的；按标签名选组件
    /// 文档本来就不让用，所以抹掉标签不损失合法写法。
    /// 不是 slot 内容返回 None（调用方沿用原链，不多一次拷贝）。
    pub fn slot_content_ancestors(
        node: &WxmlNode,
        chain: &[crate::parser::wxss::ElementDesc],
    ) -> Option<Vec<crate::parser::wxss::ElementDesc>> {
        let host = node.attributes.get(SLOT_CONTENT_ATTR)?;
        let mut v = chain.to_vec();
        if let Some(d) = v.iter_mut().rev().find(|d| &d.tag == host) {
            d.tag.clear();
        }
        Some(v)
    }

    /// 动态 slot：每个 `<slot>` 都展开一份（同名的）内容，并把 slot 上的数据交给写了 `slot:x` 的内容节点
    fn emit_dynamic_slot<'a>(node: &'a WxmlNode, data: &JsonValue, cx: &Cx<'a>, s: &SlotCx<'a, '_>, out: &mut Vec<WxmlNode>) {
        let name = node.attributes.get("name").map(|n| Self::interpolate(n, data).trim().to_string()).unwrap_or_default();
        // 第几个 slot：进结构位置，各份内容里的子组件实例键不撞
        let nth = {
            let mut used = s.used.borrow_mut();
            used.push(name.clone());
            used.len() as u32 - 1
        };
        // slot 传出来的数据（在组件作用域里求值，带类型）
        let mut provided = serde_json::Map::new();
        for (k, v) in node.attrs_sorted() {
            if k == "name" || Self::is_directive(k) {
                continue;
            }
            provided.insert(super::component::camel_case(k), super::component::typed_attr(v, data));
        }
        let mut p = s.path.clone();
        p.push(Seg::Slot);
        p.push(Seg::Iter(nth));
        let saved = cx.swap_path(p);
        for (i, child) in s.children.iter().enumerate() {
            if Self::slot_name_of(child, s.data) != name {
                continue;
            }
            // `slot:item` / `slot:listIndex="index"`：把 slot 的数据放进这个内容节点的作用域
            let binds: Vec<(String, JsonValue)> = child
                .attributes
                .iter()
                .filter_map(|(k, v)| {
                    let field = k.strip_prefix("slot:")?;
                    let alias = if v.trim().is_empty() { field.to_string() } else { v.trim().to_string() };
                    // 字段名不分大小写（属性名在有的写法里会被小写化）
                    let val = provided.iter().find(|(k, _)| k.eq_ignore_ascii_case(field)).map(|(_, v)| v.clone());
                    Some((alias, val.unwrap_or(JsonValue::Null)))
                })
                .collect();
            let _at = cx.enter(Seg::Child(i as u32));
            let kids = if binds.is_empty() {
                Self::render_nodes(std::slice::from_ref(child), s.data, cx, s.frame)
            } else {
                let mut scoped = s.data.clone();
                if let Some(o) = scoped.as_object_mut() {
                    for (k, v) in binds {
                        o.insert(k, v);
                    }
                }
                Self::render_nodes(std::slice::from_ref(child), &scoped, cx, s.frame)
            };
            for mut k in kids {
                if k.node_type == WxmlNodeType::Element && !cx.collecting() {
                    k.attributes.insert(SLOT_CONTENT_ATTR.to_string(), s.host_tag.to_string());
                }
                out.push(k);
            }
        }
        cx.swap_path(saved);
    }

    /// 子节点要进哪个 slot（文本、没写 slot 的进默认 slot = 空串）
    fn slot_name_of(n: &WxmlNode, data: &JsonValue) -> String {
        if n.node_type != WxmlNodeType::Element {
            return String::new();
        }
        n.attributes
            .get("slot")
            .map(|v| Self::interpolate(v, data).trim().to_string())
            .unwrap_or_default()
    }
}
