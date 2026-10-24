//! 预览层的动效：开 / 合（缩放 + 淡入淡出）、左右切换（临界阻尼弹簧，接住松手时的速度）、
//! 下拉关闭（跟手缩小、背景变透明；没拉够就回弹）。
//!
//! 全按时间的闭式解求值（不逐帧积分）：同一时刻画几次结果都一样，帧间隔忽大忽小也不会抖
//! （时刻用 `frame_clock::now()`，SDK 上就是 vsync）。
//!
//! 为什么切换用弹簧不用固定时长的缓动：松手时手指还在动，固定时长的缓动从速度 0 起步，
//! 图片会先「停一下」再滑走 —— 用户说的「太僵硬」。弹簧以松手速度为初速，手放开那一刻衔接得上。
use std::collections::VecDeque;
use std::time::Instant;

/// 打开：从 0.86 倍放大到 1、淡入
pub const OPEN_SECS: f32 = 0.28;
/// 关闭：在当前位置缩小、淡出
pub const CLOSE_SECS: f32 = 0.22;
/// 下拉没拉够：回到原位
pub const BACK_SECS: f32 = 0.26;
/// 切换弹簧的角频率：约 0.3s 停稳（微信的切换在 0.3s 上下）
const OMEGA: f32 = 20.0;
/// 打开时的起始缩放
pub const OPEN_FROM_SCALE: f32 = 0.86;

pub fn ease_out_cubic(t: f32) -> f32 {
    let u = 1.0 - t.clamp(0.0, 1.0);
    1.0 - u * u * u
}

/// 临界阻尼弹簧：相对目标的偏移 `x0`、初速 `v0` 起步，`t` 秒后的（偏移, 速度）
pub fn spring(x0: f32, v0: f32, t: f32) -> (f32, f32) {
    let e = (-OMEGA * t).exp();
    let c = v0 + OMEGA * x0;
    ((x0 + c * t) * e, (v0 - OMEGA * c * t) * e)
}

/// 弹簧停稳了（偏移不到千分之一屏、速度很小）
pub fn spring_done(x: f32, v: f32) -> bool {
    x.abs() < 0.001 && v.abs() < 0.02
}

/// 第一张往右拉 / 最后一张往左拉：越拉越紧，最多半屏
pub fn rubber(over: f32) -> f32 {
    over.signum() * over.abs() * 0.5 / (1.0 + over.abs())
}

/// 这一帧画成什么样
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// 第几张在正中（小数 = 两张之间）
    pub pos: f32,
    /// 当前那张的缩放（开合、下拉）
    pub scale: f32,
    /// 当前那张的位移（逻辑像素，下拉时跟手）
    pub tx: f32,
    pub ty: f32,
    /// 黑底不透明度（0 = 看得见页面）
    pub bg: f32,
    /// 图的不透明度
    pub alpha: f32,
}

impl Look {
    pub fn at(pos: f32) -> Self {
        Self { pos, scale: 1.0, tx: 0.0, ty: 0.0, bg: 1.0, alpha: 1.0 }
    }

    /// 打开到第 `t` 秒
    pub fn opening(mut self, t: f32) -> Self {
        let e = ease_out_cubic(t / OPEN_SECS);
        self.scale *= OPEN_FROM_SCALE + (1.0 - OPEN_FROM_SCALE) * e;
        self.bg *= e;
        self.alpha *= (e * 1.25).min(1.0);
        self
    }

    /// 从 `self`（按下关闭那一刻的样子）关到第 `t` 秒
    pub fn closing(mut self, t: f32) -> Self {
        let e = ease_out_cubic(t / CLOSE_SECS);
        self.scale *= 1.0 - 0.14 * e;
        self.bg *= 1.0 - e;
        self.alpha *= 1.0 - e;
        self
    }

    /// 下拉跟手：往下拉了 `dy`、横向 `dx`（逻辑像素），视口高 `vh`
    pub fn dragged_down(mut self, dx: f32, dy: f32, vh: f32) -> Self {
        let k = (dy.max(0.0) / vh.max(1.0)).min(1.0);
        self.tx = dx;
        // 往上拉不关：给一点阻力，松手回去
        self.ty = if dy >= 0.0 { dy } else { dy * 0.25 };
        self.scale *= 1.0 - 0.45 * k;
        self.bg *= 1.0 - (k * 2.2).min(1.0) * 0.92;
        self
    }
}

/// 手指最近 100ms 的轨迹：算松手速度
#[derive(Clone, Debug, Default)]
pub struct Tracker {
    samples: VecDeque<(Instant, f32, f32)>,
}

impl Tracker {
    pub fn push(&mut self, t: Instant, x: f32, y: f32) {
        self.samples.push_back((t, x, y));
        while self.samples.len() > 2 && t.duration_since(self.samples[0].0).as_secs_f32() > 0.1 {
            self.samples.pop_front();
        }
    }

    /// 松手那一刻 `now` 的（vx, vy），逻辑像素 / 秒。
    ///
    /// 只看松手前 [`VELOCITY_WINDOW`] 里的那几个点：手指拉下去又推回来、停了一会儿才松，按整段首尾算的话
    /// 是一个朝下的大速度 —— 没拉够也被当成「往下甩」关掉了（机器一忙、事件挤得稀，这种误判就冒出来）。
    /// 最后一个点离松手已经超过 [`STILL_SECS`]：手指是停着松开的，速度 0。点挤在一起（不到 12ms）也给 0：按距离判
    pub fn velocity(&self, now: Instant) -> (f32, f32) {
        let Some(last) = self.samples.back() else { return (0.0, 0.0) };
        if now.saturating_duration_since(last.0).as_secs_f32() > STILL_SECS {
            return (0.0, 0.0);
        }
        let Some(first) = self.samples.iter().find(|s| last.0.duration_since(s.0).as_secs_f32() <= VELOCITY_WINDOW) else {
            return (0.0, 0.0);
        };
        let dt = last.0.duration_since(first.0).as_secs_f32();
        if dt < 0.012 {
            return (0.0, 0.0);
        }
        ((last.1 - first.1) / dt, (last.2 - first.2) / dt)
    }
}

/// 松手速度只看最后这么长（秒）
const VELOCITY_WINDOW: f32 = 0.06;
/// 最后一次移动到松手超过这么久（秒）算停着松开
const STILL_SECS: f32 = 0.08;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 弹簧从松手速度起步_约三分之一秒停稳且不过冲() {
        // 往左甩：位置还差 0.4 屏，速度 -3 屏/秒（朝目标走）
        let (x0, v0) = (0.4, -3.0);
        let (_, v) = spring(x0, v0, 0.0);
        assert!((v - v0).abs() < 1e-4, "起步速度就是松手速度");
        let (x, v) = spring(x0, v0, 0.35);
        assert!(spring_done(x, v) || x.abs() < 0.003, "0.35s 基本停稳：{x} {v}");
        let min = (0..100).map(|i| spring(x0, v0, i as f32 * 0.01).0).fold(f32::MAX, f32::min);
        assert!(min > -0.02, "临界阻尼不该明显过冲：{min}");
    }

    #[test]
    fn 松手速度只看最后一小段_停住再松是零() {
        let t0 = Instant::now();
        let ms = |m: u64| t0 + std::time::Duration::from_millis(m);
        let mut tr = Tracker::default();
        // 往下拉 200，又推回 160（每 10ms 一个点）
        for i in 0..=10 {
            tr.push(ms(i * 10), 0.0, i as f32 * 20.0);
        }
        for i in 1..=4 {
            tr.push(ms(100 + i * 10), 0.0, 200.0 - i as f32 * 40.0);
        }
        let (_, vy) = tr.velocity(ms(140));
        assert!(vy < 0.0, "最后那段是往上推：{vy}");
        assert_eq!(tr.velocity(ms(300)), (0.0, 0.0), "停了 160ms 才松：速度 0");
    }

    #[test]
    fn 拉过头有阻力且有上限() {
        assert!(rubber(0.2) < 0.2 && rubber(0.2) > 0.0);
        assert!(rubber(10.0) < 0.5);
        assert!(rubber(-0.2) < 0.0);
    }

    #[test]
    fn 开合首尾两端() {
        let a = Look::at(0.0).opening(0.0);
        assert!(a.bg == 0.0 && a.alpha == 0.0 && (a.scale - OPEN_FROM_SCALE).abs() < 1e-6);
        assert_eq!(Look::at(0.0).opening(OPEN_SECS), Look::at(0.0));
        let c = Look::at(0.0).closing(CLOSE_SECS);
        assert!(c.bg == 0.0 && c.alpha == 0.0);
    }
}
