//! 橡皮筋与回弹弹簧、惯性减速与撞边界的交接。
//!
//! 从 `scroll_controller.rs` 拆出来的一片（**纯搬迁**，只把私有方法改成 `pub(super)`）。

use super::*;

/// 惯性每个 60Hz 帧剩下的速度比例（按真实 dt 折算：`0.92^(dt·60)`）。
/// 公开出去是因为宿主承载的原生组件要按同一条衰减曲线外推位置（host/native_motion.rs）
pub const INERTIA_DECAY_PER_60HZ_FRAME: f32 = 0.92;

impl ScrollController {
    /// 惯性减速中的速度（逻辑像素 / 秒）。拖动、回弹、程序滚动时为 None —— 那些不是这条衰减曲线
    pub fn inertia_velocity(&self) -> Option<f32> {
        (self.is_decelerating && !self.is_bouncing && self.tween.is_none()).then_some(self.velocity)
    }

    pub(super) fn rubber_band(offset: f32, dimension: f32) -> f32 {
        let c = RUBBER_BAND_C;
        let x = offset.abs() / dimension;
        let result = (1.0 - (1.0 / (x * c + 1.0))) * dimension;
        if offset < 0.0 { -result } else { result }
    }

    /// 橡皮筋在**当前越界量**处的斜率。
    ///
    /// 松手时测到的是**手指**速度，而越界区里手指走 1px 内容只走 slope 个 px。
    /// 不做这一步衰减就会把手指速度整份交给弹簧，深度越界时弹出去一大截。
    /// 由 `r = (1 - 1/(xc+1))·dim` 反解得 `slope = c·(1 - r/dim)²`。
    pub(super) fn rubber_band_slope(&self) -> f32 {
        let min_bound = self.min_bound();
        let r = if self.position < min_bound {
            min_bound - self.position
        } else if self.position > self.max_scroll {
            self.position - self.max_scroll
        } else {
            return 1.0;
        };
        let k = (1.0 - r / self.viewport_size).clamp(0.0, 1.0);
        RUBBER_BAND_C * k * k
    }

    /// 开始回弹：目标取最近的合法边界，初速度取当前速度（经橡皮筋衰减）。
    ///
    /// 微信/iOS 的越界归位是**带初速度的弹簧**，不是固定时长的缓动 ——
    /// 松手瞬间还在往外走就先多走一点再回来，松手时已经停住就直接平滑归位。
    /// 从前这里把速度清零、跑 0.3s 的 cubic ease-out，手上的感觉是「一顿一下」。
    pub(super) fn start_bounce(&mut self) {
        let v = self.velocity * self.rubber_band_slope();
        let min_bound = self.min_bound();
        let target = self.position.clamp(min_bound, self.max_scroll.max(min_bound));
        self.begin_spring(target, v);
    }

    /// 以 `target` 为目标、`v0` 为初速度启动弹簧
    pub(super) fn begin_spring(&mut self, target: f32, v0: f32) {
        // 过冲上限 ≈ v0/(ω·e)，把初速度夹到 3 个视口/秒 → 最多冲出约 5.5% 视口
        let cap = self.viewport_size * 3.0;
        self.is_bouncing = true;
        self.is_decelerating = false;
        self.bounce_timer = 0.0;
        self.bounce_target_pos = target;
        self.bounce_x0 = self.position - target;
        self.bounce_v0 = v0.clamp(-cap, cap);
        self.velocity = 0.0;
        self.idle_after_wheel = 0.0;
        self.wheel_raw = target;
    }

    /// 临界阻尼弹簧在 t 时刻的位移与速度（相对目标）。
    ///
    /// `x(t) = (x0 + (v0 + ω·x0)·t)·e^(−ω·t)`，临界阻尼 ⇒ 不会来回振荡，
    /// 与 iOS `UIScrollView` 的橡皮筋归位一致（微信小程序用的就是系统滚动）。
    pub(super) fn spring_at(&self, t: f32) -> (f32, f32) {
        let w = BOUNCE_OMEGA;
        let a = self.bounce_v0 + w * self.bounce_x0;
        let e = (-w * t).exp();
        let x = (self.bounce_x0 + a * t) * e;
        let v = (a * (1.0 - w * t) - w * self.bounce_x0) * e;
        (x, v)
    }

    /// 推进一步回弹。返回是否还在动。
    pub(super) fn tick_bounce(&mut self, dt: f32) -> bool {
        self.bounce_timer += dt;
        let (x, v) = self.spring_at(self.bounce_timer);
        // 收敛判定：位移已经小于半个物理像素、且速度也降下来了就落位。
        // 也给一个硬上限，避免极端参数下无限逼近。
        if (x.abs() < 0.05 && v.abs() < 5.0) || self.bounce_timer >= BOUNCE_MAX_SECS {
            self.position = self.bounce_target_pos;
            self.wheel_raw = self.position;
            self.is_bouncing = false;
            return false;
        }
        self.position = self.bounce_target_pos + x;
        true
    }

    /// 当前是否停在边界之外（等待回弹）
    pub(super) fn is_out_of_bounds(&self) -> bool {
        self.position < self.min_bound() - 0.01 || self.position > self.max_scroll + 0.01
    }

    /// 惯性的一步：指数衰减速度并推进位置（**不夹边界**，边界交给
    /// [`Self::handoff_at_bounds`] 处理）
    pub(super) fn step_deceleration(&mut self, dt: f32) {
        let deceleration = INERTIA_DECAY_PER_60HZ_FRAME.powf(dt * 60.0);
        self.velocity *= deceleration;
        self.position += self.velocity * dt;
    }

    /// 惯性滚到边界的交接。返回 `Some(是否撞的是底部)` 表示这一帧已经处理完边界。
    ///
    /// 从前这里是 `position.clamp(...)` + `velocity = 0`：甩一下滑到底会**当场停死**，
    /// 完全没有回弹 —— 只有「手指拖过界再松手」才弹。微信/iOS 是把剩余动量交给弹簧，
    /// 先冲出去一点再弹回来，这才是那种「顺滑」的来源。
    pub(super) fn handoff_at_bounds(&mut self) -> Option<bool> {
        let min_bound = self.min_bound();
        let over_bottom = self.position >= self.max_scroll;
        let over_top = self.position <= min_bound;
        if !over_bottom && !over_top {
            return None;
        }
        let v = self.velocity;
        self.position = if over_bottom { self.max_scroll } else { min_bound };
        if v.abs() > FLING_HANDOFF_MIN_V {
            let target = self.position;
            self.begin_spring(target, v);
        } else {
            self.velocity = 0.0;
            self.is_decelerating = false;
            self.wheel_raw = self.position;
        }
        Some(over_bottom)
    }

    /// 顶部边界。下拉刷新期间 `top_inset > 0`，内容被按住在露出指示器的位置。
    pub(super) fn min_bound(&self) -> f32 {
        self.min_scroll - self.top_inset
    }

    /// 把「未夹紧」的位置按橡皮筋映射成实际显示位置
    pub(super) fn apply_rubber_band(&self, raw: f32) -> f32 {
        let dim = self.viewport_size;
        let min_bound = self.min_bound();
        if raw < min_bound {
            min_bound - Self::rubber_band(min_bound - raw, dim)
        } else if raw > self.max_scroll {
            self.max_scroll + Self::rubber_band(raw - self.max_scroll, dim)
        } else {
            raw
        }
    }
}
