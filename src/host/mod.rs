//! 宿主层：把「一个小程序包」跑起来所需要的、**与平台无关**的那部分。
//!
//! 这些代码原来住在 `src/bin/app_window/` 里（桌面窗体的私有模块），移进 lib 的
//! 原因很直接：**移动端 SDK 需要同一套东西**。放在 bin 里的代码没法被 `.so`/`.a`
//! 里的 FFI 调用，只能另写一份 —— 那就会变成两套渲染宿主，行为迟早分叉
//! （这个仓库已经踩过一次：无交互的 `render()` 不做 swiper 分页，见
//! `doc/引擎测试说明.md` 的 G-13）。
//!
//! 划分标准是**要不要 winit**：
//! - 这里（lib）：页面加载与页面栈、自定义 tabBar、覆盖层（Toast/Loading/Modal）、
//!   picker 面板、触摸状态机与手势仲裁、侧滑返回、像素合成、组件挂载。
//! - 留在 bin：winit 事件 → 引擎调用的适配层（鼠标/键盘/输入法/滚轮）、softbuffer 上屏。
//!
//! 桌面窗体继续 `use mini_render::host::*`，所以只有一份实现。

pub mod component_mount;
/// 平台无关的引擎实例（移动端 SDK 的核心）
pub mod engine;
mod engine_input;
pub mod config;
pub mod edge_back;
pub mod gesture;
pub mod navigation;
pub mod page_loader;
pub mod picker_sheet;
pub mod region_data;
pub mod render;
pub mod tabbar;
/// 点击命中与事件派发（tap / change 的链路）
pub mod tap;
pub mod touch;
pub mod ui_overlay;

pub use config::*;
pub use navigation::*;
pub use page_loader::{load_all_pages, load_custom_tabbar, load_custom_tabbar_with_app_wxss, CustomTabBar};
pub use render::*;
pub use tabbar::*;
pub use engine::MiniEngine;
pub use ui_overlay::{render_ui_overlay, LoadingState, ModalState, ToastState};

/// 逻辑视口宽（pt/dp）。小程序的设计基准是 750rpx 对应这个宽度。
pub const LOGICAL_WIDTH: u32 = 375;
/// 逻辑视口高（pt/dp）
pub const LOGICAL_HEIGHT: u32 = 667;
/// 页面画布的初始高度（内容更高时按内容扩）
pub const CONTENT_HEIGHT: u32 = 1500;
