//! 下拉刷新（顶部 inset、触发信号、进度）与触底检测。
//!
//! 从 `scroll_controller.rs` 拆出来的一片（**纯搬迁**，只把私有方法改成 `pub(super)`）。

use super::*;

impl ScrollController {
    /// 进入/退出下拉刷新态：`inset > 0` 时把内容按住露出指示器，回弹目标随之改变。
    pub fn set_top_inset(&mut self, inset: f32) {
        let inset = inset.max(0.0);
        if (self.top_inset - inset).abs() < 0.01 {
            return;
        }
        let growing = inset > self.top_inset;
        self.top_inset = inset;
        if self.is_dragging {
            return;
        }
        let min_bound = self.min_bound();
        let target = if growing {
            // 进入刷新：把内容拉到露出指示器的位置。
            // 只在本来就贴着顶部时才拉 —— 页面滚到中间时 `wx.startPullDownRefresh()`
            // 不该把画面跳回顶部。
            if self.position <= self.min_scroll + 1.0 { min_bound } else { self.position }
        } else {
            // 结束刷新：内容归位
            self.position.clamp(min_bound, self.max_scroll.max(min_bound))
        };
        if (target - self.position).abs() > 0.01 {
            // 下拉刷新的进出场同样用弹簧（初速度 0：这是程序触发的位移，不是手势）
            self.begin_spring(target, 0.0);
        }
    }

    /// 取走「下拉到位并松手」信号（读后清零）
    pub fn take_pull_trigger(&mut self) -> bool {
        std::mem::take(&mut self.pull_triggered)
    }

    /// 当前顶部越界量（逻辑 px，未越界为 0）。宿主用它画下拉指示器的进度。
    pub fn top_overscroll(&self) -> f32 {
        (self.min_bound() - self.position).max(0.0)
    }

    /// 顶部露出的空白高度（逻辑 px）：下拉过程与刷新期间都用它定位指示器。
    /// 注意不能用 `top_overscroll()` —— 刷新期内容正好停在 `-top_inset`，不算越界。
    pub fn top_gap(&self) -> f32 {
        (self.min_scroll - self.position).max(0.0)
    }

    /// 下拉进度（0~1）：到 1 表示松手就会触发刷新
    pub fn pull_progress(&self) -> f32 {
        (self.top_gap() / self.pull_threshold).clamp(0.0, 1.0)
    }

    /// 手势结束时判定是否够触发下拉刷新
    pub(super) fn settle_pull_gesture(&mut self) {
        if self.max_over_top >= self.pull_threshold && self.top_inset <= 0.01 {
            self.pull_triggered = true;
        }
        self.max_over_top = 0.0;
    }
    
    /// 检查是否应该触发触底事件（用于触控板/鼠标滚轮滚动）
    pub fn check_reach_bottom(&mut self) -> bool {
        if self.max_scroll <= 0.0 {
            return false;
        }
        
        if self.position >= self.max_scroll - self.reach_bottom_distance && !self.reach_bottom_triggered {
            self.reach_bottom_triggered = true;
            return true;
        }
        false
    }
    
    /// 重置触底状态（当内容更新后调用）
    pub fn reset_reach_bottom(&mut self) {
        self.reach_bottom_triggered = false;
    }
}
