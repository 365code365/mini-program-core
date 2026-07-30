//! 桌面窗体的私有模块：**只放 winit/softbuffer 相关的适配层**。
//!
//! 与平台无关的宿主逻辑已经搬进 lib 的 `mini_render::host`（移动端 SDK 要用同一份，
//! 见那里的模块注释）。这里把它们原样重新导出，所以窗体代码的引用路径不变。

pub mod events;
pub mod interaction_handler;
pub mod click_handler;
pub mod event_handler;
pub mod scroll_bench;
pub mod pointer_input;
pub mod touch_dispatch;
pub mod edge_back_host;
pub mod headless_script;
// ── 窗体本体按职责拆成的几片（都是 `impl crate::MiniAppWindow`）──
/// winit 事件 → 窗体调用的适配层（唯一认识 winit 类型的地方）
pub mod winit_app;
/// 出帧闸门与帧内推进
pub mod frame;
/// 整帧渲染与上屏合成
pub mod frame_render;
/// 页面栈、路由与画布/渲染器重建
pub mod page_host;
/// 滚动与下拉刷新的宿主侧
pub mod scroll_host;
/// tabBar 的归属判定与绘制
pub mod tabbar_host;
/// 覆盖层交互入口（Modal / picker / 点击分流）
pub mod overlay_host;
/// 无头运行：整帧快照与拖动基准
pub mod headless_run;

// ── 从 lib 的宿主层重新导出（同一份实现）──
pub use mini_render::host::{
    component_mount, edge_back, gesture, navigation, page_loader, picker_sheet, render, tabbar,
    touch, ui_overlay,
};
pub use mini_render::host::{
    load_all_pages, load_custom_tabbar_with_app_wxss, CustomTabBar, CONTENT_HEIGHT,
    LOGICAL_HEIGHT, LOGICAL_WIDTH,
};
pub use mini_render::host::config::*;
pub use mini_render::host::navigation::*;
pub use mini_render::host::render::*;
pub use mini_render::host::tabbar::*;

pub use event_handler::*;
pub use interaction_handler::*;
