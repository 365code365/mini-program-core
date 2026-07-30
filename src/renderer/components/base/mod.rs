//! 组件的公共底座：类型、文本度量、样式落地、盒子绘制。
//!
//! ## 为什么拆成目录
//! 这里曾经是**一个 1925 行的 base.rs**，把六件不相干的事塞在一起：taffy 取值封装、
//! 渲染节点类型、文本度量、事件属性解析、CSS 声明落地、背景与边框绘制。改任何一件都要
//! 在无关代码里翻半天，也超了本项目「单文件 500 行」的规则近四倍。
//!
//! 现在按职责切成同名目录下的几片，**只是搬迁，一行逻辑没改** —— 判据是 65 张画廊图
//! 与 21 页整帧快照逐字节不变（固定动画时钟）。
//!
//! | 片 | 装什么 |
//! |---|---|
//! | [`types`] | `RenderNode` / `NodeStyle` / 文本相关枚举 / `ComponentContext` |
//! | [`text_metrics`] | 字形宽度、自然行高（CJK 1.375 / 西文 1.1777）、共享度量字体 |
//! | [`measure`] | 挂给 taffy 的文本度量回调、min-content 宽度 |
//! | [`wrap`] | 断行单元判定与逐行切分 |
//! | [`css_values`] | CSS 时间/缓动值、`transition` 简写 |
//! | [`event_attr`] | 六种事件绑定前缀、`data-*`、class 与文本提取 |
//! | [`style_apply`] | 取值换算、声明排序（简写 → 细项）、`build_base_style` |
//! | [`box_paint`] | 背景、渐变、圆角、逐边边框、阴影 |

use crate::parser::wxml::WxmlNode;
use crate::parser::wxss::{rpx_to_px, ElementDesc, LengthUnit, StyleSheet, StyleValue};
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use std::collections::HashMap;
use taffy::prelude::*;

pub mod box_paint;
pub mod css_values;
pub mod event_attr;
pub mod measure;
pub mod style_apply;
pub mod text_metrics;
pub mod types;
pub mod wrap;

// 对外仍然是一个平铺的命名空间：`components::base::*` 的既有引用一个都不用改。
pub use box_paint::*;
pub use css_values::*;
pub use event_attr::*;
pub use measure::*;
pub use style_apply::*;
pub use text_metrics::*;
pub use types::*;
pub use wrap::*;
