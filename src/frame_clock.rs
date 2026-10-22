//! 帧时钟：这一帧「按哪个时刻画」。
//!
//! 动画（操作菜单 / picker 面板的滑入、图片预览淡入、侧滑收尾、CSS `@keyframes`、Loading 转圈）
//! 从前在**画的那一刻**取 `Instant::now()`。画的时刻相对 vsync 并不固定（这一帧 1ms 后画、
//! 下一帧 4ms 后画），屏幕却是按 vsync 匀速换帧的 —— 于是每帧挪的距离忽大忽小。
//! 操作菜单入场头几帧每毫秒挪 7 个物理像素，差 2ms 就是 14px：一加 PLY110 上肉眼可见的「抖」。
//!
//! 宿主每帧给的时间（Android `Choreographer` 的 frameTimeNanos、iOS `CADisplayLink`）就是 vsync，
//! 间隔是均匀的。这里把它换算成 `Instant`（与动画起点同一个钟），本线程这一帧里的动画都按它取样。
//!
//! - 只在 SDK 的 `pump(now_ms)` 给了时间时生效；桌面窗体、无头快照、`pump(0)` 照旧用墙上时钟
//!   （逐字节基线不受影响）；
//! - 换算锚点只在「帧时刻跑到了墙上时钟前面」时往回挪（锚点收敛到最小的派发延迟），
//!   落后超过 [`RESYNC_MS`]（宿主换了钟、长时间卡住）时重锚 —— 平时帧与帧之间严格按宿主给的间隔走。
use std::cell::Cell;
use std::time::{Duration, Instant};

/// 帧时刻落后墙上时钟这么多就重锚
pub const RESYNC_MS: u64 = 250;

thread_local! {
    static FRAME: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// 本线程这一帧的时刻；没有宿主帧时间时就是此刻
pub fn now() -> Instant {
    FRAME.with(|f| f.get()).unwrap_or_else(Instant::now)
}

/// 设置 / 清除本线程的帧时刻（SDK 每次 pump 开头调）
pub fn set(at: Option<Instant>) {
    FRAME.with(|f| f.set(at));
}

/// 宿主时间（毫秒）→ `Instant` 的换算锚点
#[derive(Default, Debug, Clone, Copy)]
pub struct Anchor {
    at: Option<(Instant, u64)>,
}

impl Anchor {
    /// 宿主时间 `now_ms` 对应的 `Instant`；`wall` 是此刻的墙上时钟（测试里显式给）
    pub fn frame_instant(&mut self, now_ms: u64, wall: Instant) -> Instant {
        let (a, ms) = *self.at.get_or_insert((wall, now_ms));
        let t = if now_ms >= ms { a + Duration::from_millis(now_ms - ms) } else { a.checked_sub(Duration::from_millis(ms - now_ms)).unwrap_or(a) };
        if t > wall {
            // 跑到墙上时钟前面了：锚点往回挪到「这一帧 = 此刻」
            self.at = Some((wall, now_ms));
            return wall;
        }
        if wall.duration_since(t) > Duration::from_millis(RESYNC_MS) {
            self.at = Some((wall, now_ms));
            return wall;
        }
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 派发延迟忽大忽小_帧时刻仍按宿主间隔均匀走() {
        let base = Instant::now();
        let mut a = Anchor::default();
        // vsync 每 8ms 一次，每一帧被派发的延迟 0.3~5ms 不等
        let delays = [3.0f64, 0.3, 4.8, 1.2, 2.5, 0.4, 5.0, 0.9];
        let mut out = Vec::new();
        for k in 0..40u64 {
            let wall = base + Duration::from_micros(k * 8000 + (delays[k as usize % delays.len()] * 1000.0) as u64);
            out.push(a.frame_instant(1_000 + k * 8, wall));
        }
        // 锚点收敛（前几帧可能往回挪）之后，帧与帧之间恰好 8ms
        let steps: Vec<u128> = out.windows(2).skip(8).map(|w| w[1].duration_since(w[0]).as_micros()).collect();
        assert!(steps.iter().all(|s| *s == 8000), "{steps:?}");
        // 而且不会跑到墙上时钟前面
        for (k, t) in out.iter().enumerate() {
            let wall = base + Duration::from_micros(k as u64 * 8000 + (delays[k % delays.len()] * 1000.0) as u64);
            assert!(*t <= wall);
        }
    }

    #[test]
    fn 宿主时钟跳了就重锚() {
        let base = Instant::now();
        let mut a = Anchor::default();
        a.frame_instant(5_000, base);
        // 宿主时间往回跳了 10 秒（换了钟）：帧时刻会落后墙上时钟一大截 → 重锚到此刻
        let wall = base + Duration::from_millis(16);
        assert_eq!(a.frame_instant(16, wall), wall);
        assert_eq!(a.frame_instant(24, wall + Duration::from_millis(9)), wall + Duration::from_millis(8));
    }

    #[test]
    fn 没设帧时刻时就是此刻() {
        set(None);
        let t0 = Instant::now();
        assert!(now() >= t0);
        let t = t0 - Duration::from_millis(5);
        set(Some(t));
        assert_eq!(now(), t);
        set(None);
    }
}
