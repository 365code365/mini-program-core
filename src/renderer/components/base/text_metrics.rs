//! 文本度量与换行：字形宽度、自然行高、min-content 单元、段落断行
//!
//! 由 `base/mod.rs` 组合。**纯搬迁**：从 1925 行的 base.rs 按职责切开，一行代码没改。
use super::*;
use crate::text::TextRenderer;

/// 与渲染器同款的共享度量字体：build 阶段按真实字形宽度测量文本盒宽度，
/// 避免用粗糙估算导致按钮/标签等叶子组件盒子偏窄或塌缩。
static MEASURE_FONT: once_cell::sync::Lazy<Option<std::sync::Arc<TextRenderer>>> =
    once_cell::sync::Lazy::new(crate::text::shared_fonts);

/// 用真实字体度量文本宽度（物理像素）。取不到字体时按中文全宽/西文 0.6 估算回退。
pub fn intrinsic_text_width(text: &str, font_px: f32, letter_spacing_px: f32) -> f32 {
    intrinsic_text_width_weighted(text, font_px, letter_spacing_px, false)
}

/// 同 [`intrinsic_text_width`]，但按字重度量（粗体字面通常更宽，必须与绘制一致）。
pub fn intrinsic_text_width_weighted(text: &str, font_px: f32, letter_spacing_px: f32, bold: bool) -> f32 {
    if let Some(tr) = MEASURE_FONT.as_ref() {
        tr.measure_text_weighted(text, font_px, letter_spacing_px, bold)
    } else {
        text.chars()
            .map(|c| if c.is_ascii() { font_px * 0.6 } else { font_px } + letter_spacing_px)
            .sum()
    }
}

/// 给定物理字号的 CSS `line-height:normal` 行高（按含 CJK 处理）。
pub fn natural_line_height_px(font_px: f32) -> f32 {
    MEASURE_FONT
        .as_ref()
        .map(|tr| tr.natural_line_height(font_px))
        .unwrap_or(font_px * crate::text::NORMAL_LINE_HEIGHT_FACTOR)
}

/// 按文本内容选择的 `line-height:normal` 行高（含 CJK 与纯西文系数不同，见 text 模块）。
pub fn natural_line_height_px_for(text: &str, font_px: f32) -> f32 {
    MEASURE_FONT
        .as_ref()
        .map(|tr| tr.natural_line_height_for(text, font_px))
        .unwrap_or_else(|| {
            let factor = if crate::text::text_has_cjk(text) {
                crate::text::CJK_LINE_HEIGHT_FACTOR
            } else {
                crate::text::LATIN_LINE_HEIGHT_FACTOR
            };
            font_px * factor
        })
}

// ── taffy 尺寸值的读取 ──
//
// taffy 0.12 起 `Dimension` / `LengthPercentage(Auto)` 不再是枚举，而是包着
// `CompactLength`（把 tag 塞进 f32 低位的位压缩表示）的不透明结构 —— 省内存、
// 但也意味着**不能再 match**。这里把「读值」收成三个判据，免得每个调用点各写一遍
// tag 比较，将来再换表示也只改这里。

/// `Dimension` 是不是 `auto`
#[inline]
pub fn dim_is_auto(d: Dimension) -> bool {
    d.is_auto()
}

/// `Dimension` 是绝对长度时取出像素值
#[inline]
pub fn dim_length(d: Dimension) -> Option<f32> {
    (d.tag() == CompactLength::LENGTH_TAG).then(|| d.value())
}

/// `Dimension` 是百分比时取出比例（0~1，注意不是 0~100）
#[inline]
pub fn dim_percent(d: Dimension) -> Option<f32> {
    (d.tag() == CompactLength::PERCENT_TAG).then(|| d.value())
}

/// `LengthPercentageAuto` 是绝对长度时取出像素值
#[inline]
pub fn lpa_length(v: LengthPercentageAuto) -> Option<f32> {
    let raw = v.into_raw();
    (raw.tag() == CompactLength::LENGTH_TAG).then(|| raw.value())
}

/// 从 taffy 的 LengthPercentage 取出长度像素（百分比/auto 记 0）。
pub fn length_px(v: LengthPercentage) -> f32 {
    if v.into_raw().tag() == CompactLength::LENGTH_TAG {
        v.into_raw().value()
    } else {
        0.0
    }
}
