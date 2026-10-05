//! 交互元素表与组件状态：每帧重建的元素登记、勾选/输入状态、滚动控制器索引
//!
//! `InteractionManager` 的一片，由 `interaction/mod.rs` 组合。**纯搬迁**：从 1041 行的
//! interaction.rs 按职责切开，一行逻辑没改。
use super::*;

impl InteractionManager {
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

    /// 设置组件状态
    pub fn set_state(&mut self, id: String, state: ComponentState) {
        self.states.insert(id, state);
    }

    /// 获取滚动控制器
    pub fn get_scroll_controller(&self, id: &str) -> Option<&ScrollController> {
        self.scroll_controllers.get(id)
    }

    /// 获取可变滚动控制器
    pub fn get_scroll_controller_mut(&mut self, id: &str) -> Option<&mut ScrollController> {
        self.scroll_controllers.get_mut(id)
    }

    /// 本帧登记的 scroll-view 盒子：`(逻辑包围盒, 是否在 fixed 覆盖层里)`。
    /// 正常流的是内容坐标，覆盖层的是视口坐标。
    pub fn scroll_area_bounds(&self, id: &str) -> Option<(Rect, bool)> {
        self.elements
            .iter()
            .rev()
            .find(|e| e.interaction_type == InteractionType::ScrollArea && e.id == id)
            .map(|e| (e.bounds, e.is_fixed))
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
        self.press_feedback = None;
        self.hover_specs.clear();
        self.click_animations.clear();
        self.elements.clear();
        self.is_selecting_text = false;
        self.selection_anchor = None;
    }
}
