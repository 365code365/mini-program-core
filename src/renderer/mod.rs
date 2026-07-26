//! UI 渲染器 - 将 WXML/WXSS 渲染为 UI

pub mod wxml_renderer;
mod style_resolver;
pub mod components;
pub mod vdom_diff;
/// CSS 动画（@keyframes）运行时
pub mod anim;
/// transform 离屏仿射合成
pub mod compose;

pub use wxml_renderer::{WxmlRenderer, EventBinding, PickerBinding, VIEWPORT_CULL_MARGIN_PX};
pub use vdom_diff::{Patch, diff_forest, is_structural};
pub use style_resolver::StyleResolver;
pub use components::{RenderNode, NodeStyle, ComponentRegistry};
