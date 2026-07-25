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
        if (content_size - self.last_content_size).abs() > 1.0 || (self.max_scroll - (content_size - viewport_size).max(0.0)).abs() > 1.0 {
            self.last_content_size = content_size;
            // max_scroll = 内容尺寸 - 视口尺寸，确保滚动到底/右时内容底部/右边刚好贴着视口底部/右边
            self.max_scroll = (content_size - viewport_size).max(0.0).floor();
            if self.position > self.max_scroll {
                self.position = self.max_scroll;
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
        let mut new_pos = self.drag_start_scroll + delta;
        if new_pos < self.min_scroll {
            let overshoot = self.min_scroll - new_pos;
            new_pos = self.min_scroll - Self::rubber_band(overshoot, LOGICAL_HEIGHT as f32);
            // 记录超出顶部
            self.was_over_top = true;
        } else if new_pos > self.max_scroll {
            let overshoot = new_pos - self.max_scroll;
            new_pos = self.max_scroll + Self::rubber_band(overshoot, LOGICAL_HEIGHT as f32);
            // 记录超出底部
            self.was_over_bottom = true;
        }
        self.position = new_pos;
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
        self.bounce_target_pos = self.position.clamp(self.min_scroll, self.max_scroll);
        self.velocity = 0.0;
    }
    
    /// 更新滚动状态，返回是否还在动画中
    pub fn update(&mut self, dt: f32) -> bool {
        if self.is_dragging { return false; }
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
    
    pub fn handle_scroll(&mut self, delta: f32, is_precise: bool) {
        // 忽略极微小的滚动事件
        if delta.abs() < 0.1 {
            return;
        }
        
        // 如果没有可滚动空间，直接返回
        if self.max_scroll <= 0.0 {
            return;
        }
        
        let old_position = self.position;
        
        if is_precise {
            // Trackpad: 直接跟随手指移动，严格限制在边界内
            let new_pos = (self.position + delta).clamp(self.min_scroll, self.max_scroll);
            self.position = new_pos;
            
            // 不启动惯性动画，让触控板自己处理惯性
            // macOS 触控板会持续发送减速的滚动事件
            self.velocity = 0.0;
            self.is_decelerating = false;
            self.is_bouncing = false;
        } else {
            // Mouse Wheel: 脉冲式滚动，严格限制在边界内
            let new_pos = (self.position + delta * 2.0).clamp(self.min_scroll, self.max_scroll);
            self.position = new_pos;
            self.velocity = 0.0;
            self.is_decelerating = false;
            self.is_bouncing = false;
        }
        
        // 检查是否滚动到底部附近（用于触控板/鼠标滚轮）
        if self.position >= self.max_scroll - self.reach_bottom_distance && old_position < self.max_scroll - self.reach_bottom_distance {
            // 接近底部，但不在这里触发事件，让外部检查
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
    }
    pub fn get_max_scroll(&self) -> f32 { self.max_scroll }
    pub fn is_animating(&self) -> bool { self.is_decelerating || self.is_bouncing }
    
    /// 是否在顶部
    pub fn is_at_top(&self) -> bool {
        self.position <= self.min_scroll + 1.0
    }
    
    /// 是否在底部
    pub fn is_at_bottom(&self) -> bool {
        self.position >= self.max_scroll - 1.0
    }
}
