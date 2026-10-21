//! 组件自己的事件（scroll-view 的 `bindscroll`、swiper 的 `bindchange` …）派发给逻辑层。
//!
//! 与触摸事件不同：它们不走捕获 / 冒泡链，只触发组件节点自己身上的绑定；`target` 与
//! `currentTarget` 都是组件本身，没有触点。SDK 引擎与桌面窗体共用（`host::scroll_view`、`host::swiper`）。

use crate::renderer::{EventBinding, WxmlRenderer};
use crate::runtime::MiniApp;
use serde_json::{json, Value};

/// 把一个组件自己的事件派发给逻辑层（不冒泡：scroll-view / swiper 的这些都是组件事件，
/// 只触发它自己身上的绑定）。`id` 是组件 id（同 `EventBinding::component_id`）。返回是否派发了
pub(crate) fn dispatch_own(app: &mut MiniApp, r: &WxmlRenderer, id: &str, name: &str, detail: Value, time_ms: u64) -> bool {
    let mut seen: Vec<(String, String)> = Vec::new();
    let binds: Vec<&EventBinding> = r
        .get_event_bindings()
        .iter()
        .filter(|b| b.component_id == id && b.event_type == name)
        .collect();
    for b in binds {
        if seen.iter().any(|(h, o)| *h == b.handler && *o == b.owner) {
            continue;
        }
        seen.push((b.handler.clone(), b.owner.clone()));
        let node = super::touch::NodeInfo { id: &b.id, dataset: &b.data, offset_left: b.bounds.x, offset_top: b.bounds.y };
        let mut ev: Value = serde_json::from_str(&super::touch::event_json(name, &node, &node, (0.0, 0.0), 0, time_ms, detail.clone()))
            .unwrap_or(Value::Null);
        // 组件事件没有触点
        if let Some(o) = ev.as_object_mut() {
            o.insert("touches".into(), json!([]));
            o.insert("changedTouches".into(), json!([]));
        }
        let h = serde_json::to_string(&b.handler).unwrap_or_else(|_| "''".into());
        let o = serde_json::to_string(&b.owner).unwrap_or_else(|_| "''".into());
        if std::env::var("MINI_SCROLL_LOG").is_ok() {
            eprintln!("📜 {id} {name} -> {} {detail}", b.handler);
        }
        app.eval(&format!("__dispatchEvent({h}, {ev}, {o})")).ok();
    }
    !seen.is_empty()
}
