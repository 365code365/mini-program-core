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
