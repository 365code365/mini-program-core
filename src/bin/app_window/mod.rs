//! 窗口模块 - 将事件处理逻辑拆分成独立模块

pub mod config;
pub mod navigation;
pub mod tabbar;
pub mod events;
pub mod render;
pub mod interaction_handler;
pub mod ui_overlay;
pub mod page_loader;
pub mod click_handler;
pub mod event_handler;
pub mod picker_sheet;
pub mod region_data;
pub mod scroll_bench;
pub mod component_mount;

pub use config::*;
pub use navigation::*;
pub use tabbar::*;
pub use render::*;
pub use interaction_handler::*;
pub use ui_overlay::{ToastState, LoadingState, ModalState, render_ui_overlay};
pub use page_loader::{CustomTabBar, load_all_pages, load_custom_tabbar, load_custom_tabbar_with_app_wxss};
pub use click_handler::*;
pub use event_handler::*;

// 常量
pub const LOGICAL_WIDTH: u32 = 375;
pub const LOGICAL_HEIGHT: u32 = 667;
pub const CONTENT_HEIGHT: u32 = 1500;
