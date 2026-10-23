//! 宿主承载的原生组件（video / map …）跟着页面滚：给 `nativeViews` 报「这一帧的滚动取样」。
//!
//! ## 为什么只报位置不够
//! 引擎画面与宿主控件走的是两条上屏路。Android 上引擎画到自己的 SurfaceView，pump 里交了缓冲、
//! 系统合成器下一个 vsync 就取走；控件的位置却要经事件队列 → 主线程 → MethodChannel →
//! Dart 重建 → Flutter 自己的下一帧 → 混合合成。两边差 1~3 个 vsync，而且每帧差几个不固定：
//! 一加 PLY110（90Hz）上截屏量过，慢拖（150px/s）时视频框比页面落后 2~5 物理像素，
//! 中速（600px/s）落后 13~21 —— 看着就是视频「抖、不跟着页面走」。
//!
//! 宿主只拿到位置的话，只能「事件到了才挪」：事件晚到一帧就停一帧，一帧到两条就跳两格。
//! 报上这一帧的 vsync 时刻、滚动位置、速度与 vsync 周期，宿主就能按**自己那一帧**的 vsync
//! 推算页面此刻在哪（Flutter SDK 的 `native_layer.dart`），与事件什么时候到无关。
//!
//! ## 速度怎么算（真机上踩过的两处）
//! - **不按单帧差分**：拖动时引擎常有一帧页面没挪（这一帧的触摸还没到、重采样停一帧），
//!   单帧差分就报 0，宿主停推、按位置摆，下一帧又推 —— 视频框照样一顿一顿。改成取最近
//!   [`WINDOW_MS`] 里的平均速度；只有页面真停了（不在拖、没有惯性 / 回弹）才报 0，那一刻立即报。
//! - **周期不拿平均间隔估**：复杂页面上 pump 常常隔两三个 vsync 才来一次（22、33ms），平均进去
//!   周期就估大了（实测 13.8 而真实是 11.1），宿主按它数帧就数错。与拖动重采样同一个估法
//!   （engine_resample.rs）：往小走立刻跟、往大走慢慢跟，收敛到单个 vsync 的间隔。
//!
//! 位置用**未取整**的滚动值：合成时按设备像素截断（`compose_frame` 的 `scroll_px`），
//! 截断后的差分一帧 3 像素一帧 4 像素，拿来算速度会把这份锯齿带进去。
use std::collections::VecDeque;

/// 一帧的滚动取样（报给宿主，字段见 engine_events.rs 的事件表）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionSample {
    /// 这一帧取样用的 vsync 时刻（宿主时钟，毫秒；`pump_ex` 传进来的那个）
    pub frame_ms: u64,
    /// 页面滚动位置（逻辑像素，未取整）
    pub scroll: f32,
    /// 滚动速度（逻辑像素 / 毫秒；正 = 内容往上走）
    pub velocity: f32,
    /// vsync 周期估计（毫秒）
    pub period_ms: f32,
    /// 速度的指数衰减率（每毫秒；惯性时是滚动控制器那条曲线，拖动时 0 = 匀速外推）
    pub decay: f32,
}

/// 惯性那条衰减曲线的每毫秒衰减率：`v(t) = v₀·e^(−decay·t)`（scroll_controller 的 `0.92^(dt·60)`）
pub fn inertia_decay_per_ms() -> f32 {
    -crate::ui::scroll_controller::INERTIA_DECAY_PER_60HZ_FRAME.ln() * 60.0 / 1000.0
}

#[derive(Debug, Default)]
pub struct ScrollMotion {
    /// 最近几帧的 (vsync 毫秒, 滚动)，按时间有序
    history: VecDeque<(u64, f32)>,
    period_ms: f32,
    /// 上一次报给宿主的速度不是 0：引擎要再出一帧把「停了」报出去，
    /// 否则宿主一直按最后那个速度往前推（engine_wants.rs）
    pub(crate) unsettled: bool,
}

/// 没估出周期之前用的值
const DEFAULT_PERIOD_MS: f32 = 1000.0 / 60.0;
/// 速度取最近这么久里的平均（够盖住一两帧没挪的情况，又不至于跟不上加减速）
pub const WINDOW_MS: u64 = 40;
/// 上一帧隔得比这还久（停过再动）：不拿它算速度
const MAX_GAP_MS: u64 = 60;

impl ScrollMotion {
    pub fn period_ms(&self) -> f32 {
        if self.period_ms == 0.0 { DEFAULT_PERIOD_MS } else { self.period_ms.clamp(4.0, 34.0) }
    }

    /// 每次 pump 末尾取一次样。`moving`：页面在拖、或有惯性 / 回弹 / 滚动动画；
    /// `inertia`：惯性减速中的速度（逻辑像素 / 毫秒）—— 有它就不用窗口平均：惯性是一条已知的
    /// 指数衰减曲线，窗口平均的是 20ms 之前的速度（大约多 10%），宿主拿它外推四五帧会冲过头
    pub fn sample(&mut self, frame_ms: u64, scroll: f32, moving: bool, inertia: Option<f32>) -> MotionSample {
        match self.history.back().copied() {
            Some((t0, _)) if frame_ms > t0 => {
                let d = (frame_ms - t0) as f32;
                // 停过一阵再来的那一次不算；往小走立刻跟，往大走慢慢跟（同 engine_resample.rs，
                // 往大的权重更小：隔帧 pump 的 22 / 33ms 在这里很常见，不能把周期带大）
                if d < 50.0 {
                    self.period_ms = if self.period_ms == 0.0 || d < self.period_ms { d } else { self.period_ms * 0.98 + d * 0.02 };
                }
                if frame_ms - t0 > MAX_GAP_MS {
                    self.history.clear();
                }
                self.history.push_back((frame_ms, scroll));
            }
            // 同一个 vsync 里又 pump 了一次（宿主在帧外补调）：时刻不动，只换位置
            Some(_) => {
                if let Some(last) = self.history.back_mut() {
                    last.1 = scroll;
                }
            }
            None => self.history.push_back((frame_ms, scroll)),
        }
        // 窗口里至少留一个比当前早的样本（一帧没挪也算得出速度）
        while self.history.len() > 2 && frame_ms.saturating_sub(self.history[1].0) >= WINDOW_MS {
            self.history.pop_front();
        }
        let (t1, s1) = *self.history.back().expect("刚放进去");
        let (velocity, decay) = match (inertia, self.history.front()) {
            (Some(v), _) => (v, inertia_decay_per_ms()),
            (None, Some(&(t0, s0))) if moving && t1 > t0 => ((s1 - s0) / (t1 - t0) as f32, 0.0),
            _ => (0.0, 0.0),
        };
        MotionSample { frame_ms: t1, scroll, velocity, period_ms: self.period_ms(), decay }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 90Hz 的第 i 个 vsync（宿主给的是取整后的毫秒：11、11、12、11…）
    fn vsync(i: u64) -> u64 {
        1_000_000 + (i as f64 * 1000.0 / 90.0) as u64
    }

    #[test]
    fn 整数毫秒的_vsync_也给出平稳的速度与周期() {
        // 页面每帧走 5px
        let mut m = ScrollMotion::default();
        let mut vs = vec![];
        for i in 0..60u64 {
            vs.push(m.sample(vsync(i), i as f32 * 5.0, true, None));
        }
        let s = vs.last().unwrap();
        assert!((s.period_ms - 11.11).abs() < 0.3, "周期估计 {}", s.period_ms);
        let v = 5.0 / (1000.0 / 90.0);
        for s in &vs[10..] {
            assert!((s.velocity - v).abs() < v * 0.04, "速度 {} 应 ≈ {v}", s.velocity);
        }
    }

    #[test]
    fn 隔两三个_vsync_才_pump_一次_周期仍是单个_vsync() {
        // 复杂页面：pump 隔 1、2、3 个 vsync 不等（真机日志里 22、33ms 很常见）
        let mut m = ScrollMotion::default();
        let mut i = 0u64;
        let mut last = None;
        for k in 0..40u64 {
            i += 1 + k % 3;
            let s = m.sample(vsync(i), i as f32 * 5.0, true, None);
            if k > 5 {
                assert!((s.period_ms - 11.11).abs() < 0.6, "周期估计 {}（不能被 22/33ms 的间隔带大）", s.period_ms);
            }
            last = Some(s);
        }
        let s = last.unwrap();
        let v = 5.0 / (1000.0 / 90.0);
        assert!((s.velocity - v).abs() < v * 0.05, "速度 {} 应 ≈ {v}", s.velocity);
    }

    #[test]
    fn 拖动中一帧没挪不报零_真停了立即报零() {
        let mut m = ScrollMotion::default();
        for i in 0..10u64 {
            m.sample(vsync(i), i as f32 * 5.0, true, None);
        }
        // 这一帧触摸没到，页面没挪：速度照旧（窗口平均），不能掉到 0
        let held = m.sample(vsync(10), 45.0, true, None);
        assert!(held.velocity > 0.3, "{held:?}");
        // 松手、没有惯性：这一帧就报 0
        assert_eq!(m.sample(vsync(11), 45.0, false, None).velocity, 0.0);
    }

    #[test]
    fn 停过再动的第一帧不拿老样本算速度() {
        let mut m = ScrollMotion::default();
        m.sample(1000, 0.0, false, None);
        m.sample(1016, 0.0, false, None);
        // 隔了 500ms 再动：窗口里只有自己，速度 0（宿主按位置摆），下一帧就有了
        assert_eq!(m.sample(1516, 4.0, true, None).velocity, 0.0);
        assert!((m.sample(1532, 8.0, true, None).velocity - 0.25).abs() < 1e-4);
    }

    #[test]
    fn 惯性用滚动控制器的瞬时速度与衰减曲线() {
        // 真跑一段惯性：宿主按报的速度与衰减外推 k 帧，必须落在滚动控制器 k 帧之后的位置上
        let mut sc = crate::ui::scroll_controller::ScrollController::new(20000.0, 667.0);
        sc.begin_drag(600.0, 0);
        sc.update_drag(400.0, 50);
        assert!(sc.end_drag(), "甩出去了");
        let mut m = ScrollMotion::default();
        let period = 1000.0 / 90.0;
        let mut traj = vec![];
        for i in 0..40u64 {
            if i > 0 {
                sc.update_with_events(period / 1000.0);
            }
            let s = m.sample(vsync(i), sc.get_position(), true, sc.inertia_velocity().map(|v| v / 1000.0));
            traj.push((s, sc.get_position()));
        }
        let (s, _) = traj[10];
        assert!(s.decay > 0.0);
        for k in 1..=5usize {
            // 与 Flutter SDK 的 MiniNativeMotion.shiftAt 同一个离散求和
            let d = (-s.decay * period).exp();
            let ahead = s.velocity * period * d * (1.0 - d.powi(k as i32)) / (1.0 - d);
            let truth = traj[10 + k].1 - traj[10].1;
            assert!((ahead - truth).abs() < 0.05 * truth.abs().max(1.0), "推 {k} 帧：{ahead} vs 真 {truth}");
        }
    }

    #[test]
    fn 同一个_vsync_里补调不改时刻() {
        let mut m = ScrollMotion::default();
        m.sample(1000, 0.0, true, None);
        let a = m.sample(1016, 8.0, true, None);
        let b = m.sample(1016, 8.0, true, None);
        assert_eq!((b.frame_ms, b.velocity), (1016, a.velocity));
    }
}
