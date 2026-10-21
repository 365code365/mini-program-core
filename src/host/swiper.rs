//! swiper 的事件派发（`bindchange` / `bindtransition` / `bindanimationfinish`），SDK 与桌面共用。
//!
//! 事件在 swiper 状态机里产生（松手、自动翻页、动画走完、拖动 / 动画中位置变化，见
//! `renderer::components::swiper::state`），宿主每帧取走、按组件 id 找到 swiper 节点上的绑定派发。

use crate::renderer::WxmlRenderer;
use crate::runtime::MiniApp;

/// 派发这一帧积攒的 swiper 事件。返回是否派发过（宿主据此看要不要取数据、重画）
pub fn tick(app: &mut MiniApp, renderer: Option<&WxmlRenderer>, time_ms: u64) -> bool {
    use crate::renderer::components::picker_view;
    let mut events = crate::renderer::components::take_swiper_events();
    // <picker-view> 的滚轮与 swiper 共用手势仲裁，事件也在这里一起派（change / pickstart / pickend）
    picker_view::tick();
    events.extend(picker_view::take_events());
    let Some(r) = renderer else { return false };
    let mut fired = false;
    for (id, name, detail) in events {
        fired |= super::own_events::dispatch_own(app, r, &id, name, detail, time_ms);
    }
    if fired {
        app.pump_jobs();
    }
    fired
}
