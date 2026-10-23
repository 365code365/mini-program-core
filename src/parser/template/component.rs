//! 自定义组件的展开：**每一处使用一个实例**。
//!
//! 从前是「一个标签一个实例」：属性只取第一处使用、只在挂载时求一次值，组件模板统一用
//! `data["$comp"][标签名]` 渲染 —— 同一个组件用两次就显示成两份一样的内容，列表里的组件
//! 全拿第一项的数据，页面 setData 之后属性也不会变。微信里每个组件节点都是独立实例
//! （custom-component/component.html），这里按「出现位置」给每处使用一个实例键：
//!
//! - 渲染时按键取实例 data（`$comp[键]`）展开组件模板；
//! - 收集模式（`TemplateEngine::collect_components`）把每处使用的键、**带类型**的属性、
//!   宿主上的事件绑定列出来，宿主拿去逻辑层对账（建实例、改属性、detached）。
//!
//! 两条路径共用这一份展开代码，所以两边算出的键必然一致。

use super::scope::{warn_once, MAX_COMPONENT_DEPTH};
use super::slot::SlotCx;
use super::*;
use serde::Serialize;

/// 一处组件使用（宿主对账的输入，序列化后交给逻辑层 `__reconcileComponents`）
#[derive(Debug, Clone, Default, Serialize)]
pub struct Occurrence {
    /// 实例键，见 [`instance_key`]
    pub key: String,
    /// 组件标签（逻辑层按页面登记的「标签 → 组件路径」找定义）
    pub tag: String,
    /// 使用方的实例键（页面为空串）：`triggerEvent` 派发给它，`selectComponent` 也按它分组
    pub owner: String,
    /// 属性：已求值且带类型（`count="{{3}}"` 是数字 3，`title="A"` 是字符串），属性名转驼峰。
    /// 按组件 `properties` 声明做类型转换在逻辑层做（定义在 JS 里）
    pub props: serde_json::Map<String, JsonValue>,
    /// 宿主节点的 `id`（`selectComponent('#x')`、`triggerEvent` 的 `target.id`）
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    /// 宿主节点的 class（`selectComponent('.x')`）
    #[serde(skip_serializing_if = "String::is_empty")]
    pub class: String,
    /// 宿主节点的 `data-*`（键转驼峰，绑定保留类型）：`triggerEvent` 事件对象的 dataset
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
    pub dataset: serde_json::Map<String, JsonValue>,
    /// 宿主节点上的事件绑定：事件名 → 处理函数名数组（捕获阶段的在前）
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
    pub events: serde_json::Map<String, JsonValue>,
    /// `triggerEvent(…, {bubbles})` 的冒泡路径：宿主节点自己在第一个，之后是它在**使用方模板**里的
    /// 祖先（由内向外，只收写了事件绑定的）。slot 内容的祖先链按使用方的模板算（含它所在的那个组件宿主）
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub chain: Vec<ChainFrame>,
}

/// 冒泡路径上的一个节点：事件对象的 `currentTarget`（id / dataset）与它身上的绑定
#[derive(Debug, Clone, Default, Serialize)]
pub struct ChainFrame {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
    pub dataset: serde_json::Map<String, JsonValue>,
    /// 事件名 → `[[处理函数, 标记], …]`，标记：`b` 冒泡 / `c` catch / `C` 捕获 / `X` 捕获并截断 / `m` 互斥
    pub ev: serde_json::Map<String, JsonValue>,
}

impl ChainFrame {
    /// 节点上的绑定与身份（在节点所在的作用域里求值）。没有任何事件绑定时给 None
    pub(super) fn of(node: &WxmlNode, data: &JsonValue, always: bool) -> Option<Self> {
        let mut fr = ChainFrame::default();
        for (k, v) in node.attrs_sorted() {
            if TemplateEngine::is_directive(k) {
                continue;
            }
            if let Some(name) = k.strip_prefix("data-") {
                fr.dataset.insert(camel_case(name), typed_attr(v, data));
            } else if k == "id" {
                fr.id = TemplateEngine::interpolate(v, data);
            } else if let Some((ev, is_catch, phase, mut_bind)) = crate::renderer::components::parse_event_attr(k) {
                let handler = TemplateEngine::interpolate(v, data).trim().to_string();
                if handler.is_empty() {
                    continue;
                }
                let capture = phase == crate::renderer::components::EventPhase::Capture;
                let flag = match (capture, is_catch, mut_bind) {
                    (true, true, _) => "X",
                    (true, false, _) => "C",
                    (false, true, _) => "c",
                    (false, false, true) => "m",
                    _ => "b",
                };
                let entry = fr.ev.entry(ev.to_string()).or_insert_with(|| JsonValue::Array(Vec::new()));
                if let Some(a) = entry.as_array_mut() {
                    a.push(serde_json::json!([handler, flag]));
                }
            }
        }
        (always || !fr.ev.is_empty()).then_some(fr)
    }
}

/// 实例键：`<使用方实例键>/<标签>@<结构位置>`。页面里的是 `/my-card@0.2[1]`，
/// 它模板里的子组件是 `/my-card@0.2[1]/my-tag@0.0`，页面写进它 slot 里的是 `/x@0.2[1].s.0`。
///
/// 结构位置是节点在**使用方模板**里的下标路径（含 `wx:for` 的第几项），而不是「第几个
/// 同名组件」：前面兄弟的 `wx:if` 翻转不会让后面的组件换实例；列表按下标复用实例，
/// 与微信在没有 `wx:key` 时的行为一致。支持 `wx:key` 时只需要换这一个函数。
pub fn instance_key(owner: &str, tag: &str, site: &str) -> String {
    format!("{owner}/{tag}@{site}")
}

impl TemplateEngine {
    /// 展开一个自定义组件：保留一个以组件标签命名的**宿主节点**（承载使用方写在标签上的
    /// class/style，并让组件 WXSS 里的 `:host` 与作用域前缀能命中），
    /// 其内容用**这个实例**自己的 data 渲染。
    ///
    /// 组件模板是一个独立的作用域根：它的 `<template is>` 只查组件自己文件（及其 import）
    /// 里的定义，看不见页面的模板 —— 两边各写一个同名模板是常见的事。
    pub(super) fn emit_component<'a>(
        node: &'a WxmlNode,
        tpl: &'a [WxmlNode],
        data: &JsonValue,
        cx: &Cx<'a>,
        f: &Frame<'a, '_>,
        out: &mut Vec<WxmlNode>,
    ) {
        let tag = node.tag_name.as_str();
        let key = instance_key(f.owner, tag, &cx.site());
        let inst = cx.instance_data(&key, tag);
        let mut host = WxmlNode::new_element(tag);
        // 收集模式：这个宿主节点也是它子树（slot 内容）在使用方模板里的祖先
        let host_frame = if cx.collecting() { ChainFrame::of(node, data, false) } else { None };
        if let Some(sink) = &cx.collect {
            let mut occ = Self::occurrence(node, data, &key, f.owner);
            occ.chain = cx.chain_for(f.owner, ChainFrame::of(node, data, true));
            sink.borrow_mut().push(occ);
            // 还没建出实例的组件不往里收（见 `collect_components`）
            if inst.is_none() {
                out.push(host);
                return;
            }
        } else {
            for (k, v) in &node.attributes {
                if Self::is_directive(k) {
                    continue;
                }
                host.attributes.insert(k.clone(), Self::interpolate_attr(k, v, data));
            }
            host.attributes.insert(COMPONENT_INSTANCE_ATTR.to_string(), key.clone());
            let ff: Vec<&str> = [("formField", "field"), ("formFieldGroup", "group"), ("formFieldButton", "button")]
                .iter()
                .filter(|(opt, _)| cx.comp_option(tag, opt))
                .map(|(_, v)| *v)
                .collect();
            if !ff.is_empty() {
                host.attributes.insert(FORM_FIELD_ATTR.to_string(), ff.join(" "));
            }
            // 宿主节点属于使用方：它身上的 `bindtap` 是使用方的方法
            Self::stamp_owner(&mut host, f.owner);
        }
        if f.cdepth >= MAX_COMPONENT_DEPTH {
            warn_once(format!("<{tag}>：组件嵌套超过 {MAX_COMPONENT_DEPTH} 层，疑似无终止的自引用，已截断"));
            out.push(host);
            return;
        }
        // 没有实例 data（逻辑层还没建出来）时用空对象渲染：模板里的静态结构照样出来
        let empty = JsonValue::Object(Default::default());
        let scope: &JsonValue = inst.unwrap_or(&empty);
        let slot = SlotCx::new(&node.children, data, f, cx.path_snapshot(), cx.multiple_slots(tag), tag)
            .dynamic(cx.comp_option(tag, "dynamicSlots"));
        let inner = Frame {
            table: cx.table_of(tpl),
            root: scope,
            depth: f.depth,
            owner: &key,
            slot: Some(&slot),
            cdepth: f.cdepth + 1,
            // 组件模板是它自己的 WXML 文件：只看得见组件文件里定义的 wxs 模块
            wxs: cx.wxs_of(tpl),
        };
        // 组件模板里的结构位置从零记起（键前缀已经是本实例的键）
        let saved = cx.swap_path(Vec::new());
        let pushed = cx.push_chain(f.owner, host_frame);
        host.children = Self::render_nodes(tpl, scope, cx, &inner);
        cx.pop_chain(pushed);
        cx.swap_path(saved);
        if !cx.collecting() && cx.comp_option(tag, "virtualHost") {
            // 引擎内置组件（movable-view）的 `__mergeHost`：模板的根节点**就是**这个组件的节点 ——
            // 标签、使用方写的 class / style / id / data-* 都并到它身上（微信里它是一个原生节点，页面 CSS
            // 的宽高、背景、子项对齐直接作用在装内容的那一层；分成宿主 + 内层两层的话内层的 flex 对齐就丢了）
            let mut merge = cx.comp_option(tag, "__mergeHost").then(|| std::mem::take(&mut host.attributes));
            for mut c in host.children {
                if c.node_type == WxmlNodeType::Element {
                    if let Some(h) = merge.take() {
                        Self::merge_host(tag, &h, &mut c);
                        out.push(c);
                        continue;
                    }
                }
                if c.node_type == WxmlNodeType::Element {
                    let v = match c.attributes.get(VIRTUAL_HOST_ATTR) {
                        Some(inner) => format!("{tag} {inner}"),
                        None => tag.to_string(),
                    };
                    c.attributes.insert(VIRTUAL_HOST_ATTR.to_string(), v);
                }
                out.push(c);
            }
            return;
        }
        out.push(host);
    }

    /// `__mergeHost`：把宿主的标签与使用方写的 class / style / id / data-* 并到模板根节点上。
    /// 事件绑定不并：根节点归组件实例（它自己的 touch 处理函数），使用方的 bind 要派给使用方
    fn merge_host(tag: &str, host: &HashMap<String, String>, c: &mut WxmlNode) {
        c.tag_name = tag.to_string();
        let join = |a: Option<&String>, b: Option<&String>, sep: &str| {
            let (a, b) = (a.map(|s| s.trim()).unwrap_or(""), b.map(|s| s.trim()).unwrap_or(""));
            match (a.is_empty(), b.is_empty()) {
                (true, _) => b.to_string(),
                (_, true) => a.to_string(),
                _ => format!("{}{sep}{b}", a.trim_end_matches(';')),
            }
        };
        let class = join(host.get("class"), c.attributes.get("class"), " ");
        // 组件自己的 style（位置）在后：同名属性组件的赢
        let style = join(host.get("style"), c.attributes.get("style"), ";");
        c.attributes.insert("class".into(), class);
        c.attributes.insert("style".into(), style);
        for (k, v) in host {
            if k == "id" || k.starts_with("data-") {
                c.attributes.insert(k.clone(), v.clone());
            }
        }
    }

    /// 一处使用的对账信息：在**使用方的作用域**里求值（含使用方的 `wx:for` 变量）
    fn occurrence(node: &WxmlNode, data: &JsonValue, key: &str, owner: &str) -> Occurrence {
        let mut o = Occurrence {
            key: key.to_string(),
            tag: node.tag_name.clone(),
            owner: owner.to_string(),
            ..Default::default()
        };
        // 捕获阶段的处理函数先于冒泡阶段（与触摸事件的派发顺序一致）
        let mut capture: Vec<(String, String)> = Vec::new();
        let mut bubble: Vec<(String, String)> = Vec::new();
        for (k, v) in node.attrs_sorted() {
            if Self::is_directive(k) {
                continue;
            }
            if let Some(name) = k.strip_prefix("data-") {
                o.dataset.insert(camel_case(name), typed_attr(v, data));
            } else if let Some((ev, _, phase, _)) = crate::renderer::components::parse_event_attr(k) {
                let handler = Self::interpolate(v, data).trim().to_string();
                if handler.is_empty() {
                    continue;
                }
                let list = if phase == crate::renderer::components::EventPhase::Capture { &mut capture } else { &mut bubble };
                list.push((ev.to_string(), handler));
            } else if k == "id" {
                o.id = Self::interpolate(v, data);
            } else if k == "class" {
                o.class = Self::interpolate_attr(k, v, data);
            } else if is_property_attr(k) {
                o.props.insert(camel_case(k), typed_attr(v, data));
            }
        }
        for (ev, handler) in capture.into_iter().chain(bubble) {
            let entry = o.events.entry(ev).or_insert_with(|| JsonValue::Array(Vec::new()));
            if let Some(a) = entry.as_array_mut() {
                a.push(JsonValue::String(handler));
            }
        }
        o
    }
}

/// 属性值求成**带类型**的 JSON：整个值就是一个 `{{表达式}}` 时取表达式的值
/// （数字还是数字、对象还是对象 —— 微信的属性传递就是这样，组件里 `count + 1` 不能变成字符串拼接）；
/// 混着文本的插值与纯文本都是字符串。
pub(super) fn typed_attr(raw: &str, data: &JsonValue) -> JsonValue {
    let t = raw.trim();
    if let Some(inner) = t.strip_prefix("{{").and_then(|s| s.strip_suffix("}}")) {
        if !inner.contains("{{") && !inner.contains("}}") {
            return expr::eval_str(inner.trim(), data);
        }
    }
    JsonValue::String(TemplateEngine::interpolate(raw, data))
}

/// 哪些属性算「传给组件的属性」：排除 class/style/id、事件绑定、指令、wxs 观察器，
/// 以及 `slot`（它说的是「我要进使用方组件的哪个 slot」，不是属性）。
///
/// 规则从 `host::component_mount` 原样搬来（只加了 `slot`）：模板展开时就要用。
pub fn is_property_attr(key: &str) -> bool {
    if key.contains(':') {
        return false; // wx:if / change:eS / bind:tap
    }
    if key.starts_with("bind") || key.starts_with("catch") || key.starts_with("data-") {
        return false;
    }
    if crate::renderer::components::parse_event_attr(key).is_some() {
        return false; // capture-bindtap / mut-bindtap 这类不带冒号的写法
    }
    !matches!(key, "class" | "style" | "id" | "hidden" | "eS" | "eA" | "slot")
}

/// `foo-bar` 属性 ↔ `fooBar` 属性名（微信规则；uni-app 的 `u-p` / `u-i` 也走这条）
pub fn camel_case(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    let mut upper = false;
    for ch in key.chars() {
        if ch == '-' {
            upper = true;
        } else if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}
