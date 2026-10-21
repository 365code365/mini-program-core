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
    if let Some(nav) = renderer.navigator_at(x, y, fixed, interaction) {
        let open_type = if nav.mini_program.is_some() { "miniProgram".to_string() } else { nav.open_type.clone() };
        let mp = nav.mini_program.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default()).unwrap_or_default();
        if std::env::var("MINI_NAV_LOG").is_ok() {
            eprintln!("🧭 navigator {open_type} {} delta={}", nav.url, nav.delta);
        }
        let js = format!(
            "__navigatorTap({}, {}, {}, {})",
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
    let target = renderer.label_target_at(x, y, fixed, interaction)?;
    let bounds = interaction.elements().iter().find(|e| e.id == target && e.is_fixed == fixed).map(|e| e.bounds)?;
    let (cx, cy) = (bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0);
    let r = interaction.handle_click_scoped(cx, cy, fixed)?;
    super::tap::dispatch_component_events(app, renderer, interaction, cx, cy, scope, tap_ctx, Some(&r));
    Some(r)
}
