//! 指针交互：点击分流、滑块拖动、按压态
//!
//! `InteractionManager` 的一片，由 `interaction/mod.rs` 组合。**纯搬迁**：从 1041 行的
//! interaction.rs 按职责切开，一行逻辑没改。
use super::*;

impl InteractionManager {
    /// 处理点击事件
    pub fn handle_click(&mut self, x: f32, y: f32) -> Option<InteractionResult> {
        self.handle_click_scoped(x, y, false)
    }

    /// 同上。`fixed_only` 为真时只考虑 `position: fixed` 覆盖层里的元素 ——
    /// 弹窗弹着的时候，点击不该让下层页面的输入框获得焦点、开关被拨动。
    pub fn handle_click_scoped(&mut self, x: f32, y: f32, fixed_only: bool) -> Option<InteractionResult> {
        // 必须用 `hit_test_fixed`，不能「先全局 hit_test 再判 is_fixed」——
        // 元素表里覆盖层的元素排在**前面**（正常流每帧重建、追加在后），而 `hit_test`
        // 是从后往前找，于是先撞上正常流的元素、再被 `is_fixed` 过滤掉，结果返回 None。
        // 表现就是：覆盖层里的输入框点不出焦点、开关拨不动 —— 而且只在「这一帧没重绘
        // 覆盖层」时才犯（重绘那一帧覆盖层元素刚好被追加到最后），所以偶发得像随机失灵。
        let element = if fixed_only {
            self.hit_test_fixed(x, y)?.clone()
        } else {
            self.hit_test(x, y)?.clone()
        };
        
        match element.interaction_type {
            InteractionType::Checkbox | InteractionType::Switch => {
                let current = self.states.get(&element.id)
                    .map(|s| s.checked)
                    .unwrap_or(element.checked);
                let new_checked = !current;
                
                self.states.insert(element.id.clone(), ComponentState {
                    checked: new_checked,
                    value: element.value.clone(),
                });
                // 记录切换时刻：switch 的滑块靠它做缓动位移（微信是 0.3s 左右的滑动，
                // 直接跳变会没有"拨动开关"的手感）
                self.transitions.insert(element.id.clone(), crate::renderer::anim::now_secs());
                
                Some(InteractionResult::Toggle {
                    id: element.id,
                    checked: new_checked,
                })
            }
            InteractionType::Radio => {
                // Radio 是互斥的，需要取消同一组内其他 radio 的选中状态
                // 简单实现：取消所有其他 radio 的选中状态（假设页面上只有一组 radio）
                // 更好的实现应该基于 radio-group 或父容器
                let radio_ids: Vec<String> = self.elements.iter()
                    .filter(|e| e.interaction_type == InteractionType::Radio && e.id != element.id)
                    .map(|e| e.id.clone())
                    .collect();
                
                for id in radio_ids {
                    self.states.insert(id, ComponentState {
                        checked: false,
                        value: String::new(),
                    });
                }
                
                self.states.insert(element.id.clone(), ComponentState {
                    checked: true,
                    value: element.value.clone(),
                });
                
                Some(InteractionResult::Select {
                    id: element.id,
                    value: element.value,
                })
            }
            InteractionType::Slider => {
                let progress = ((x - element.bounds.x) / element.bounds.width).clamp(0.0, 1.0);
                let value = element.min + progress * (element.max - element.min);
                
                self.states.insert(element.id.clone(), ComponentState {
                    checked: false,
                    value: format!("{}", value as i32),
                });
                
                self.dragging_slider = Some(DraggingSlider {
                    id: element.id.clone(),
                    bounds: element.bounds,
                    min: element.min,
                    max: element.max,
                });
                
                Some(InteractionResult::SliderChange {
                    id: element.id,
                    value: value as i32,
                })
            }
            InteractionType::Input => {
                // 获取当前值（如果没有状态，使用空字符串而不是 element.value）
                let current_value = self.states.get(&element.id)
                    .map(|s| s.value.clone())
                    .unwrap_or_else(|| {
                        // 如果 element.value 是空的，说明没有初始值
                        if element.value.is_empty() {
                            String::new()
                        } else {
                            element.value.clone()
                        }
                    });
                
                // 初始化状态（如果还没有）
                if !self.states.contains_key(&element.id) {
                    self.states.insert(element.id.clone(), ComponentState {
                        checked: false,
                        value: current_value.clone(),
                    });
                }
                
                // 一聚焦就让光标立刻可见，并从这一刻开始数闪烁拍子（微信行为）
                crate::renderer::components::reset_cursor_blink();
                self.focused_input = Some(FocusedInput {
                    id: element.id.clone(),
                    value: current_value.clone(),
                    cursor_pos: current_value.chars().count(),
                    selection_start: None,
                    selection_end: None,
                    is_password: false,
                    bounds: element.bounds,
                    is_fixed: element.is_fixed,
                    text_offset: 0.0, // 初始偏移为0，会在渲染时更新
                    maxlength: 140, // 默认值，会在外部更新
                    input_type: "text".to_string(), // 默认值，会在外部更新
                });

                // 记录点击位置用于后续计算光标位置
                let click_x = x - element.bounds.x;

                Some(InteractionResult::Focus {
                    id: element.id,
                    bounds: element.bounds,
                    click_x,
                    is_fixed: element.is_fixed,
                    value: current_value,
                })
            }
            InteractionType::Button => {
                // 按钮点击不需要在这里触发动画，按下状态由鼠标按下/松开控制
                Some(InteractionResult::ButtonClick {
                    id: element.id,
                    bounds: element.bounds,
                })
            }
            InteractionType::View => None,
            InteractionType::ScrollArea => None,
        }
    }

    /// 处理鼠标移动（用于滑块拖动）
    pub fn handle_mouse_move(&mut self, x: f32, _y: f32) -> Option<InteractionResult> {
        if let Some(ref slider) = self.dragging_slider {
            let progress = ((x - slider.bounds.x) / slider.bounds.width).clamp(0.0, 1.0);
            let value = slider.min + progress * (slider.max - slider.min);
            
            self.states.insert(slider.id.clone(), ComponentState {
                checked: false,
                value: format!("{}", value as i32),
            });
            
            return Some(InteractionResult::SliderChange {
                id: slider.id.clone(),
                value: value as i32,
            });
        }
        None
    }

    /// 处理鼠标释放
    pub fn handle_mouse_release(&mut self) -> Option<InteractionResult> {
        if let Some(slider) = self.dragging_slider.take() {
            return Some(InteractionResult::SliderEnd { id: slider.id });
        }
        None
    }

    /// 是否正在拖动滑块
    pub fn is_dragging_slider(&self) -> bool {
        self.dragging_slider.is_some()
    }

    /// 登记一个元素的点击态计时（每帧绘制时刷新；没登记的按按钮默认 20/70）
    pub fn note_hover(&mut self, id: String, spec: HoverSpec) {
        self.hover_specs.insert(id, spec);
    }

    /// 设置按钮按下状态。同时登记过渡起始时刻，`transition` 才能从常态平滑到按压态。
    ///
    /// 点击态不在按下瞬间出现：微信按钮默认等 `hover-start-time`（20ms），
    /// 并且父节点上写了 hover-class 的也会一起亮，除非命中的元素带
    /// `hover-stop-propagation`。
    pub fn set_button_pressed(&mut self, id: String, bounds: Rect) {
        if self
            .press_feedback
            .as_ref()
            .map(|p| p.hit_id == id && p.hide_at.is_none())
            .unwrap_or(false)
        {
            return;
        }
        let spec = self.hover_specs.get(&id).copied().unwrap_or(HoverSpec {
            start_ms: 20,
            stay_ms: 70,
            stop: false,
        });
        let ids = self.hover_chain(&id);
        for hid in &ids {
            self.transitions.insert(hid.clone(), crate::renderer::anim::now_secs());
        }
        self.pressed_button = Some(PressedButton { id: id.clone(), bounds });
        self.press_feedback = Some(PressFeedback {
            hit_id: id,
            ids,
            show_at: std::time::Instant::now() + std::time::Duration::from_millis(spec.start_ms),
            hide_at: None,
            stay_ms: spec.stay_ms,
        });
    }

    /// 从外到内，点落在哪些「有点击态」的元素上，截到命中的那个为止。
    /// 链上有 `hover-stop-propagation` 时，更外层的祖先不再进入点击态。
    fn hover_chain(&self, hit_id: &str) -> Vec<String> {
        let Some((hx, hy, hw, hh, fixed)) = self
            .elements
            .iter()
            .rev()
            .find(|e| e.id == hit_id)
            .map(|e| (e.bounds.x, e.bounds.y, e.bounds.width, e.bounds.height, e.is_fixed))
        else {
            return vec![hit_id.to_string()];
        };
        let (x, y) = (hx + hw * 0.5, hy + hh * 0.5);
        let mut chain: Vec<&InteractiveElement> = self
            .elements
            .iter()
            .filter(|e| {
                e.is_fixed == fixed
                    && self.hover_specs.contains_key(&e.id)
                    && x >= e.bounds.x
                    && x <= e.bounds.x + e.bounds.width
                    && y >= e.bounds.y
                    && y <= e.bounds.y + e.bounds.height
            })
            .collect();
        if let Some(i) = chain.iter().rposition(|e| e.id == hit_id) {
            chain.truncate(i + 1);
        } else {
            return vec![hit_id.to_string()];
        }
        if let Some(i) = chain.iter().rposition(|e| self.hover_specs.get(&e.id).map(|s| s.stop).unwrap_or(false))
        {
            chain.drain(..i);
        }
        chain.into_iter().map(|e| e.id.clone()).collect()
    }

    /// 手指移出某个元素就去掉它的点击态（微信：移出即取消，不等松手）
    pub fn update_press_position(&mut self, x: f32, y_viewport: f32, y_content: f32) -> bool {
        let Some(feedback) = &mut self.press_feedback else { return false };
        if feedback.hide_at.is_some() {
            return false;
        }
        let ids = feedback.ids.clone();
        let keep: Vec<String> = ids
            .into_iter()
            .filter(|id| {
                self.elements.iter().rev().find(|e| e.id == *id).map(|e| {
                    let y = if e.is_fixed { y_viewport } else { y_content };
                    x >= e.bounds.x
                        && x <= e.bounds.x + e.bounds.width
                        && y >= e.bounds.y
                        && y <= e.bounds.y + e.bounds.height
                }).unwrap_or(false)
            })
            .collect();
        let Some(feedback) = &mut self.press_feedback else { return false };
        let changed = keep.len() != feedback.ids.len();
        feedback.ids = keep;
        if self.press_feedback.as_ref().map(|p| p.ids.is_empty()).unwrap_or(false) {
            self.press_feedback = None;
            self.pressed_button = None;
        }
        changed
    }

    /// 清除按钮按下状态（松手）。还没到出现时间就直接取消；否则再保留 `hover-stay-time`。
    pub fn clear_button_pressed(&mut self) {
        let (show_at, ids, stay) = {
            let Some(feedback) = self.press_feedback.as_ref() else {
                self.pressed_button = None;
                return;
            };
            (feedback.show_at, feedback.ids.clone(), feedback.stay_ms)
        };
        let now = std::time::Instant::now();
        if now < show_at {
            self.press_feedback = None;
            self.pressed_button = None;
            return;
        }
        for id in ids {
            self.transitions.insert(id, crate::renderer::anim::now_secs());
        }
        if let Some(feedback) = self.press_feedback.as_mut() {
            feedback.hide_at = Some(now + std::time::Duration::from_millis(stay));
        }
        self.pressed_button = None;
    }

    /// 检查按钮是否处于点击态（含出现延迟和松手后的停留）
    pub fn is_button_pressed(&self, id: &str) -> bool {
        let Some(feedback) = &self.press_feedback else { return false };
        if !feedback.ids.iter().any(|i| i == id) {
            return false;
        }
        let now = std::time::Instant::now();
        match feedback.hide_at {
            Some(hide) => now < hide,
            None => now >= feedback.show_at,
        }
    }

    /// 点击态还在「等出现」或「松手后停留」：宿主要继续出帧，否则那一下永远画不出来
    pub fn press_feedback_pending(&self) -> bool {
        let Some(feedback) = &self.press_feedback else { return false };
        let now = std::time::Instant::now();
        // 多留一帧的时间，跨过 show_at / hide_at 的那一帧才画得到终态
        let slop = std::time::Duration::from_millis(40);
        match feedback.hide_at {
            Some(hide) => now < hide + slop,
            None => now < feedback.show_at + slop,
        }
    }
}
