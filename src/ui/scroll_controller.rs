//! 微信小程序风格滚动控制器
//!
//! 按职责分片：`spring`（橡皮筋、回弹弹簧、惯性与撞边界）、`wheel`（滚轮 / 触控板）、
//! `pull`（下拉刷新与触底）。本文件放状态、构造、拖动与逐帧推进。

/// 橡皮筋、回弹弹簧、惯性与撞边界
mod spring;
/// 滚轮 / 触控板
mod wheel;
/// 下拉刷新与触底
mod pull;
/// 程序发起的定时滚动（滚到指定位置，带时长与缓动）
mod tween;

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
    /// 触控板手势的 (累计位移, 时刻) 采样，抬手时据此算出惯性初速度
    wheel_samples: Vec<(f32, std::time::Instant)>,
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
    /// 程序发起的定时滚动（`scroll-with-animation`、`wx.pageScrollTo` 的 `duration`），见 `tween.rs`
    tween: Option<tween::Tween>,
    /// 边界弹性（scroll-view 的 `bounces`）。关掉时拖过边界硬夹住，不出橡皮筋
    bounces: bool,
    
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
            wheel_samples: Vec::new(),
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
            tween: None,
            bounces: true,
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
        // 手指按上来就接管：程序滚动到一半也停在当前位置（与惯性中按下停住一致）
        self.tween = None;
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
        self.drag_preview(y);
        self.velocity_samples.push((y, timestamp));
        // Keep samples from last 100ms
        self.velocity_samples.retain(|(_, t)| timestamp >= *t && timestamp - *t < 100);
    }

    /// 只按手指位置摆页面，不记速度采样（`update_drag` 的前一半）。
    /// SDK 的页面拖动按事件时间重采样后每帧调它（host/engine_resample.rs），速度仍按原始事件算
    pub fn drag_preview(&mut self, y: f32) {
        if !self.is_dragging { return; }
        let delta = self.drag_start_pos - y;
        let raw = self.drag_start_scroll + delta;
        if raw < self.min_bound() {
            self.was_over_top = true;
        } else if raw > self.max_scroll {
            self.was_over_bottom = true;
        }
        self.position = if self.bounces {
            self.apply_rubber_band(raw)
        } else {
            raw.clamp(self.min_bound(), self.max_scroll.max(self.min_bound()))
        };
        self.wheel_raw = self.position;
        self.max_over_top = self.max_over_top.max(self.top_overscroll());
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

    /// 更新滚动状态，返回是否还在动画中
    pub fn update(&mut self, dt: f32) -> bool {
        if self.is_dragging { return false; }
        if let Some(moving) = self.tick_tween(dt) { return moving; }
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

    /// 更新滚动状态并检查事件，返回 (是否还在动画中, 可能的事件)
    pub fn update_with_events(&mut self, dt: f32) -> (bool, Option<ScrollEvent>) {
        if self.is_dragging { return (false, None); }
        if let Some(moving) = self.tick_tween(dt) {
            // 程序滚到底也算触底（`wx.pageScrollTo` 到底部时 `onReachBottom` 照样触发）
            if !moving && self.position >= self.max_scroll - self.reach_bottom_distance && !self.reach_bottom_triggered && self.max_scroll > 0.0 {
                self.reach_bottom_triggered = true;
                return (false, Some(ScrollEvent::ReachBottom));
            }
            return (moving, None);
        }
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

    pub fn get_position(&self) -> f32 { self.position }
    /// 直接设置滚动位置（用于导航后回到顶部、快照渲染等确定性场景）
    pub fn set_position(&mut self, position: f32) {
        self.tween = None;
        self.position = position.clamp(self.min_bound(), self.max_scroll.max(self.min_bound()));
        self.wheel_raw = self.position;
        self.velocity = 0.0;
        self.is_decelerating = false;
        self.is_bouncing = false;
        self.idle_after_wheel = 0.0;
        self.wheel_samples.clear();
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
        self.tween.is_some()
            || self.is_decelerating
            || self.is_bouncing
            || (!self.is_dragging && self.is_out_of_bounds())
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
