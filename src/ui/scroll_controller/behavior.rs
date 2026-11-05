//! scroll-view 的手感开关：`paging-enabled`（松手吸到整页）、`fast-deceleration`（惯性衰减快）、
//! ScrollViewContext 的 `decelerationDisabled`（松手不带惯性），以及 `dragend` 要报的松手速度。
//!
//! 渲染器每次登记 scroll-view 时按属性 / Context 的写值调一次 [`ScrollController::set_behavior`]（只是几个字段赋值）。
//! 页面滚动的控制器从来不调，默认值与从前逐字节相同。

use super::*;

/// iOS `UIScrollView.DecelerationRate.fast`：每毫秒剩 0.99 的速度，折成每个 60Hz 帧 ≈ 0.846。
/// 普通速率沿用 [`INERTIA_DECAY_PER_60HZ_FRAME`]（引擎原有的手感，不动）
const FAST_DECAY_PER_60HZ_FRAME: f32 = 0.846;

/// 分页：松手速度超过这个（逻辑 px/s）就按方向翻一页，否则吸到最近的一页（与 swiper 同一个阈值）
const PAGE_FLICK_VELOCITY: f32 = 300.0;

/// 分页吸附的时长（秒）。官方没写，按 iOS `isPagingEnabled` 的观感取 0.3s（待验）
const PAGE_SNAP_SECS: f32 = 0.3;

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Behavior {
    paging: bool,
    fast: bool,
    no_inertia: bool,
    /// 最近一次松手时的速度（逻辑 px/s，正 = 位置增大）
    release_v: f32,
}

impl ScrollController {
    /// 分页 / 快速减速 / 关掉惯性（scroll-view 的 `paging-enabled` / `fast-deceleration`，Context 的 `decelerationDisabled`）
    pub fn set_behavior(&mut self, paging: bool, fast: bool, no_inertia: bool) {
        self.behavior.paging = paging;
        self.behavior.fast = fast;
        self.behavior.no_inertia = no_inertia;
    }

    /// 最近一次松手时的速度（逻辑 px/s，正 = scrollTop 变大的方向）：`dragend` 的 `velocity`
    pub fn release_velocity(&self) -> f32 {
        self.behavior.release_v
    }

    /// 惯性每个 60Hz 帧剩下的速度比例
    pub(super) fn decay_per_frame(&self) -> f32 {
        if self.behavior.fast {
            FAST_DECAY_PER_60HZ_FRAME
        } else {
            INERTIA_DECAY_PER_60HZ_FRAME
        }
    }

    /// `end_drag` 算完松手速度、定了惯性 / 回弹之后调：分页吸附、关惯性
    pub(super) fn after_release(&mut self) {
        self.behavior.release_v = self.velocity;
        if self.is_bouncing {
            // 拖过了边界：照常弹回边界（边界本来就在整页上 / 末页）
            return;
        }
        if self.behavior.paging {
            let page = self.viewport_size.max(1.0);
            let start = (self.drag_start_scroll / page).round();
            let here = self.position / page;
            let mut target = if self.velocity > PAGE_FLICK_VELOCITY {
                here.floor() + 1.0
            } else if self.velocity < -PAGE_FLICK_VELOCITY {
                here.ceil() - 1.0
            } else {
                here.round()
            };
            // 一次最多翻一页（与 iOS 一致）
            target = target.clamp(start - 1.0, start + 1.0);
            self.is_decelerating = false;
            self.velocity = 0.0;
            self.animate_to(target * page, PAGE_SNAP_SECS);
        } else if self.behavior.no_inertia {
            self.is_decelerating = false;
            self.velocity = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fling(c: &mut ScrollController, from: f32, to: f32, ms: u64) {
        c.begin_drag(from, 0);
        let steps = 6;
        for i in 1..=steps {
            let y = from + (to - from) * i as f32 / steps as f32;
            c.update_drag(y, ms * i as u64 / steps as u64);
        }
        c.end_drag();
    }

    fn settle(c: &mut ScrollController) {
        for _ in 0..240 {
            c.update(1.0 / 60.0);
        }
    }

    #[test]
    fn 分页_慢拖吸到最近一页_快甩翻一页() {
        let mut c = ScrollController::new(1000.0, 200.0);
        c.set_behavior(true, false, false);
        fling(&mut c, 300.0, 230.0, 600);
        settle(&mut c);
        assert_eq!(c.get_position(), 0.0, "拖了 70（不到半页）慢慢松手：回到第 0 页");
        fling(&mut c, 300.0, 200.0, 600);
        settle(&mut c);
        assert_eq!(c.get_position(), 200.0, "拖过半页：第 1 页");
        fling(&mut c, 300.0, 260.0, 60);
        settle(&mut c);
        assert_eq!(c.get_position(), 400.0, "快甩一小段：按方向翻一页");
        assert!(c.release_velocity() > 300.0, "{}", c.release_velocity());
    }

    #[test]
    fn 快速减速走得比普通近_关惯性松手就停() {
        let run = |fast: bool, off: bool| {
            let mut c = ScrollController::new(100000.0, 600.0);
            c.set_behavior(false, fast, off);
            fling(&mut c, 500.0, 300.0, 60);
            let at_release = c.get_position();
            settle(&mut c);
            (at_release, c.get_position())
        };
        let (_, normal) = run(false, false);
        let (_, fast) = run(true, false);
        let (rel, off) = run(false, true);
        assert!(fast < normal - 100.0, "fast {fast} normal {normal}");
        assert_eq!(off, rel, "decelerationDisabled：松手停在原地");
    }
}
