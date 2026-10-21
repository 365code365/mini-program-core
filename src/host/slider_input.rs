//! slider 拖动的事件派发：拖动中每次换值发 `changing`，松手发 `change`。桌面窗体与 SDK 共用。
//!
//! 从前这两个事件**一个都不发**：桌面把拖动结果只 `println!` 了一下，SDK 干脆丢掉；
//! `bindchange` 只有「点一下没动」时走 tap 链路才会触发。页面上 `bindchange` 里写 setData 的
//! 滑块（几乎所有用法）拖完什么都不发生。
//!
//! 拖动的**推进**不在这里：横向锁定后由手势仲裁交给滑块（`host::input::apply_gesture_move`
//! 的 `BeginSlider` / `UpdateSlider`），这里只负责把结果告诉逻辑层。

use crate::renderer::WxmlRenderer;
use crate::runtime::MiniApp;
use crate::ui::interaction::InteractionManager;

/// 给 `id` 那个 slider 派发一次 `event`（`detail.value` 是 number，整数不带小数点）。
///
/// 命中点取滑块盒子的中心：手指拖出盒子之外（竖着偏了一点）事件照样要落到滑块上。
#[allow(clippy::too_many_arguments)]
fn dispatch(
    app: &mut MiniApp,
    renderer: Option<&WxmlRenderer>,
    interaction: &InteractionManager,
    id: &str,
    event: &str,
    value: f64,
    identifier: u32,
    time_ms: u64,
) -> bool {
    let (Some(r), Some(spec)) = (renderer, interaction.slider_spec(id)) else { return false };
    let p = spec.center();
    let detail = serde_json::json!({ "value": super::form::number(value) });
    if std::env::var("MINI_SLIDER_LOG").is_ok() {
        eprintln!("🎚 {event} {id} value={value}");
    }
    super::touch::dispatch_to_js(app, r, event, p, p, Some(spec.is_fixed), identifier, time_ms, detail)
}

/// 手势推进后值变了（[`super::input::GestureEffect::slider_changed`]）：发 `changing`
pub fn dispatch_changing(
    app: &mut MiniApp,
    renderer: Option<&WxmlRenderer>,
    interaction: &InteractionManager,
    identifier: u32,
    time_ms: u64,
) -> bool {
    let Some(d) = interaction.dragging_slider.as_ref() else { return false };
    let (id, value) = (d.id.clone(), d.value);
    dispatch(app, renderer, interaction, &id, "changing", value, identifier, time_ms)
}

/// 抬手 / 触摸被取消：结束拖动，拖动中换过值就发 `change`。
///
/// 返回这次触摸**是不是一次滑块拖动**：是的话宿主不要再把它当成 tap ——
/// 位移不到 tap 的判定距离（10px）时状态机照样会产出 Tap，再走一遍点按跳转
/// 就会多发一次 `change`，值还按手指的绝对位置算（抓着圆点拖时差出半个圆点）。
pub fn finish(
    app: &mut MiniApp,
    renderer: Option<&WxmlRenderer>,
    interaction: &mut InteractionManager,
    identifier: u32,
    time_ms: u64,
) -> bool {
    let Some((id, value, moved)) = interaction.end_slider_drag() else { return false };
    if moved {
        dispatch(app, renderer, interaction, &id, "change", value, identifier, time_ms);
    }
    moved
}
