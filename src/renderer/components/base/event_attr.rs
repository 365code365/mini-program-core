//! 事件属性解析：六种绑定前缀、data-*、class/文本提取
//!
//! 由 `base/mod.rs` 组合。**纯搬迁**：从 1925 行的 base.rs 按职责切开，一行代码没改。
use super::*;

/// 事件的传播阶段（微信的 `capture-bind:` / `capture-catch:` 走捕获）

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EventPhase {
    /// 冒泡阶段：由内向外
    #[default]
    Bubble,
    /// 捕获阶段：由外向内，先于冒泡
    Capture,
}

/// 一条事件绑定（节点上的 `bind*` / `catch*` / `mut-bind:*` / `capture-*`）
#[derive(Clone, Debug)]
pub struct EventBind {
    /// 事件名（`bindtap` → `tap`，`bind:touchstart` → `touchstart`）
    pub event_type: String,
    /// 处理函数名（页面或组件实例上的方法）
    pub handler: String,
    /// `data-*` 数据集（外加 input 类组件的若干属性）
    pub data: HashMap<String, String>,
    /// `catch*`：阻止继续传播
    pub is_catch: bool,
    /// 捕获还是冒泡
    pub phase: EventPhase,
    /// `mut-bind:*`：互斥绑定，一条触发后其它 mut-bind 不再触发（`bind`/`catch` 不受影响）
    pub mut_bind: bool,
    /// 声明这条绑定的自定义组件标签名（页面模板里声明的是空串）。
    /// 派发时据此把事件送给**组件实例**，见 `parser::template::COMPONENT_OWNER_ATTR`。
    pub owner: String,
}

/// 解析事件属性名 → (事件名, 是否 catch, 阶段, 是否互斥绑定)。
///
/// 微信的写法有六种前缀，冒号可省：`bind` / `catch` / `mut-bind` /
/// `capture-bind` / `capture-catch`（`bindtap` 与 `bind:tap` 等价）。
/// 之前引擎是**按固定属性名白名单**匹配的（只认 bindtap/catchtap/bindchange…），
/// 于是 `bindtouchstart`、`bind:tap`、`catchtouchmove`、`capture-bind:tap`
/// 这些全部被无声忽略 —— 自定义手势、遮罩阻止滚动、事件捕获统统失效。
pub fn parse_event_attr(attr: &str) -> Option<(&str, bool, EventPhase, bool)> {
    // 长前缀必须先匹配（`capture-bind` 也以 `bind` 结尾之外的形式出现）
    const PREFIXES: &[(&str, bool, EventPhase, bool)] = &[
        ("capture-catch:", true, EventPhase::Capture, false),
        ("capture-catch", true, EventPhase::Capture, false),
        ("capture-bind:", false, EventPhase::Capture, false),
        ("capture-bind", false, EventPhase::Capture, false),
        ("mut-bind:", false, EventPhase::Bubble, true),
        ("mut-bind", false, EventPhase::Bubble, true),
        ("catch:", true, EventPhase::Bubble, false),
        ("catch", true, EventPhase::Bubble, false),
        ("bind:", false, EventPhase::Bubble, false),
        ("bind", false, EventPhase::Bubble, false),
    ];
    for (prefix, is_catch, phase, mut_bind) in PREFIXES {
        if let Some(rest) = attr.strip_prefix(prefix) {
            let name = rest.trim();
            if name.is_empty() {
                return None;
            }
            return Some((name, *is_catch, *phase, *mut_bind));
        }
    }
    None
}

/// 提取节点上的全部事件绑定
pub fn extract_events(node: &WxmlNode) -> Vec<EventBind> {
    let mut events = vec![];
    // 数据集只算一次：同一节点上的多个绑定共享 `data-*`
    let mut data = HashMap::new();
    for (k, v) in &node.attributes {
        if let Some(name) = k.strip_prefix("data-") {
            data.insert(name.to_string(), v.clone());
        }
    }
    // input/textarea：把宿主侧编辑需要的属性一并带上（maxlength/type/password）
    if node.tag_name == "input" || node.tag_name == "textarea" {
        for attr in ["maxlength", "type", "password"] {
            if let Some(v) = node.get_attr(attr) {
                data.insert(attr.to_string(), v.to_string());
            }
        }
    }
    let owner = node
        .get_attr(crate::parser::template::COMPONENT_OWNER_ATTR)
        .unwrap_or("")
        .to_string();
    for (attr, handler) in &node.attributes {
        let Some((event_type, is_catch, phase, mut_bind)) = parse_event_attr(attr) else {
            continue;
        };
        if handler.trim().is_empty() {
            continue;
        }
        events.push(EventBind {
            event_type: event_type.to_string(),
            handler: handler.clone(),
            data: data.clone(),
            is_catch,
            phase,
            mut_bind,
            owner: owner.clone(),
        });
    }
    // 属性表是 HashMap，顺序不定；排序让绑定顺序稳定（快照/测试要可复现）
    events.sort_by(|a, b| (&a.event_type, &a.handler).cmp(&(&b.event_type, &b.handler)));
    events
}

/// 获取节点的 class 列表
pub fn get_classes(node: &WxmlNode) -> Vec<&str> {
    node.get_attr("class").map(|s| s.split_whitespace().collect()).unwrap_or_default()
}

/// 获取节点的文本内容
pub fn get_text_content(node: &WxmlNode) -> String {
    use crate::parser::wxml::WxmlNodeType;
    let mut s = String::new();
    for c in &node.children {
        if c.node_type == WxmlNodeType::Text { 
            s.push_str(&c.text_content); 
        } else { 
            s.push_str(&get_text_content(c)); 
        }
    }
    s.trim().into()
}
