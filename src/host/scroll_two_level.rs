//! scroll-view 的下拉二级（Skyline `refresher-two-level-*`，官方 component/scroll-view「下拉二级」一节）：
//! 刷新状态机（host/scroll_view.rs）里多出来的那一档。
//!
//! - 打开：拉过 `refresher-two-level-threshold`（默认 150）松手；`ScrollViewContext.triggerTwoLevel()`；
//!   `refresher-two-level-triggered` 从 false 变 true。打开 = 顶部 inset 设成盒子高，内容推到盒子下面，整块露出刷新区
//!   （`slot="refresher"` 的节点照常画在露出来的那段里）；
//! - 关闭：`closeTwoLevel()`；属性从 true 变 false；开了 `refresher-two-level-scroll-enabled` 时往上推过
//!   `refresher-two-level-close-threshold`（默认 80）松手。推得不够松手弹回打开的位置；
//!   没开 scroll-enabled 时二级里拖不动（每帧按回打开的位置）；
//! - `bindrefresherstatuschange` 的 5 CanTwoLevel / 6 TwoLevelOpening / 7 TwoLeveling / 8 TwoLevelClosing 由 scroll_view.rs 按这里的结果报。
//!
//! 待验 / 没做：`refresher-two-level-pinned`（「即将打开二级时定住」，官方没写清定在哪）、
//! `refresher-ballistic-refresh-enabled`（惯性滚动触发刷新）只读不用；属性受控只认值的变化（与 refresher-triggered 的
//! 「设回同值也算」不同：用户自己滑动关掉之后页面要先设 false 再设 true 才能再打开）。
use crate::renderer::wxml_renderer::TwoLevelConfig;
use crate::ui::scroll_controller::ScrollController;
use crate::ui::scroll_view_state::RefresherPhase;

/// 这一帧二级要不要开 / 关（`open`、`close` 至多一个为真）
pub(super) struct Step {
    pub open: bool,
    pub close: bool,
}

/// 推进一帧。`prev_triggered` 是上一帧的 `refresher-two-level-triggered`，`request` 是 Context 的请求
#[allow(clippy::too_many_arguments)]
pub(super) fn step(
    tl: &TwoLevelConfig,
    phase: RefresherPhase,
    prev_triggered: Option<bool>,
    request: Option<bool>,
    dragging: bool,
    was_dragging: bool,
    gap: f32,
    c: Option<&mut ScrollController>,
) -> Step {
    let open_now = phase == RefresherPhase::TwoLevel;
    if !open_now {
        let open = request == Some(true) || (tl.triggered && prev_triggered == Some(false));
        return Step { open, close: false };
    }
    let mut close = request == Some(false) || (!tl.triggered && prev_triggered == Some(true));
    if let Some(c) = c {
        if dragging && !tl.scroll_enabled {
            // 二级里不许拖：按回打开的位置
            c.set_position(-tl.height);
        } else if !dragging && was_dragging && !close {
            if tl.scroll_enabled && tl.height - gap >= tl.close_threshold {
                close = true;
            } else {
                // 推得不够：弹回打开的位置（在 inset 范围里，控制器自己不回弹）
                c.animate_to(-tl.height, 0.3);
            }
        }
    }
    Step { open: false, close }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> TwoLevelConfig {
        TwoLevelConfig { threshold: 150.0, close_threshold: 80.0, scroll_enabled: true, triggered: false, height: 600.0 }
    }

    #[test]
    fn 属性与请求开关_推过关闭阈值才关() {
        let tl = cfg();
        assert!(step(&tl, RefresherPhase::Idle, Some(false), Some(true), false, false, 0.0, None).open, "triggerTwoLevel");
        let on = TwoLevelConfig { triggered: true, ..tl };
        assert!(step(&on, RefresherPhase::Idle, Some(false), None, false, false, 0.0, None).open, "属性 false → true");
        assert!(!step(&on, RefresherPhase::Idle, Some(true), None, false, false, 0.0, None).open, "一直是 true 不再开");
        assert!(step(&tl, RefresherPhase::TwoLevel, Some(true), None, false, false, 600.0, None).close, "属性 true → false");
        assert!(step(&tl, RefresherPhase::TwoLevel, None, Some(false), false, false, 600.0, None).close, "closeTwoLevel");
        let mut c = ScrollController::new(2000.0, 600.0);
        assert!(!step(&tl, RefresherPhase::TwoLevel, None, None, false, true, 560.0, Some(&mut c)).close, "只推了 40 < 80：弹回");
        assert!(step(&tl, RefresherPhase::TwoLevel, None, None, false, true, 500.0, Some(&mut c)).close, "推了 100 ≥ 80：关");
        let fixed = TwoLevelConfig { scroll_enabled: false, ..tl };
        assert!(!step(&fixed, RefresherPhase::TwoLevel, None, None, false, true, 400.0, Some(&mut c)).close, "不许拖时松手不关");
    }
}
