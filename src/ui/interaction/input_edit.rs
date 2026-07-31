//! 输入框编辑：键盘输入、失焦、文本选择
//!
//! `InteractionManager` 的一片，由 `interaction/mod.rs` 组合。**纯搬迁**：从 1041 行的
//! interaction.rs 按职责切开，一行逻辑没改。
use super::*;

impl InteractionManager {
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
}
