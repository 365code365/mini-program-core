//! 手势组件代理原生容器（`<vertical-drag-gesture-handler native-view="scroll-view">`，
//! `builtin_components/gesture.js`）的宿主一侧。SDK 引擎与桌面窗体共用。
//!
//! 官方（framework/runtime/skyline/gesture「示例三：代理原生组件内部手势」）：容器照常滚，手势回调一直跟到松手
//! （触顶之后 scroll 不再触发、手势回调照常）；`should-accept-gesture` 返回 false 时这串触摸容器不滚，
//! `should-response-on-move` 返回 false 时这一下不滚。所以：
//!
//! - 按下点落在带 `bind:__nativeview` 的节点上（手势组件并进容器的那条标记绑定）时，这串触摸被滚动接管
//!   也**不补 touchcancel**（[`keeps_touch`]）—— 手势组件靠 touchmove 推进；
//! - 每一下 move 在派完 touchmove 之后问一次逻辑层要不要拦（[`move_blocked`]，读 `__ghNativeBlock` /
//!   `__ghNativeReject`）。只有这种触摸才问：别的触摸一次 eval 都不多。
use crate::renderer::WxmlRenderer;
use crate::runtime::MiniApp;
use std::cell::Cell;

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
}

/// 按下：起点（与绑定同一坐标系：覆盖层视口坐标、正常流内容坐标）有没有代理容器的手势组件
pub fn begin(r: Option<&WxmlRenderer>, x: f32, y: f32) {
    let p = crate::Point::new(x, y);
    let on = r.map(|r| r.get_event_bindings().iter().any(|b| b.event_type == "__nativeview" && b.bounds.contains(&p))).unwrap_or(false);
    ACTIVE.with(|a| a.set(on));
}

/// 这串触摸被滚动接管时是否照样留着（不补 touchcancel）
pub fn keeps_touch() -> bool {
    ACTIVE.with(|a| a.get())
}

/// 这一下 move 容器该不该停住（手势组件的 should-accept-gesture / should-response-on-move 返回了 false）
pub fn move_blocked(app: &MiniApp) -> bool {
    keeps_touch()
        && app
            .eval("(globalThis.__ghNativeBlock || globalThis.__ghNativeReject) ? 1 : 0")
            .map(|s| s.trim() == "1")
            .unwrap_or(false)
}

/// 松手 / 取消
pub fn end() {
    ACTIVE.with(|a| a.set(false));
}

/// 起点（与绑定同一坐标系）下的 `<draggable-sheet>` 面板：它的 `bind:__dsheet` 标记绑定的值是 `max` / `mid`
/// （draggable-sheet.wxml）。`area` 是起点下最内层的滚动区域：它在面板里面才算「面板里的列表」。
/// 只在按下时走一次（绑定表里按名字找，没有面板的页面只是一次遍历比较）
pub fn sheet_at(
    r: Option<&WxmlRenderer>,
    im: &crate::ui::InteractionManager,
    x: f32,
    y: f32,
    area: Option<&str>,
) -> Option<super::gesture::SheetHit> {
    let p = crate::Point::new(x, y);
    let b = r?
        .get_event_bindings()
        .iter()
        .filter(|b| b.event_type == "__dsheet" && b.bounds.contains(&p))
        // 套着的几层面板：最内层（最小的）那个
        .min_by(|a, b| (a.bounds.width * a.bounds.height).total_cmp(&(b.bounds.width * b.bounds.height)))?;
    let inside = area.and_then(|id| im.scroll_area_bounds(id)).map(|(ab, _)| {
        let c = crate::Point::new(ab.x + ab.width / 2.0, ab.y + ab.height / 2.0);
        b.bounds.contains(&c) && ab.width * ab.height <= b.bounds.width * b.bounds.height + 1.0
    });
    let at_top = area.and_then(|id| im.get_scroll_controller(id)).map(|c| c.get_position() <= 0.5).unwrap_or(true);
    Some(super::gesture::SheetHit { at_max: b.handler.trim() == "max", area_inside: inside.unwrap_or(false), area_at_top: at_top })
}
