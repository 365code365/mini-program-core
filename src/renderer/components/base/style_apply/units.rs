//! 单位换算与取值：px/rpx/%/vw/vh → 像素，以及 Dimension/Color 解析
//!
//! `style_apply_new` 的一片。**纯搬迁**：从 792 行按职责切开，一行逻辑没改。
use super::*;

/// 写入一条 `top/right/bottom/left`。
///
/// - 百分比 → `LengthPercentageAuto::Percent`，由布局引擎按包含块解析（CSS 语义）
/// - `auto` → `LengthPercentageAuto::auto()`
/// - 其它长度 → 像素
///
/// 同时记录一份「相对视口」的像素值给 `position:fixed` 的固定层使用。
pub(super) fn set_inset(
    value: &StyleValue,
    ctx: &ComponentContext,
    sf: f32,
    axis: Axis,
    slot: &mut LengthPercentageAuto,
    fixed_slot: &mut Option<f32>,
) {
    let viewport = if axis == Axis::Horizontal { ctx.screen_width } else { ctx.screen_height };
    match value {
        StyleValue::Auto => {
            *slot = LengthPercentageAuto::auto();
            *fixed_slot = None;
        }
        StyleValue::Length(n, LengthUnit::Percent) => {
            *slot = LengthPercentageAuto::percent(*n / 100.0);
            *fixed_slot = Some(*n / 100.0 * viewport * sf);
        }
        _ => {
            let px = match value {
                StyleValue::Number(n) => Some(*n),
                other => to_px(other, ctx.screen_width, viewport),
            };
            if let Some(px) = px {
                *slot = LengthPercentageAuto::length(px * sf);
                *fixed_slot = Some(px * sf);
            }
        }
    }
}

pub fn to_px(v: &StyleValue, screen_width: f32, screen_height: f32) -> Option<f32> {
    match v {
        StyleValue::Length(n, u) => Some(match u {
            LengthUnit::Px => *n,
            LengthUnit::Rpx => rpx_to_px(*n, screen_width),
            LengthUnit::Percent => *n / 100.0 * screen_width,
            LengthUnit::Em | LengthUnit::Rem => *n * 16.0,
            LengthUnit::Vw => *n / 100.0 * screen_width,
            LengthUnit::Vh => *n / 100.0 * screen_height,
        }),
        StyleValue::Number(n) => Some(*n),
        StyleValue::String(s) if s.trim_start().starts_with("calc(") => calc_px(s, screen_width, screen_height),
        _ => None,
    }
}

/// 解析期合不成一个单位的 `calc()`（`calc(100vh - 52px)`、`calc(100vw - 30rpx)`）：落样式时按视口换算成 px。
///
/// 解析器只折叠同单位的加减（calc_env.rs），单位一混就整条留成字符串 —— 从前到这里认不出、整条声明作废，
/// `height: calc(100vh - 104rpx)` 的 scroll-view 被内容撑高、滚不动（火车票城市页、阅读器目录页）。
/// 支持 px / rpx / vw / vh / em / rem 与无单位数的 `+ - * /`（乘除按无单位数，先乘除后加减）。
/// 带 `%` 的（`calc(100% - 20px)`）要按包含块解析，这里没有包含块，仍不支持（返回 None，与从前一样）
pub(super) fn calc_px(s: &str, sw: f32, sh: f32) -> Option<f32> {
    let inner = s.trim().strip_prefix("calc(")?.strip_suffix(')')?;
    // 一个操作数：(值, 是不是长度)
    let operand = |tok: &str| -> Option<(f32, bool)> {
        let tok = tok.trim();
        if let Ok(n) = tok.parse::<f32>() {
            return Some((n, false));
        }
        let units: [(&str, f32); 6] = [("rpx", sw / 750.0), ("px", 1.0), ("vw", sw / 100.0), ("vh", sh / 100.0), ("rem", 16.0), ("em", 16.0)];
        let (num, k) = units.iter().find_map(|(u, k)| tok.strip_suffix(u).map(|n| (n, *k)))?;
        Some((num.trim().parse::<f32>().ok()? * k, true))
    };
    let tokens: Vec<&str> = inner.split_whitespace().collect();
    if tokens.is_empty() || tokens.len() % 2 == 0 {
        return None;
    }
    // 先乘除：把「项」合并成 (值, 是不是长度)，再按加减累加
    let mut terms: Vec<(char, f32, bool)> = Vec::new();
    let (v, l) = operand(tokens[0])?;
    let mut cur = (v, l);
    let mut sign = '+';
    for pair in tokens[1..].chunks(2) {
        let (op, rhs) = (pair[0], operand(pair[1])?);
        match op {
            "*" => cur = (cur.0 * rhs.0, cur.1 || rhs.1),
            "/" if !rhs.1 && rhs.0 != 0.0 => cur = (cur.0 / rhs.0, cur.1),
            "+" | "-" => {
                terms.push((sign, cur.0, cur.1));
                sign = op.chars().next().unwrap();
                cur = rhs;
            }
            _ => return None,
        }
    }
    terms.push((sign, cur.0, cur.1));
    Some(terms.iter().map(|(s, v, _)| if *s == '-' { -v } else { *v }).sum())
}

/// 将 StyleValue 转换为 Dimension
pub fn to_dimension(v: &StyleValue, screen_width: f32, screen_height: f32, sf: f32) -> Option<Dimension> {
    match v {
        StyleValue::Auto => Some(Dimension::auto()),
        StyleValue::Length(n, LengthUnit::Percent) => Some(percent(*n / 100.0)),
        _ => to_px(v, screen_width, screen_height).map(|px| length(px * sf)),
    }
}

/// 解析盒模型简写（margin/padding），返回 [top, right, bottom, left]（像素，未乘 sf）。
///
/// 支持 CSS 1-4 值语法：
/// - 1 值：四边相同
/// - 2 值：上下 / 左右
/// - 3 值：上 / 左右 / 下
/// - 4 值：上 / 右 / 下 / 左
///
/// 之前只处理单值，`padding: 10rpx 20rpx` 这类会被静默丢弃。
pub(super) fn box_sides_px(value: &StyleValue, ctx: &ComponentContext) -> Option<[f32; 4]> {
    match value {
        StyleValue::String(s) => {
            let vals: Vec<f32> = s
                .split_whitespace()
                .filter_map(|p| to_px(&parse_inline_value(p), ctx.screen_width, ctx.screen_height))
                .collect();
            match vals.len() {
                1 => Some([vals[0], vals[0], vals[0], vals[0]]),
                2 => Some([vals[0], vals[1], vals[0], vals[1]]),
                3 => Some([vals[0], vals[1], vals[2], vals[1]]),
                4 => Some([vals[0], vals[1], vals[2], vals[3]]),
                _ => None,
            }
        }
        _ => to_px(value, ctx.screen_width, ctx.screen_height).map(|v| [v, v, v, v]),
    }
}

/// 解析内联样式值
pub(super) fn parse_inline_value(value: &str) -> StyleValue {
    let value = value.trim();
    if value == "auto" { return StyleValue::Auto; }
    if value.ends_with("rpx") {
        if let Ok(n) = value.trim_end_matches("rpx").parse() { return StyleValue::Length(n, LengthUnit::Rpx); }
    }
    if value.ends_with("px") {
        if let Ok(n) = value.trim_end_matches("px").parse() { return StyleValue::Length(n, LengthUnit::Px); }
    }
    if value.ends_with("%") {
        if let Ok(n) = value.trim_end_matches("%").parse() { return StyleValue::Length(n, LengthUnit::Percent); }
    }
    if let Ok(n) = value.parse() { return StyleValue::Number(n); }
    
    // Color
    if value.starts_with('#') || value.starts_with("rgb") {
        if let Some(c) = parse_color_str(value) { return StyleValue::Color(c); }
    }
    
    StyleValue::String(value.to_string())
}

/// 从 StyleValue 解析颜色（Color 直取，String 解析）
pub(super) fn color_value(v: &StyleValue) -> Option<Color> {
    match v {
        StyleValue::Color(c) => Some(*c),
        StyleValue::String(s) => parse_color_str(s),
        _ => None,
    }
}
