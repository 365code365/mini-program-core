//! 点击动画与过渡计时
//!
//! `InteractionManager` 的一片，由 `interaction/mod.rs` 组合。**纯搬迁**：从 1041 行的
//! interaction.rs 按职责切开，一行逻辑没改。
use super::*;

impl InteractionManager {
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
