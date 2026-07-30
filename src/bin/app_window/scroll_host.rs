//! 滚动与下拉刷新的宿主侧：把控制器的位置变化接到出帧与页面回调上
//!
//! 从 `src/bin/window.rs` 拆出来的一片（`impl crate::MiniAppWindow`）。**纯搬迁**，
//! 一行逻辑没改；判据是 65 张画廊图与逐页整帧快照逐字节不变。
#![allow(clippy::too_many_arguments)]
use super::*;
use crate::*;

impl crate::MiniAppWindow {
    /// 当前页是否声明了 `enablePullDownRefresh`
    pub(crate) fn page_enables_pull_down(&self) -> bool {
        self.page_stack
            .last()
            .and_then(|p| self.pages.get(&p.path))
            .map(|info| info.enable_pull_down_refresh)
            .unwrap_or(false)
    }

    /// 进入/退出下拉刷新态。
    ///
    /// 进入时把内容按住在露出指示器的位置（等价 iOS 的 contentInset.top），
    /// 并回调 `onPullDownRefresh`；退出时收回 inset，内容回弹归位。
    pub(crate) fn set_pull_refreshing(&mut self, on: bool) {
        if self.pull_refreshing == on {
            return;
        }
        self.pull_refreshing = on;
        self.scroll
            .set_top_inset(if on { PULL_REFRESH_HEIGHT } else { 0.0 });
        self.needs_redraw = true;
        if on {
            println!("📜 onPullDownRefresh");
            self.app
                .eval("if(__currentPage && __currentPage.onPullDownRefresh) __currentPage.onPullDownRefresh()")
                .ok();
            print_js_output(&self.app);
        }
    }

    /// 处理逻辑层的 `wx.startPullDownRefresh` / `wx.stopPullDownRefresh`
    pub(crate) fn apply_pull_down_request(&mut self, request: Option<bool>) {
        match request {
            Some(true) => {
                if self.page_enables_pull_down() {
                    self.set_pull_refreshing(true);
                }
            }
            Some(false) => self.set_pull_refreshing(false),
            None => {}
        }
    }

    pub(crate) fn update_scroll(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        
        let before = self.scroll.get_position();
        let (animating, event) = self.scroll.update_with_events(dt);
        if (self.scroll.get_position() - before).abs() > 0.01 {
            self.last_scroll_at = Some(now);
        }
        if let Some(e) = event { evt::handle_scroll_event(e, &mut self.app); self.needs_redraw = true; }
        // `onPageScroll`：位置变了就回调一次（微信语义）。放在这里而不是各条输入路径里 ——
        // 拖动、惯性、回弹、滚轮、脚本设置位置最终都会经过这一帧检查，只写一处不会漏。
        let pos = self.scroll.get_position();
        if (pos - self.last_page_scroll_sent).abs() > 0.5 {
            self.last_page_scroll_sent = pos;
            evt::dispatch_page_scroll(&mut self.app, pos);
        }
        
        let mut changed = animating;
        for c in self.interaction.scroll_controllers.values_mut() { if c.update(dt) { changed = true; } }
        if self.tick_edge_back(dt) { changed = true; }
        if changed { if let Some(w) = &self.window { w.request_redraw(); } }
    }

    /// 让所有滚动动画（惯性、边界回弹）走完，最多等 `budget_ms`。
    ///
    /// 手势快照必须等这一步：松手瞬间位置多半还在越界区（橡皮筋），
    /// 直接截图会截到一张回弹中途的图 —— 同一条命令两次跑出来的还不一样。
    pub(crate) fn settle_scroll_animations(&mut self, budget_ms: u64) {
        let deadline = Instant::now() + Duration::from_millis(budget_ms);
        loop {
            let area_anim = self.interaction.scroll_controllers.values().any(|c| c.is_animating());
            // 侧滑返回的收尾动画也要等：它跑完才知道这一页是留下还是退栈
            let back_anim = self.edge_back.as_ref().map(|e| e.is_settling()).unwrap_or(false);
            if !self.scroll.is_animating() && !area_anim && !back_anim { break; }
            if Instant::now() >= deadline { break; }
            self.pump_one_frame();
            std::thread::sleep(Duration::from_millis(8));
        }
    }
}
