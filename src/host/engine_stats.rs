//! SDK 的 `MINI_FPS` 统计（设计 §6.6；v2 修订 R10）。
//!
//! 桌面窗体早就有 `MINI_FPS=1` 的每秒一行，SDK 一直没有 —— 移动端「滑起来顿」只能靠
//! 肉眼。这里给 `MiniEngine` 补上同口径的一行，进 Xcode 控制台 / logcat：
//!
//! ```text
//! 📊 [后端 CPU] 60 帧/秒（pump 61 次），pump 最慢 3.2ms p99 2.9ms，工作 最慢 3.2ms p99 2.9ms，等 surface 最慢 0.0ms，帧间隔峰值 17.1ms
//! ```
//!
//! ## 为什么分开记 pump / 等 surface / 工作（R10）
//! GPU 上屏时 acquire 会按 vsync 阻塞：整个 `pump` 的耗时里混着「等显示器」的时间，
//! 拿它和 CPU 比会把 GPU 冤枉成更慢。所以 `work = pump − acquire` 单独报，跨后端比较
//! 只看 `work`。CPU 后端没有 surface，acquire 恒为 0，`work == pump`。
//!
//! ## 口径
//! - 只统计**产出了新画面**的 pump（「N 帧/秒」也是这个数）；空转的 pump 只计次数。
//!   空转 pump 只有几微秒，混进去会把 p99 拉低，看着比实际流畅。
//! - 帧间隔 = 相邻两次 pump 的开始时刻之差，只取「结束在一次出帧上」的间隔算峰值。
//!   与桌面一致：看最慢一帧和帧间隔峰值，不看平均（steering 10）。
//! - p99 用最近秩法 `sorted[ceil(0.99·n) − 1]`，与基准（§12.3）同一个算法。
use crate::compositor::{diag, BackendStatus};
use std::time::{Duration, Instant};

/// 统计窗口长度
const WINDOW: Duration = Duration::from_secs(1);

/// 一个统计窗口的汇总
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FpsSummary {
    /// 产出了新画面的 pump 数
    pub frames: u32,
    /// 全部 pump 数（含空转）
    pub pumps: u32,
    pub pump_max_ms: f32,
    pub pump_p99_ms: f32,
    pub work_max_ms: f32,
    pub work_p99_ms: f32,
    pub acquire_max_ms: f32,
    pub interval_peak_ms: f32,
}

/// 按秒滚动的帧统计。关掉时（没设 `MINI_FPS`）[`record`](Self::record) 什么也不做。
#[derive(Clone, Debug)]
pub struct FrameStats {
    enabled: bool,
    window_start: Option<Instant>,
    last_pump: Option<Instant>,
    pumps: u32,
    pump_ms: Vec<f32>,
    work_ms: Vec<f32>,
    acquire_max_ms: f32,
    interval_peak_ms: f32,
}

/// 最近秩法分位数（`sorted` 已升序、非空）
pub fn nearest_rank(sorted: &[f32], q: f32) -> f32 {
    let n = sorted.len();
    let rank = (q * n as f32).ceil() as usize;
    sorted[rank.clamp(1, n) - 1]
}

fn max_and_p99(v: &mut [f32]) -> (f32, f32) {
    if v.is_empty() {
        return (0.0, 0.0);
    }
    v.sort_by(f32::total_cmp);
    (v[v.len() - 1], nearest_rank(v, 0.99))
}

impl FrameStats {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            window_start: None,
            last_pump: None,
            pumps: 0,
            pump_ms: Vec::new(),
            work_ms: Vec::new(),
            acquire_max_ms: 0.0,
            interval_peak_ms: 0.0,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// 记一次 pump：`begin` 是它开始的时刻，`pump_ms` 是整个 pump 的耗时，
    /// `acquire_ms` 是其中等 surface 的部分（CPU 为 0），`produced` 是否出了新画面。
    ///
    /// 这次 pump 让窗口满了 1 秒、且窗口里出过帧时，返回这个窗口的汇总（含本次）。
    pub fn record(&mut self, begin: Instant, pump_ms: f32, acquire_ms: f32, produced: bool) -> Option<FpsSummary> {
        if !self.enabled {
            return None;
        }
        let start = *self.window_start.get_or_insert(begin);
        if let Some(prev) = self.last_pump.replace(begin) {
            if produced {
                let gap = begin.saturating_duration_since(prev).as_secs_f32() * 1000.0;
                self.interval_peak_ms = self.interval_peak_ms.max(gap);
            }
        }
        self.pumps += 1;
        if produced {
            self.pump_ms.push(pump_ms);
            self.work_ms.push((pump_ms - acquire_ms).max(0.0));
            self.acquire_max_ms = self.acquire_max_ms.max(acquire_ms);
        }
        if begin.saturating_duration_since(start) < WINDOW {
            return None;
        }
        let summary = self.take_summary();
        self.window_start = Some(begin);
        summary
    }

    /// 结束当前窗口：有出帧就给汇总，然后清零（`last_pump` 保留，下一个窗口的
    /// 第一个间隔照样算得出来）
    fn take_summary(&mut self) -> Option<FpsSummary> {
        let frames = self.pump_ms.len() as u32;
        let (pump_max_ms, pump_p99_ms) = max_and_p99(&mut self.pump_ms);
        let (work_max_ms, work_p99_ms) = max_and_p99(&mut self.work_ms);
        let out = FpsSummary {
            frames,
            pumps: self.pumps,
            pump_max_ms,
            pump_p99_ms,
            work_max_ms,
            work_p99_ms,
            acquire_max_ms: self.acquire_max_ms,
            interval_peak_ms: self.interval_peak_ms,
        };
        self.pumps = 0;
        self.pump_ms.clear();
        self.work_ms.clear();
        self.acquire_max_ms = 0.0;
        self.interval_peak_ms = 0.0;
        (frames > 0).then_some(out)
    }
}

/// SDK 的 `MINI_FPS` 行（前缀 `📊 [后端 …]` 由 [`diag::fps_line`] 给）
pub fn fps_line(status: &BackendStatus, s: &FpsSummary) -> String {
    let rest = format!(
        "{} 帧/秒（pump {} 次），pump 最慢 {:.1}ms p99 {:.1}ms，工作 最慢 {:.1}ms p99 {:.1}ms，\
         等 surface 最慢 {:.1}ms，帧间隔峰值 {:.1}ms",
        s.frames,
        s.pumps,
        s.pump_max_ms,
        s.pump_p99_ms,
        s.work_max_ms,
        s.work_p99_ms,
        s.acquire_max_ms,
        s.interval_peak_ms,
    );
    diag::fps_line(status, &rest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::{BackendPref, PrefSource};

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn disabled_records_nothing() {
        let mut s = FrameStats::new(false);
        let t0 = Instant::now();
        assert_eq!(s.record(t0, 5.0, 0.0, true), None);
        assert_eq!(s.record(t0 + ms(2000), 5.0, 0.0, true), None);
        assert!(s.pump_ms.is_empty());
    }

    #[test]
    fn one_summary_per_second_with_peak_and_p99() {
        let mut s = FrameStats::new(true);
        let t0 = Instant::now();
        // 0..=59 每 16ms 一次出帧，第 30 次慢（9ms），第 40 次之前空转一次
        let mut out = None;
        let mut t = t0;
        for i in 0..70u64 {
            let produced = i != 40;
            let cost = if i == 30 { 9.0 } else { 2.0 };
            t = t0 + ms(i * 16) + if i > 50 { ms(30) } else { ms(0) };
            if let Some(sum) = s.record(t, cost, 0.0, produced) {
                assert!(out.is_none(), "1 秒内只能有一次汇总");
                out = Some(sum);
            }
        }
        let sum = out.expect("满 1 秒应给出汇总");
        assert!(t >= t0 + WINDOW);
        assert_eq!(sum.pump_max_ms, 9.0);
        assert_eq!(sum.work_max_ms, 9.0, "CPU：没有 acquire，工作 = pump");
        assert_eq!(sum.acquire_max_ms, 0.0);
        assert_eq!(sum.frames + 1, sum.pumps, "空转的那次只计次数");
        // 第 51 次 pump 前多停了 30ms：间隔 46ms 是峰值；第 40 次空转后的第 41 次间隔仍是 16ms
        assert!((sum.interval_peak_ms - 46.0).abs() < 0.01, "{}", sum.interval_peak_ms);
    }

    #[test]
    fn idle_window_gives_no_line_and_acquire_is_split_out() {
        let mut s = FrameStats::new(true);
        let t0 = Instant::now();
        assert_eq!(s.record(t0, 0.1, 0.0, false), None);
        assert_eq!(s.record(t0 + ms(1500), 0.1, 0.0, false), None, "窗口里没出帧就不打");
        // 下一个窗口：GPU 那种「pump 里一半在等 surface」
        assert_eq!(s.record(t0 + ms(1600), 8.0, 6.0, true), None);
        let sum = s.record(t0 + ms(2600), 3.0, 1.0, true).unwrap();
        assert_eq!(sum.frames, 2);
        assert_eq!(sum.pump_max_ms, 8.0);
        assert_eq!(sum.work_max_ms, 2.0);
        assert_eq!(sum.acquire_max_ms, 6.0);
    }

    #[test]
    fn nearest_rank_matches_the_bench_definition() {
        let v: Vec<f32> = (1..=100).map(|i| i as f32).collect();
        assert_eq!(nearest_rank(&v, 0.99), 99.0);
        assert_eq!(nearest_rank(&v, 0.5), 50.0);
        assert_eq!(nearest_rank(&[7.0], 0.99), 7.0);
        let v: Vec<f32> = (1..=10).map(|i| i as f32).collect();
        assert_eq!(nearest_rank(&v, 0.99), 10.0);
    }

    #[test]
    fn line_format_has_the_fps_prefix() {
        let st = BackendStatus::preferred_cpu(PrefSource::Default);
        let sum = FpsSummary {
            frames: 60,
            pumps: 61,
            pump_max_ms: 3.21,
            pump_p99_ms: 2.9,
            work_max_ms: 3.21,
            work_p99_ms: 2.9,
            acquire_max_ms: 0.0,
            interval_peak_ms: 17.1,
        };
        assert_eq!(
            fps_line(&st, &sum),
            "📊 [后端 CPU] 60 帧/秒（pump 61 次），pump 最慢 3.2ms p99 2.9ms，工作 最慢 3.2ms p99 2.9ms，\
             等 surface 最慢 0.0ms，帧间隔峰值 17.1ms"
        );
        let nc = BackendStatus::cpu(BackendPref::Gpu, PrefSource::Env, "未编译 GPU 支持");
        assert!(fps_line(&nc, &sum).starts_with("📊 [后端 CPU（未编译 GPU 支持）] 60 帧/秒"));
    }
}
