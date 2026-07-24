//! Mini App Engine - 小程序渲染引擎
//! 支持基本图形绑制、UI组件、事件处理、QuickJS 脚本

mod canvas;
mod color;
mod geometry;
mod paint;
mod path;
pub mod text;

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
