//! 慢慢拖动页面时「一顿一顿」：按事件时间重采样手指位置。
//!
//! Flutter 宿主的触摸事件到引擎线程要过两道线程（Dart → 平台线程 → 引擎线程），
//! 一帧里收到几个移动是碰运气：这一帧赶上了 vsync 前的那个，下一帧没赶上……
//! 页面按「最后收到的那个」走，于是一帧挪两格、一帧挪零格。慢拖时手指每帧只挪几个像素，
//! 这点差别肉眼就看得出来（一加 PLY110 上用户原话：「慢慢滑动会有抖动的感觉」）。
//!
//! 改成：移动事件带上它自己的时间戳（宿主的单调时钟，与 `pump` 的 `now_ms` 同一个钟），
//! 每一帧取 `now − (一帧 + 5ms)` 这个时刻、在前后两个采样之间插值，当作这一帧的手指位置。
//! 往回取一整帧，是为了让这个时刻前后的采样几乎一定都已经到了；于是每帧挪多少只取决于手指本身。
//! 代价是固定多约一帧的跟手延迟（Android 系统自己的重采样取 vsync − 5ms，Flutter 框架自带的
//! 重采样取 −38ms）。
//!
//! - 只重采样**页面**的拖动：scroll-view、swiper、picker 滚轮、侧滑返回照旧逐事件；
//! - 松手惯性的速度仍按原始采样算（`ScrollController::velocity_samples`），手感不变；
//! - 松手时页面停在**屏上那一帧**的位置，不跳到手指最后的位置（惯性接着往前走）；
//! - 事件时间与引擎时钟差出 1 秒以上（宿主用的不是同一个钟）：不重采样，退回逐事件；
//! - `MINI_NO_RESAMPLE=1`（安卓 `debug.mini.no_resample`）关掉，逐事件走（对照 / 排查用）。
use super::engine::MiniEngine;
use std::collections::VecDeque;

/// 最多留几个采样（够盖住三帧以上）
const KEEP: usize = 16;
/// 在「一帧」之外再往回取的毫秒数（Android 系统重采样的 RESAMPLE_LATENCY）
const EXTRA_MS: f64 = 5.0;
/// 事件时间与引擎时钟差这么多就认为不是同一个钟
const SAME_CLOCK_MS: f64 = 1000.0;
/// 事件晚到时最多往前推多少毫秒（与 Android 系统重采样的 RESAMPLE_MAX_PREDICTION 同值）
const MAX_PREDICT_MS: f64 = 8.0;
/// 最后两个采样隔得比这还久：手指停过，不往前推
const MAX_GAP_MS: f64 = 20.0;

#[derive(Default)]
pub(crate) struct DragResampler {
    /// (事件时间 ms, 页面视口坐标的手指 y)，按时间有序
    samples: VecDeque<(f64, f32)>,
    /// 上一帧屏上用的手指位置与页面位置
    shown: Option<f32>,
    shown_pos: Option<f32>,
    /// pump 间隔的平滑估计（ms）
    period: f64,
    last_pump: u64,
    /// 这一次 `pointer_move` 是 `pointer_move_at` 转进来的（带时间戳，不要清采样）
    timed: bool,
    /// 这一次移动的事件时间（ms）：手势推进（含松手速度的采样）按它记，不按到达时刻。
    /// 事件过两道线程才到，常常几个一批地到：按到达时刻算，最后 100ms 里的采样挤在 1ms 内，
    /// 松手速度就是真实速度的十几倍（一加真机慢拖 2.3px/帧，松手惯性第一帧 44.5px）
    event_ms: Option<u64>,
}

impl DragResampler {
    fn clear(&mut self) {
        self.samples.clear();
        self.shown = None;
        self.shown_pos = None;
    }

    fn push(&mut self, t: f64, y: f32) {
        // 时间倒退（宿主重排了事件）：丢掉，保持有序
        if self.samples.back().is_some_and(|&(lt, _)| t < lt) {
            return;
        }
        self.samples.push_back((t, y));
        while self.samples.len() > KEEP {
            self.samples.pop_front();
        }
    }

    fn note_pump(&mut self, now: u64) {
        if self.last_pump > 0 && now > self.last_pump {
            let d = (now - self.last_pump) as f64;
            // 停过一阵再来的那一次不算（静止页面不出帧）。往小走立刻跟（拖起来之前常常是
            // 隔帧才 pump，估成 16ms 的话一开头多落后一帧），往大走慢慢跟（偶尔一帧晚到不算数）
            if d < 50.0 {
                self.period = if self.period == 0.0 || d < self.period { d } else { self.period * 0.95 + d * 0.05 };
            }
        }
        self.last_pump = now;
    }

    fn period(&self) -> f64 {
        if self.period == 0.0 {
            1000.0 / 120.0
        } else {
            self.period.clamp(4.0, 17.0)
        }
    }

    /// `target` 时刻的手指位置：两采样之间线性插值；早于第一个取第一个。
    ///
    /// 晚于最后一个（这一帧该到的事件还没到）：按最后两个采样的速度往前推，最多推 [`MAX_PREDICT_MS`]。
    /// 从前直接停在最后一个采样上 —— 事件晚到一帧，这一帧就几乎不动、下一帧补两格
    /// （一加真机慢拖每帧位移里成对出现的 0.47 / 2.98）。两个采样隔太久（手指停过）不推
    fn at(&self, target: f64) -> Option<f32> {
        let &(t_first, y_first) = self.samples.front()?;
        if target <= t_first {
            return Some(y_first);
        }
        for (&(t0, y0), &(t1, y1)) in self.samples.iter().zip(self.samples.iter().skip(1)) {
            if target <= t1 {
                let f = if t1 > t0 { ((target - t0) / (t1 - t0)) as f32 } else { 1.0 };
                return Some(y0 + (y1 - y0) * f);
            }
        }
        let n = self.samples.len();
        let (t1, y1) = self.samples[n - 1];
        if n >= 2 {
            let (t0, y0) = self.samples[n - 2];
            let dt = t1 - t0;
            if dt > 0.5 && dt < MAX_GAP_MS {
                let ahead = (target - t1).min(MAX_PREDICT_MS);
                return Some(y1 + (y1 - y0) * (ahead / dt) as f32);
            }
        }
        Some(y1)
    }

    /// 这一帧要摆的位置：[`at`](Self::at)，但不往回退 —— 前一帧推多了、这一帧的真实采样
    /// 落在它后面时停一帧，不让页面倒着跳一下（手指真的反向时照常跟）
    fn next_shown(&self, target: f64) -> Option<f32> {
        let y = self.at(target)?;
        let n = self.samples.len();
        let (Some(prev), true) = (self.shown, n >= 2) else { return Some(y) };
        let dir = self.samples[n - 1].1 - self.samples[n - 2].1;
        let step = y - prev;
        Some(if dir != 0.0 && step != 0.0 && dir.signum() != step.signum() { prev } else { y })
    }
}

impl MiniEngine {
    /// 手指移动，带宿主的事件时间（ms，与 [`pump`](Self::pump) 的 `now_ms` 同一个单调时钟：
    /// Android 是 `MotionEvent.getEventTime()` / `SystemClock.uptimeMillis`，
    /// iOS 是 `UITouch.timestamp` / `CACurrentMediaTime`）。页面拖动按它重采样，见文件头。
    pub fn pointer_move_at(&mut self, x: f32, y: f32, t_ms: f64) {
        let same_clock = t_ms > 0.0 && (t_ms - self.mono_ms() as f64).abs() < SAME_CLOCK_MS;
        self.resample.timed = true;
        self.resample.event_ms = (same_clock && !self.env.no_resample).then_some(t_ms.max(0.0) as u64);
        self.pointer_move(x, y);
        self.resample.event_ms = None;
        let page_drag = self.scroll.is_dragging && self.interaction.dragging_scroll_area.is_none() && !self.env.no_resample;
        if !(page_drag && same_clock) {
            self.resample.clear();
            return;
        }
        if self.resample.samples.is_empty() && self.dmg.log {
            crate::compositor::diag::emit(&format!(
                "✋ 页面拖动按事件时间重采样（事件比引擎时钟早 {:.1}ms，一帧 {:.2}ms）",
                self.mono_ms() as f64 - t_ms,
                self.resample.period()
            ));
        }
        self.resample.push(t_ms, y - self.nav_height());
    }

    /// 手势推进用的时刻：带时间戳的移动用事件时间，否则 `now`（到达时刻）
    pub(super) fn gesture_clock(&self, now: u64) -> u64 {
        self.resample.event_ms.unwrap_or(now)
    }

    /// `pointer_move` 开头调：不带时间戳的移动（别的宿主、测试）清掉采样，退回逐事件
    pub(super) fn resample_untimed_move(&mut self) {
        if !std::mem::take(&mut self.resample.timed) {
            self.resample.clear();
        }
    }

    /// 每帧：页面在拖时，按重采样的手指位置摆页面。返回屏上位置是否变了
    pub(super) fn resample_page_drag(&mut self, now: u64) -> bool {
        self.resample.note_pump(now);
        if !self.scroll.is_dragging || self.interaction.dragging_scroll_area.is_some() {
            return false;
        }
        let target = now as f64 - (self.resample.period() + EXTRA_MS);
        let Some(y) = self.resample.next_shown(target) else { return false };
        self.scroll.drag_preview(y);
        let pos = self.scroll.get_position();
        self.resample.shown = Some(y);
        self.resample.shown_pos.replace(pos) != Some(pos)
    }

    /// 松手 / 取消 / 新的按下：页面留在屏上那一帧的位置（松手惯性从这里接着走），清掉采样
    pub(super) fn resample_release(&mut self) {
        if let Some(y) = self.resample.shown {
            if self.scroll.is_dragging && self.interaction.dragging_scroll_area.is_none() {
                self.scroll.drag_preview(y);
            }
        }
        self.resample.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 插值_早于第一个取第一个_晚于最后一个取最后一个() {
        let mut r = DragResampler::default();
        assert_eq!(r.at(5.0), None);
        r.push(10.0, 100.0);
        r.push(20.0, 80.0);
        r.push(15.0, 0.0); // 时间倒退：丢掉
        assert_eq!(r.at(0.0), Some(100.0));
        assert_eq!(r.at(15.0), Some(90.0));
        // 晚于最后一个：按最后两个的速度往前推（-2px/ms），最多推 8ms
        assert_eq!(r.at(25.0), Some(70.0));
        assert_eq!(r.at(60.0), Some(64.0));
        // 两个采样隔太久（手指停过）：不推
        r.push(50.0, 79.0);
        assert_eq!(r.at(55.0), Some(79.0));
    }

    #[test]
    fn 事件成对晚到_每帧位移仍然均匀且不倒退() {
        // 手指匀速 0.2px/ms，事件每 4ms 一个；主线程忙时事件两帧一批地到（真机慢拖的 0.47 / 2.98）
        let mut r = DragResampler::default();
        let mut shown = Vec::new();
        let mut next = 0u64;
        for k in 4..60u64 {
            let now = k * 8;
            r.note_pump(now);
            // 偶数帧只到了 now−20ms 之前的事件，奇数帧补齐到 now−2ms
            let arrived = if k % 2 == 0 { now.saturating_sub(20) } else { now.saturating_sub(2) };
            while next * 4 <= arrived {
                r.push((next * 4) as f64, (next * 4) as f32 * 0.2);
                next += 1;
            }
            let y = r.next_shown(now as f64 - (r.period() + EXTRA_MS)).unwrap();
            r.shown = Some(y);
            shown.push(y);
        }
        let steps: Vec<f32> = shown.windows(2).skip(4).map(|w| w[1] - w[0]).collect();
        assert!(steps.iter().all(|s| *s >= 0.0), "不倒退：{steps:?}");
        assert!(steps.iter().all(|s| (s - 1.6).abs() < 0.05), "每帧都该挪 1.6px：{steps:?}");
    }

    #[test]
    fn 到达时刻乱跳_每帧位移仍然均匀() {
        // 手指匀速 1px/ms，事件每 8ms 一个；第 k 帧（now = 8k）时只收到了一部分事件（随机晚到）
        let mut r = DragResampler::default();
        let mut shown = Vec::new();
        let mut next = 0u32;
        for k in 4..40u64 {
            let now = k * 8;
            r.note_pump(now);
            // 晚到 0~7ms：时间 ≤ now − 晚到量 的事件已到
            let late = (k * 37 % 8) as u32;
            while next * 8 + late <= now as u32 {
                r.push((next * 8) as f64, (next * 8) as f32);
                next += 1;
            }
            shown.push(r.at(now as f64 - (r.period() + EXTRA_MS)).unwrap());
        }
        // 第一帧还没量出帧间隔（按 120Hz 估），从第二帧起比
        let steps: Vec<f32> = shown.windows(2).skip(1).map(|w| w[1] - w[0]).collect();
        assert!(steps.iter().all(|s| (s - 8.0).abs() < 0.01), "每帧都该恰好挪 8px：{steps:?}");
    }
}
