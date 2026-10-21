//! SDK 引擎的指针：宿主浮层先吃（picker 面板、操作菜单 / 图片预览、全屏视频、
//! 带蒙层的提示框、Modal）。
//!
//! 从 `engine_input.rs` 按职责拆出来的一片（`impl MiniEngine`）：拆的时候是纯搬迁（三段指针开头
//! 那几个「浮层弹着就到此为止」的分支原样搬过来），之后加了操作菜单与图片预览。浮层都画在合成那一步，
//! 它们的按压反馈只要重新合成，页面一个像素不用动。
use super::engine::MiniEngine;
use crate::host::picker_sheet;
use crate::host::ui_overlay;

impl MiniEngine {
    /// 按下：浮层吃掉了返回 true（调用方到此为止）
    pub(super) fn overlay_pointer_down(&mut self, x: f32, y: f32) -> bool {
        // picker 面板在最上层，先于弹窗/页面吃事件
        if picker_sheet::on_press(&mut self.picker_sheet, x, y) {
            self.dmg.compose_dirty = true;
            return true;
        }
        // 操作菜单 / 图片预览（ui_overlay::{action_sheet, image_preview}）
        if ui_overlay::sheet_overlay_press(self.viewport_logical(), x, y) {
            self.dmg.compose_dirty = true;
            return true;
        }
        // 全屏视频盖住整个页面：这串触摸归它（抬手时处理，见 overlay_pointer_up）
        if crate::renderer::components::fullscreen_video().is_some() {
            return true;
        }
        // 提示框的透明蒙层（`mask: true`）浮在 Modal 之上：连弹窗按钮也点不到。
        // 不带 mask 的 Toast / Loading 不挡，触摸照常往下走（与微信一致）
        if self.tip_blocks_input() {
            return true;
        }
        if self.modal.as_ref().map(|m| m.visible).unwrap_or(false) {
            let vp = self.viewport_logical();
            let sf = self.dpr;
            let Self { modal, text, .. } = self;
            if ui_overlay::modal_press(modal, vp, sf, text.as_deref(), x, y) {
                self.dmg.compose_dirty = true;
            }
            return true;
        }
        false
    }

    /// 移动：浮层吃掉了返回 true
    pub(super) fn overlay_pointer_move(&mut self, x: f32, y: f32) -> bool {
        // picker 面板弹着：拖动归滚轮，一点都不能漏到底下的页面
        match picker_sheet::on_move(&mut self.picker_sheet, x, y) {
            picker_sheet::SheetMove::NotOpen => {}
            picker_sheet::SheetMove::Idle => return true,
            picker_sheet::SheetMove::Moved => {
                self.dmg.compose_dirty = true;
                return true;
            }
        }
        if ui_overlay::sheet_overlay_move(x) {
            self.dmg.compose_dirty = true;
            return true;
        }
        crate::renderer::components::fullscreen_video().is_some()
    }

    /// 抬起：浮层吃掉了返回 true
    pub(super) fn overlay_pointer_up(&mut self, x: f32, y: f32) -> bool {
        // 面板 / 弹窗的回调里有 setData 的话，下一次 pump 照常取到数据脏
        if picker_sheet::on_release(&mut self.picker_sheet, &mut self.app, x, y) {
            self.page_data_dirty = true;
            self.dmg.compose_dirty = true;
            return true;
        }
        let vp = self.viewport_logical();
        if ui_overlay::sheet_overlay_release(&mut self.app, vp, x, y) {
            self.page_data_dirty = true;
            self.dmg.compose_dirty = true;
            return true;
        }
        // 全屏视频：左上角退出，别处切换播放 / 暂停（事件下一帧由 host::video::tick 发）
        if crate::renderer::components::fullscreen_tap(x, y) {
            self.dmg.compose_dirty = true;
            self.needs_redraw = true;
            return true;
        }
        if self.modal.as_ref().map(|m| m.visible).unwrap_or(false) {
            let vp = self.viewport_logical();
            let sf = self.dpr;
            let Self { modal, app, text, .. } = self;
            ui_overlay::modal_release(modal, app, vp, sf, text.as_deref(), x, y);
            self.dmg.compose_dirty = true;
            return true;
        }
        false
    }
}
