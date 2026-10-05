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
    pub bounds: Rect, // 输入框位置（**逻辑**坐标，页面内容坐标系）
    /// 是否在 `position:fixed` 覆盖层里。宿主定位输入法候选框时要据此决定
    /// 要不要减掉页面滚动量。
    pub is_fixed: bool,
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

/// 一个元素的点击态计时（毫秒）
#[derive(Clone, Copy, Debug)]
pub struct HoverSpec {
    pub start_ms: u64,
    pub stay_ms: u64,
    /// `hover-stop-propagation`：祖先不再进入点击态
    pub stop: bool,
}

/// 这一次按住产生的点击态
#[derive(Clone, Debug)]
struct PressFeedback {
    /// 命中的那个元素（祖先在 `ids` 里，且排在它前面）
    hit_id: String,
    ids: Vec<String>,
    /// 点击态开始显示的时刻
    show_at: std::time::Instant,
    /// 松手后点击态消失的时刻；按住期间是 None
    hide_at: Option<std::time::Instant>,
    stay_ms: u64,
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
    /// 点击态（hover-class）的出现/保留计时。微信默认按钮按下 20ms 后出现，松手后再留 70ms
    press_feedback: Option<PressFeedback>,
    /// 每个可点元素的 hover 计时（登记时写入）
    hover_specs: HashMap<String, HoverSpec>,
    /// 点击动画
    pub click_animations: Vec<ClickAnimation>,
    /// 当前页面的交互元素
    elements: Vec<InteractiveElement>,
    /// 是否正在拖动选择文本
    pub is_selecting_text: bool,
    /// 选择起始位置（用于拖动选择）
    pub selection_anchor: Option<usize>,
}

// ── 按职责切开的几片（都是 `impl InteractionManager`）──
/// 点击动画与过渡计时
mod anim;
/// 交互元素表与组件状态
mod elements;
/// 命中测试
mod hit;
/// 输入框编辑与文本选择
mod input_edit;
/// 指针交互（点击 / 滑块 / 按压态）
mod pointer;

impl InteractionManager {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            transitions: HashMap::new(),
            focused_input: None,
            dragging_slider: None,
            pressed_button: None,
            press_feedback: None,
            hover_specs: HashMap::new(),
            click_animations: Vec::new(),
            elements: Vec::new(),
            scroll_controllers: HashMap::new(),
            dragging_scroll_area: None,
            is_selecting_text: false,
            selection_anchor: None,
        }
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
