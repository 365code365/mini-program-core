//! 交互元素表与组件状态：每帧重建的元素登记、勾选/输入状态、滚动控制器索引
//!
//! `InteractionManager` 的一片，由 `interaction/mod.rs` 组合。从 1041 行的 interaction.rs
//! 按职责切开时是纯搬迁；之后 `clear_page_state` 多了一步「收掉全屏视频 / 操作菜单 / 图片预览」。
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

    /// 本帧登记的交互元素（文档顺序；测试与诊断用）
    pub fn elements(&self) -> &[InteractiveElement] {
        &self.elements
    }

    /// 输入框的受控值：页面 `setData` 改了 `value` 属性（与上一次见到的不同）就以页面为准，
    /// 覆盖掉输入框自己记着的内容（含聚焦中的那份，光标夹到末尾）；没变就保留用户打的字。返回这一帧该显示的值。
    ///
    /// 从前交互状态永远压过属性：打过字之后 `setData({ keyword: '' })` 清不掉输入框
    /// （news-app 搜索页点「×」，数据清了、×也没了，框里的字还在）。与微信一致：value 是受控的
    pub fn sync_input_value(&mut self, id: &str, attr: &str) -> String {
        let changed = self.input_attrs.get(id).is_some_and(|last| last != attr);
        self.input_attrs.insert(id.to_string(), attr.to_string());
        if changed {
            if let Some(s) = self.states.get_mut(id) {
                s.value = attr.to_string();
            }
            if let Some(f) = self.focused_input.as_mut().filter(|f| f.id == id) {
                if f.value != attr {
                    // 光标保持「离末尾的字数」不变：页面在 bindinput 里格式化（手机号加空格、金额加千分位）时，
                    // 在末尾打字光标还在末尾，在中间改光标跟着那段字走。从前按下标夹住，
                    // 每插一个空格光标就落后一位（「139 5901|48」）
                    let old_len = f.value.chars().count();
                    let new_len = attr.chars().count();
                    let from_end = old_len.saturating_sub(f.cursor_pos);
                    f.value = attr.to_string();
                    f.cursor_pos = new_len.saturating_sub(from_end);
                    f.selection_start = None;
                    f.selection_end = None;
                    self.focused_value_changed = true;
                }
            }
        }
        self.states.get(id).map(|s| s.value.clone()).unwrap_or_else(|| attr.to_string())
    }

    /// 把输入框内容整个换掉（`bindinput` 返回了字符串）：光标放到末尾、选区清掉
    pub fn replace_input_value(&mut self, id: &str, value: &str) {
        if let Some(s) = self.states.get_mut(id) {
            s.value = value.to_string();
        }
        if let Some(f) = self.focused_input.as_mut().filter(|f| f.id == id) {
            f.value = value.to_string();
            f.cursor_pos = value.chars().count();
            f.selection_start = None;
            f.selection_end = None;
            self.focused_value_changed = true;
        }
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

    /// 页面被盖住 / tab 页切走：把这一页**要留着**的交互状态拿走（scroll-view 位置与受控状态、
    /// switch / checkbox 这类组件状态、slider 的值），其余照 [`clear_page_state`](Self::clear_page_state) 清掉。
    /// 回来时 [`restore_page_state`](Self::restore_page_state) 原样放回 —— 微信里返回上一页，
    /// 里面的横向列表还停在原来的位置
    pub fn take_page_state(&mut self) -> PageUiState {
        let s = PageUiState {
            states: std::mem::take(&mut self.states),
            toggled_at: std::mem::take(&mut self.toggled_at),
            sliders: std::mem::take(&mut self.sliders),
            slider_origin: std::mem::take(&mut self.slider_origin),
            input_attrs: std::mem::take(&mut self.input_attrs),
            scroll_controllers: std::mem::take(&mut self.scroll_controllers),
            scroll_views: std::mem::take(&mut self.scroll_views),
        };
        self.clear_page_state();
        s
    }

    /// 回到那一页：先清掉离开的那页的，再放回 [`take_page_state`](Self::take_page_state) 拿走的
    pub fn restore_page_state(&mut self, s: PageUiState) {
        self.clear_page_state();
        // 惯性 / 回弹停在离开那一刻的位置：回来时不该接着滑
        let mut controllers = s.scroll_controllers;
        for c in controllers.values_mut() {
            if c.is_animating() {
                c.stop();
            }
        }
        self.states = s.states;
        self.toggled_at = s.toggled_at;
        self.sliders = s.sliders;
        self.slider_origin = s.slider_origin;
        self.input_attrs = s.input_attrs;
        self.scroll_controllers = controllers;
        self.scroll_views = s.scroll_views;
    }

    /// 页面切换时清除状态
    pub fn clear_page_state(&mut self) {
        // 全屏视频、操作菜单、图片预览属于离开的那一页：不收掉的话新页面一直被它们盖着、点击全被吃掉
        crate::renderer::components::clear_fullscreen();
        crate::renderer::components::video_ui::clear();
        crate::host::ui_overlay::clear_sheet_overlays();
        // setBackgroundColor 只对当前页面生效
        crate::host::window_api::clear();
        crate::renderer::components::picker_view::clear();
        crate::renderer::components::clear_danmu();
        self.states.clear();
        self.transitions.clear();
        self.toggled_at.clear();
        self.focused_input = None;
        self.dragging_slider = None;
        self.sliders.clear();
        self.slider_origin.clear();
        self.input_attrs.clear();
        self.scroll_controllers.clear();
        self.scroll_views.clear();
        self.dragging_scroll_area = None;
        self.pressed_button = None;
        self.press_feedback = None;
        self.hover_specs.clear();
        self.click_animations.clear();
        self.elements.clear();
        self.is_selecting_text = false;
        self.selection_anchor = None;
        self.text_layouts.clear();
    }
}
