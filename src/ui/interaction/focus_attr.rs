//! input / textarea 的 `focus` / `auto-focus` 属性：页面用数据控制聚焦。
//!
//! 官方（component/input.html、textarea.html）：`focus`「获取焦点」、`auto-focus`「自动聚焦，拉起键盘」
//! （input 上标「即将废弃，请直接使用 focus」）。语义按受控属性做（与 `checked`、`value` 同一套，elements.rs）：
//! - 挂载时为真、或从假变真：聚焦并拉起键盘（宿主收到 `inputFocus`），派 `bindfocus`；
//! - 从真变假且它正聚焦着：失焦（`bindblur`、收键盘）；
//! - 属性一直为真、用户点别处失焦了：不再抢回来（属性没变，微信同样要页面再 setData 一次）。
//!
//! 只在绘制登记交互元素时比对（`wxml_renderer/interactions.rs`），请求排到这一帧画完、由宿主取走执行
//! （`host::input_focus`）：那时元素的盒子与属性都是新的。每帧代价：每个画到的输入框一次表查找。
//!
//! 已知取舍：没写 `id` 的输入框身份里含坐标（`get_component_id`），在 scroll-view 里滚动后会被当成「新挂载」，
//! 属性仍为真时会再聚焦一次；`wx:if` 拆掉又装回、`focus` 一直为真时不会再聚焦（没见过它被拆掉）。
use super::*;

/// 排到这一帧画完再执行的聚焦变化
#[derive(Clone, Debug, PartialEq)]
pub enum FocusRequest {
    Focus(String),
    Blur(String),
}

impl InteractionManager {
    /// 登记时调：`want` 是 `focus || auto-focus` 求值之后的真假
    pub fn sync_focus_attr(&mut self, id: &str, want: bool, disabled: bool) {
        let key = format!("focus#{id}");
        let now = if want { "1" } else { "0" };
        let prev = self.input_attrs.insert(key, now.to_string());
        if prev.as_deref() == Some(now) {
            return;
        }
        let focused_here = self.focused_input.as_ref().is_some_and(|f| f.id == id);
        if want && !disabled && !focused_here {
            self.focus_request = Some(FocusRequest::Focus(id.to_string()));
        } else if !want && prev.is_some() && focused_here {
            self.focus_request = Some(FocusRequest::Blur(id.to_string()));
        }
    }

    /// 宿主每帧画完取走
    pub fn take_focus_request(&mut self) -> Option<FocusRequest> {
        self.focus_request.take()
    }

    /// 按属性聚焦（不经点击）：光标在末尾，`cursor` / `selection-*` 由宿主按属性补（host::input_focus）。
    /// 元素这一帧没登记（不在视口里）返回 None
    pub fn focus_by_id(&mut self, id: &str) -> Option<InteractionResult> {
        let el = self.elements.iter().rev().find(|e| e.id == id && e.interaction_type == InteractionType::Input)?.clone();
        if el.disabled {
            return None;
        }
        let value = self.states.get(id).map(|s| s.value.clone()).unwrap_or_else(|| el.value.clone());
        self.states.entry(id.to_string()).or_insert_with(|| ComponentState { checked: false, value: value.clone() });
        crate::renderer::components::reset_cursor_blink();
        self.focused_input = Some(FocusedInput::new(id.to_string(), value.clone(), el.bounds, el.is_fixed));
        Some(InteractionResult::Focus { id: id.to_string(), bounds: el.bounds, click_x: 0.0, is_fixed: el.is_fixed, value })
    }
}
