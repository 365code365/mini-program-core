//! 微信小程序风格滚动控制器

/// 视口高度常量
pub const LOGICAL_HEIGHT: u32 = 667;

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
    pub is_dragging: bool,
    drag_start_pos: f32,
    drag_start_scroll: f32,
    // (position, timestamp_ms)
    velocity_samples: Vec<(f32, u64)>,
    is_decelerating: bool,
    is_bouncing: bool,
    bounce_timer: f32,
    bounce_start_pos: f32,
    bounce_target_pos: f32,
    
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
            is_dragging: false,
            drag_start_pos: 0.0,
            drag_start_scroll: 0.0,
            velocity_samples: Vec::with_capacity(10),
            is_decelerating: false,
            is_bouncing: false,
            bounce_timer: 0.0,
            bounce_start_pos: 0.0,
            bounce_target_pos: 0.0,
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
        if raw < self.min_scroll {
            self.was_over_top = true;
        } else if raw > self.max_scroll {
            self.was_over_bottom = true;
        }
        self.position = self.apply_rubber_band(raw);
        self.wheel_raw = self.position;
        self.velocity_samples.push((y, timestamp));
        // Keep samples from last 100ms
        self.velocity_samples.retain(|(_, t)| timestamp >= *t && timestamp - *t < 100);
    }
    
    pub fn end_drag(&mut self) -> bool {
        if !self.is_dragging { return false; }
        self.is_dragging = false;
        self.velocity = self.calculate_release_velocity();
        if self.position < self.min_scroll || self.position > self.max_scroll {
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
        let c = 0.55;
        let x = offset.abs() / dimension;
        let result = (1.0 - (1.0 / (x * c + 1.0))) * dimension;
        if offset < 0.0 { -result } else { result }
    }
    
    fn start_bounce(&mut self) {
        self.is_bouncing = true;
        self.is_decelerating = false;
        self.bounce_timer = 0.0;
        self.bounce_start_pos = self.position;
        self.bounce_target_pos = self.position.clamp(self.min_scroll, self.max_scroll.max(self.min_scroll));
        self.velocity = 0.0;
        self.idle_after_wheel = 0.0;
        self.wheel_raw = self.bounce_target_pos;
    }

    /// 当前是否停在边界之外（等待回弹）
    fn is_out_of_bounds(&self) -> bool {
        self.position < self.min_scroll - 0.01 || self.position > self.max_scroll + 0.01
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
            self.bounce_timer += dt;
            let duration = 0.3;
            if self.bounce_timer >= duration {
                self.position = self.bounce_target_pos;
                self.is_bouncing = false;
                return false;
            }
            let t = self.bounce_timer / duration;
            let ease = 1.0 - (1.0 - t).powi(3);
            self.position = self.bounce_start_pos + (self.bounce_target_pos - self.bounce_start_pos) * ease;
            return true;
        }
        if self.is_decelerating {
            // 正常惯性减速
            let deceleration = 0.92_f32.powf(dt * 60.0);
            self.velocity *= deceleration;
            self.position += self.velocity * dt;
            
            // 严格限制在边界内
            self.position = self.position.clamp(self.min_scroll, self.max_scroll);
            
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
        if self.tick_wheel_bounce(dt) { return (true, None); }
        
        if self.is_bouncing {
            self.bounce_timer += dt;
            let duration = 0.3;
            if self.bounce_timer >= duration {
                self.position = self.bounce_target_pos;
                self.is_bouncing = false;
                
                // 回弹结束时检查事件
                let event = self.check_bounce_end_event();
                return (false, event);
            }
            let t = self.bounce_timer / duration;
            let ease = 1.0 - (1.0 - t).powi(3);
            self.position = self.bounce_start_pos + (self.bounce_target_pos - self.bounce_start_pos) * ease;
            return (true, None);
        }
        
        if self.is_decelerating {
            // 正常惯性减速
            let deceleration = 0.92_f32.powf(dt * 60.0);
            self.velocity *= deceleration;
            self.position += self.velocity * dt;
            
            // 检查是否到达边界
            if self.position >= self.max_scroll {
                self.position = self.max_scroll;
                self.velocity = 0.0;
                self.is_decelerating = false;
                
                // 惯性滚动到底部
                if !self.reach_bottom_triggered {
                    self.reach_bottom_triggered = true;
                    return (false, Some(ScrollEvent::ReachBottom));
                }
                return (false, None);
            }
            
            if self.position <= self.min_scroll {
                self.position = self.min_scroll;
                self.velocity = 0.0;
                self.is_decelerating = false;
                return (false, None);
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
        if self.was_over_top && self.bounce_target_pos <= self.min_scroll {
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
        if self.wheel_raw > self.min_scroll && self.wheel_raw < self.max_scroll {
            self.wheel_raw = self.position;
        }
        self.wheel_raw += step;
        self.position = self.apply_rubber_band(self.wheel_raw);
        if self.position < self.min_scroll {
            self.was_over_top = true;
        } else if self.position > self.max_scroll {
            self.was_over_bottom = true;
        }
    }

    /// 触控板手势结束（`TouchPhase::Ended`）：越界则回弹
    pub fn end_wheel_gesture(&mut self) -> bool {
        self.wheel_raw = self.position;
        if self.position < self.min_scroll || self.position > self.max_scroll {
            self.start_bounce();
            return true;
        }
        false
    }

    /// 把「未夹紧」的位置按橡皮筋映射成实际显示位置
    fn apply_rubber_band(&self, raw: f32) -> f32 {
        let dim = self.viewport_size;
        if raw < self.min_scroll {
            self.min_scroll - Self::rubber_band(self.min_scroll - raw, dim)
        } else if raw > self.max_scroll {
            self.max_scroll + Self::rubber_band(raw - self.max_scroll, dim)
        } else {
            raw
        }
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
        self.position = position.clamp(self.min_scroll, self.max_scroll.max(self.min_scroll));
        self.wheel_raw = self.position;
        self.velocity = 0.0;
        self.is_decelerating = false;
        self.is_bouncing = false;
        self.idle_after_wheel = 0.0;
    }
    pub fn get_max_scroll(&self) -> f32 { self.max_scroll }
    /// 越界待回弹也算「在动」：宿主要继续出帧，回弹才有机会开始
    pub fn is_animating(&self) -> bool {
        self.is_decelerating || self.is_bouncing || (!self.is_dragging && self.is_out_of_bounds())
    }
    
    /// 是否在顶部
    pub fn is_at_top(&self) -> bool {
        self.position <= self.min_scroll + 1.0
    }
    
    /// 是否在底部
    pub fn is_at_bottom(&self) -> bool {
        self.position >= self.max_scroll - 1.0
    }
}
