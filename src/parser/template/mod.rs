//! 模板引擎 - 处理数据绑定、条件渲染、列表渲染
//!
//! 表达式求值统一委托给 `super::expr`（完整的 `{{}}` 表达式引擎），
//! 本模块只负责 WXML 控制流：`wx:if / wx:elif / wx:else`、`wx:for`、
//! `<block>` 透传，`<template is>` / `<include>` 展开，以及文本/属性插值。
//! 输出是一棵**不含任何指令**的节点树（没有 block / template / import / include / wxs）。
//!
//! 按职责分片：`emit`（输出单个元素 / 展开自定义组件）、`for_loop`（`wx:for`）、
//! `interp`（文本与属性插值、条件求值）、`scope`（模板表与作用域）、
//! `fragment`（`<template is>` 与 `<include>` 的展开）。
//! import / include 的**文件**解析在加载阶段做，见 `parser::wxml_file`。

use super::wxml::{WxmlNode, WxmlNodeType};
use super::expr;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// 自定义组件的展开：实例键、带类型的属性、出现位置的收集
mod component;
/// 输出单个元素
mod emit;
/// `wx:for` 列表渲染
mod for_loop;
/// `<slot>`：把使用方的子节点放进组件模板
mod slot;
/// `<template is>` 与 `<include>` 的展开
mod fragment;
/// 文本/属性插值与条件求值
mod interp;
/// 模板表（哪个 `is` 看得见哪些 `name`）、展开深度与总量上限
mod scope;
/// 每个 WXML 文件里定义的 wxs 模块（`{{m.f(x)}}` 的求值作用域）
mod wxs_scope;
/// 展开时记下的绑定信息（`refresher-triggered` 绑定了哪些字段）
mod bindings;
/// `this.animate` / applyAnimatedStyle 按选择器写给节点的样式（data 的 `__nodeStyles`）
mod node_styles;
pub use bindings::{binding_roots, INTO_VIEW_KEYS_ATTR, REFRESHER_KEYS_ATTR};
#[cfg(test)]
mod tests;

use scope::{Cx, Frame, Seg};

pub use component::{camel_case, instance_key, is_property_attr, Occurrence};
pub use slot::SLOT_CONTENT_ATTR;
pub use wxs_scope::{bundle_of as wxs_bundle_of, WXS_BUNDLE_ATTR};
pub use for_loop::KEYED_ID_ATTR;
/// 哪些 dataset 键的值是绑定求出来的字符串（逗号分隔，隐藏属性）：派发时保持字符串，不按字面量还原
/// （emit.rs 写、`components::base::event_attr` 带进事件、逻辑层 `__coerceDataset` 认）
pub const DATASET_STR_ATTR: &str = "__ds_s";

/// 自定义组件模板表：标签名 → 组件 WXML 根节点
pub type ComponentTemplates = HashMap<String, Vec<WxmlNode>>;

/// 组件实例数据在渲染数据里的存放位置：`data["$comp"]["<实例键>"]`（实例键见
/// [`instance_key`]；老式单实例挂载按标签名存，展开时作为后备）。
/// 用 `$` 开头是因为 WXML 表达式里不会出现这样的标识符，不会和页面字段撞名。
pub const COMPONENT_DATA_KEY: &str = "$comp";

/// 组件选项在渲染数据里的位置：`data["$copt"]["<标签名>"] = { multipleSlots: true }`。
/// slot 怎么分配要在**展开时**就知道，而 `options` 写在组件 js 里，只能由逻辑层带过来。
pub const COMPONENT_OPTIONS_KEY: &str = "$copt";

/// 组件宿主节点上的实例键（隐藏属性，不进 dataset）
pub const COMPONENT_INSTANCE_ATTR: &str = "__inst";

/// `options.virtualHost` 的组件不出宿主节点，它模板的第一层节点直接进使用方的树
/// （glass-easel styling/virtual_host.md；宿主上的 class / style 不生效）。这些节点打上这个隐藏属性，
/// 值是被虚化掉的组件标签（由外向内、空格分隔）：布局建祖先链时补回这几层，组件的作用域样式
/// （`标签 .x`）照样命中
pub const VIRTUAL_HOST_ATTR: &str = "__vhost";

/// 组件带的内置表单 behavior（`__ff`，空格分隔的 `field` / `group` / `button`，对应
/// `wx://form-field` / `wx://form-field-group` / `wx://form-field-button`）。form 据此决定
/// 收不收组件的值、看不看得见组件内部的控件与按钮（`renderer::wxml_renderer::form`）
pub const FORM_FIELD_ATTR: &str = "__ff";

/// 展开自定义组件时打在子树上的归属标记（属性名，值是组件**实例键**）。
///
/// 事件必须知道自己是**哪个组件模板**里声明的：组件模板里的 `bindtap="{{item.e}}"`
/// 绑的是组件实例的方法，而 uni-app 给每个组件/页面各自生成 `e0_0`、`e1_1` 这种
/// 短名字 —— 页面和组件很容易撞名。只按名字在页面上先找一遍的话，点底部导航会
/// 执行到**页面**的同名方法上（tea-app 上的表现：点任何一个 tab 都跳到 AI 结果页）。
///
/// 值是实例键而不是标签名：同一个组件用两次，第二个里的点击必须落到第二个实例上。
/// slot 内容不打（或打使用方的键）—— 它属于使用方，`bindtap` 绑的是使用方的方法。
///
/// 不用 `data-` 前缀：那会进 `dataset`，页面里就能看见这个内部标记。
pub const COMPONENT_OWNER_ATTR: &str = "__comp";

/// 模板引擎
pub struct TemplateEngine;

impl TemplateEngine {
    /// 渲染模板，替换 {{}} 表达式
    pub fn render(nodes: &[WxmlNode], data: &JsonValue) -> Vec<WxmlNode> {
        Self::render_root(nodes, data, &ComponentTemplates::new())
    }
    
    /// 渲染模板（保留 viewport 形参以兼容旧调用；虚拟列表裁剪在绘制阶段处理）
    pub fn render_with_virtual_list(
        nodes: &[WxmlNode],
        data: &JsonValue,
        _viewport: Option<(f32, f32)>,
    ) -> Vec<WxmlNode> {
        Self::render_root(nodes, data, &ComponentTemplates::new())
    }

    /// 渲染模板，并把 `usingComponents` 声明的自定义组件标签展开成组件自己的模板。
    ///
    /// 组件子树用**这处使用的实例自己的 data** 求值（放在 `data["$comp"][实例键]`，
    /// 找不到时按标签名兜底，见 [`COMPONENT_DATA_KEY`]）——
    /// 组件的 `{{a}}` 与页面的 `{{a}}` 是两套东西，共用一个作用域会互相串味。
    pub fn render_with_components(
        nodes: &[WxmlNode],
        data: &JsonValue,
        components: &ComponentTemplates,
    ) -> Vec<WxmlNode> {
        Self::render_root(nodes, data, components)
    }

    /// 收集这一帧里**每一处**自定义组件的出现（实例键、带类型的属性、宿主上的事件绑定），
    /// 按文档顺序（父组件在前）。宿主拿它去逻辑层对账：建实例 / 更新属性 / detached。
    ///
    /// 走的就是渲染那一套展开代码（只是不做普通元素的插值），所以实例键与真正渲染时
    /// 算出来的**一定相同** —— 两套代码各算各的，迟早会对不上。
    /// 还没有实例 data 的组件不往里展开：它的子组件等它建出来、有了 data 再收，
    /// 否则子组件会先拿一份「父组件空 data 下」的属性建出来，下一轮再改一次。
    pub fn collect_components(
        nodes: &[WxmlNode],
        data: &JsonValue,
        components: &ComponentTemplates,
    ) -> Vec<Occurrence> {
        if components.is_empty() {
            return Vec::new();
        }
        let mut cx = Cx::new(components, data);
        cx.track_keys_in(nodes);
        cx.collect = Some(std::cell::RefCell::new(Vec::new()));
        let f = Self::root_frame(nodes, data, &cx);
        let _ = Self::render_nodes(nodes, data, &cx, &f);
        cx.collect.take().map(|c| c.into_inner()).unwrap_or_default()
    }

    /// 一次渲染的入口：`nodes` 是一个文件的顶层（作用域根），`data` 是它的数据根
    fn render_root(nodes: &[WxmlNode], data: &JsonValue, comps: &ComponentTemplates) -> Vec<WxmlNode> {
        let mut cx = Cx::new(comps, data);
        cx.track_keys_in(nodes);
        let f = Self::root_frame(nodes, data, &cx);
        Self::render_nodes(nodes, data, &cx, &f)
    }

    fn root_frame<'a, 'r>(nodes: &'a [WxmlNode], data: &'r JsonValue, cx: &Cx<'a>) -> Frame<'a, 'r> {
        Frame {
            table: cx.table_of(nodes),
            root: data,
            depth: 0,
            owner: "",
            slot: None,
            cdepth: 0,
            wxs: cx.wxs_of(nodes),
        }
    }
    
    /// 渲染一组兄弟节点，处理 if/elif/else 链
    fn render_nodes<'a>(
        nodes: &'a [WxmlNode],
        data: &JsonValue,
        cx: &Cx<'a>,
        f: &Frame<'a, '_>,
    ) -> Vec<WxmlNode> {
        Self::render_nodes_where(nodes, data, cx, f, None)
    }

    /// 同上，但只输出 `keep` 认可的节点（slot 分配用）。条件链照常按**全部**兄弟求值：
    /// `wx:if` 落在 A slot、`wx:else` 落在 B slot 时，B 是否出现仍取决于 A 的条件。
    fn render_nodes_where<'a>(
        nodes: &'a [WxmlNode],
        data: &JsonValue,
        cx: &Cx<'a>,
        f: &Frame<'a, '_>,
        keep: Option<&dyn Fn(&WxmlNode) -> bool>,
    ) -> Vec<WxmlNode> {
        let mut out = Vec::new();
        // 这组兄弟节点里的表达式按**这个帧所在文件**的 wxs 模块求值（slot 内容是使用方的文件、
        // 模板体是定义模板的文件）。整个渲染一个模块都没见过时什么都不做
        let _wxs = cx
            .wxs_seen
            .get()
            .then(|| crate::parser::expr::wxs::enter(f.wxs.clone()));
        // if/elif/else 链状态
        let mut chain_active = false; // 当前是否处于一个 wx:if 链中
        let mut chain_taken = false;  // 链中是否已有分支命中
        let kept = |n: &WxmlNode| keep.map_or(true, |k| k(n));
        
        for (i, node) in nodes.iter().enumerate() {
            // 结构位置只在有组件时才记（见 `Cx::enter`）
            let _at = cx.enter(Seg::Child(i as u32));
            match node.node_type {
                WxmlNodeType::Comment => { /* 丢弃注释 */ }
                WxmlNodeType::Text => {
                    chain_active = false;
                    if kept(node) && !cx.collecting() {
                        let text = Self::interpolate(&node.text_content, data);
                        out.push(WxmlNode::new_text(&text));
                    }
                }
                WxmlNodeType::Element => {
                    // 模板定义 / import / wxs：编译期的东西，不出节点，也不打断条件链
                    if Self::is_declaration(node) {
                        continue;
                    }
                    // wx:for 优先级最高
                    if node.attributes.contains_key("wx:for") {
                        chain_active = false;
                        if kept(node) {
                            Self::render_for(node, data, cx, f, &mut out);
                        }
                        continue;
                    }
                    
                    if let Some(cond) = node.attributes.get("wx:if") {
                        chain_active = true;
                        chain_taken = Self::eval_condition(cond, data);
                        if chain_taken && kept(node) {
                            Self::emit_element(node, data, cx, f, &mut out);
                        }
                    } else if let Some(cond) = node.attributes.get("wx:elif") {
                        if chain_active && !chain_taken && Self::eval_condition(cond, data) {
                            chain_taken = true;
                            if kept(node) {
                                Self::emit_element(node, data, cx, f, &mut out);
                            }
                        }
                    } else if node.attributes.contains_key("wx:else") {
                        if chain_active && !chain_taken && kept(node) {
                            Self::emit_element(node, data, cx, f, &mut out);
                        }
                        chain_active = false;
                        chain_taken = false;
                    } else {
                        chain_active = false;
                        if kept(node) {
                            Self::emit_element(node, data, cx, f, &mut out);
                        }
                    }
                }
            }
        }
        
        out
    }
}
