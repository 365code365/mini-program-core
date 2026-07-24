//! CSS 取值解析（纯函数，无布局/上下文依赖）。
//!
//! 从 `base.rs` 拆出，集中放置颜色、`box-shadow`、`transform`、`border` 简写、
//! 长度单位等的解析逻辑，便于独立测试、复用与维护。这些函数不触碰 Taffy 布局树，
//! 只负责把字符串解析为结构化的样式值。

use super::base::{NodeStyle, BoxShadow, Transform, LinearGradientBg};
use crate::Color;

/// 解析 `linear-gradient(<方向|角度>, c1 [pos], c2 [pos], ...)`。
/// 方向支持 `to top/right/bottom/left` 及对角，或 `Ndeg`；缺省为 `to bottom`(180deg)。
pub fn parse_linear_gradient(s: &str) -> Option<LinearGradientBg> {
    let s = s.trim();
    let inner = s.strip_prefix("linear-gradient(")?.strip_suffix(")")?.trim();
    // 顶层逗号切分（跳过 rgba(...) 内的逗号）
    let parts = split_top_commas(inner);
    if parts.is_empty() { return None; }

    let mut idx = 0;
    let mut angle = 180.0f32; // 默认向下
    let first = parts[0].trim();
    let is_dir = first.starts_with("to ") || first.ends_with("deg")
        || first.ends_with("turn") || first.ends_with("rad");
    if is_dir {
        angle = parse_gradient_angle(first);
        idx = 1;
    }

    let mut stops: Vec<(f32, Color)> = Vec::new();
    let color_tokens: Vec<&str> = parts[idx..].iter().map(|s| s.trim()).collect();
    let n = color_tokens.len();
    for (i, tok) in color_tokens.iter().enumerate() {
        // "color pos%" 或 "color"
        let mut it = tok.rsplitn(2, char::is_whitespace);
        let last = it.next().unwrap_or("");
        let (color_str, pos): (&str, Option<f32>) = if last.ends_with('%') {
            let head = it.next().unwrap_or("").trim();
            let p = last.trim_end_matches('%').parse::<f32>().ok().map(|v| v / 100.0);
            if head.is_empty() { (tok.trim(), None) } else { (head, p) }
        } else {
            (tok.trim(), None)
        };
        if let Some(c) = parse_color_str(color_str) {
            let position = pos.unwrap_or_else(|| if n <= 1 { 0.0 } else { i as f32 / (n - 1) as f32 });
            stops.push((position.clamp(0.0, 1.0), c));
        }
    }
    if stops.is_empty() { return None; }
    stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    Some(LinearGradientBg { angle_deg: angle, stops })
}

fn parse_gradient_angle(dir: &str) -> f32 {
    let d = dir.trim();
    if let Some(rest) = d.strip_suffix("deg") {
        return rest.trim().parse::<f32>().unwrap_or(180.0);
    }
    if let Some(rest) = d.strip_suffix("turn") {
        return rest.trim().parse::<f32>().map(|t| t * 360.0).unwrap_or(180.0);
    }
    if let Some(rest) = d.strip_suffix("rad") {
        return rest.trim().parse::<f32>().map(|r| r.to_degrees()).unwrap_or(180.0);
    }
    match d {
        "to top" => 0.0,
        "to right" => 90.0,
        "to bottom" => 180.0,
        "to left" => 270.0,
        "to top right" | "to right top" => 45.0,
        "to bottom right" | "to right bottom" => 135.0,
        "to bottom left" | "to left bottom" => 225.0,
        "to top left" | "to left top" => 315.0,
        _ => 180.0,
    }
}

/// 顶层逗号切分（保留 rgba(...) / rgb(...) 内部逗号）
fn split_top_commas(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for ch in s.chars() {
        match ch {
            '(' => { depth += 1; cur.push(ch); }
            ')' => { depth -= 1; cur.push(ch); }
            ',' if depth == 0 => { out.push(cur.trim().to_string()); cur.clear(); }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() { out.push(cur.trim().to_string()); }
    out
}

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
