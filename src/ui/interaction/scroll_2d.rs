//! 横竖两个方向都能滚的 scroll-view（`scroll-x` 与 `scroll-y` 同时写）。
//!
//! 主控制器（键 = 组件 id）照旧管纵向：命中测试、阈值事件、下拉刷新、吸顶、滚动条、受控 `scroll-top`
//! 都不用改。横向另放一个控制器，键是 `"{id}#x"`（[`scroll_x_key`]），手势、滚轮、`scroll-left`、
//! `scrollTo({left})` 驱动它，绘制时子节点再按它往左挪。
//!
//! 为什么不给 `InteractiveElement` 加字段：它在十几处按结构体字面量造，加一个字段要改一圈；
//! 而「是不是双向」只有手势仲裁、绘制、事件三处要问，问 `scroll_views[id].both_axes` 就够了。
//!
//! 损伤区：两个控制器各自记「上次画在哪」（engine_damage_area.rs 遍历全部控制器），
//! 横向那个的盒子按去掉后缀的 id 查（[`scroll_base_id`]），于是横着滚也只重画这个盒子。
use super::*;

/// 横向控制器键的后缀。组件 id 里不会有 `#`（`get_component_id` 用 id 属性或 `tag_x_y`）
pub const SCROLL_X_SUFFIX: &str = "#x";

/// 双向 scroll-view 的横向控制器键
pub fn scroll_x_key(id: &str) -> String {
    format!("{id}{SCROLL_X_SUFFIX}")
}

/// 控制器键 → scroll-view 的组件 id（横向控制器去掉后缀，其余原样）
pub fn scroll_base_id(key: &str) -> &str {
    key.strip_suffix(SCROLL_X_SUFFIX).unwrap_or(key)
}

impl InteractionManager {
    /// 这个 scroll-view 是不是双向的（渲染器登记时写进 `scroll_views`）
    pub fn scroll_is_2d(&self, id: &str) -> bool {
        self.scroll_views.get(id).is_some_and(|s| s.both_axes)
    }

    /// 双向 scroll-view 的横向位置（逻辑 px）；不是双向的返回 0
    pub fn scroll_x_of(&self, id: &str) -> f32 {
        if !self.scroll_is_2d(id) {
            return 0.0;
        }
        self.scroll_controllers.get(&scroll_x_key(id)).map(|c| c.get_position()).unwrap_or(0.0)
    }

    /// 横向控制器：没有就建，有就跟上内容宽度（每帧登记时调，查表命中时不分配键以外的东西）
    pub fn ensure_scroll_x(&mut self, id: &str, content_width: f32, viewport_width: f32) {
        let key = scroll_x_key(id);
        match self.scroll_controllers.get_mut(&key) {
            Some(c) => c.update_content_height(content_width, viewport_width),
            None => {
                self.scroll_controllers.insert(key, ScrollController::new_horizontal(content_width, viewport_width));
            }
        }
        match self.scroll_views.get_mut(id) {
            Some(s) => s.both_axes = true,
            None => {
                self.scroll_views.insert(id.to_string(), crate::ui::scroll_view_state::ScrollViewState { both_axes: true, ..Default::default() });
            }
        }
    }

    /// 不再是双向的了（页面 setData 拿掉了 `scroll-x`）：横向控制器拿掉、位置归零
    pub fn drop_scroll_x(&mut self, id: &str) {
        if self.scroll_views.get(id).is_some_and(|s| s.both_axes) {
            self.scroll_controllers.remove(&scroll_x_key(id));
            if let Some(s) = self.scroll_views.get_mut(id) {
                s.both_axes = false;
            }
        }
    }

    /// 结束正在拖的 scroll-view（双向的两个控制器一起），返回它的 id
    pub fn end_area_drag(&mut self) -> Option<String> {
        let id = self.dragging_scroll_area.take()?;
        if let Some(c) = self.scroll_controllers.get_mut(&id) {
            c.end_drag();
        }
        if self.scroll_is_2d(&id) {
            if let Some(c) = self.scroll_controllers.get_mut(&scroll_x_key(&id)) {
                c.end_drag();
            }
        }
        Some(id)
    }
}
