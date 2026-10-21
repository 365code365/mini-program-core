//! `<form>` 的提交与重置：点了 `form-type="submit|reset"` 的按钮之后。
//!
//! 依据官方 component/form.html：`bindsubmit` 的 `event.detail = {value: {name: value}, formId: ''}`，
//! 值的类型按控件：input / textarea 是字符串、switch 是布尔、slider 是数字、checkbox-group 是
//! 选中项 value 的数组、radio-group 是选中项的 value（没选是空串）、picker 与它 `bindchange`
//! 的 `detail.value` 同型。`bindreset` 不带数据。
//!
//! 重置：输入框清空、开关与勾选取消、滑块回到 `min`（与微信一致：重置不是「回到页面 data 里的值」，
//! 页面 data 不会被改）。picker 自己不留状态，重置不动它。
//!
//! 按钮自己的 `bindtap` 照常先发（由调用方的 tap 链负责），这里只管表单那一下。
//! 桌面窗体与 SDK 引擎共用（都经 `host::tap::handle_content_click`）。

use crate::renderer::{FormAction, FormControl, FormHit, WxmlRenderer};
use crate::runtime::MiniApp;
use crate::ui::interaction::{ComponentState, InteractionManager, InteractionResult};
use serde_json::{json, Map, Value};

/// 一次点击之后：点的是 form-type 按钮就提交 / 重置所在表单。返回是否处理了（重置改了控件状态，要重画）。
///
/// `result` 是交互层对这次点击的结论：拨开关、选单选、拖滑块、聚焦输入框的点击不可能是按钮，
/// 直接跳过；`ButtonClick` 与「没命中有状态元素」（按钮里带 hover-class 的子 view 会盖住按钮）都要查。
#[allow(clippy::too_many_arguments)]
pub(crate) fn after_click(
    app: &mut MiniApp,
    renderer: &WxmlRenderer,
    interaction: &mut InteractionManager,
    result: Option<&InteractionResult>,
    x: f32,
    y: f32,
    scope: Option<bool>,
    time_ms: u64,
) -> bool {
    if !matches!(result, None | Some(InteractionResult::ButtonClick { .. })) {
        return false;
    }
    let Some(hit) = renderer.form_button_at(x, y, scope == Some(true), interaction) else { return false };
    let (event, detail) = match hit.action {
        FormAction::Submit => ("submit", json!({ "value": values_with(app, &hit, interaction), "formId": "" })),
        FormAction::Reset => {
            reset(&hit, interaction);
            ("reset", json!({}))
        }
    };
    if std::env::var("MINI_FORM_LOG").is_ok() {
        eprintln!("📝 form {event} {detail}");
    }
    // 表单节点上的这条绑定：命中点在按钮里，按钮在表单里，所以表单的盒子一定包着它
    let form = renderer
        .dispatch_chain(x, y, event, scope)
        .into_iter()
        .find(|b| b.tag == "form")
        .map(|b| b.component_id);
    if let Some(id) = form {
        super::own_events::dispatch_own(app, renderer, &id, event, detail, time_ms);
    }
    true
}

/// 提交的 `{name: value}`：只收写了 `name` 的控件，同名的后者覆盖前者
pub fn values(hit: &FormHit, im: &InteractionManager) -> Value {
    let mut out = Map::new();
    for f in &hit.fields {
        let Some(name) = &f.name else { continue };
        out.insert(name.clone(), control_value(&f.control, im));
    }
    Value::Object(out)
}

/// 同上，`wx://form-field` 组件的值向逻辑层要（组件实例 data 里的 `value`，类型原样）
fn values_with(app: &MiniApp, hit: &FormHit, im: &InteractionManager) -> Value {
    let mut out = match values(hit, im) {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    for f in &hit.fields {
        let (Some(name), FormControl::Component { key }) = (&f.name, &f.control) else { continue };
        let js = format!("__compFieldValue({})", serde_json::Value::String(key.clone()));
        let v = app
            .eval(&js)
            .ok()
            .and_then(|s| serde_json::from_str::<String>(&s).ok().or(Some(s)))
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .unwrap_or(Value::Null);
        out.insert(name.clone(), v);
    }
    Value::Object(out)
}

fn control_value(c: &FormControl, im: &InteractionManager) -> Value {
    let checked = |id: &str, initial: bool| im.get_state(id).map(|s| s.checked).unwrap_or(initial);
    match c {
        FormControl::Text { id, initial } => {
            // 正在输入的那个以焦点里的文本为准（每次编辑也会同步进 states，这里是双保险）
            let focused = im.focused_input.as_ref().filter(|f| &f.id == id).map(|f| f.value.clone());
            json!(focused.or_else(|| im.get_state(id).map(|s| s.value.clone())).unwrap_or_else(|| initial.clone()))
        }
        FormControl::Switch { id, initial } => json!(checked(id, *initial)),
        // 交互状态在页面改过 value 之后作废（受控），所以不能直接读 states
        FormControl::Slider { id, initial, .. } => number(im.slider_value(id, *initial).unwrap_or(*initial)),
        FormControl::Picker { value } => value.clone(),
        // 组件的值在逻辑层（见 values_with）
        FormControl::Component { .. } => Value::Null,
        FormControl::Group { multi, items } => {
            let mut picked = items.iter().filter(|(id, _, init)| checked(id, *init)).map(|(_, v, _)| v.clone());
            if *multi {
                json!(picked.collect::<Vec<_>>())
            } else {
                json!(picked.next().unwrap_or_default())
            }
        }
    }
}

/// 整数就给整数（`50` 而不是 `50.0`：页面里常拿它直接拼字符串）。slider 的事件 detail 也用它。
pub(crate) fn number(v: f64) -> Value {
    if v.fract() == 0.0 && v.abs() < 9.0e15 {
        json!(v as i64)
    } else {
        json!(v)
    }
}

/// 把表单里的控件拨回初始：写进 `states`，下一帧绘制按它画
pub fn reset(hit: &FormHit, im: &mut InteractionManager) {
    let off = |im: &mut InteractionManager, id: &str| {
        let value = im.get_state(id).map(|s| s.value.clone()).unwrap_or_default();
        im.set_state(id.to_string(), ComponentState { checked: false, value });
    };
    for f in &hit.fields {
        match &f.control {
            FormControl::Text { id, .. } => {
                im.set_state(id.clone(), ComponentState { checked: false, value: String::new() });
                if let Some(fi) = im.focused_input.as_mut().filter(|fi| &fi.id == id) {
                    fi.value.clear();
                    fi.cursor_pos = 0;
                    fi.clear_selection();
                }
            }
            FormControl::Switch { id, .. } => off(im, id),
            // 从前写的是 `min as i32`：min 带小数时被截断
            FormControl::Slider { id, min, .. } => im.set_slider_value(id, *min),
            FormControl::Picker { .. } | FormControl::Component { .. } => {}
            FormControl::Group { items, .. } => {
                for (id, _, _) in items {
                    off(im, id);
                }
            }
        }
    }
}
