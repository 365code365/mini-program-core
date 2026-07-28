//! 组件交互管理器
//! 处理所有组件的交互状态和事件

use crate::Rect;
use std::collections::HashMap;
use super::scroll_controller::ScrollController;

/// 组件交互状态
#[derive(Clone, Debug, Default)]
pub struct ComponentState {
    pub checked: bool,
    pub value: String,
}

/// 聚焦的输入框
#[derive(Clone, Debug)]
pub struct FocusedInput {
    pub id: String,
    pub value: String,
    pub cursor_pos: usize,
    pub selection_start: Option<usize>, // 选择起始位置
    pub selection_end: Option<usize>,   // 选择结束位置
    pub is_password: bool,
    pub bounds: Rect, // 输入框位置
    pub text_offset: f32, // 文本滚动偏移（物理像素）
    pub maxlength: i32, // 最大输入长度，-1 为不限制
    pub input_type: String, // 输入类型：text/number/idcard/digit
}

impl FocusedInput {
    /// 是否有选中文本
    pub fn has_selection(&self) -> bool {
        self.selection_start.is_some() && self.selection_end.is_some()
    }
    
    /// 获取选中范围 (start, end)，保证 start <= end
    pub fn get_selection_range(&self) -> Option<(usize, usize)> {
        match (self.selection_start, self.selection_end) {
            (Some(s), Some(e)) if s != e => {
                Some((s.min(e), s.max(e)))
            }
            _ => None
        }
    }
    
    /// 全选
    pub fn select_all(&mut self) {
        self.selection_start = Some(0);
        self.selection_end = Some(self.value.chars().count());
        self.cursor_pos = self.value.chars().count();
    }
    
    /// 清除选择
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
    }
    
    /// 删除选中的文本，返回是否有删除
    pub fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.get_selection_range() {
            let chars: Vec<char> = self.value.chars().collect();
            let mut new_chars = Vec::new();
            for (i, c) in chars.into_iter().enumerate() {
                if i < start || i >= end {
                    new_chars.push(c);
                }
            }
            self.value = new_chars.into_iter().collect();
            self.cursor_pos = start;
            self.clear_selection();
            true
        } else {
            false
        }
    }
    
    /// 验证字符是否符合输入类型要求
    pub fn validate_char(&self, c: char) -> bool {
        match self.input_type.as_str() {
            "number" => c.is_ascii_digit() || c == '-',
            "digit" => c.is_ascii_digit() || c == '.',
            "idcard" => c.is_ascii_digit() || c == 'X' || c == 'x',
            _ => true,
        }
    }
    
    /// 检查是否可以插入更多字符（maxlength 限制）
    pub fn can_insert(&self, insert_len: usize) -> bool {
        if self.maxlength < 0 {
            return true; // -1 表示不限制
        }
        let current_len = self.value.chars().count();
        let selection_len = self.get_selection_range()
            .map(|(s, e)| e - s)
            .unwrap_or(0);
        // 当前长度 - 选中长度 + 插入长度 <= maxlength
        (current_len - selection_len + insert_len) <= self.maxlength as usize
    }
}

/// 计算光标位置
///
/// 根据 x 坐标（相对于输入框左边缘）和每个字符的宽度，计算光标应该在的位置
/// 返回光标位置（字符索引）
/// 
/// - text: 输入框文本
/// - char_widths: 每个字符的宽度（物理像素）
/// - click_x: 点击位置相对于输入框左边缘（物理像素）
/// - padding_left: 左边距（物理像素）
/// - text_offset: 文本滚动偏移（物理像素，负值表示向左滚动）
pub fn calculate_cursor_position(text: &str, char_widths: &[f32], click_x: f32, padding_left: f32, text_offset: f32) -> usize {
    // 减去左边距后的点击位置，再减去文本偏移（因为文本向左滚动时，点击位置相对于文本起点更靠右）
    let click_offset = click_x - padding_left - text_offset;

    if click_offset <= 0.0 {
        // 点击在文本左侧，光标在开头
        return 0;
    }

    let mut cumulative_width = 0.0;
    for (i, &width) in char_widths.iter().enumerate() {
        let next_width = cumulative_width + width;

        if click_offset < next_width - width / 2.0 {
            // 点击在字符的前半部分，光标在字符之前
            return i;
        } else if click_offset >= next_width - width / 2.0 && click_offset < next_width {
            // 点击在字符的后半部分，光标在字符之后
            return i + 1;
        }

        cumulative_width = next_width;
    }

    // 点击在所有字符之后，光标在末尾
    text.chars().count()
}

/// 拖动中的滑块
#[derive(Clone, Debug)]
pub struct DraggingSlider {
    pub id: String,
    pub bounds: Rect,
    pub min: f32,
    pub max: f32,
}

/// 交互组件类型
#[derive(Debug, Clone, PartialEq)]
pub enum InteractionType {
    Checkbox,
    Radio,
    Switch,
    Slider,
    Input,
    Button,
    ScrollArea,
    View,
}

/// 可交互组件信息
#[derive(Debug, Clone)]
pub struct InteractiveElement {
    pub interaction_type: InteractionType,
    pub id: String,
    pub bounds: Rect,
    pub checked: bool,
    pub value: String,
    pub disabled: bool,
    pub min: f32,
    pub max: f32,
    // Scroll area specific
    pub content_height: f32,
    pub viewport_height: f32,
    pub content_width: f32,
    pub viewport_width: f32,
    pub is_horizontal: bool,
    pub is_fixed: bool,
}

/// 按下的按钮
#[derive(Clone, Debug)]
pub struct PressedButton {
    pub id: String,
    pub bounds: Rect,
}

/// 点击动画状态
#[derive(Clone, Debug)]
pub struct ClickAnimation {
    pub id: String,
    pub start_time: std::time::Instant,
    pub duration_ms: u64,
}

/// 交互管理器
pub struct InteractionManager {
    /// 组件状态
    pub states: HashMap<String, ComponentState>,
    /// 状态切换时刻（秒，全局动画时钟），用于 switch 滑块等状态过渡动画
    pub transitions: HashMap<String, f32>,
    /// 聚焦的输入框
    pub focused_input: Option<FocusedInput>,
    /// 拖动中的滑块
    pub dragging_slider: Option<DraggingSlider>,
    /// 滚动控制器集合
    pub scroll_controllers: HashMap<String, ScrollController>,
    /// 正在拖动的滚动区域 ID
    pub dragging_scroll_area: Option<String>,
    /// 按下的按钮
    pub pressed_button: Option<PressedButton>,
    /// 点击动画
    pub click_animations: Vec<ClickAnimation>,
    /// 当前页面的交互元素
    elements: Vec<InteractiveElement>,
    /// 是否正在拖动选择文本
    pub is_selecting_text: bool,
    /// 选择起始位置（用于拖动选择）
    pub selection_anchor: Option<usize>,
}

impl InteractionManager {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            transitions: HashMap::new(),
            focused_input: None,
            dragging_slider: None,
            pressed_button: None,
            click_animations: Vec::new(),
            elements: Vec::new(),
            scroll_controllers: HashMap::new(),
            dragging_scroll_area: None,
            is_selecting_text: false,
            selection_anchor: None,
        }
    }
    
    /// 清除交互元素列表（每次渲染前调用）
    pub fn clear_elements(&mut self) {
        self.elements.clear();
    }
    
    /// 丢掉正常流的交互元素，保留 `position: fixed` 覆盖层的。
    ///
    /// 每帧页面重绘前调用。元素表从前只增不减：滚动过后旧位置留下的陈旧元素还在表里，
    /// 而 `hit_test` 取「最后一个命中」—— 陈旧元素会遮住当前元素，点击落到一个
    /// 没有处理器的僵尸元素上就什么都不发生（表现为「页面点击全部失效」）。
    /// 覆盖层的元素不能在这里丢：它可能整帧都不重绘（性能优化），要单独重建。
    pub fn retain_only_fixed_elements(&mut self) {
        self.elements.retain(|e| e.is_fixed);
    }

    /// 丢掉覆盖层的交互元素（覆盖层重绘前调用）
    pub fn clear_fixed_elements(&mut self) {
        self.elements.retain(|e| !e.is_fixed);
    }

    /// 注册交互元素
    pub fn register_element(&mut self, element: InteractiveElement) {
        if element.interaction_type == InteractionType::ScrollArea {
            if !self.scroll_controllers.contains_key(&element.id) {
                let controller = if element.is_horizontal {
                    ScrollController::new_horizontal(element.content_width, element.viewport_width)
                } else {
                    ScrollController::new(element.content_height, element.viewport_height)
                };
                self.scroll_controllers.insert(element.id.clone(), controller);
            } else if let Some(controller) = self.scroll_controllers.get_mut(&element.id) {
                if element.is_horizontal {
                    controller.update_content_height(element.content_width, element.viewport_width);
                } else {
                    controller.update_content_height(element.content_height, element.viewport_height);
                }
            }
        }
        self.elements.push(element);
    }
    
    /// 获取组件状态
    pub fn get_state(&self, id: &str) -> Option<&ComponentState> {
        self.states.get(id)
    }

    /// 获取滚动控制器
    pub fn get_scroll_controller(&self, id: &str) -> Option<&ScrollController> {
        self.scroll_controllers.get(id)
    }

    /// 获取可变滚动控制器
    pub fn get_scroll_controller_mut(&mut self, id: &str) -> Option<&mut ScrollController> {
        self.scroll_controllers.get_mut(id)
    }
    
    /// 设置组件状态
    pub fn set_state(&mut self, id: String, state: ComponentState) {
        self.states.insert(id, state);
    }
    
    /// 点击测试 - 返回点击到的交互元素
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&InteractiveElement> {
        self.elements.iter().rev().find(|e| {
            !e.disabled && 
            x >= e.bounds.x && x <= e.bounds.x + e.bounds.width &&
            y >= e.bounds.y && y <= e.bounds.y + e.bounds.height
        })
    }
    
    /// 只在 `position: fixed` 覆盖层的元素里做命中测试（坐标是视口坐标）。
    ///
    /// 直接用 `hit_test` 再判 `is_fixed` 是不对的：正常流的元素按内容坐标登记，
    /// 视口坐标下同样可能落在包围盒里，且注册顺序在覆盖层之后，
    /// 于是把真正的覆盖层元素挡掉。
    pub fn hit_test_fixed(&self, x: f32, y: f32) -> Option<&InteractiveElement> {
        self.elements.iter().rev().find(|e| {
            e.is_fixed && !e.disabled &&
            x >= e.bounds.x && x <= e.bounds.x + e.bounds.width &&
            y >= e.bounds.y && y <= e.bounds.y + e.bounds.height
        })
    }

    /// 命中点上**最内层的可滚区域**（面积最小的那个 scroll-view）。
    ///
    /// 手势仲裁要的是「这一下可能交给谁滚」，而 `hit_test` 只会给出最上层的元素 ——
    /// 卡片、按钮盖在 scroll-view 上面时它返回的是卡片，于是整页只会走页面滚动，
    /// 真正装着内容的 scroll-view 一动不动。
    pub fn hit_test_scroll_area(&self, x: f32, y: f32, fixed: bool) -> Option<&InteractiveElement> {
        self.scroll_areas_at(x, y, fixed).next()
    }

    /// 命中点上的所有可滚区域，**由内到外**（面积从小到大）。
    ///
    /// 嵌套滚动传递需要整条链：内层不可滚、或已经推到边界时要接着问外层，
    /// 最后才落到页面上（浏览器/微信的 scroll chaining）。
    pub fn scroll_areas_at(
        &self,
        x: f32,
        y: f32,
        fixed: bool,
    ) -> impl Iterator<Item = &InteractiveElement> {
        let mut hits: Vec<&InteractiveElement> = self
            .elements
            .iter()
            .filter(|e| {
                e.is_fixed == fixed
                    && e.interaction_type == InteractionType::ScrollArea
                    && !e.disabled
                    && x >= e.bounds.x
                    && x <= e.bounds.x + e.bounds.width
                    && y >= e.bounds.y
                    && y <= e.bounds.y + e.bounds.height
            })
            .collect();
        hits.sort_by(|a, b| {
            let area_a = a.bounds.width * a.bounds.height;
            let area_b = b.bounds.width * b.bounds.height;
            area_a.partial_cmp(&area_b).unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.into_iter()
    }

    /// `area` 范围内所有**选中**的 `kind` 元素的 `value`，按文档顺序。
    ///
    /// 微信里 `checkbox-group` / `radio-group` 的 `change` 事件
    /// `detail.value` 就是这个集合（多选是数组），而不是被点那一个的开关状态。
    /// 组的成员用「中心点落在组的包围盒内」界定 —— 渲染层没有回传父子关系，
    /// 而组本身就是一个把成员包住的盒子。
    pub fn checked_values_in(&self, area: &Rect, kind: InteractionType) -> Vec<String> {
        self.elements
            .iter()
            .filter(|e| e.interaction_type == kind)
            .filter(|e| {
                let c = crate::Point::new(
                    e.bounds.x + e.bounds.width / 2.0,
                    e.bounds.y + e.bounds.height / 2.0,
                );
                area.contains(&c)
            })
            .filter(|e| self.states.get(&e.id).map(|s| s.checked).unwrap_or(e.checked))
            .map(|e| e.value.clone())
            .collect()
    }

    /// 只在正常流的元素里做命中测试（坐标是内容坐标，即已加上页面滚动量）
    pub fn hit_test_flow(&self, x: f32, y: f32) -> Option<&InteractiveElement> {
        self.elements.iter().rev().find(|e| {
            !e.is_fixed && !e.disabled &&
            x >= e.bounds.x && x <= e.bounds.x + e.bounds.width &&
            y >= e.bounds.y && y <= e.bounds.y + e.bounds.height
        })
    }

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
                
                self.focused_input = Some(FocusedInput {
                    id: element.id.clone(),
                    value: current_value.clone(),
                    cursor_pos: current_value.chars().count(),
                    selection_start: None,
                    selection_end: None,
                    is_password: false,
                    bounds: element.bounds,
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
    
    /// 处理键盘输入
    pub fn handle_key_input(&mut self, key: KeyInput) -> Option<InteractionResult> {
        let input = self.focused_input.as_mut()?;
        
        match key {
            KeyInput::Char(c) => {
                // 验证字符是否符合输入类型
                if !input.validate_char(c) {
                    return None;
                }
                
                // 检查 maxlength 限制
                if !input.can_insert(1) {
                    return None;
                }
                
                // 如果有选中文本，先删除
                input.delete_selection();
                
                let mut chars: Vec<char> = input.value.chars().collect();
                chars.insert(input.cursor_pos, c);
                input.value = chars.into_iter().collect();
                input.cursor_pos += 1;
                
                // 同步到状态
                self.states.insert(input.id.clone(), ComponentState {
                    checked: false,
                    value: input.value.clone(),
                });
                
                Some(InteractionResult::InputChange {
                    id: input.id.clone(),
                    value: input.value.clone(),
                })
            }
            KeyInput::Backspace => {
                // 如果有选中文本，删除选中部分
                if input.delete_selection() {
                    self.states.insert(input.id.clone(), ComponentState {
                        checked: false,
                        value: input.value.clone(),
                    });
                    return Some(InteractionResult::InputChange {
                        id: input.id.clone(),
                        value: input.value.clone(),
                    });
                }
                
                if input.cursor_pos > 0 {
                    let mut chars: Vec<char> = input.value.chars().collect();
                    chars.remove(input.cursor_pos - 1);
                    input.value = chars.into_iter().collect();
                    input.cursor_pos -= 1;
                    
                    self.states.insert(input.id.clone(), ComponentState {
                        checked: false,
                        value: input.value.clone(),
                    });
                    
                    Some(InteractionResult::InputChange {
                        id: input.id.clone(),
                        value: input.value.clone(),
                    })
                } else {
                    None
                }
            }
            KeyInput::Delete => {
                // 如果有选中文本，删除选中部分
                if input.delete_selection() {
                    self.states.insert(input.id.clone(), ComponentState {
                        checked: false,
                        value: input.value.clone(),
                    });
                    return Some(InteractionResult::InputChange {
                        id: input.id.clone(),
                        value: input.value.clone(),
                    });
                }
                
                let chars: Vec<char> = input.value.chars().collect();
                if input.cursor_pos < chars.len() {
                    let mut chars = chars;
                    chars.remove(input.cursor_pos);
                    input.value = chars.into_iter().collect();
                    
                    self.states.insert(input.id.clone(), ComponentState {
                        checked: false,
                        value: input.value.clone(),
                    });
                    
                    Some(InteractionResult::InputChange {
                        id: input.id.clone(),
                        value: input.value.clone(),
                    })
                } else {
                    None
                }
            }
            KeyInput::Left => {
                input.clear_selection();
                if input.cursor_pos > 0 {
                    input.cursor_pos -= 1;
                }
                None
            }
            KeyInput::Right => {
                input.clear_selection();
                if input.cursor_pos < input.value.chars().count() {
                    input.cursor_pos += 1;
                }
                None
            }
            KeyInput::Home => {
                input.clear_selection();
                input.cursor_pos = 0;
                None
            }
            KeyInput::End => {
                input.clear_selection();
                input.cursor_pos = input.value.chars().count();
                None
            }
            KeyInput::SelectAll => {
                input.select_all();
                None
            }
            KeyInput::Copy => {
                // 返回选中的文本用于复制
                if let Some((start, end)) = input.get_selection_range() {
                    let selected: String = input.value.chars().skip(start).take(end - start).collect();
                    return Some(InteractionResult::CopyText { text: selected });
                }
                None
            }
            KeyInput::Cut => {
                // 剪切：复制并删除
                if let Some((start, end)) = input.get_selection_range() {
                    let selected: String = input.value.chars().skip(start).take(end - start).collect();
                    input.delete_selection();
                    
                    self.states.insert(input.id.clone(), ComponentState {
                        checked: false,
                        value: input.value.clone(),
                    });
                    
                    return Some(InteractionResult::CutText { 
                        text: selected,
                        id: input.id.clone(),
                        value: input.value.clone(),
                    });
                }
                None
            }
            KeyInput::Paste(text) => {
                // 过滤不符合输入类型的字符
                let filtered_text: String = text.chars()
                    .filter(|&c| input.validate_char(c))
                    .collect();
                
                if filtered_text.is_empty() {
                    return None;
                }
                
                // 检查 maxlength 限制
                let insert_len = filtered_text.chars().count();
                if !input.can_insert(insert_len) {
                    // 截断到允许的长度
                    let current_len = input.value.chars().count();
                    let selection_len = input.get_selection_range()
                        .map(|(s, e)| e - s)
                        .unwrap_or(0);
                    let max_insert = if input.maxlength < 0 {
                        insert_len
                    } else {
                        (input.maxlength as usize).saturating_sub(current_len - selection_len)
                    };
                    if max_insert == 0 {
                        return None;
                    }
                    let truncated: String = filtered_text.chars().take(max_insert).collect();
                    
                    // 如果有选中文本，先删除
                    input.delete_selection();
                    
                    // 插入截断后的文本
                    let mut chars: Vec<char> = input.value.chars().collect();
                    for (i, c) in truncated.chars().enumerate() {
                        chars.insert(input.cursor_pos + i, c);
                    }
                    input.value = chars.into_iter().collect();
                    input.cursor_pos += truncated.chars().count();
                } else {
                    // 如果有选中文本，先删除
                    input.delete_selection();
                    
                    // 插入粘贴的文本
                    let mut chars: Vec<char> = input.value.chars().collect();
                    for (i, c) in filtered_text.chars().enumerate() {
                        chars.insert(input.cursor_pos + i, c);
                    }
                    input.value = chars.into_iter().collect();
                    input.cursor_pos += filtered_text.chars().count();
                }
                
                self.states.insert(input.id.clone(), ComponentState {
                    checked: false,
                    value: input.value.clone(),
                });
                
                Some(InteractionResult::InputChange {
                    id: input.id.clone(),
                    value: input.value.clone(),
                })
            }
            KeyInput::ShiftLeft => {
                // 扩展选择向左
                if input.selection_start.is_none() {
                    input.selection_start = Some(input.cursor_pos);
                    input.selection_end = Some(input.cursor_pos);
                }
                if input.cursor_pos > 0 {
                    input.cursor_pos -= 1;
                    input.selection_end = Some(input.cursor_pos);
                }
                None
            }
            KeyInput::ShiftRight => {
                // 扩展选择向右
                if input.selection_start.is_none() {
                    input.selection_start = Some(input.cursor_pos);
                    input.selection_end = Some(input.cursor_pos);
                }
                if input.cursor_pos < input.value.chars().count() {
                    input.cursor_pos += 1;
                    input.selection_end = Some(input.cursor_pos);
                }
                None
            }
            KeyInput::ShiftHome => {
                // 选择到开头
                if input.selection_start.is_none() {
                    input.selection_start = Some(input.cursor_pos);
                }
                input.cursor_pos = 0;
                input.selection_end = Some(0);
                None
            }
            KeyInput::ShiftEnd => {
                // 选择到结尾
                if input.selection_start.is_none() {
                    input.selection_start = Some(input.cursor_pos);
                }
                let len = input.value.chars().count();
                input.cursor_pos = len;
                input.selection_end = Some(len);
                None
            }
            KeyInput::Enter => {
                // Enter 键触发 confirm 事件
                let id = input.id.clone();
                let value = input.value.clone();
                // 注意：confirm 事件不一定要失焦，取决于 confirm-hold 属性
                // 这里默认失焦，如果需要保持焦点，可以在外部处理
                self.focused_input = None;
                Some(InteractionResult::InputConfirm { id, value })
            }
            KeyInput::Escape => {
                let id = input.id.clone();
                let value = input.value.clone();
                self.focused_input = None;
                Some(InteractionResult::InputBlur { id, value })
            }
        }
    }
    
    /// 取消输入框聚焦
    pub fn blur_input(&mut self) -> Option<InteractionResult> {
        if let Some(input) = self.focused_input.take() {
            return Some(InteractionResult::InputBlur {
                id: input.id,
                value: input.value,
            });
        }
        None
    }
    
    /// 是否有输入框聚焦
    pub fn has_focused_input(&self) -> bool {
        self.focused_input.is_some()
    }
    
    /// 是否正在拖动滑块
    pub fn is_dragging_slider(&self) -> bool {
        self.dragging_slider.is_some()
    }
    
    /// 页面切换时清除状态
    pub fn clear_page_state(&mut self) {
        self.states.clear();
        self.transitions.clear();
        self.focused_input = None;
        self.dragging_slider = None;
        self.scroll_controllers.clear();
        self.dragging_scroll_area = None;
        self.pressed_button = None;
        self.click_animations.clear();
        self.elements.clear();
        self.is_selecting_text = false;
        self.selection_anchor = None;
    }
    
    /// 准备文本选择（鼠标按下时调用）
    /// 只移动光标到点击位置，清除之前的选择，记录锚点位置
    /// 实际选择在鼠标移动时才开始
    pub fn prepare_text_selection(&mut self, cursor_pos: usize) {
        if let Some(input) = &mut self.focused_input {
            // 清除之前的选择
            input.clear_selection();
            // 移动光标到点击位置
            input.cursor_pos = cursor_pos;
            // 记录锚点位置，但不开始选择
            self.selection_anchor = Some(cursor_pos);
            self.is_selecting_text = false; // 还没开始选择
        }
    }
    
    /// 开始文本选择（鼠标按下时调用，用于需要立即开始选择的情况）
    pub fn begin_text_selection(&mut self, cursor_pos: usize) {
        if let Some(input) = &mut self.focused_input {
            self.is_selecting_text = true;
            self.selection_anchor = Some(cursor_pos);
            input.selection_start = Some(cursor_pos);
            input.selection_end = Some(cursor_pos);
            input.cursor_pos = cursor_pos;
        }
    }
    
    /// 更新文本选择（鼠标移动时调用）
    pub fn update_text_selection(&mut self, cursor_pos: usize) {
        // 如果有锚点但还没开始选择，现在开始
        if let Some(anchor) = self.selection_anchor {
            if let Some(input) = &mut self.focused_input {
                // 只有当光标位置与锚点不同时才开始选择
                if cursor_pos != anchor {
                    self.is_selecting_text = true;
                    input.selection_start = Some(anchor);
                    input.selection_end = Some(cursor_pos);
                }
                input.cursor_pos = cursor_pos;
            }
        }
    }
    
    /// 结束文本选择（鼠标释放时调用）
    pub fn end_text_selection(&mut self) {
        self.is_selecting_text = false;
        self.selection_anchor = None;
        // 如果选择范围为空，清除选择
        if let Some(input) = &mut self.focused_input {
            if let (Some(start), Some(end)) = (input.selection_start, input.selection_end) {
                if start == end {
                    input.clear_selection();
                }
            }
        }
    }
    
    /// 是否正在选择文本（或准备选择）
    pub fn is_selecting(&self) -> bool {
        self.selection_anchor.is_some()
    }
    
    /// 是否真正在拖动选择（有实际的选择范围）
    pub fn is_dragging_selection(&self) -> bool {
        self.is_selecting_text
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
    
    /// 触发点击动画
    pub fn trigger_click_animation(&mut self, id: String) {
        // 移除该 id 的旧动画
        self.click_animations.retain(|a| a.id != id);
        // 添加新动画
        self.click_animations.push(ClickAnimation {
            id,
            start_time: std::time::Instant::now(),
            duration_ms: 150, // 150ms 动画
        });
    }
    
    /// 更新动画状态，返回是否还有动画在进行
    pub fn update_animations(&mut self) -> bool {
        let now = std::time::Instant::now();
        self.click_animations.retain(|a| {
            now.duration_since(a.start_time).as_millis() < a.duration_ms as u128
        });
        !self.click_animations.is_empty()
    }
    
    /// 检查按钮是否在点击动画中
    pub fn is_in_click_animation(&self, id: &str) -> bool {
        let now = std::time::Instant::now();
        self.click_animations.iter().any(|a| {
            a.id == id && now.duration_since(a.start_time).as_millis() < a.duration_ms as u128
        })
    }
    
    /// 是否有动画在进行
    pub fn has_animations(&self) -> bool {
        !self.click_animations.is_empty()
    }
}

/// 键盘输入类型
#[derive(Debug, Clone)]
pub enum KeyInput {
    Char(char),
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    Enter,
    Escape,
    SelectAll,      // Ctrl+A
    Copy,           // Ctrl+C
    Cut,            // Ctrl+X
    Paste(String),  // Ctrl+V
    ShiftLeft,      // Shift+Left (扩展选择)
    ShiftRight,     // Shift+Right
    ShiftHome,      // Shift+Home
    ShiftEnd,       // Shift+End
}

/// 交互结果
#[derive(Debug, Clone)]
pub enum InteractionResult {
    Toggle { id: String, checked: bool },
    Select { id: String, value: String },
    SliderChange { id: String, value: i32 },
    SliderEnd { id: String },
    /// 输入框获得焦点。`value` 是聚焦瞬间的文本，微信的 `bindfocus` 事件
    /// `detail` 里带的就是它（`{ value, height }`）。
    Focus { id: String, bounds: Rect, click_x: f32, is_fixed: bool, value: String },
    InputChange { id: String, value: String },
    InputBlur { id: String, value: String },
    InputConfirm { id: String, value: String }, // 按下回车/完成键
    ButtonClick { id: String, bounds: Rect },
    CopyText { text: String },
    CutText { text: String, id: String, value: String },
}
