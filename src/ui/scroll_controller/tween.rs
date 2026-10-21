//! 程序发起的定时滚动：`scroll-view` 的 `scroll-with-animation`（改 `scroll-top` /
//! `scroll-into-view` 时滑过去）、`wx.pageScrollTo({ duration })`。
//!
//! 这不是手势：没有初速度、不越界、不出橡皮筋，按给定时长从当前位置缓动到目标。
//! 缓动用 ease-in-out（三次）：官方文档只给了时长（`pageScrollTo` 默认 300ms），没写曲线；
//! 选「两头慢」是因为程序滚动的起点是静止的，ease-out 会让起步那一帧跳一下。
//!
//! 手指按上来（`begin_drag`）、`set_position` / `stop` 都会打断它。

use super::*;

#[derive(Debug, Clone, Copy)]
pub(super) struct Tween {
    from: f32,
    to: f32,
    elapsed: f32,
    duration: f32,
}

impl ScrollController {
    /// 用 `duration_s` 秒滚到 `target`（夹在可滚范围内）。时长不大于 0 时直接跳过去。
    pub fn animate_to(&mut self, target: f32, duration_s: f32) {
        let min = self.min_bound();
        let target = target.clamp(min, self.max_scroll.max(min));
        if duration_s <= 0.0 || (target - self.position).abs() < 0.01 {
            self.set_position(target);
            return;
        }
        let from = self.position;
        // 停掉惯性 / 回弹（`set_position` 会顺带清掉 tween，所以先清再装）
        self.set_position(from);
        self.tween = Some(Tween { from, to: target, elapsed: 0.0, duration: duration_s });
    }

    /// 是否正在做程序滚动
    pub fn is_tweening(&self) -> bool {
        self.tween.is_some()
    }

    /// 推进一步。没有 tween 返回 None（调用方照常处理惯性 / 回弹）；
    /// 有则返回是否还在动（走完那一帧返回 `Some(false)`）
    pub(super) fn tick_tween(&mut self, dt: f32) -> Option<bool> {
        let mut t = self.tween?;
        t.elapsed += dt.max(0.0);
        let p = (t.elapsed / t.duration).clamp(0.0, 1.0);
        let eased = if p < 0.5 { 4.0 * p * p * p } else { 1.0 - (-2.0 * p + 2.0).powi(3) / 2.0 };
        self.position = t.from + (t.to - t.from) * eased;
        self.wheel_raw = self.position;
        if p >= 1.0 {
            self.position = t.to;
            self.wheel_raw = t.to;
            self.tween = None;
            return Some(false);
        }
        self.tween = Some(t);
        Some(true)
    }

    /// 边界弹性开关（scroll-view 的 `bounces`，默认开）
    pub fn set_bounces(&mut self, on: bool) {
        self.bounces = on;
    }

    /// 下拉刷新的触发阈值（逻辑 px；页面是 64，scroll-view 的 `refresher-threshold` 默认 45）
    pub fn set_pull_threshold(&mut self, px: f32) {
        self.pull_threshold = px.max(1.0);
    }

    /// 当前的下拉刷新阈值（逻辑 px）
    pub fn pull_threshold(&self) -> f32 {
        self.pull_threshold
    }
}
