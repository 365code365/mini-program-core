//! 交互与滚动：渲染层之外、宿主之内的那部分状态。
//!
//! 这里**没有**「UI 组件类」。小程序的节点由 `renderer/components/*` 按 WXML 建树，
//! 不存在第二套命令式控件树 —— 早期那套（`View`/`Text`/`Button`/`ComponentTree`…）
//! 已经删掉：它和 WXML 那条链路是两份实现，只有一个旧示例在用，留着只会误导。

pub mod interaction;
pub mod scroll_controller;
pub mod scroll_cache;

pub use interaction::{InteractionManager, InteractiveElement, InteractionType, InteractionResult, KeyInput, ComponentState, calculate_cursor_position};
pub use scroll_controller::ScrollController;
pub use scroll_cache::{ScrollViewCache, ScrollCacheManager};
