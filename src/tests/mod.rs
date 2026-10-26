//! 单元测试模块
//! 覆盖布局、渲染、CSS 解析、组件等功能

pub mod layout_tests;
pub mod flex_shrink_tests;
pub mod custom_component_tests;
pub mod component_instance_tests;
pub mod component_runtime_tests;
pub mod font_family_tests;
pub mod touch_event_tests;
pub mod complex_layout_tests;
pub mod css_tests;
pub mod component_tests;
pub mod style_reuse_tests;
pub mod renderer_tests;
pub mod inline_style_tests;
pub mod ui_overlay_tests;
pub mod component_render_tests;
pub mod interaction_tests;
pub mod slider_interaction_tests;
pub mod scroll_tests;
pub mod route_tests;
pub mod event_tests;
pub mod gif_tests;
pub mod network_tests;
pub mod inline_text_layout_tests;
pub mod nested_style_tests;
pub mod wx_api_tests;
pub mod wx_semantics_tests;
pub mod component_attr_tests;
/// `<button>` 点击态：按下去保持默认外观（居中/圆角/类型色），只按 button-hover 变淡
pub mod button_press_tests;
pub mod image_event_tests;
pub mod wxss_feature_tests;
pub mod wxss_media_tests;
pub mod memory_tests;
pub mod input_caret_tests;
/// 输入框编辑：textarea 换行、快捷键（按行 / 按词移动、撤销）、控制字符过滤
pub mod text_edit_tests;
/// textarea 多行排版：折行规则、换行不画豆腐块、光标跟行、光标行滚入可视区
pub mod textarea_render_tests;
/// 移动端 SDK（MiniEngine）的指针链路：手势仲裁 / tabBar / picker / Modal
pub mod engine_input_tests;
pub mod engine_slider_tests;
/// 移动端 SDK 的出帧：页面画布按内容扩容、条带内滚动只重新合成
pub mod engine_render_tests;
/// 移动端 SDK 的提示框与弹窗：Loading 的圈真的转、mask 才挡触摸、Toast/Loading 共用槽位
pub mod engine_feedback_tests;
/// scroll-view 离屏缓存里的有状态组件（占位符 / 开关进度）
pub mod scroll_cache_state_tests;
/// JS ↔ Rust 桥的契约：两层的函数名必须对得上
pub mod native_bridge_contract_tests;
/// `<template name/is/data>` 的模板阶段语义（定义不渲染、data 即全部作用域、递归上限）
pub mod wxml_template_tests;
/// `<import>` / `<include>` 的文件级语义（不传递、不嵌模板、路径、缺文件、成环）
pub mod wxml_import_include_tests;
/// template/include 写法与手抄写法逐像素等价；H5 导出带上模板表与 include 内容
pub mod wxml_template_render_tests;
/// wxs 模块：`{{m.f(x)}}` 求值、模块可见性、`<wxs src>` / `require` 打包、退化与像素等价
pub mod wxs_module_tests;
/// `wx:key`：组件实例键与元素的交互状态键跟着 key 走
pub mod wx_key_tests;
/// swiper 状态机（自动播放、衔接、拖动、受控 current、事件）与三条绘制路径的分页
pub mod swiper_tests;
/// `<switch type="checkbox">`：选中态用 color 填底 + 白勾，看得见
pub mod switch_checkbox_tests;
/// Skyline 新组件：grid-view（网格 / 瀑布流）、cover-image、未接管原生组件的占位与默认尺寸
pub mod skyline_component_tests;
/// 小游戏 G0：前置代码按应用种类分组、包种类入口、call_function 转义
pub mod game_profile_tests;
