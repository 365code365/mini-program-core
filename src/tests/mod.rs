//! 单元测试模块
//! 覆盖布局、渲染、CSS 解析、组件等功能

pub mod layout_tests;
pub mod flex_shrink_tests;
pub mod custom_component_tests;
pub mod font_family_tests;
pub mod touch_event_tests;
pub mod complex_layout_tests;
pub mod css_tests;
pub mod component_tests;
pub mod renderer_tests;
pub mod inline_style_tests;
pub mod ui_overlay_tests;
pub mod component_render_tests;
pub mod interaction_tests;
pub mod scroll_tests;
pub mod route_tests;
pub mod event_tests;
pub mod gif_tests;
pub mod network_tests;
pub mod inline_text_layout_tests;
pub mod nested_style_tests;
pub mod wx_api_tests;
pub mod component_attr_tests;
pub mod image_event_tests;
pub mod wxss_feature_tests;
pub mod memory_tests;
pub mod input_caret_tests;
/// 移动端 SDK（MiniEngine）的指针链路：手势仲裁 / tabBar / picker / Modal
pub mod engine_input_tests;
/// 移动端 SDK 的出帧：页面画布按内容扩容、条带内滚动只重新合成
pub mod engine_render_tests;
/// scroll-view 离屏缓存里的有状态组件（占位符 / 开关进度）
pub mod scroll_cache_state_tests;
/// JS ↔ Rust 桥的契约：两层的函数名必须对得上
pub mod native_bridge_contract_tests;
