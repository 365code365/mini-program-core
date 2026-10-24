//! 覆盖层的交互入口：点击分流、Modal 按钮、picker 面板（判定都在 host::，这里只接窗口状态）
//!
//! 从 `src/bin/window.rs` 拆出来的一片（`impl crate::MiniAppWindow`）。**纯搬迁**，
//! 一行逻辑没改；判据是 65 张画廊图与逐页整帧快照逐字节不变。
#![allow(clippy::too_many_arguments)]
use super::*;
use crate::*;

impl crate::MiniAppWindow {
    /// 视口的逻辑尺寸（弹窗等覆盖层的居中算式要用）
    pub(crate) fn viewport_logical(&self) -> (f32, f32) {
        (LOGICAL_WIDTH as f32, LOGICAL_HEIGHT as f32)
    }

    /// 提示框里转圈的时钟：跟页面的 CSS 动画同一个（`--time` 定住时快照逐字节可比）
    pub(crate) fn overlay_clock(&self) -> f32 {
        self.renderer
            .as_ref()
            .map(|r| r.animation_clock())
            .unwrap_or_else(mini_render::renderer::anim::now_secs)
    }

    /// 提示框带 `mask: true`：透明蒙层挡住页面（不带 mask 的 Toast / Loading 可以点穿）
    pub(crate) fn tip_blocks_input(&self) -> bool {
        ui_overlay::tip_blocks_input(&self.toast, &self.loading)
    }

    pub(crate) fn handle_click(&mut self, x: f32, y: f32) {
        if self.picker_sheet.as_ref().map(|s| s.visible).unwrap_or(false) { return; }
        if self.tip_blocks_input() { return; }
        if self.modal.as_ref().map(|m| m.visible).unwrap_or(false) { self.handle_modal_click(x, y); return; }
        // 点在 picker 上：弹出选择面板，不再走普通内容点击
        if self.open_picker_if_hit(x, y) { return; }
        
        let page = match self.page_stack.last() { Some(p) => p, None => return };
        let has_tabbar = self.is_tabbar_page(&page.path);
        let tabbar_y = if has_tabbar { (LOGICAL_HEIGHT - tabbar_height()) as f32 } else { LOGICAL_HEIGHT as f32 };
        
        if has_tabbar && y >= tabbar_y {
            let nav = app_window::tabbar::nav_at(
                x, y - tabbar_y,
                if self.is_custom_tabbar() { self.tabbar_renderer.as_ref() } else { None },
                self.app_config.tab_bar.as_ref(),
                &page.path,
            );
            if let Some(n) = nav { self.pending_navigation = Some(n); if let Some(w) = &self.window { w.request_redraw(); } }
        } else {
            let tap_ctx = (self.touch.identifier(), self.event_time_ms());
            if let Some(nav) = click::handle_content_click(x, y, &self.scroll, has_tabbar, &mut self.interaction,
                self.renderer.as_ref(), &mut self.app, self.scale_factor, self.text_renderer.as_deref(), self.window.as_ref(), &mut self.clipboard, tap_ctx) {
                self.pending_navigation = Some(nav);
            }
            self.needs_redraw = true;
        }
    }

    pub(crate) fn handle_modal_press(&mut self, x: f32, y: f32) -> bool {
        let vp = self.viewport_logical();
        let sf = self.scale_factor as f32;
        let Self { modal, text_renderer, .. } = self;
        let consumed = ui_overlay::modal_press(modal, vp, sf, text_renderer.as_deref(), x, y);
        if consumed {
            self.needs_redraw = true;
            if let Some(w) = &self.window { w.request_redraw(); }
        }
        consumed
    }

    pub(crate) fn handle_modal_release(&mut self, x: f32, y: f32) {
        let vp = self.viewport_logical();
        let sf = self.scale_factor as f32;
        let Self { modal, app, text_renderer, .. } = self;
        ui_overlay::modal_release(modal, app, vp, sf, text_renderer.as_deref(), x, y);
        self.needs_redraw = true;
        if let Some(w) = &self.window { w.request_redraw(); }
    }

    pub(crate) fn handle_modal_click(&mut self, x: f32, y: f32) { self.handle_modal_release(x, y); }

    /// 点击落在某个 `<picker>` 上时弹出底部选择面板。返回是否弹出了面板。
    pub(crate) fn open_picker_if_hit(&mut self, x: f32, y: f32) -> bool {
        let sheet = picker_sheet::open_if_hit(
            self.renderer.as_ref(), self.scroll.get_position(), x, y,
        );
        let opened = sheet.is_some();
        if opened {
            self.picker_sheet = sheet;
            self.needs_redraw = true;
            if let Some(w) = &self.window { w.request_redraw(); }
        }
        opened
    }

    /// 面板可见时的按下：记录按压的头部按钮以给出反馈。返回是否消费了事件。
    pub(crate) fn handle_picker_sheet_press(&mut self, x: f32, y: f32) -> bool {
        // 操作菜单 / 图片预览、全屏视频盖住整个页面：这串触摸归它们（抬手时处理）
        let vp = (LOGICAL_WIDTH as f32, LOGICAL_HEIGHT as f32);
        let consumed = picker_sheet::on_press(&mut self.picker_sheet, x, y)
            || mini_render::host::ui_overlay::sheet_overlay_press(vp, x, y)
            || mini_render::renderer::components::video_fullscreen::fullscreen_press(x, y, vp.0, vp.1);
        if consumed {
            self.needs_redraw = true;
            if let Some(w) = &self.window { w.request_redraw(); }
        }
        consumed
    }

    /// 面板可见时的移动：滚轮跟手。返回是否消费了事件（面板弹着就一律吞掉）。
    pub(crate) fn handle_picker_sheet_move(&mut self, x: f32, y: f32) -> bool {
        match picker_sheet::on_move(&mut self.picker_sheet, x, y) {
            picker_sheet::SheetMove::NotOpen => {
                if mini_render::host::ui_overlay::sheet_overlay_move(x, y) {
                    self.needs_redraw = true;
                    if let Some(w) = &self.window { w.request_redraw(); }
                    return true;
                }
                // 全屏视频：拖进度跟手（全屏那一层画在合成那一步）
                let consumed = mini_render::renderer::components::video_fullscreen::fullscreen_move(x, y, LOGICAL_WIDTH as f32, LOGICAL_HEIGHT as f32);
                if consumed {
                    self.needs_redraw = true;
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
                consumed
            }
            picker_sheet::SheetMove::Idle => true,
            picker_sheet::SheetMove::Moved => {
                self.needs_redraw = true;
                if let Some(w) = &self.window { w.request_redraw(); }
                true
            }
        }
    }

    /// 面板可见时的抬手：确定/取消/选项滚动/点遮罩关闭。返回是否消费了事件。
    pub(crate) fn handle_picker_sheet_release(&mut self, x: f32, y: f32) -> bool {
        // 全屏视频：顶栏返回 / 控制条按钮 / 拖完进度 / 点画面切换控制条（事件下一帧由 host::video::tick 发）
        let vp = (LOGICAL_WIDTH as f32, LOGICAL_HEIGHT as f32);
        let picked = picker_sheet::on_release(&mut self.picker_sheet, &mut self.app, x, y);
        if picked {
            // 面板定下的 change / cancel / columnchange 经事件链派发（与 SDK 同一份 picker_sheet::emit）
            picker_sheet::emit::flush(&mut self.picker_sheet, &mut self.app, self.renderer.as_ref(), 0);
        }
        let consumed = picked
            || mini_render::host::ui_overlay::sheet_overlay_release(&mut self.app, vp, x, y)
            || mini_render::renderer::components::video_fullscreen::fullscreen_release(x, y, vp.0, vp.1);
        if consumed {
            self.needs_redraw = true;
            if let Some(w) = &self.window { w.request_redraw(); }
        }
        consumed
    }

    /// 退场动画走完后丢弃面板
    pub(crate) fn reap_picker_sheet(&mut self) {
        if picker_sheet::reap(&mut self.picker_sheet) {
            self.needs_redraw = true;
        }
    }
}
