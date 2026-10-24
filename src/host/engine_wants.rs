//! [`MiniEngine`] 要不要续帧（`MR_FRAME_WANTS_MORE`）。
//!
//! 从 `engine_frame.rs` 原样搬出（那边超了 500 行）：一个函数，一行没改。
use super::engine::MiniEngine;

impl MiniEngine {
    /// 宿主是否该在下一个 vsync 再 pump 一次（`MR_FRAME_WANTS_MORE`）。
    ///
    /// 与桌面窗体 `is_animating` 同一组条件：滚动 / 惯性、scroll-view、CSS 动画、
    /// swiper 自动播放、JS 定时器、网络请求与远程图片在飞、逻辑层没办完的宿主活
    /// （画布指令、没答的 SelectorQuery）、Toast（要到时收起）、Loading（圈在转）、
    /// picker 面板动画、播放中的视频、看得见的动图、聚焦的输入框（光标闪烁），外加**按住不动的手指**
    /// （长按靠帧驱动，手指不动就没有输入事件进来）。
    ///
    /// 返回 false 时宿主可以停掉 vsync 回调，等下一个输入事件再 pump —— 静止页面零耗电。
    /// 从前没有这个信号：Android 的 View 只在 `pump` 返回「有新画面」时续帧，
    /// `setTimeout` / `wx.request` 回包 / swiper 自动播放在静止页面上全部停住。
    pub fn wants_next_frame(&self) -> bool {
        use crate::renderer::components as c;
        self.scroll.is_animating()
            || self.scroll.is_dragging
            || self.interaction.scroll_controllers.values().any(|c| c.is_animating() || c.is_dragging)
            || self.renderer.as_ref().map(|r| r.has_active_animations()).unwrap_or(false)
            || self.tabbar_renderer.as_ref().map(|r| r.has_active_animations()).unwrap_or(false)
            || self.interaction.has_focused_input()
            || self.interaction.press_feedback_pending()
            || self.touch.is_active()
            || c::has_autoplay_swiper()
            || c::swiper_needs_frame()
            || self.app.has_active_timers()
            || self.app.has_pending_network()
            // 逻辑层发了、还没办完的：画布指令、没答的 SelectorQuery（type="2d" 画布靠它拿节点）
            || self.app.has_pending_host_work()
            || c::image_net::has_pending()
            || self.toast.as_ref().map(|t| t.visible).unwrap_or(false)
            // Loading 的圈要一直转（`wx.showLoading` 没有时长，等逻辑层 hideLoading）
            || crate::host::ui_overlay::tip_spinning(&self.toast, &self.loading)
            || self.picker_sheet.as_ref().map(|s| s.animating()).unwrap_or(false)
            || crate::host::ui_overlay::sheet_overlay_wants_frames()
            || self.edge_back.as_ref().map(|e| e.is_settling()).unwrap_or(false)
            || c::has_playing_video()
            // 看得见的动图：醒着等它换帧（多数帧问下来不用重画，见 engine_damage::plan_redraw）
            || self.animated_images_visible()
            // surface 这次没取到：宿主不续帧的话 PENDING 那一帧永远补不上
            || self.present_pending
            // 导航栏前景色在这一帧末尾才改：胶囊换配色要再来一帧
            || self.capsule_wants_frame()
            // 页面转场在走、引擎画的导航栏在转圈或还欠一次重画
            || self.route.active()
            || self.nav_wants_frame()
            // 报给宿主的滚动速度还不是 0：再出一帧把「停了」报出去（native_motion.rs），
            // 否则宿主按最后那个速度把视频框一直往前推
            || self.events.motion.unsettled
    }
}
