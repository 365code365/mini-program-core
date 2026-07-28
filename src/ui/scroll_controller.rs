//! 微信小程序风格滚动控制器

/// 视口高度常量
pub const LOGICAL_HEIGHT: u32 = 667;

/// 下拉刷新指示器区域高度（逻辑 px），同时也是触发刷新的下拉阈值。
/// 与微信一致：下拉不到这个距离只回弹，不触发 `onPullDownRefresh`。
pub const PULL_REFRESH_HEIGHT: f32 = 64.0;

/// 橡皮筋阻尼系数（与 iOS `UIScrollView` 同量级）：越界量趋于一个视口时位移趋于饱和
const RUBBER_BAND_C: f32 = 0.55;

/// 回弹弹簧的角频率（rad/s）。临界阻尼下 3/ω ≈ 0.15s 完成 95%，
/// 整体落位约 0.35~0.45s —— 与 iOS 橡皮筋归位的观感一致。
const BOUNCE_OMEGA: f32 = 20.0;

/// 回弹的硬上限时长（秒），兜底防止极端参数下一直逼近不落位
const BOUNCE_MAX_SECS: f32 = 1.0;

/// 惯性撞到边界时，速度低于这个值（px/s）就直接停在边界上。
/// 再小的过冲肉眼看不出来，却会让「慢慢挨到底」多抖一下。
const FLING_HANDOFF_MIN_V: f32 = 60.0;

/// 滚动事件类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollEvent {
    /// 滚动到底部（回弹结束后触发）
    ReachBottom,
    /// 滚动到顶部/下拉刷新（回弹结束后触发）
    ReachTop,
    /// 滚动到右边（横向滚动）
    ReachRight,
    /// 滚动到左边（横向滚动）
    ReachLeft,
}

/// 滚动方向
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ScrollDirection {
    #[default]
    Vertical,
    Horizontal,
}

/// 微信小程序风格滚动控制器
pub struct ScrollController {
    position: f32,
    velocity: f32,
    min_scroll: f32,
    max_scroll: f32,
    last_content_size: f32,
    /// 视口尺寸：回弹阻尼的参考维度。从前这里写死 667，
    /// 于是页面内的小 scroll-view 也按整屏高度算阻尼，越界能拉出半屏空白。
    viewport_size: f32,
    /// 滚轮/触控板的「未夹紧」累计位置。越界时由它经橡皮筋映射成 `position`，
    /// 手势结束（或滚轮停下）后回弹。没有它就只能硬夹边界 —— 也就是没有回弹。
    wheel_raw: f32,
    /// 越界且没有新滚动事件的累计时长（秒）：滚轮没有「抬手」事件，靠它判定手势结束
    idle_after_wheel: f32,
    /// 顶部 inset（等价 iOS 的 contentInset.top）：下拉刷新期间把内容按住不归位，
    /// 露出的这段空间给刷新指示器。回弹目标随之变成 `-top_inset`。
    top_inset: f32,
    /// 本次手势内到达过的最大顶部越界量（逻辑 px），用于判断是否够触发下拉刷新
    max_over_top: f32,
    /// 「下拉到位并松手」的一次性信号，宿主取走后决定是否进入刷新态
    pull_triggered: bool,
    /// 触发下拉刷新的阈值（逻辑 px），与微信的指示器高度一致
    pull_threshold: f32,
    pub is_dragging: bool,
    drag_start_pos: f32,
    drag_start_scroll: f32,
    // (position, timestamp_ms)
    velocity_samples: Vec<(f32, u64)>,
    is_decelerating: bool,
    is_bouncing: bool,
    /// 回弹已经跑过的时长（秒）
    bounce_timer: f32,
    bounce_target_pos: f32,
    /// 回弹起点相对目标的位移
    bounce_x0: f32,
    /// 回弹的初速度（位置单位/秒，正 = 位置增大的方向）
    bounce_v0: f32,
    
    // 触底/触顶事件相关
    /// 是否曾经超出底部边界（用于检测触底）
    was_over_bottom: bool,
    /// 是否曾经超出顶部边界（用于检测触顶/下拉刷新）
    was_over_top: bool,
    /// 触底阈值（距离底部多少像素时触发，微信默认50）
    reach_bottom_distance: f32,
    /// 是否已经触发过触底事件（防止重复触发）
    reach_bottom_triggered: bool,
    
    /// 滚动方向
    pub direction: ScrollDirection,
}

impl ScrollController {
    pub fn new(content_size: f32, viewport_size: f32) -> Self {
        Self {
            position: 0.0,
            velocity: 0.0,
            min_scroll: 0.0,
            max_scroll: (content_size - viewport_size).max(0.0),
            last_content_size: content_size,
            viewport_size: viewport_size.max(1.0),
            wheel_raw: 0.0,
            idle_after_wheel: 0.0,
            top_inset: 0.0,
            max_over_top: 0.0,
            pull_triggered: false,
            pull_threshold: PULL_REFRESH_HEIGHT,
            is_dragging: false,
            drag_start_pos: 0.0,
            drag_start_scroll: 0.0,
            velocity_samples: Vec::with_capacity(10),
            is_decelerating: false,
            is_bouncing: false,
            bounce_timer: 0.0,
            bounce_target_pos: 0.0,
            bounce_x0: 0.0,
            bounce_v0: 0.0,
            was_over_bottom: false,
            was_over_top: false,
            reach_bottom_distance: 50.0,
            reach_bottom_triggered: false,
            direction: ScrollDirection::Vertical,
        }
    }
    
    /// 创建横向滚动控制器
    pub fn new_horizontal(content_width: f32, viewport_width: f32) -> Self {
        let mut controller = Self::new(content_width, viewport_width);
        controller.direction = ScrollDirection::Horizontal;
        controller
    }
    
    /// 更新内容尺寸（当实际内容尺寸变化时调用）
    pub fn update_content_height(&mut self, content_size: f32, viewport_size: f32) {
        self.viewport_size = viewport_size.max(1.0);
        if (content_size - self.last_content_size).abs() > 1.0 || (self.max_scroll - (content_size - viewport_size).max(0.0)).abs() > 1.0 {
            self.last_content_size = content_size;
            // max_scroll = 内容尺寸 - 视口尺寸，确保滚动到底/右时内容底部/右边刚好贴着视口底部/右边
            self.max_scroll = (content_size - viewport_size).max(0.0).floor();
            // 内容变短（换页、列表收起）时位置必须跟着收回，否则会停在画布之外的空白上，
            // 而且因为不越界也不会触发回弹 —— 看起来就是「能拉很远、拉完不回来」。
            if self.position > self.max_scroll && !self.is_dragging {
                self.position = self.max_scroll;
                self.wheel_raw = self.position;
                self.velocity = 0.0;
                self.is_decelerating = false;
                self.is_bouncing = false;
            }
            // 内容尺寸变化时重置触底状态
            self.reach_bottom_triggered = false;
        }
    }
    
    /// 设置滚动方向
    pub fn set_direction(&mut self, direction: ScrollDirection) {
        self.direction = direction;
    }
    
    /// 获取滚动方向
    pub fn get_direction(&self) -> ScrollDirection {
        self.direction
    }
    
    pub fn begin_drag(&mut self, y: f32, timestamp: u64) {
        self.is_dragging = true;
        self.is_decelerating = false;
        self.is_bouncing = false;
        self.drag_start_pos = y;
        self.drag_start_scroll = self.position;
        self.wheel_raw = self.position;
        self.velocity = 0.0;
        self.velocity_samples.clear();
        self.velocity_samples.push((y, timestamp));
        // 重置超出边界标记
        self.was_over_bottom = false;
        self.was_over_top = false;
    }
    
    pub fn update_drag(&mut self, y: f32, timestamp: u64) {
        if !self.is_dragging { return; }
        let delta = self.drag_start_pos - y;
        let raw = self.drag_start_scroll + delta;
        if raw < self.min_bound() {
            self.was_over_top = true;
        } else if raw > self.max_scroll {
            self.was_over_bottom = true;
        }
        self.position = self.apply_rubber_band(raw);
        self.wheel_raw = self.position;
        self.max_over_top = self.max_over_top.max(self.top_overscroll());
        self.velocity_samples.push((y, timestamp));
        // Keep samples from last 100ms
        self.velocity_samples.retain(|(_, t)| timestamp >= *t && timestamp - *t < 100);
    }
    
    pub fn end_drag(&mut self) -> bool {
        if !self.is_dragging { return false; }
        self.is_dragging = false;
        self.settle_pull_gesture();
        self.velocity = self.calculate_release_velocity();
        if self.position < self.min_bound() || self.position > self.max_scroll {
            self.start_bounce();
        } else if self.velocity.abs() > 50.0 {
            self.is_decelerating = true;
        }
        self.is_decelerating || self.is_bouncing
    }

    fn calculate_release_velocity(&self) -> f32 {
        if self.velocity_samples.len() < 2 { return 0.0; }
        let first = self.velocity_samples.first().unwrap();
        let last = self.velocity_samples.last().unwrap();
        // timestamp is in ms, convert to seconds
        let dt = (last.1.saturating_sub(first.1)) as f32 / 1000.0;
        if dt < 0.001 { return 0.0; }
        (first.0 - last.0) / dt * 0.8
    }
    
    fn rubber_band(offset: f32, dimension: f32) -> f32 {
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
    fn rubber_band_slope(&self) -> f32 {
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
    fn start_bounce(&mut self) {
        let v = self.velocity * self.rubber_band_slope();
        let min_bound = self.min_bound();
        let target = self.position.clamp(min_bound, self.max_scroll.max(min_bound));
        self.begin_spring(target, v);
    }

    /// 以 `target` 为目标、`v0` 为初速度启动弹簧
    fn begin_spring(&mut self, target: f32, v0: f32) {
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
    fn spring_at(&self, t: f32) -> (f32, f32) {
        let w = BOUNCE_OMEGA;
        let a = self.bounce_v0 + w * self.bounce_x0;
        let e = (-w * t).exp();
        let x = (self.bounce_x0 + a * t) * e;
        let v = (a * (1.0 - w * t) - w * self.bounce_x0) * e;
        (x, v)
    }

    /// 推进一步回弹。返回是否还在动。
    fn tick_bounce(&mut self, dt: f32) -> bool {
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
    fn is_out_of_bounds(&self) -> bool {
        self.position < self.min_bound() - 0.01 || self.position > self.max_scroll + 0.01
    }

    /// 滚轮/触控板停下后自动回弹：越界且静默超过阈值就开始回弹。
    /// 触控板通常会给 `TouchPhase::Ended`，但鼠标滚轮不会，这里统一兜底。
    fn tick_wheel_bounce(&mut self, dt: f32) -> bool {
        if self.is_dragging || self.is_bouncing || self.is_decelerating || !self.is_out_of_bounds() {
            return false;
        }
        self.idle_after_wheel += dt;
        if self.idle_after_wheel >= 0.06 {
            self.start_bounce();
        }
        true
    }
    
    /// 更新滚动状态，返回是否还在动画中
    pub fn update(&mut self, dt: f32) -> bool {
        if self.is_dragging { return false; }
        if self.tick_wheel_bounce(dt) { return true; }
        if self.is_bouncing {
            return self.tick_bounce(dt);
        }
        if self.is_decelerating {
            self.step_deceleration(dt);
            if let Some(_hit_bottom) = self.handoff_at_bounds() {
                return self.is_bouncing;
            }
            // 停止条件
            if self.velocity.abs() < 3.0 {
                self.velocity = 0.0;
                self.is_decelerating = false;
                return false;
            }
            return true;
        }
        false
    }

    /// 惯性的一步：指数衰减速度并推进位置（**不夹边界**，边界交给
    /// [`Self::handoff_at_bounds`] 处理）
    fn step_deceleration(&mut self, dt: f32) {
        let deceleration = 0.92_f32.powf(dt * 60.0);
        self.velocity *= deceleration;
        self.position += self.velocity * dt;
    }

    /// 惯性滚到边界的交接。返回 `Some(是否撞的是底部)` 表示这一帧已经处理完边界。
    ///
    /// 从前这里是 `position.clamp(...)` + `velocity = 0`：甩一下滑到底会**当场停死**，
    /// 完全没有回弹 —— 只有「手指拖过界再松手」才弹。微信/iOS 是把剩余动量交给弹簧，
    /// 先冲出去一点再弹回来，这才是那种「顺滑」的来源。
    fn handoff_at_bounds(&mut self) -> Option<bool> {
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
    
    /// 更新滚动状态并检查事件，返回 (是否还在动画中, 可能的事件)
    pub fn update_with_events(&mut self, dt: f32) -> (bool, Option<ScrollEvent>) {
        if self.is_dragging { return (false, None); }
        if self.tick_wheel_bounce(dt) { return (true, None); }
        
        if self.is_bouncing {
            if self.tick_bounce(dt) {
                return (true, None);
            }
            // 回弹结束时检查事件
            let event = self.check_bounce_end_event();
            return (false, event);
        }
        
        if self.is_decelerating {
            self.step_deceleration(dt);
            
            // 撞到边界：动量交给弹簧（见 handoff_at_bounds），触底事件照旧在撞到的那一帧发
            if let Some(hit_bottom) = self.handoff_at_bounds() {
                let animating = self.is_bouncing;
                if hit_bottom && !self.reach_bottom_triggered {
                    self.reach_bottom_triggered = true;
                    return (animating, Some(ScrollEvent::ReachBottom));
                }
                return (animating, None);
            }
            
            // 停止条件
            if self.velocity.abs() < 3.0 {
                self.velocity = 0.0;
                self.is_decelerating = false;
                
                // 检查是否接近底部
                if self.position >= self.max_scroll - self.reach_bottom_distance && !self.reach_bottom_triggered {
                    self.reach_bottom_triggered = true;
                    return (false, Some(ScrollEvent::ReachBottom));
                }
                return (false, None);
            }
            return (true, None);
        }
        (false, None)
    }
    
    /// 回弹结束时检查是否触发事件
    fn check_bounce_end_event(&mut self) -> Option<ScrollEvent> {
        // 如果之前超出了顶部边界，现在回弹到顶部，触发 ReachTop
        if self.was_over_top && self.bounce_target_pos <= self.min_bound() {
            self.was_over_top = false;
            return Some(ScrollEvent::ReachTop);
        }
        
        // 如果之前超出了底部边界，现在回弹到底部，触发 ReachBottom
        if self.was_over_bottom && self.bounce_target_pos >= self.max_scroll {
            self.was_over_bottom = false;
            if !self.reach_bottom_triggered {
                self.reach_bottom_triggered = true;
                return Some(ScrollEvent::ReachBottom);
            }
        }
        
        None
    }
    
    /// 滚轮 / 触控板滚动。
    ///
    /// 与拖拽走同一套语义：越界不硬夹，而是经橡皮筋衰减，手势停下后回弹。
    /// 触控板抬手由 [`Self::end_wheel_gesture`] 通知；鼠标滚轮没有抬手事件，
    /// 靠 `update()` 里的静默计时兜底。
    pub fn handle_scroll(&mut self, delta: f32, is_precise: bool) {
        // 忽略极微小的滚动事件
        if delta.abs() < 0.1 {
            return;
        }
        // 不可滚动的页面仍然允许越界橡皮筋（微信里内容不满一屏也能拉出一点再弹回），
        // 但不做任何位移积累
        self.is_decelerating = false;
        self.is_bouncing = false;
        self.velocity = 0.0;
        self.idle_after_wheel = 0.0;

        // 触控板一次事件就是一次真实位移；滚轮是脉冲，放大到接近一"格"的观感
        let step = if is_precise { delta } else { delta * 2.0 };
        // 越界方向上先把累计位置对齐到当前显示位置，避免来回切换方向时跳变
        if self.wheel_raw > self.min_bound() && self.wheel_raw < self.max_scroll {
            self.wheel_raw = self.position;
        }
        self.wheel_raw += step;
        self.position = self.apply_rubber_band(self.wheel_raw);
        if self.position < self.min_bound() {
            self.was_over_top = true;
        } else if self.position > self.max_scroll {
            self.was_over_bottom = true;
        }
        self.max_over_top = self.max_over_top.max(self.top_overscroll());
    }

    /// 触控板手势结束（`TouchPhase::Ended`）：越界则回弹
    pub fn end_wheel_gesture(&mut self) -> bool {
        self.wheel_raw = self.position;
        self.settle_pull_gesture();
        if self.is_out_of_bounds() {
            self.start_bounce();
            return true;
        }
        false
    }

    /// 顶部边界。下拉刷新期间 `top_inset > 0`，内容被按住在露出指示器的位置。
    fn min_bound(&self) -> f32 {
        self.min_scroll - self.top_inset
    }

    /// 把「未夹紧」的位置按橡皮筋映射成实际显示位置
    fn apply_rubber_band(&self, raw: f32) -> f32 {
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

    /// 进入/退出下拉刷新态：`inset > 0` 时把内容按住露出指示器，回弹目标随之改变。
    pub fn set_top_inset(&mut self, inset: f32) {
        let inset = inset.max(0.0);
        if (self.top_inset - inset).abs() < 0.01 {
            return;
        }
        let growing = inset > self.top_inset;
        self.top_inset = inset;
        if self.is_dragging {
            return;
        }
        let min_bound = self.min_bound();
        let target = if growing {
            // 进入刷新：把内容拉到露出指示器的位置。
            // 只在本来就贴着顶部时才拉 —— 页面滚到中间时 `wx.startPullDownRefresh()`
            // 不该把画面跳回顶部。
            if self.position <= self.min_scroll + 1.0 { min_bound } else { self.position }
        } else {
            // 结束刷新：内容归位
            self.position.clamp(min_bound, self.max_scroll.max(min_bound))
        };
        if (target - self.position).abs() > 0.01 {
            // 下拉刷新的进出场同样用弹簧（初速度 0：这是程序触发的位移，不是手势）
            self.begin_spring(target, 0.0);
        }
    }

    /// 取走「下拉到位并松手」信号（读后清零）
    pub fn take_pull_trigger(&mut self) -> bool {
        std::mem::take(&mut self.pull_triggered)
    }

    /// 当前顶部越界量（逻辑 px，未越界为 0）。宿主用它画下拉指示器的进度。
    pub fn top_overscroll(&self) -> f32 {
        (self.min_bound() - self.position).max(0.0)
    }

    /// 顶部露出的空白高度（逻辑 px）：下拉过程与刷新期间都用它定位指示器。
    /// 注意不能用 `top_overscroll()` —— 刷新期内容正好停在 `-top_inset`，不算越界。
    pub fn top_gap(&self) -> f32 {
        (self.min_scroll - self.position).max(0.0)
    }

    /// 下拉进度（0~1）：到 1 表示松手就会触发刷新
    pub fn pull_progress(&self) -> f32 {
        (self.top_gap() / self.pull_threshold).clamp(0.0, 1.0)
    }

    /// 手势结束时判定是否够触发下拉刷新
    fn settle_pull_gesture(&mut self) {
        if self.max_over_top >= self.pull_threshold && self.top_inset <= 0.01 {
            self.pull_triggered = true;
        }
        self.max_over_top = 0.0;
    }
    
    /// 检查是否应该触发触底事件（用于触控板/鼠标滚轮滚动）
    pub fn check_reach_bottom(&mut self) -> bool {
        if self.max_scroll <= 0.0 {
            return false;
        }
        
        if self.position >= self.max_scroll - self.reach_bottom_distance && !self.reach_bottom_triggered {
            self.reach_bottom_triggered = true;
            return true;
        }
        false
    }
    
    /// 重置触底状态（当内容更新后调用）
    pub fn reset_reach_bottom(&mut self) {
        self.reach_bottom_triggered = false;
    }
    
    pub fn get_position(&self) -> f32 { self.position }
    /// 直接设置滚动位置（用于导航后回到顶部、快照渲染等确定性场景）
    pub fn set_position(&mut self, position: f32) {
        self.position = position.clamp(self.min_bound(), self.max_scroll.max(self.min_bound()));
        self.wheel_raw = self.position;
        self.velocity = 0.0;
        self.is_decelerating = false;
        self.is_bouncing = false;
        self.idle_after_wheel = 0.0;
    }
    /// 立刻停住惯性/回弹，位置保持不变。
    ///
    /// 惯性滚动中手指按下时要用：iOS/微信里那一下只是「停住」，
    /// 不该继续减速、也不该算成一次点击。
    pub fn stop(&mut self) {
        let pos = self.position;
        self.set_position(pos);
    }

    pub fn get_max_scroll(&self) -> f32 { self.max_scroll }
    /// 越界待回弹也算「在动」：宿主要继续出帧，回弹才有机会开始
    pub fn is_animating(&self) -> bool {
        self.is_decelerating || self.is_bouncing || (!self.is_dragging && self.is_out_of_bounds())
    }
    
    /// 是否在顶部
    pub fn is_at_top(&self) -> bool {
        self.position <= self.min_bound() + 1.0
    }
    
    /// 是否在底部
    pub fn is_at_bottom(&self) -> bool {
        self.position >= self.max_scroll - 1.0
    }
}
