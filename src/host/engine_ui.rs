//! [`MiniEngine`] 的覆盖层状态：逻辑层发来的 Toast / Loading / Modal 指令落地，
//! 以及宿主要问的「现在有没有弹窗盖着」。
//!
//! 从 `engine.rs` 按职责拆出来的一片（`impl MiniEngine`）。**纯搬迁**，一行逻辑没改。
//! 覆盖层的**绘制**不在这里（`host::ui_overlay` / `host::picker_sheet`，与桌面窗体共用），
//! 这里只管状态。
use super::engine::MiniEngine;
use crate::host::ui_overlay::{LoadingState, ModalState, ToastState};
use crate::runtime::UiEvent;

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

    pub(super) fn drain_ui_events(&mut self) {
        for e in self.app.drain_ui_events() {
            match e {
                UiEvent::ShowToast { title, icon, duration } => {
                    self.toast = Some(ToastState {
                        title,
                        icon,
                        visible: true,
                        start_time: std::time::Instant::now(),
                        duration_ms: duration,
                    });
                }
                UiEvent::HideToast => self.toast = None,
                UiEvent::ShowLoading { title } => {
                    self.loading = Some(LoadingState { title, visible: true })
                }
                UiEvent::HideLoading => self.loading = None,
                UiEvent::ShowModal { title, content, show_cancel, cancel_text, confirm_text } => {
                    self.modal = Some(ModalState {
                        title,
                        content,
                        show_cancel,
                        cancel_text,
                        confirm_text,
                        visible: true,
                        pressed_button: None,
                    });
                }
                UiEvent::HideModal => self.modal = None,
                UiEvent::StartPullDownRefresh | UiEvent::StopPullDownRefresh => {}
            }
            self.needs_redraw = true;
        }
    }
}
