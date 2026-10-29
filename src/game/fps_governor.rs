//! 自适应帧率（创建选项 `adaptiveFps: true`，默认关）：游戏稳定跑不满 60 时降到 30，余量够了再回 60。
//!
//! 为什么要：一帧做不完 16.7ms 时，rAF 有时隔一个 vsync、有时隔两个（farm001 在中核上 25ms 一帧 ≈ 40fps），
//! 帧间隔 16.7 / 33.3ms 来回跳，手上就是「一顿一顿」。锁在 30fps 每帧都是 33.3ms，均匀，反而更顺 ——
//! 微信也给低端机这么做（这是宿主行为，不改 `wx.setPreferredFramesPerSecond` 的语义：游戏要 30 就是 30，
//! 这里只会把「游戏要的」往下压，不会往上抬）。
//!
//! 判据（只看 rAF 帧的工作时间 = 这一次 pump 从开始到主画布收尾，不含等 vsync）：
//! - 在 60：最近 [`WINDOW`] 个 rAF 帧里有 ≥ [`DROP_SHARE`] 超过预算的 90%（15ms）→ 降到 30；
//! - 在 30：降下来之后至少 [`DWELL`] 个 rAF 帧，且最近 [`DWELL`] 个的 p90 < 预算的 70%（11.7ms）→ 回 60。
//!   两道线隔得开：工作时间在 12~15ms 之间时不来回跳。
//!
//! 固定时钟（无头、测试）下不启用：工作时间是墙上时间，开了像素基线就不确定了。
use std::collections::VecDeque;

const BUDGET_US: u64 = 1_000_000 / 60;
const WINDOW: usize = 30;
const DROP_SHARE: usize = 20;
const DWELL: usize = 90;

#[derive(Default, Clone, Debug)]
pub struct FpsGovernor {
    work: VecDeque<u64>,
    /// 现在的上限：60 或 30
    capped: bool,
}

impl FpsGovernor {
    /// 当前上限（60 / 30）
    pub fn cap(&self) -> u32 {
        if self.capped { 30 } else { 60 }
    }

    /// 记一个 rAF 帧的工作时间（µs）。上限变了返回 Some(说明)
    pub fn note(&mut self, work_us: u64) -> Option<String> {
        self.work.push_back(work_us);
        let keep = if self.capped { DWELL } else { WINDOW };
        while self.work.len() > keep {
            self.work.pop_front();
        }
        if !self.capped {
            if self.work.len() < WINDOW {
                return None;
            }
            let over = self.work.iter().filter(|&&w| w > BUDGET_US * 9 / 10).count();
            if over >= DROP_SHARE {
                let msg = format!("🎚 自适应帧率 60 → 30（最近 {WINDOW} 帧有 {over} 帧超过 15ms，中位 {:.1}ms）", median(&self.work) as f64 / 1000.0);
                self.capped = true;
                self.work.clear();
                return Some(msg);
            }
        } else if self.work.len() >= DWELL {
            let mut v: Vec<u64> = self.work.iter().copied().collect();
            v.sort_unstable();
            let p90 = v[v.len() * 9 / 10];
            if p90 < BUDGET_US * 7 / 10 {
                let msg = format!("🎚 自适应帧率 30 → 60（最近 {DWELL} 帧 p90 {:.1}ms）", p90 as f64 / 1000.0);
                self.capped = false;
                self.work.clear();
                return Some(msg);
            }
        }
        None
    }
}

fn median(v: &VecDeque<u64>) -> u64 {
    let mut s: Vec<u64> = v.iter().copied().collect();
    s.sort_unstable();
    s[s.len() / 2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 跑不满就降_余量够了才回() {
        let mut g = FpsGovernor::default();
        // 偶尔一两帧慢：不降
        for i in 0..100 {
            assert!(g.note(if i % 5 == 0 { 20_000 } else { 9_000 }).is_none());
        }
        assert_eq!(g.cap(), 60);
        // 持续 25ms：降到 30
        let mut changed = None;
        for _ in 0..WINDOW {
            changed = changed.or(g.note(25_000));
        }
        assert!(changed.is_some());
        assert_eq!(g.cap(), 30);
        // 13ms（介于两道线之间）：不回
        for _ in 0..200 {
            assert!(g.note(13_000).is_none());
        }
        assert_eq!(g.cap(), 30);
        // 8ms 持续 DWELL 帧：回 60
        let mut back = None;
        for _ in 0..DWELL {
            back = back.or(g.note(8_000));
        }
        assert!(back.is_some());
        assert_eq!(g.cap(), 60);
    }
}
