//! 帧节拍器：交互窗体与 `--bench` 共用的「睡到差一点 + 自旋对准」。
//!
//! 从 `winit_app.rs` 的 `RedrawRequested` 里搬出来（**纯搬迁**，算法一行没改）：
//! 基准要和交互窗体用**同一个**节拍器，否则测出来的帧间隔是另一套调度的，
//! 拿去解释手上的顿挫没有意义（设计 §12.1）。
//!
//! 为什么要限速：softbuffer 的 present 不阻塞垂直同步，不限速会以 2~4 倍刷新率空转，
//! 白烧一个核画没人看得见的帧。
use std::time::{Duration, Instant};

/// sleep 只睡到「还差这么多」，剩下的用让出时间片的忙等把边缘对准。
/// sleep 的分辨率（macOS 上过冲约 0.5~2ms）不足以稳定命中 6.9ms 的节拍。
pub const SPIN_MARGIN: Duration = Duration::from_micros(900);

/// 固定节拍的下一个时点
#[derive(Clone, Copy, Debug)]
pub struct Pacer {
    next_at: Instant,
}

impl Pacer {
    pub fn new(now: Instant) -> Self {
        Self { next_at: now }
    }

    /// 阻塞到下一个节拍点。
    ///
    /// 固定节拍：下一帧时点是「上一个时点 + 一个刷新周期」的累加，不是「本帧开始 + 一个周期」。
    /// 后者每帧都把 sleep 的过冲算进新的起点，节拍会越走越偏且忽快忽慢 ——
    /// 平均帧率看着达标，实际帧间隔在抖，这正是「高刷没体现出来」的手感。
    pub fn wait_next(&mut self, interval: Duration) {
        let now = Instant::now();
        let mut target = self.next_at + interval;
        // 落后超过一整帧（大重绘、系统抢占）就重新对齐，避免追帧追出一串挤压帧。
        // 对齐到「现在」而不是「现在 + 一个周期」：一帧干了 10ms 活本来就已经
        // 超预算了，再补睡 6.9ms 会把帧间隔顶到 17ms —— 手上感觉到的顿挫是
        // **帧间隔**，不是干活时间，白睡的这一下等于把一次超时放大成两帧。
        if target <= now {
            target = now;
        }
        self.next_at = target;
        if let Some(coarse) = target.checked_sub(SPIN_MARGIN) {
            let now = Instant::now();
            if coarse > now {
                std::thread::sleep(coarse - now);
            }
        }
        while Instant::now() < target {
            std::hint::spin_loop();
        }
    }

    /// 空闲（不再连续出帧）时把节拍起点挪到现在：下次动起来从那一刻起算，
    /// 不会因为「上一个时点」早已过去而连着冲出几帧
    pub fn reset(&mut self, now: Instant) {
        self.next_at = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_accumulate_from_the_previous_target() {
        let start = Instant::now();
        let mut p = Pacer::new(start);
        let step = Duration::from_millis(3);
        p.wait_next(step);
        p.wait_next(step);
        // 两个节拍点是「起点 + 2 个周期」，与中途 sleep 过冲多少无关
        assert_eq!(p.next_at, start + step * 2);
        assert!(Instant::now() >= start + step * 2);
    }

    #[test]
    fn falling_behind_realigns_to_now_instead_of_catching_up() {
        let start = Instant::now();
        let mut p = Pacer::new(start);
        std::thread::sleep(Duration::from_millis(5));
        let before = Instant::now();
        p.wait_next(Duration::from_millis(1));
        // 落后了 4ms：对齐到「现在」，不补睡、也不追帧
        assert!(p.next_at >= before);
        assert!(p.next_at < before + Duration::from_millis(1));
    }

    #[test]
    fn reset_moves_the_origin() {
        let mut p = Pacer::new(Instant::now() - Duration::from_secs(1));
        let now = Instant::now();
        p.reset(now);
        assert_eq!(p.next_at, now);
    }
}
