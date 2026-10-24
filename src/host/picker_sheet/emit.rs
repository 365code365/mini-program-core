//! 面板定下来的事件（change / cancel / columnchange）怎么交给页面。
//!
//! 走与 tap 同一条事件链（`touch::dispatch_to_js`），从点开 picker 的那一点命中：
//! `e.type` 是 `change` 而不是 `tap`，`e.currentTarget.dataset` 是 picker 自己的 `data-*`
//! （列表里一行一个 picker、靠 `data-index` 区分是常态），能冒泡、能被 `catch` 截断，
//! 组件模板里的 picker 交给声明它的组件实例。`detail` 原样交出去，不经 dataset 的字面量还原 ——
//! 从前走 `__callPageMethod`：`type` 是 `tap`、dataset 被换成了 detail、`fields="year"` 的 "2024"
//! 被还原成数字 2024。
//!
//! 渲染器不在手边（老的调用方）、或那一点已经找不到绑定（面板开着时页面重排过）时，
//! 按登记的处理函数名退回老入口，至少回调不丢。

use super::PickerSheetState;
use crate::renderer::WxmlRenderer;
use crate::runtime::MiniApp;

/// `bindchange` 的 detail：`{value}`（selector 是下标 number、multiSelector / region 是数组、time / date 是字符串，
/// 见 [`PickerSheetState::change_value`]）
pub(super) fn change_detail(_sheet: &PickerSheetState, value: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "value": value })
}

/// 把抬手里排下的事件派发给页面。宿主在 `on_release` 之后调；没有排着的事件什么都不做
pub fn flush(sheet: &mut Option<PickerSheetState>, app: &mut MiniApp, renderer: Option<&WxmlRenderer>, time_ms: u64) {
    let Some(sheet) = sheet.as_mut() else { return };
    for (event_type, detail) in std::mem::take(&mut sheet.outbox) {
        let sent = match (renderer, sheet.origin) {
            // target 是 picker 自己（链上第一个绑了这个事件的节点），不是手指下面最里层的那行字
            (Some(r), Some((x, y, fixed))) => crate::host::touch::dispatch_chain_to_js(
                app,
                &r.dispatch_chain(x, y, event_type, Some(fixed)),
                None,
                event_type,
                (x, y),
                0,
                time_ms,
                detail.clone(),
            ),
            _ => false,
        };
        if sent {
            continue;
        }
        let handler = match event_type {
            "change" => sheet.handler.clone(),
            "cancel" => sheet.cancel_handler.clone(),
            _ => sheet.column_handler.clone(),
        };
        if let Some(h) = handler {
            let name = serde_json::to_string(&h).unwrap_or_else(|_| "''".into());
            app.eval(&format!("__callPageMethod({name}, {detail})")).ok();
        }
    }
}
