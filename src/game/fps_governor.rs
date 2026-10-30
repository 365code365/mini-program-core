//! 自适应帧率（创建选项 `adaptiveFps: true`，默认关）：游戏稳定跑不满 60 时降到 30，余量够了再回 60。
//!
//! 为什么要：一帧做不完 16.7ms 时，rAF 有时隔一个 vsync、有时隔两个（farm001 在中核上 25ms 一帧 ≈ 40fps），
//! 帧间隔 16.7 / 33.3ms 来回跳，手上就是「一顿一顿」。锁在 30fps 每帧都是 33.3ms，均匀，反而更顺 ——
//! 微信也给低端机这么做（这是宿主行为，不改 `wx.setPreferredFramesPerSecond` 的语义：游戏要 30 就是 30，
//! 这里只会把「游戏要的」往下压，不会往上抬）。
//!
//! 判据（只看 rAF 帧的工作时间 = 这一次 pump 从开始到主画布收尾，不含等 vsync）：
//! - 在 60：最近 [`WINDOW`] 个 rAF 帧里有 ≥ [`DROP_SHARE`] 个**真的超了一帧**（> 16.7ms + 0.5ms）→ 降到 30；
//! - 在 30：每 `dwell` 个 rAF 帧看一次 p90。p90 < 预算的 70% 直接回 60；p90 还在预算以内（< 16.7ms）就**试探着回 60**。
//!
//! 为什么要试探、为什么降的线是 100% 而不是从前的 90%：手机的调频 / 调度跟着负载走（iPhone 16 Pro Max 实测）。
//! 降到 30 之后线程一半时间在睡，系统把它挪到能效核、降频，同一帧的活从 10ms 涨到 14ms —— 从前「p90 < 11.7ms 才回」
//! 永远等不到，掉到 30 就再也回不来；回到 60 之后负载上来，系统又会把它挪回性能核。所以在 30 的时候看到的工作时间
//! 偏大，只要还在一帧预算以内就值得再试一次。试探失败（回去不到 [`PROBE_GRACE`] 帧又降了）下一次等的时间翻倍，
//! 最长 [`DWELL_MAX`]，免得 3 秒一跳；回 60 之后稳住 [`STABLE`] 帧，等待时间复位。
//! 从前降的线是 15ms：13~15ms 的帧在 60 下其实都赶得上，白白降到 30。
//!
//! 固定时钟（无头、测试）下不启用：工作时间是墙上时间，开了像素基线就不确定了。
use std::collections::VecDeque;

const BUDGET_US: u64 = 1_000_000 / 60;
/// 真超了一帧（留 0.5ms 给 vsync 抖动）
const DROP_US: u64 = BUDGET_US + 500;
const WINDOW: usize = 30;
const DROP_SHARE: usize = 20;
/// 在 30 至少待这么多个 rAF 帧再看（30fps 下 3 秒）
const DWELL: usize = 90;
/// 试探失败之后等待时间翻倍的上限（30fps 下 20 秒）
const DWELL_MAX: usize = 600;
/// 回 60 之后这么多帧内又降了 = 试探失败
const PROBE_GRACE: usize = 120;
/// 回 60 之后稳住这么多帧，等待时间复位
const STABLE: usize = 600;

#[derive(Clone, Debug)]
pub struct FpsGovernor {
    work: VecDeque<u64>,
    /// 现在的上限：60 或 30
    capped: bool,
    /// 在 30 时这次要待多少帧
    dwell: usize,
    /// 回到 60 之后过了多少帧（判断试探成败）
    since_up: usize,
}

impl Default for FpsGovernor {
    fn default() -> Self {
        Self { work: VecDeque::new(), capped: false, dwell: DWELL, since_up: usize::MAX }
    }
}

impl FpsGovernor {
    /// 当前上限（60 / 30）
    pub fn cap(&self) -> u32 {
        if self.capped { 30 } else { 60 }
    }

    /// 记一个 rAF 帧的工作时间（µs）。上限变了返回 Some(说明)
    pub fn note(&mut self, work_us: u64) -> Option<String> {
        self.work.push_back(work_us);
        let keep = if self.capped { self.dwell } else { WINDOW };
        while self.work.len() > keep {
            self.work.pop_front();
        }
        if !self.capped {
            self.since_up = self.since_up.saturating_add(1);
            if self.since_up == STABLE {
                self.dwell = DWELL;
            }
            if self.work.len() < WINDOW {
                return None;
            }
            let over = self.work.iter().filter(|&&w| w > DROP_US).count();
            if over >= DROP_SHARE {
                if self.since_up < PROBE_GRACE {
                    self.dwell = (self.dwell * 2).min(DWELL_MAX);
                }
                let msg = format!(
                    "🎚 自适应帧率 60 → 30（最近 {WINDOW} 帧有 {over} 帧超过 {:.1}ms，中位 {:.1}ms；下次 {} 帧后再试 60）",
                    DROP_US as f64 / 1000.0, median(&self.work) as f64 / 1000.0, self.dwell
                );
                self.capped = true;
                self.work.clear();
                return Some(msg);
            }
        } else if self.work.len() >= self.dwell {
            let mut v: Vec<u64> = self.work.iter().copied().collect();
            v.sort_unstable();
            let p90 = v[v.len() * 9 / 10];
            if p90 < BUDGET_US {
                let why = if p90 < BUDGET_US * 7 / 10 { "余量够" } else { "还在一帧以内，试探" };
                let msg = format!("🎚 自适应帧率 30 → 60（最近 {} 帧 p90 {:.1}ms，{why}）", self.dwell, p90 as f64 / 1000.0);
                self.capped = false;
                self.since_up = 0;
                self.work.clear();
                return Some(msg);
            }
            // 在 30 都超一帧：再等一轮
            self.work.clear();
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

    fn feed(g: &mut FpsGovernor, n: usize, us: u64) -> Option<String> {
        let mut changed = None;
        for _ in 0..n {
            changed = changed.or(g.note(us));
        }
        changed
    }

    #[test]
    fn 赶得上一帧就不降_真超了才降() {
        let mut g = FpsGovernor::default();
        // 偶尔一两帧慢、或者一直 15ms（赶得上 16.7ms 的 vsync）：都不降
        for i in 0..100 {
            assert!(g.note(if i % 5 == 0 { 20_000 } else { 9_000 }).is_none());
        }
        assert!(feed(&mut g, 200, 15_500).is_none());
        assert_eq!(g.cap(), 60);
        // 持续 25ms：降到 30
        assert!(feed(&mut g, WINDOW, 25_000).is_some());
        assert_eq!(g.cap(), 30);
        // 在 30 也超一帧：不回
        assert!(feed(&mut g, DWELL * 3, 20_000).is_none());
        assert_eq!(g.cap(), 30);
        // 8ms：回 60（窗口里还剩着几帧 20ms，满两轮之内回）
        assert!(feed(&mut g, DWELL * 2, 8_000).is_some());
        assert_eq!(g.cap(), 60);
    }

    #[test]
    fn 在_30_看到一帧以内就试探回_60_失败了等待翻倍() {
        let mut g = FpsGovernor::default();
        feed(&mut g, WINDOW, 25_000);
        assert_eq!(g.cap(), 30);
        // 降下来之后系统降频，工作时间 14ms（从前要 < 11.7ms 才回，永远回不去）
        assert!(feed(&mut g, DWELL, 14_000).is_some(), "一帧以内就试探");
        assert_eq!(g.cap(), 60);
        // 回去马上又超：降，下次等两倍
        assert!(feed(&mut g, WINDOW, 25_000).is_some());
        assert_eq!((g.cap(), g.dwell), (30, DWELL * 2));
        assert!(feed(&mut g, DWELL, 14_000).is_none(), "等待翻倍了，90 帧还不试");
        assert!(feed(&mut g, DWELL, 14_000).is_some());
        // 这次稳住了：STABLE 帧之后等待复位
        assert!(feed(&mut g, STABLE, 10_000).is_none());
        assert_eq!((g.cap(), g.dwell), (60, DWELL));
    }
}
