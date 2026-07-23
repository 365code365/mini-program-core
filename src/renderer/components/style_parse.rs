//! CSS 取值解析（纯函数，无布局/上下文依赖）。
//!
//! 从 `base.rs` 拆出，集中放置颜色、`box-shadow`、`transform`、`border` 简写、
//! 长度单位等的解析逻辑，便于独立测试、复用与维护。这些函数不触碰 Taffy 布局树，
//! 只负责把字符串解析为结构化的样式值。

use super::base::{NodeStyle, BoxShadow, Transform};
use crate::Color;

/// 解析颜色字符串：支持 `#rgb` / `#rrggbb` / `rgb(...)`。
pub fn parse_color_str(s: &str) -> Option<Color> {
    let s = s.trim();
    if s.starts_with('#') {
        let hex = s.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::new(r, g, b, 255));
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            return Some(Color::new(r, g, b, 255));
        }
    } else if s.starts_with("rgb") {
        let nums: Vec<u8> = s.chars()
            .filter(|c| c.is_ascii_digit() || *c == ',')
            .collect::<String>()
            .split(',')
            .filter_map(|n| n.trim().parse().ok())
            .collect();
        if nums.len() >= 3 {
            return Some(Color::new(nums[0], nums[1], nums[2], 255));
        }
    }
    None
}

/// 简单长度解析：返回 (数值, 单位字符串)。无单位按 px 处理。
pub fn parse_length_simple(s: &str) -> Option<(f32, &str)> {
    let s = s.trim();
    for unit in ["rpx", "px", "em", "rem", "vh", "vw", "%"] {
        if s.ends_with(unit) {
            if let Ok(num) = s.trim_end_matches(unit).parse::<f32>() {
                return Some((num, unit));
            }
        }
    }
    s.parse::<f32>().ok().map(|n| (n, "px"))
}

/// 解析 `box-shadow: offsetX offsetY blur spread color [inset]`。
pub fn parse_box_shadow(s: &str, screen_width: f32) -> Option<BoxShadow> {
    let s = s.trim();
    if s == "none" { return None; }

    let mut shadow = BoxShadow::default();
    shadow.color = Color::new(0, 0, 0, 128);

    let parts: Vec<&str> = s.split_whitespace().collect();
    let mut num_idx = 0;

    for part in parts {
        if part == "inset" {
            shadow.inset = true;
        } else if part.starts_with('#') || part.starts_with("rgb") {
            if let Some(color) = parse_color_str(part) {
                shadow.color = color;
            }
        } else if let Some((num, unit)) = parse_length_simple(part) {
            let px = match unit {
                "rpx" => num * screen_width / 750.0,
                _ => num,
            };
            match num_idx {
                0 => shadow.offset_x = px,
                1 => shadow.offset_y = px,
                2 => shadow.blur = px,
                3 => shadow.spread = px,
                _ => {}
            }
            num_idx += 1;
        }
    }

    Some(shadow)
}

/// 解析 `transform`：translate/scale/rotate/skew 及其 X/Y 变体。
pub fn parse_transform(s: &str) -> Option<Transform> {
    let mut transform = Transform::new();
    let mut remaining = s.trim();

    while !remaining.is_empty() {
        if let Some(paren_start) = remaining.find('(') {
            let func_name = remaining[..paren_start].trim();
            if let Some(paren_end) = remaining.find(')') {
                let args = &remaining[paren_start + 1..paren_end];

                match func_name {
                    "translate" | "translateX" | "translateY" => {
                        let values: Vec<f32> = args.split(',')
                            .filter_map(|v| parse_length_simple(v.trim()).map(|(n, _)| n))
                            .collect();
                        if func_name == "translateX" && !values.is_empty() {
                            transform.translate_x = values[0];
                        } else if func_name == "translateY" && !values.is_empty() {
                            transform.translate_y = values[0];
                        } else if !values.is_empty() {
                            transform.translate_x = values[0];
                            if values.len() > 1 { transform.translate_y = values[1]; }
                        }
                    }
                    "scale" | "scaleX" | "scaleY" => {
                        let values: Vec<f32> = args.split(',')
                            .filter_map(|v| v.trim().parse().ok())
                            .collect();
                        if func_name == "scaleX" && !values.is_empty() {
                            transform.scale_x = values[0];
                        } else if func_name == "scaleY" && !values.is_empty() {
                            transform.scale_y = values[0];
                        } else if !values.is_empty() {
                            transform.scale_x = values[0];
                            transform.scale_y = if values.len() > 1 { values[1] } else { values[0] };
                        }
                    }
                    "rotate" => {
                        let angle = args.trim().trim_end_matches("deg");
                        if let Ok(deg) = angle.parse::<f32>() { transform.rotate = deg; }
                    }
                    "skew" | "skewX" | "skewY" => {
                        let values: Vec<f32> = args.split(',')
                            .filter_map(|v| v.trim().trim_end_matches("deg").parse().ok())
                            .collect();
                        if func_name == "skewX" && !values.is_empty() {
                            transform.skew_x = values[0];
                        } else if func_name == "skewY" && !values.is_empty() {
                            transform.skew_y = values[0];
                        } else if !values.is_empty() {
                            transform.skew_x = values[0];
                            if values.len() > 1 { transform.skew_y = values[1]; }
                        }
                    }
                    _ => {}
                }
                remaining = remaining[paren_end + 1..].trim();
            } else { break; }
        } else { break; }
    }
    Some(transform)
}

/// 解析 `border: <width> <style> <color>` 简写（style 关键字忽略）。
pub fn parse_border_shorthand(s: &str, ns: &mut NodeStyle, screen_width: f32, sf: f32) {
    for part in s.split_whitespace() {
        if part.starts_with('#') || part.starts_with("rgb") {
            if let Some(color) = parse_color_str(part) { ns.border_color = Some(color); }
        } else if let Some((num, unit)) = parse_length_simple(part) {
            let px = match unit { "rpx" => num * screen_width / 750.0, _ => num };
            ns.border_width = px * sf;
        }
    }
}

/// 解析单边 `border-top/right/bottom/left: <width> <style> <color>`，
/// 返回 (宽度像素*sf, 颜色)。style 关键字（solid/dashed 等）忽略。
pub fn parse_border_side(s: &str, screen_width: f32, sf: f32) -> (Option<f32>, Option<Color>) {
    let (mut width, mut color) = (None, None);
    for part in s.split_whitespace() {
        if part.starts_with('#') || part.starts_with("rgb") {
            if let Some(c) = parse_color_str(part) { color = Some(c); }
        } else if let Some((num, unit)) = parse_length_simple(part) {
            let px = match unit { "rpx" => num * screen_width / 750.0, _ => num };
            width = Some(px * sf);
        }
    }
    (width, color)
}
