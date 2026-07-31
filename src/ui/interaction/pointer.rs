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

    /// 设置按钮按下状态。同时登记过渡起始时刻，`transition` 才能从常态平滑到按压态。
    pub fn set_button_pressed(&mut self, id: String, bounds: Rect) {
        if self.pressed_button.as_ref().map(|b| b.id != id).unwrap_or(true) {
            self.transitions.insert(id.clone(), crate::renderer::anim::now_secs());
        }
        self.pressed_button = Some(PressedButton { id, bounds });
    }

    /// 清除按钮按下状态（松手），并登记回弹到常态的过渡起始时刻
    pub fn clear_button_pressed(&mut self) {
        if let Some(b) = self.pressed_button.take() {
            self.transitions.insert(b.id, crate::renderer::anim::now_secs());
        }
    }

    /// 检查按钮是否被按下
    pub fn is_button_pressed(&self, id: &str) -> bool {
        self.pressed_button.as_ref().map(|b| b.id == id).unwrap_or(false)
    }
}
