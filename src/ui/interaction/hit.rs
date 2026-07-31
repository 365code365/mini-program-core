//! 命中测试：覆盖层与正常流分开找、最内层滚动区、组内选中值
//!
//! `InteractionManager` 的一片，由 `interaction/mod.rs` 组合。**纯搬迁**：从 1041 行的
//! interaction.rs 按职责切开，一行逻辑没改。
use super::*;

impl InteractionManager {
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
}
