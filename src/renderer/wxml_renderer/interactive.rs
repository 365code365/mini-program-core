//! 有状态组件的**唯一**一份「状态 → 本帧节点」与「带交互的绘制」。
//!
//! ## 为什么要合并
//! 这两段逻辑曾经在 `draw_interactive.rs` 里各有多份拷贝：
//!
//! | 路径 | 状态落地 | 组件绘制 |
//! |---|---|---|
//! | `draw_with_interaction`（页面顶层节点） | 完整版 | 完整版 |
//! | `draw_child_with_interaction`（子节点递归） | 完整版（与上面逐字节相同） | 完整版（与上面只差一个空行） |
//! | `draw_child_to_cache`（`scroll-view` 的离屏缓存） | **退化版** | 通用绘制 |
//!
//! 退化版不是「简化」，是**漏了两件事**，而且只在滚动容器里犯：
//! 1. **不落 `input`/`textarea` 的当前值** —— 用户在滚动容器里的输入框敲的字**不显示**。
//!    占位符照旧能看见（那是组件自己按 attrs 画的），所以现象是「敲了字框里还是灰提示」，
//!    很容易被当成输入法没生效。回归判据见 `tests/scroll_cache_state_tests.rs`
//!    （实测退化时墨迹 396 vs 正常 1500）；
//! 2. `switch` 与 `checkbox` 被合成一条分支，于是开关被套上勾选框的选中配色，
//!    并且丢掉拨动进度 —— 手指拖到一半的开关在 `scroll-view` 里直接跳到 0/1。
//!
//! 三份拷贝里有两份逐字节相同、第三份少两个分支，这正是复制粘贴的典型下场：
//! 修的时候只改到了看得见的那两份。合并之后缓存路径自动获得同样的语义。

use super::*;

impl WxmlRenderer {
    /// 把交互状态（勾选 / 开关进度 / 滑块值 / 输入框文本）落到「本帧要画的那份节点」上。
    ///
    /// `switch_progress` 是拨动动画的进度（`None` 表示按 checked 取 0/1）：它要 `&mut self`
    /// 才算得出来，而离屏缓存那条路径只有 `&self`，所以那边传 `None` —— 与合并前一致。
    pub(super) fn apply_interaction_state(
        node: &RenderNode,
        target: &mut RenderNode,
        interaction: &InteractionManager,
        component_id: &str,
        switch_progress: Option<f32>,
    ) {
        /// 勾选类组件的选中/未选中配色（微信默认绿，未选中是白底浅灰边）
        fn checked_palette(node: &RenderNode, target: &mut RenderNode, checked: bool) {
            let color = node
                .attrs
                .get("color")
                .and_then(|c| crate::renderer::components::parse_color_str(c))
                .unwrap_or(Color::from_hex(0x09BB07));
            if checked {
                target.style.background_color = Some(color);
                target.style.border_color = Some(color);
            } else {
                target.style.background_color = Some(Color::WHITE);
                target.style.border_color = Some(Color::from_hex(0xD1D1D1));
            }
        }

        if let Some(state) = interaction.get_state(component_id) {
            match node.tag.as_str() {
                // switch 的配色由组件自己按进度求值（轨道关态是微信的浅灰，不是纯白）
                "switch" => {
                    target.style.custom_data =
                        switch_progress.unwrap_or(if state.checked { 1.0 } else { 0.0 });
                }
                "checkbox" => {
                    target.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                    checked_palette(node, target, state.checked);
                }
                "radio" => {
                    target.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                    checked_palette(node, target, state.checked);
                }
                "slider" => {
                    if let Ok(v) = state.value.parse::<f32>() {
                        target.style.custom_data = v / 100.0;
                        if !target.text.is_empty() {
                            target.text = format!("{}", v as i32);
                        }
                    }
                }
                "input" | "textarea" => {
                    let placeholder = node.attrs.get("placeholder").cloned().unwrap_or_default();
                    let is_focused = interaction
                        .focused_input
                        .as_ref()
                        .map(|f| f.id == component_id)
                        .unwrap_or(false);
                    if state.value.is_empty() && !is_focused {
                        // 没有输入值且未聚焦：显示占位符
                        target.text = placeholder;
                        target.style.text_color = Some(Color::from_hex(0xBFBFBF));
                    } else {
                        // 有值或已聚焦：显示实际值（聚焦时即使为空也不显示占位符）
                        target.text = state.value.clone();
                        target.style.text_color = Some(Color::BLACK);
                    }
                }
                _ => {}
            }
        } else if matches!(node.tag.as_str(), "input" | "textarea") {
            // 输入框还没有交互状态：显示占位符或 WXML 里写的初始值
            let placeholder = node.attrs.get("placeholder").cloned().unwrap_or_default();
            let initial_value = Self::input_value_attr(node);
            if initial_value.is_empty() {
                target.text = placeholder;
                target.style.text_color = Some(Color::from_hex(0xBFBFBF));
            } else {
                target.text = initial_value;
            }
        }
    }

    /// 画一个可能带交互状态的组件：输入框要带光标与选区，按钮要带按压态，
    /// 其余走通用绘制。
    ///
    /// 输入框还会回写 `text_offset`（超长文本的横向滚动量）—— 点击定位光标要用它，
    /// 所以这里需要 `&mut InteractionManager`。
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_interactive_component(
        &self,
        canvas: &mut Canvas,
        node: &RenderNode,
        target: &RenderNode,
        interaction: &mut InteractionManager,
        component_id: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        sf: f32,
    ) {
        match node.tag.as_str() {
            "input" | "textarea" => {
                let focused = interaction
                    .focused_input
                    .as_ref()
                    .map(|f| f.id == component_id)
                    .unwrap_or(false);
                let (cursor_pos, selection) = if focused {
                    let f = interaction.focused_input.as_ref().unwrap();
                    (f.cursor_pos, f.get_selection_range())
                } else {
                    (0, None)
                };
                InputComponent::draw_with_selection(
                    target,
                    canvas,
                    self.text_renderer.as_deref(),
                    x,
                    y,
                    w,
                    h,
                    sf,
                    focused,
                    cursor_pos,
                    selection,
                );
                // 回写 text_offset：文本比可视区宽时，光标要能推着文字横向滚动
                if focused {
                    if let Some(tr) = self.text_renderer.as_deref() {
                        let font_size = target.style.font_size * sf;
                        let padding_left = 12.0 * sf;
                        let padding_right = 12.0 * sf;
                        let available_width = w - padding_left - padding_right;
                        let text_width = tr.measure_text(&target.text, font_size);
                        let mut text_offset = 0.0;
                        if text_width > available_width {
                            let cursor_text: String =
                                target.text.chars().take(cursor_pos).collect();
                            let cursor_x_in_text = tr.measure_text(&cursor_text, font_size);
                            if cursor_x_in_text > available_width {
                                text_offset = available_width - cursor_x_in_text - font_size;
                            }
                        }
                        if let Some(input) = &mut interaction.focused_input {
                            input.text_offset = text_offset;
                        }
                    }
                }
            }
            "button" => {
                let pressed = interaction.is_button_pressed(component_id);
                ButtonComponent::draw_with_state(
                    target,
                    canvas,
                    self.text_renderer.as_deref(),
                    x,
                    y,
                    w,
                    h,
                    sf,
                    pressed,
                );
            }
            _ => {
                self.draw_component(canvas, target, x, y, w, h, sf);
            }
        }
    }
}
