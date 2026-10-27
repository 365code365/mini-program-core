//! rAF 节拍器（game-doc/02 §4.2）：宿主每个 vsync 来一次，这里决定这一帧要不要跑 rAF 回调。
//!
//! 官方：rAF 默认 60fps，`wx.setPreferredFramesPerSecond` 有效范围 1~60 —— **高刷屏上也是 60**，
//! 按 60 写的游戏逻辑（按帧计时的动画）在 120Hz 上不能快一倍。
//!
//! 算法：**按理想时刻累加**，整数微秒。首帧 `ideal = t`；之后 `t >= ideal + T - period/2` 时是 rAF 帧并 `ideal += T`；
//! 落后超过 2T（卡顿、后台回来）就 `ideal = t`，不补帧。
//!
//! **锁相**：显示周期是目标间隔的整数分之一（±5%，60.27Hz、120Hz、59.9Hz 这类）时，改成数 vsync：每 k 个 vsync 一次，
//! 时刻跟着真实 vsync 走。按理想时刻累加在这里会出错：一加 PLY110 的「60Hz」实测 16.59ms 一帧，理想时刻每帧比 vsync
//! 落后 76µs，攒到半个周期（约 230 帧、3.85 秒）就跳过一个 vsync —— 屏上每 3.85 秒顿一下（真机实测，2026-10-27）。
//! 只有非整数倍的屏（90Hz、144Hz）才按理想时刻累加。
//!
//! 反例（别这么写）：拿「上一次实际触发的时刻」比。144Hz 上 2 个周期是 13.89ms ≥ 16.67 - 3.47，于是每 2 个周期就触发，
//! 跑成 72fps，而且误差一直累积不会回到 60（用例 `一四四_赫兹`守着）。

/// 节拍器状态
#[derive(Clone, Debug)]
pub struct RafPacer {
    fps: u32,
    /// 下一个理想的 rAF 时刻之前的那个理想时刻（µs）；None = 还没跑过
    ideal_us: Option<u64>,
    /// 最近的 vsync 间隔（µs），取中位数估显示器周期
    intervals: [u64; 16],
    n_intervals: usize,
    last_vsync_us: Option<u64>,
    /// 锁相模式下上一次 rAF 帧的 vsync 时刻
    last_fire_us: Option<u64>,
}

pub const DEFAULT_FPS: u32 = 60;

impl Default for RafPacer {
    fn default() -> Self {
        Self::new(DEFAULT_FPS)
    }
}

impl RafPacer {
    pub fn new(fps: u32) -> Self {
        Self { fps: fps.clamp(1, 60), ideal_us: None, intervals: [0; 16], n_intervals: 0, last_vsync_us: None, last_fire_us: None }
    }

    pub fn fps(&self) -> u32 {
        self.fps
    }

    /// `wx.setPreferredFramesPerSecond`：夹到 1~60。改了立刻生效：理想时刻从下一次 rAF 帧重新算
    pub fn set_fps(&mut self, fps: u32) {
        let fps = fps.clamp(1, 60);
        if fps != self.fps {
            self.fps = fps;
            self.ideal_us = None;
            self.last_fire_us = None;
        }
    }

    fn period_us(&self) -> u64 {
        if self.n_intervals == 0 {
            return 1_000_000 / 60;
        }
        let mut v: Vec<u64> = self.intervals[..self.n_intervals.min(16)].to_vec();
        v.sort_unstable();
        v[v.len() / 2]
    }

    /// 显示器周期估计（毫秒，诊断与「节拍不均匀」判断用）
    pub fn display_period_ms(&self) -> f64 {
        self.period_us() as f64 / 1000.0
    }

    /// 显示刷新率不是目标帧率的整数倍时 rAF 间隔必然一长一短（90Hz 上 1、2 个周期交替）
    pub fn uneven(&self) -> bool {
        if self.n_intervals.min(16) < 4 {
            return false;
        }
        let hz = 1_000_000.0 / self.period_us() as f64;
        let ratio = hz / self.fps as f64;
        (ratio - ratio.round()).abs() > 0.05
    }

    /// 一个 vsync 到了（时刻 `t_us`，单调递增）：返回这一帧是不是 rAF 帧
    pub fn on_vsync(&mut self, t_us: u64) -> bool {
        if let Some(last) = self.last_vsync_us {
            let d = t_us.saturating_sub(last);
            // 后台 / 卡顿造成的长空档不算进显示器周期
            if d > 0 && d < 100_000 {
                // 环形：写指针取模，满 16 个以后覆盖最老的（中位数与顺序无关）
                self.intervals[self.n_intervals % 16] = d;
                self.n_intervals += 1;
            }
        }
        self.last_vsync_us = Some(t_us);
        let target = 1_000_000 / self.fps as u64;
        let period = self.period_us();
        let half = period / 2;
        // 锁相：显示周期 ≈ 目标 / k（k 为整数）时数 vsync（见文件头）
        let ratio = target as f64 / period.max(1) as f64;
        let k = ratio.round();
        if self.n_intervals >= 4 && k >= 1.0 && (ratio - k).abs() < 0.05 * k {
            let fire = match self.last_fire_us {
                None => true,
                // 迟到的 vsync（回调晚了一个周期）按时间算：过了 k 个周期就是 rAF 帧
                Some(last) => t_us + half >= last + k as u64 * period,
            };
            if fire {
                self.last_fire_us = Some(t_us);
                self.ideal_us = Some(t_us);
            }
            return fire;
        }
        match self.ideal_us {
            None => {
                self.ideal_us = Some(t_us);
                self.last_fire_us = Some(t_us);
                true
            }
            Some(ideal) => {
                if t_us + half >= ideal + target {
                    let next = ideal + target;
                    // 落后超过两帧：重锚到此刻，不补帧
                    self.ideal_us = Some(if t_us > next + 2 * target { t_us } else { next });
                    // 两种模式共用：从累加模式切进锁相时，以这一次为相位起点（不会一进锁相就多跑一帧）
                    self.last_fire_us = Some(t_us);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// 后台回来：从下一个 vsync 重新起算
    pub fn reset(&mut self) {
        self.ideal_us = None;
        self.last_vsync_us = None;
        self.last_fire_us = None;
        self.n_intervals = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 在 `hz` 刷新率上跑 `secs` 秒，返回 rAF 帧的时刻（µs）
    fn run(hz: u64, fps: u32, secs: u64) -> Vec<u64> {
        let mut p = RafPacer::new(fps);
        (0..hz * secs).map(|i| i * 1_000_000 / hz).filter(|&t| p.on_vsync(t)).collect()
    }

    #[test]
    fn 各刷新率都是_60() {
        for hz in [60, 90, 120, 144, 165, 240] {
            let n = run(hz, 60, 10).len() as i64;
            assert!((n - 600).abs() <= 1, "{hz}Hz 10 秒得到 {n} 次");
        }
    }

    #[test]
    fn 一四四_赫兹_间隔只有两种且平均_16_67() {
        let t = run(144, 60, 4);
        let d: Vec<u64> = t.windows(2).map(|w| w[1] - w[0]).collect();
        for x in &d {
            assert!((13_888..=13_889).contains(x) || (20_833..=20_834).contains(x), "意外间隔 {x}µs");
        }
        let mean = d.iter().sum::<u64>() as f64 / d.len() as f64;
        assert!((mean - 16_666.7).abs() < 30.0, "平均 {mean}µs：节拍在累积误差");
    }

    #[test]
    fn 一二零赫兹_恰好隔一帧() {
        let t = run(120, 60, 2);
        assert!(t.windows(2).all(|w| (16_666..=16_667).contains(&(w[1] - w[0]))));
    }

    #[test]
    fn 九十赫兹_报节拍不均匀() {
        let mut p = RafPacer::new(60);
        for i in 0..30 {
            p.on_vsync(i * 1_000_000 / 90);
        }
        assert!(p.uneven());
        let mut q = RafPacer::new(60);
        for i in 0..30 {
            q.on_vsync(i * 1_000_000 / 120);
        }
        assert!(!q.uneven());
    }

    #[test]
    fn 设帧率与夹取() {
        assert_eq!(run(60, 30, 2).len(), 60);
        assert_eq!(run(120, 20, 3).len(), 60);
        assert_eq!(RafPacer::new(0).fps(), 1);
        assert_eq!(RafPacer::new(1000).fps(), 60);
        // 中途改：下一次就按新帧率
        let mut p = RafPacer::new(60);
        let mut hits = vec![];
        for i in 0..240u64 {
            if i == 120 {
                p.set_fps(30);
            }
            if p.on_vsync(i * 1_000_000 / 120) {
                hits.push(i);
            }
        }
        let after: Vec<_> = hits.iter().filter(|&&i| i >= 120).collect();
        assert!((after.len() as i64 - 30).abs() <= 1, "{}", after.len());
    }

    /// 真机的「60Hz」不是正好 60：一加 PLY110 实测 16.59ms（60.27Hz）。每个 vsync 都要是 rAF 帧，不能隔几秒跳一个
    #[test]
    fn 略快略慢于_60_赫兹_每个_vsync_都跑() {
        for period in [16_590u64, 16_700, 16_666] {
            let mut p = RafPacer::new(60);
            let n = (0..1200u64).filter(|i| p.on_vsync(i * period)).count();
            assert!(n >= 1199, "周期 {period}µs：1200 个 vsync 只跑了 {n} 次");
        }
        // 120.5Hz：每两个 vsync 一次
        let mut p = RafPacer::new(60);
        let n = (0..1200u64).filter(|i| p.on_vsync(i * 8_298)).count();
        assert!((599..=601).contains(&n), "{n}");
    }

    #[test]
    fn 卡顿之后不补帧() {
        let mut p = RafPacer::new(60);
        let mut fired = 0;
        for i in 0..60u64 {
            p.on_vsync(i * 16_667);
        }
        // 卡了 500ms：接下来的第一个 vsync 是 rAF 帧，之后一秒恰好 60 次
        let base = 60 * 16_667 + 500_000;
        for i in 0..60u64 {
            if p.on_vsync(base + i * 16_667) {
                fired += 1;
            }
        }
        assert!((fired as i64 - 60).abs() <= 1, "{fired}");
    }
}
