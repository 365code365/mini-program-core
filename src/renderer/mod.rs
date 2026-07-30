//! UI 渲染器 - 将 WXML/WXSS 渲染为 UI

pub mod wxml_renderer;
pub mod components;
/// CSS 动画（@keyframes）运行时
pub mod anim;
/// transform 离屏仿射合成
pub mod compose;
/// 绘制耗时按组件类型归因（`MINI_DRAW_LOG=1`）
pub mod draw_profile;

pub use wxml_renderer::{WxmlRenderer, EventBinding, PickerBinding, FramePlan, ImageEvent, ImageEventDetail, VIEWPORT_CULL_MARGIN_PX};
pub use components::{RenderNode, NodeStyle};
