//! 组件自己的事件（scroll-view 的 `bindscroll`、swiper 的 `bindchange` …）派发给逻辑层。
//!
//! 与触摸事件不同：它们不走捕获 / 冒泡链，只触发组件节点自己身上的绑定；`target` 与
//! `currentTarget` 都是组件本身，没有触点。SDK 引擎与桌面窗体共用（`host::scroll_view`、`host::swiper`）。
//!
//! Skyline 的 worklet 事件（`worklet:onscrollstart` / `onscrollupdate` / `onscrollend`，scroll-view 与 swiper）
//! 也在这里：引擎没有 UI 线程，worklet 回调在逻辑层同步调（与 bind 的同一次 eval 之内）。参数是事件对象本身：
//! detail 的字段放在顶层（`evt.scrollTop`），`evt.detail` 也照样给（官方示例两种写法都有）。
//! swiper 没有「开始 / 结束」这两个组件事件：第一次 `transition` 之前补一次 scrollstart，`animationfinish` 当 scrollend。

use crate::renderer::{EventBinding, WxmlRenderer};
use crate::runtime::MiniApp;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    /// 正在滑的 swiper（组件 id → 最近一次 transition 的 {dx, dy}）：worklet:onscrollstart / onscrollend 用
    static SWIPING: RefCell<HashMap<String, Value>> = RefCell::new(HashMap::new());
}

/// 组件事件 → 要一并触发的 worklet 事件（`EventBinding::event_type`，见 `event_attr::extract_events`）
fn worklet_events(id: &str, name: &str, detail: &Value) -> Vec<(&'static str, Value)> {
    match name {
        "scrollstart" => vec![("worklet:scrollstart", detail.clone())],
        "scroll" => vec![("worklet:scrollupdate", detail.clone())],
        "scrollend" => vec![("worklet:scrollend", detail.clone())],
        "transition" => SWIPING.with(|s| {
            let first = s.borrow_mut().insert(id.to_string(), detail.clone()).is_none();
            let mut v = Vec::with_capacity(2);
            if first {
                v.push(("worklet:scrollstart", detail.clone()));
            }
            v.push(("worklet:scrollupdate", detail.clone()));
            v
        }),
        "animationfinish" => SWIPING.with(|s| match s.borrow_mut().remove(id) {
            Some(last) => vec![("worklet:scrollend", last)],
            None => Vec::new(),
        }),
        _ => Vec::new(),
    }
}

/// 把一个组件自己的事件派发给逻辑层（不冒泡：scroll-view / swiper 的这些都是组件事件，
/// 只触发它自己身上的绑定）。`id` 是组件 id（同 `EventBinding::component_id`）。返回是否派发了
pub(crate) fn dispatch_own(app: &mut MiniApp, r: &WxmlRenderer, id: &str, name: &str, detail: Value, time_ms: u64) -> bool {
    let sent = dispatch_named(app, r, id, name, name, &detail, time_ms, false);
    // 没写 worklet:on* 的组件这里只多一次绑定表扫描（每个事件一次，不分配）：swiper 每帧的 transition 不进 SWIPING
    if !r.get_event_bindings().iter().any(|b| b.component_id == id && b.event_type.starts_with("worklet:")) {
        return sent;
    }
    let mut wk = false;
    for (ev, d) in worklet_events(id, name, &detail) {
        wk |= dispatch_named(app, r, id, ev, name, &d, time_ms, true);
    }
    sent || wk
}

/// 按绑定名（`event_type`）派发。`flat`：worklet 回调 —— detail 的字段也放在事件对象顶层
#[allow(clippy::too_many_arguments)]
fn dispatch_named(app: &mut MiniApp, r: &WxmlRenderer, id: &str, binding: &str, name: &str, detail: &Value, time_ms: u64, flat: bool) -> bool {
    let mut seen: Vec<(&str, &str)> = Vec::new();
    let binds = r.get_event_bindings().iter().filter(|b| b.component_id == id && b.event_type == binding);
    let binds: Vec<&EventBinding> = binds.collect();
    if binds.is_empty() {
        return false;
    }
    for b in binds {
        if seen.iter().any(|(h, o)| *h == b.handler && *o == b.owner) {
            continue;
        }
        seen.push((&b.handler, &b.owner));
        let node = super::touch::NodeInfo { id: &b.id, dataset: &b.data, offset_left: b.bounds.x, offset_top: b.bounds.y };
        let mut ev: Value = serde_json::from_str(&super::touch::event_json(name, &node, &node, (0.0, 0.0), 0, time_ms, detail.clone()))
            .unwrap_or(Value::Null);
        if let Some(o) = ev.as_object_mut() {
            // 组件事件没有触点
            o.insert("touches".into(), json!([]));
            o.insert("changedTouches".into(), json!([]));
            if flat {
                if let Some(d) = detail.as_object() {
                    for (k, v) in d {
                        o.entry(k.clone()).or_insert_with(|| v.clone());
                    }
                }
            }
        }
        let h = serde_json::to_string(&b.handler).unwrap_or_else(|_| "''".into());
        let o = serde_json::to_string(&b.owner).unwrap_or_else(|_| "''".into());
        if std::env::var("MINI_SCROLL_LOG").is_ok() {
            eprintln!("📜 {id} {binding} -> {} {detail}", b.handler);
        }
        app.eval(&format!("__dispatchEvent({h}, {ev}, {o})")).ok();
    }
    true
}

/// `<image>` 的 `bindload` / `binderror`（微信语义：load 的 detail 是图片**原始**像素尺寸 `{width, height}`，
/// error 是 `{errMsg}`）。渲染器在画图时攒下（`WxmlRenderer::take_image_events`，每张图只发一次），
/// SDK 在每次重画之后取走派发（engine_frame.rs `render_scoped`）；桌面窗体有自己的一份（app_window）。
/// 走 `__dispatchEvent`：带上组件归属（组件模板里的 bindload 发给组件实例），detail 不过 dataset 还原。
/// 返回派发了几条（宿主据此标数据脏）
pub(crate) fn dispatch_image_events(app: &mut MiniApp, r: &mut WxmlRenderer, time_ms: u64) -> usize {
    use crate::renderer::ImageEventDetail;
    let events = r.take_image_events();
    for e in &events {
        let node = json!({ "id": e.id, "dataset": e.data });
        let detail = match e.detail {
            ImageEventDetail::Load { width, height } => json!({ "width": width, "height": height }),
            ImageEventDetail::Error => json!({ "errMsg": "GET_IMAGE_FAILED" }),
        };
        let event = json!({
            "type": e.event_type, "timeStamp": time_ms,
            "target": node, "currentTarget": node, "detail": detail,
            "touches": [], "changedTouches": [],
        });
        let h = serde_json::to_string(&e.handler).unwrap_or_else(|_| "''".into());
        let o = serde_json::to_string(&e.owner).unwrap_or_else(|_| "''".into());
        app.eval(&format!("__dispatchEvent({h}, {event}, {o})")).ok();
    }
    events.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swiper_的_worklet_滑动开始_更新_结束() {
        let names = |v: Vec<(&'static str, Value)>| v.into_iter().map(|(n, _)| n).collect::<Vec<_>>();
        let d = json!({ "dx": 3, "dy": 0 });
        assert_eq!(names(worklet_events("sw-t", "transition", &d)), ["worklet:scrollstart", "worklet:scrollupdate"], "第一次 transition 补 scrollstart");
        assert_eq!(names(worklet_events("sw-t", "transition", &json!({ "dx": 9, "dy": 0 }))), ["worklet:scrollupdate"]);
        let end = worklet_events("sw-t", "animationfinish", &json!({ "current": 1, "source": "touch" }));
        assert_eq!(names(end.clone()), ["worklet:scrollend"]);
        assert_eq!(end[0].1, json!({ "dx": 9, "dy": 0 }), "scrollend 的 detail 是 {{dx, dy}}（官方），不是 animationfinish 的 {{current, source}}");
        assert!(worklet_events("sw-t", "animationfinish", &d).is_empty(), "没在滑的 animationfinish（setData 改 current）不发");
        assert_eq!(names(worklet_events("sv", "scroll", &d)), ["worklet:scrollupdate"]);
    }
}
