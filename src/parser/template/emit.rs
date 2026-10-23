//! 输出一个已通过条件判断的元素：`<block>` 透传、`<template is>` / `<include>` 展开、
//! 普通元素插值与归属标记。自定义组件的展开在 `component.rs`，`<slot>` 在 `slot.rs`。

use super::*;

impl TemplateEngine {
    /// 输出一个已经通过条件判断的元素。
    /// `<block>` / `<template>` / `<include>` 不产生真实节点，只展开内容；
    /// `<import>` / `<wxs>` 什么都不产生（`render_nodes` 一般已经跳过了它们，
    /// 这里兜住 `wx:for` 直接写在它们身上的情形）。
    pub(super) fn emit_element<'a>(
        node: &'a WxmlNode,
        data: &JsonValue,
        cx: &Cx<'a>,
        f: &Frame<'a, '_>,
        out: &mut Vec<WxmlNode>,
    ) {
        match node.tag_name.as_str() {
            "block" => {
                let children = Self::render_nodes(&node.children, data, cx, f);
                out.extend(children);
                return;
            }
            // 有 name 的是定义，不渲染
            "template" => {
                if !node.attributes.contains_key("name") {
                    Self::emit_template(node, data, cx, f, out);
                }
                return;
            }
            "include" => return Self::emit_include(node, cx, f, out),
            "import" | "wxs" => return,
            // 只有在组件模板里 `<slot>` 才是插槽；页面里写的 slot 按普通未知标签处理（保持旧行为）
            "slot" if f.slot.is_some() => return Self::emit_slot(node, data, cx, f, out),
            _ => {}
        }
        if let Some(tpl) = cx.comps.get(&node.tag_name) {
            Self::emit_component(node, tpl, data, cx, f, out);
            return;
        }
        
        let mut new_node = WxmlNode::new_element(&node.tag_name);
        // 收集模式只关心组件出现在哪，普通元素的属性不必插值
        if !cx.collecting() {
            for (key, value) in &node.attributes {
                if Self::is_directive(key) {
                    continue;
                }
                new_node.attributes.insert(key.clone(), Self::interpolate_attr(key, value, data));
            }
            Self::stamp_owner(&mut new_node, f.owner);
            // `this.animate` / applyAnimatedStyle 按选择器写的样式（作用域根 data 的 `__nodeStyles`，node_styles.rs）
            if let Some(m) = f.root.get("__nodeStyles").and_then(JsonValue::as_object).filter(|m| !m.is_empty()) {
                super::node_styles::apply(&mut new_node, m);
            }
            // scroll-view 的 `refresher-triggered` 绑定了哪些字段：展开后只剩求出来的值，
            // 宿主要靠字段名判断「页面有没有 setData 过它」（见 `bindings.rs`）
            if node.tag_name == "scroll-view" {
                if let Some(raw) = node.attributes.get("refresher-triggered").filter(|r| r.contains("{{")) {
                    new_node.attributes.insert(super::REFRESHER_KEYS_ATTR.to_string(), super::binding_roots(raw).join(","));
                }
            }
            // 带 wx:key 的列表项里的元素：给一个跟着 key 走的身份，渲染器用它当交互状态的键
            if let Some(k) = cx.keyed_identity(f.owner) {
                new_node.attributes.insert(super::for_loop::KEYED_ID_ATTR.to_string(), k);
            }
        }
        // 收集模式：带事件绑定的元素记进冒泡路径（组件 triggerEvent 的 bubbles 要用）
        let pushed = if cx.collecting() { cx.push_chain(f.owner, super::component::ChainFrame::of(node, data, false)) } else { false };
        new_node.children = Self::render_nodes(&node.children, data, cx, f);
        cx.pop_chain(pushed);
        out.push(new_node);
    }

    /// 打归属标记：组件模板里出来的节点记上实例键，事件据此派发给**这个实例**。
    ///
    /// 从前是展开完再整棵子树递归补打（已有标记的分支跳过），现在在输出每个元素时
    /// 按所在作用域直接打：slot 内容是在使用方的作用域里输出的，自然带使用方的归属
    /// （页面的 slot 内容不打 —— 页面归属就是空串），不会被组件「补打」成组件的。
    pub(super) fn stamp_owner(node: &mut WxmlNode, owner: &str) {
        if !owner.is_empty() {
            node.attributes.insert(COMPONENT_OWNER_ATTR.to_string(), owner.to_string());
        }
    }
}
