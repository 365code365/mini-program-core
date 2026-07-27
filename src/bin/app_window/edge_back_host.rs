//! 左边缘侧滑返回的宿主胶水：给被覆盖的页留一张视口图，并推进松手后的收尾动画。
//!
//! 判定与合成都在 [`super::edge_back`]（纯逻辑 + 纯像素搬运），这里只做两件
//! 离不开宿主状态的事：
//!
//! - **留图**：侧滑时下面那一页要真的显示出来，而那时它已经不是当前页 ——
//!   渲染器、交互表、滚动位置全换成新页了，没法重新画。所以在它被覆盖的那一刻
//!   用 `present_to_buffer` 合成一张，和真正上屏的结果逐像素一致。
//! - **收尾**：动画跑完且判定为「返回」时真的退栈（与 `wx.navigateBack()` 同一条路径）。

use super::*;
use crate::app_window;

impl crate::MiniAppWindow {
    /// 上屏缓冲的像素尺寸（无头模式下没有窗体，按逻辑尺寸 × 缩放推算）
    pub(crate) fn viewport_pixels(&self) -> (u32, u32) {
        if let Some(w) = &self.window {
            let s = w.inner_size();
            if s.width > 0 && s.height > 0 {
                return (s.width, s.height);
            }
        }
        (
            (LOGICAL_WIDTH as f64 * self.scale_factor) as u32,
            (LOGICAL_HEIGHT as f64 * self.scale_factor) as u32,
        )
    }

    /// 把当前这一帧（页面 + tabBar + fixed 覆盖层）合成到一张离屏像素表。
    ///
    /// 侧滑返回时下面那一页必须真的显示出来，而那时它已经不是当前页 ——
    /// 渲染器、交互表、滚动位置全都换成新页了，没法重新画。所以在**它被覆盖的那一刻**
    /// 留一张图；用的是 `present_to_buffer` 本身，和真正上屏的合成结果逐像素一致。
    pub(crate) fn capture_viewport(&self) -> Option<app_window::edge_back::PageShot> {
        let canvas = self.canvas.as_ref()?;
        let page = self.page_stack.last()?;
        let has_tabbar = self.is_tabbar_page(&page.path);
        let (w, h) = self.viewport_pixels();
        if w == 0 || h == 0 {
            return None;
        }
        let present_bg = self
            .renderer
            .as_ref()
            .and_then(|r| r.page_style().background)
            .map(|c| ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32)
            .unwrap_or(0xF5F5F5);
        let mut pixels = vec![present_bg; (w as usize) * (h as usize)];
        present_to_buffer(
            &mut pixels, w, h, canvas, self.fixed_canvas.as_ref(), self.tabbar_canvas.as_ref(),
            (self.scroll.get_position() * self.scale_factor as f32) as i32, has_tabbar,
            if has_tabbar { (tabbar_height() as f64 * self.scale_factor) as u32 } else { 0 },
            self.fixed_rows, present_bg,
        );
        Some(app_window::edge_back::PageShot { width: w, height: h, pixels })
    }

    // ── 无头输入：与交互窗体走同一条链路 ──
    //
    // 脚本化的输入必须复用真实链路（触摸状态机 + 滚动/交互处理 + 事件派发），
    // 否则「无头能过、真机不行」——测的就不是同一套东西。

    /// 推进左边缘侧滑返回的收尾动画；动画结束且判定为「返回」时真的退栈。
    /// 返回是否还需要继续出帧。
    pub(crate) fn tick_edge_back(&mut self, dt: f32) -> bool {
        let Some(eb) = &mut self.edge_back else { return false };
        if !eb.is_settling() {
            return false; // 还在跟手，帧由输入事件驱动
        }
        match eb.tick(dt) {
            None => true,
            Some(commit) => {
                self.edge_back = None;
                if commit {
                    self.navigate_back().ok();
                    self.update_renderers();
                }
                // 收手这一帧必须整帧重绘：位移归零，上屏不再挪，画布内容也换了页
                self.needs_redraw = true;
                self.fixed_dirty = true;
                true
            }
        }
    }
}
