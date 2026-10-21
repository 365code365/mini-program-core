//! 滚轮 / 触控板：越界橡皮筋、抬手惯性、滚轮停下后的回弹兜底。
//!
//! 从 `scroll_controller.rs` 拆出来的一片（**纯搬迁**，只把私有方法改成 `pub(super)`）。

use super::*;

impl ScrollController {
    /// 滚轮/触控板停下后自动回弹：越界且静默超过阈值就开始回弹。
    /// 触控板通常会给 `TouchPhase::Ended`，但鼠标滚轮不会，这里统一兜底。
    pub(super) fn tick_wheel_bounce(&mut self, dt: f32) -> bool {
        if self.is_dragging || self.is_bouncing || self.is_decelerating || !self.is_out_of_bounds() {
            return false;
        }
        self.idle_after_wheel += dt;
        if self.idle_after_wheel >= 0.06 {
            self.start_bounce();
        }
        true
    }

    /// 滚轮 / 触控板滚动。
    ///
    /// 与拖拽走同一套语义：越界不硬夹，而是经橡皮筋衰减，手势停下后回弹。
    /// 触控板抬手由 [`Self::end_wheel_gesture`] 通知；鼠标滚轮没有抬手事件，
    /// 靠 `update()` 里的静默计时兜底。
    pub fn handle_scroll(&mut self, delta: f32, is_precise: bool) {
        // 忽略极微小的滚动事件
        if delta.abs() < 0.1 {
            return;
        }
        // 不可滚动的页面仍然允许越界橡皮筋（微信里内容不满一屏也能拉出一点再弹回），
        // 但不做任何位移积累
        self.is_decelerating = false;
        self.is_bouncing = false;
        self.velocity = 0.0;
        self.idle_after_wheel = 0.0;

        // 触控板一次事件就是一次真实位移；滚轮是脉冲，放大到接近一"格"的观感
        let step = if is_precise { delta } else { delta * 2.0 };
        if is_precise {
            let now = std::time::Instant::now();
            let total = self.wheel_samples.last().map(|s| s.0).unwrap_or(0.0) + step;
            self.wheel_samples.retain(|(_, t)| now.duration_since(*t).as_millis() < 100);
            self.wheel_samples.push((total, now));
        } else {
            self.wheel_samples.clear();
        }
        // 越界方向上先把累计位置对齐到当前显示位置，避免来回切换方向时跳变
        if self.wheel_raw > self.min_bound() && self.wheel_raw < self.max_scroll {
            self.wheel_raw = self.position;
        }
        self.wheel_raw += step;
        self.position = self.apply_rubber_band(self.wheel_raw);
        if self.position < self.min_bound() {
            self.was_over_top = true;
        } else if self.position > self.max_scroll {
            self.was_over_bottom = true;
        }
        self.max_over_top = self.max_over_top.max(self.top_overscroll());
    }

    /// 触控板手势结束（`TouchPhase::Ended`）：越界则回弹，否则按抬手前的速度进入惯性。
    ///
    /// 惯性由这里自己跑（与手指拖动松手同一套减速），宿主应当丢弃系统随后送来的
    /// 惯性滚轮事件：macOS 的惯性是按桌面大窗口调的，在手机大小的视口里一甩就是好几屏。
    pub fn end_wheel_gesture(&mut self) -> bool {
        self.wheel_raw = self.position;
        self.settle_pull_gesture();
        let velocity = self.wheel_release_velocity();
        self.wheel_samples.clear();
        if self.is_out_of_bounds() {
            self.start_bounce();
            return true;
        }
        if velocity.abs() > 50.0 {
            self.velocity = velocity;
            self.is_decelerating = true;
            return true;
        }
        false
    }

    /// 抬手前 100ms 内的触控板平均速度（px/s），与拖动松手同样打 0.8 折
    pub(super) fn wheel_release_velocity(&self) -> f32 {
        let now = std::time::Instant::now();
        let recent: Vec<&(f32, std::time::Instant)> = self
            .wheel_samples
            .iter()
            .filter(|(_, t)| now.duration_since(*t).as_millis() < 100)
            .collect();
        let (Some(first), Some(last)) = (recent.first(), recent.last()) else { return 0.0 };
        if recent.len() < 2 {
            return 0.0;
        }
        let dt = last.1.duration_since(first.1).as_secs_f32();
        if dt < 0.008 {
            return 0.0;
        }
        (last.0 - first.0) / dt * 0.8
    }
}
