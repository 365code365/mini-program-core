//! 声明落地：取值换算、声明排序、build_base_style 与逐属性应用
//!
//! 由 `base/mod.rs` 组合。**纯搬迁**：从 1925 行的 base.rs 按职责切开，一行代码没改。
use super::*;

/// 将 StyleValue 转换为像素值
/// 定位偏移所在的轴（决定百分比在「相对视口」场景下参照宽还是高）
/// 定位偏移所在的轴（决定百分比在「相对视口」场景下参照宽还是高）
#[derive(Clone, Copy, PartialEq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

// ── 按职责切开的几片 ──
/// 由 CSS 声明构建基础样式
mod base_style;
/// 声明生效顺序
mod order;
/// 逐条属性落地
mod property;
/// 单位换算与取值
mod units;

pub use base_style::build_base_style;
pub use units::{to_dimension, to_px};
use order::sorted_declarations;
use property::apply_style_property;
use units::{box_sides_px, color_value, parse_inline_value, set_inset};

pub use crate::renderer::components::gradient::draw_linear_gradient;
