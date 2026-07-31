//! 小程序源码 → HTML/CSS 编译（transpile）
//!
//! 由于 WXSS 基本是标准 CSS、WXML 与 HTML 结构一一对应，本模块把小程序页面
//! 编译成浏览器可原生渲染的 HTML + CSS：
//! - WXML(+data) → HTML：`TemplateEngine` 先展开 `wx:for/wx:if/{{}}`，再做标签映射
//! - WXSS → CSS：仅需把 `rpx` 换算为 `px`（1rpx = 0.5px @375 宽），其余原样保留
//!
//! 用途：① 浏览器端原生渲染预览（极快、真实 DOM 可交互）；② 静态导出 HTML 工程；
//! ③ 作为后续「编译到各端原生源码」的中间层。

use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::parser::TemplateEngine;
use serde_json::Value as JsonValue;

// ── 按职责切开的几片 ──
/// 节点输出（紧凑 / 美化）
mod emit;
/// 需要内部结构的组件
mod inner;
/// 标签映射与开标签
mod tags;
/// WXSS → CSS
mod css;

pub use css::{base_css, convert_rpx, make_html_doc, wxss_to_css};
pub use emit::{wxml_to_html, wxml_to_html_pretty};
