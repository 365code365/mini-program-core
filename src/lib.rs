//! Mini App Engine - 小程序渲染引擎
//! 支持基本图形绑制、UI组件、事件处理、QuickJS 脚本

mod canvas;
mod color;
mod geometry;
mod paint;
mod path;
pub mod text;
/// `font-family` → 具体字体（按字体栈解析 + 每族一份渲染器）
pub mod text_family;
// 彩色 Emoji 位图字形（Apple sbix）
pub mod emoji;
// 小程序资源根目录（图片等相对路径解析）
pub mod assets;

/// 示例小程序目录的定位（裸名字 / `sample/xxx` / 任意路径都能解析）
pub mod app_dir;

/// `wx.request` 等网络 API 的原生实现（后台线程 + 按帧取回）
pub mod net;

/// 引擎的可写数据目录（storage 落盘、图片磁盘缓存），由宿主指定
pub mod data_dir;

/// `wx.setStorageSync` 的跨启动落盘
pub mod storage_file;

/// 页面 json 的 `usingComponents`：自定义组件三件套加载与样式作用域
pub mod using_components;

pub use canvas::Canvas;
pub use color::Color;
pub use geometry::{Point, Rect, Size};
pub use paint::{Paint, PaintStyle};
pub use path::Path;
pub use text::TextRenderer;

// UI 组件系统
pub mod ui;

// 事件系统
pub mod event;

// JS 引擎绑定
pub mod js;

// 应用运行时
pub mod runtime;

// WXML/WXSS 解析器
pub mod parser;

// UI 渲染器
pub mod renderer;

// 多端编译器框架（HTML / 未来 Android、iOS ...）
pub mod compiler;

// Yoga 布局引擎
pub mod layout;

// FFI 导出
mod ffi;
pub use ffi::*;

// 单元测试
#[cfg(test)]
mod tests;

/// 内存回收：宿主收到系统内存告警（iOS `didReceiveMemoryWarning`、
/// Android `onTrimMemory`、鸿蒙 `onMemoryLevel`）或退到后台时调用。
///
/// 放掉的是「可以重建的东西」：
/// - 解码后的图片与动图帧（下次要用时重新解码）；
/// - 自定义 `font-family` 的字体（下次要用时重新解析）。
///
/// **不动**系统主字体：它是所有文字的兜底，放掉下一帧就要重新解析几百 MB，
/// 反而会在内存最紧张的时候制造一次尖峰。
///
/// 正在使用中的资源不会被打断：调用方手上的 `Arc` 仍然有效，当前帧照常画完。
pub fn trim_memory() {
    renderer::components::clear_image_caches();
    text_family::clear_caches();
}

/// 各缓存的占用现状（一行文本，接内存监控/日志用）
pub fn memory_report() -> String {
    format!(
        "{}；自定义字体 {} 个（{}）",
        renderer::components::image_cache_report(),
        text_family::loaded_font_files().len(),
        text_family::loaded_font_files().join(", ")
    )
}
