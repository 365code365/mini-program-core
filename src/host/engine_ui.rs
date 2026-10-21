//! [`MiniEngine`] 的覆盖层状态：逻辑层发来的 Toast / Loading / Modal 指令落地，
//! 以及宿主要问的「现在有没有弹窗盖着」。
//!
//! 从 `engine.rs` 按职责拆出来的一片（`impl MiniEngine`）。拆出来时是纯搬迁；之后改过：
//! 覆盖层状态变化只标「重新合成」（`dmg.compose_dirty`），不再整页重画；
//! 指令怎么落到状态上（Toast 与 Loading 共用一个槽位、hide 的「混用」）改由
//! `ui_overlay::apply_feedback_event` 决定，与桌面窗体同一份。
//! 覆盖层的**绘制**不在这里（`host::ui_overlay` / `host::picker_sheet`，与桌面窗体共用），
//! 这里只管状态。
use super::engine::MiniEngine;
use crate::host::ui_overlay;

impl MiniEngine {
    /// 当前是否有 `<picker>` 的底部面板 / `wx.showModal` 的弹窗盖着。
    ///
    /// 宿主的返回键要看它：微信里弹窗盖着时按返回是**关弹窗**，不是退页面。
    pub fn picker_sheet_open(&self) -> bool {
        self.picker_sheet.as_ref().map(|s| s.visible && !s.closing).unwrap_or(false)
    }

    pub fn modal_open(&self) -> bool {
        self.modal.as_ref().map(|m| m.visible).unwrap_or(false)
    }

    /// 提示框带 `mask: true`：页面的触摸到此为止
    pub(super) fn tip_blocks_input(&self) -> bool {
        ui_overlay::tip_blocks_input(&self.toast, &self.loading)
    }

    /// 提示框里转圈的时钟：与 CSS 动画同一个（`set_animation_time` 定住时快照逐字节可比）
    pub(super) fn overlay_clock(&self) -> f32 {
        self.anim_time.unwrap_or_else(crate::renderer::anim::now_secs)
    }

    /// Toast 到时收起（与桌面窗体 `update_toast_timeout` 同一规则）。返回是否收了。
    pub(super) fn expire_toast(&mut self) -> bool {
        let expired = self
            .toast
            .as_ref()
            .map(|t| t.visible && t.start_time.elapsed().as_millis() as u32 >= t.duration_ms)
            .unwrap_or(false);
        if expired {
            self.toast = None;
        }
        expired
    }

    pub(super) fn drain_ui_events(&mut self) {
        for e in self.app.drain_ui_events() {
            // 下拉刷新的开始 / 结束 SDK 不画指示器（宿主自己有），其余都落到覆盖层状态上
            let _ = ui_overlay::apply_feedback_event(e, &mut self.toast, &mut self.loading, &mut self.modal);
            // Toast / Loading / Modal 画在合成那一步：重新合成就够，页面不用重画
            self.dmg.compose_dirty = true;
        }
    }
}
