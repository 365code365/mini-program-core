//! 点击之后内置组件自己的行为：`<form>` 的提交 / 重置、`<navigator>` 的跳转、`<label>` 转发给控件。
//!
//! 页面绑定的 `bindtap` 照常先发（调用方的 tap 链负责），这里在它之后。三个出口（覆盖层没处理器、
//! 覆盖层有处理器、正常流）都走这一个函数，桌面窗体与 SDK 引擎共用（都经 `host::tap::handle_content_click`）。

use crate::renderer::WxmlRenderer;
use crate::runtime::MiniApp;
use crate::ui::interaction::{InteractionManager, InteractionResult};

/// 返回 label 转发产生的交互结果（点 label 聚焦了输入框 / 拨了开关），调用方按普通点击的结果处理
#[allow(clippy::too_many_arguments)]
pub(crate) fn after_click(
    app: &mut MiniApp,
    renderer: &WxmlRenderer,
    interaction: &mut InteractionManager,
    result: Option<&InteractionResult>,
    x: f32,
    y: f32,
    scope: Option<bool>,
    tap_ctx: (u32, u64),
) -> Option<InteractionResult> {
    super::form::after_click(app, renderer, interaction, result, x, y, scope, tap_ctx.1);
    let fixed = scope == Some(true);
    // `<button open-type>`：按钮自己的 bindtap 已经发过，这里做开放能力（open-type.js）
    if let Some(b) = renderer.dispatch_chain(x, y, "__opentype", scope).into_iter().next() {
        let binds = own_binds(renderer, &b.component_id, OPEN_TYPE_EVENTS);
        let target = serde_json::json!({ "id": b.id, "dataset": b.data });
        let js = format!("__buttonOpenType({}, {}, {target})", b.handler, binds);
        if let Err(e) = app.eval(&js) {
            eprintln!("⚠  button open-type 失败: {e}");
        }
    }
    if let Some(nav) = renderer.navigator_at(x, y, fixed, interaction) {
        // target="miniProgram" 时 open-type 照样分流（navigate / navigateBack / exit），route.js `__navigatorTap`
        let open_type = nav.open_type.clone();
        let mp = nav.mini_program.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default()).unwrap_or_default();
        if std::env::var("MINI_NAV_LOG").is_ok() {
            eprintln!("🧭 navigator {open_type} {} delta={}", nav.url, nav.delta);
        }
        // navigator 自己的 bind:success / fail / complete：命中链里最内层的 navigator 上的那条
        let list: Vec<_> = ["success", "fail", "complete"]
            .iter()
            .filter_map(|ev| renderer.dispatch_chain(x, y, ev, scope).into_iter().find(|b| b.tag == "navigator"))
            .collect();
        let binds = binds_json(list.iter());
        let js = format!(
            "__navigatorTap({}, {}, {}, {}, {binds})",
            serde_json::Value::String(open_type),
            serde_json::Value::String(nav.url),
            nav.delta,
            serde_json::Value::String(mp)
        );
        if let Err(e) = app.eval(&js) {
            eprintln!("⚠️  navigator 跳转失败: {e}");
        }
        return None;
    }
    // label：只在这次点击没落到别的控件上时转发（点到 label 里的开关本身，开关已经自己响应了）
    if !matches!(result, None) {
        return None;
    }
    let (target, tag) = renderer.label_target(x, y, fixed, interaction)?;
    let Some(bounds) = interaction.elements().iter().find(|e| e.id == target && e.is_fixed == fixed).map(|e| e.bounds) else {
        // `for` 指向的控件不在视口里（本帧没登记）：直接改它的状态，滚回来、提交表单时就是新值。
        // 它的 change 绑定也没登记（绑定只在画到时登记），这一下不派 change（已知取舍）
        return label_offscreen(renderer, interaction, &target, &tag);
    };
    let (cx, cy) = (bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0);
    let r = interaction.handle_click_scoped(cx, cy, fixed)?;
    super::tap::dispatch_component_events(app, renderer, interaction, cx, cy, scope, tap_ctx, Some(&r));
    // label 包着 / 指向的是 form-type 按钮：照样提交 / 重置表单（官方 label：「触发对应控件」）。
    // 从前 label 只转发了按钮的 tap，表单没提交
    if matches!(r, InteractionResult::ButtonClick { .. }) {
        super::form::after_click(app, renderer, interaction, Some(&r), cx, cy, scope, tap_ctx.1);
    }
    Some(r)
}

/// label 指向的控件不在本帧的交互元素表里：checkbox / switch 翻转、radio 选中（同组其它项取消）
fn label_offscreen(renderer: &WxmlRenderer, interaction: &mut InteractionManager, id: &str, tag: &str) -> Option<InteractionResult> {
    use crate::ui::interaction::ComponentState;
    let group = renderer.choice_group(id, interaction);
    let member = group.as_ref().and_then(|g| g.iter().find(|m| m.0 == id).cloned());
    let (value, attr) = member.map(|m| (m.1, m.2)).unwrap_or_default();
    let now = interaction.effective_checked(id, attr);
    match tag {
        "checkbox" | "switch" => {
            interaction.set_state(id.to_string(), ComponentState { checked: !now, value });
            Some(InteractionResult::Toggle { id: id.to_string(), checked: !now })
        }
        "radio" if !now => {
            for (mid, v, _) in group.into_iter().flatten() {
                interaction.set_state(mid.clone(), ComponentState { checked: mid == id, value: v });
            }
            Some(InteractionResult::Select { id: id.to_string(), value })
        }
        _ => None,
    }
}

/// button open-type 的回调事件（官方 component/button 的 bind:*）：只派给按钮自己，不冒泡
const OPEN_TYPE_EVENTS: &[&str] = &[
    "getuserinfo", "contact", "createliveactivity", "getphonenumber", "getrealtimephonenumber", "error", "opensetting",
    "launchapp", "chooseavatar", "agreeprivacyauthorization",
];

/// 节点自己身上这几种事件的绑定
fn own_binds(r: &WxmlRenderer, component_id: &str, events: &[&str]) -> String {
    binds_json(r.get_event_bindings().iter().filter(|b| b.component_id == component_id && events.contains(&b.event_type.as_str())))
}

/// 绑定 → 逻辑层派发要的 `[{type, handler, owner, id, dataset}]`（open-type.js 异步回来时自己 `__dispatchEvent`）
fn binds_json<'a>(list: impl Iterator<Item = &'a crate::renderer::EventBinding>) -> String {
    let v: Vec<serde_json::Value> = list
        .map(|b| serde_json::json!({ "type": b.event_type, "handler": b.handler, "owner": b.owner, "id": b.id, "dataset": b.data }))
        .collect();
    serde_json::Value::Array(v).to_string()
}

/// 选中了 `id` 这个 radio：同一个 radio-group 里的其它项取消（不在组里的 radio 各管各的）。
/// 组成员按布局树找（`WxmlRenderer::choice_group`），滚出视口的也取消
pub(crate) fn exclusive_radio(interaction: &mut InteractionManager, renderer: &WxmlRenderer, id: &str) {
    let Some(members) = renderer.choice_group(id, interaction) else { return };
    for (mid, value, _) in members {
        if mid != id {
            interaction.set_state(mid, crate::ui::interaction::ComponentState { checked: false, value });
        }
    }
}
