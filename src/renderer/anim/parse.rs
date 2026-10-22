//! `@keyframes` 里 transform 值的解析：`translate` / `rotate` / `scale` 与它们的单位（px / rpx / deg …）。
//!
//! 从 `anim.rs` 原样搬出（那边超了 500 行）：几个函数一行没改。
use crate::renderer::components::Transform;

/// 解析 `transform`，长度单位换算为**物理像素**（rpx 按 750 设计宽换算）。
///
/// 独立于 `style_parse::parse_transform` 的原因：后者不做单位换算，
/// `translateY(-16rpx)` 会被当成 -16px，动画位移比浏览器大一倍。
pub fn parse_transform_px(s: &str, screen_width: f32, scale_factor: f32) -> Option<Transform> {
    let mut transform = Transform::new();
    let mut rest = s.trim();
    let mut matched = false;
    while let Some(open) = rest.find('(') {
        let func = rest[..open].trim().trim_start_matches(',').trim();
        let Some(close) = rest[open..].find(')').map(|i| i + open) else { break };
        let args = &rest[open + 1..close];
        let lengths: Vec<f32> = args
            .split(',')
            .filter_map(|v| length_to_px(v.trim(), screen_width, scale_factor))
            .collect();
        let numbers: Vec<f32> = args.split(',').filter_map(|v| v.trim().parse::<f32>().ok()).collect();
        let angles: Vec<f32> = args.split(',').filter_map(|v| parse_angle(v.trim())).collect();
        match func {
            "translate" => {
                if let Some(v) = lengths.first() { transform.translate_x = *v; matched = true; }
                if let Some(v) = lengths.get(1) { transform.translate_y = *v; }
            }
            "translateX" => if let Some(v) = lengths.first() { transform.translate_x = *v; matched = true; },
            "translateY" => if let Some(v) = lengths.first() { transform.translate_y = *v; matched = true; },
            "scale" => {
                if let Some(v) = numbers.first() {
                    transform.scale_x = *v;
                    transform.scale_y = *numbers.get(1).unwrap_or(v);
                    matched = true;
                }
            }
            "scaleX" => if let Some(v) = numbers.first() { transform.scale_x = *v; matched = true; },
            "scaleY" => if let Some(v) = numbers.first() { transform.scale_y = *v; matched = true; },
            "rotate" | "rotateZ" => if let Some(v) = angles.first() { transform.rotate = *v; matched = true; },
            "skew" => {
                if let Some(v) = angles.first() { transform.skew_x = *v; matched = true; }
                if let Some(v) = angles.get(1) { transform.skew_y = *v; }
            }
            "skewX" => if let Some(v) = angles.first() { transform.skew_x = *v; matched = true; },
            "skewY" => if let Some(v) = angles.first() { transform.skew_y = *v; matched = true; },
            "none" => {}
            _ => {}
        }
        rest = &rest[close + 1..];
    }
    if matched { Some(transform) } else { None }
}

fn parse_angle(token: &str) -> Option<f32> {
    let t = token.trim();
    if let Some(v) = t.strip_suffix("deg") {
        return v.trim().parse().ok();
    }
    if let Some(v) = t.strip_suffix("turn") {
        return v.trim().parse::<f32>().ok().map(|n| n * 360.0);
    }
    if let Some(v) = t.strip_suffix("rad") {
        return v.trim().parse::<f32>().ok().map(|n| n.to_degrees());
    }
    t.parse().ok()
}

fn length_to_px(token: &str, screen_width: f32, scale_factor: f32) -> Option<f32> {
    let t = token.trim();
    if t.is_empty() {
        return None;
    }
    let (num, unit) = split_unit(t)?;
    let px = match unit {
        "rpx" => num * screen_width / 750.0,
        "px" | "" => num,
        "vw" => num * screen_width / 100.0,
        "em" | "rem" => num * 16.0,
        _ => num,
    };
    Some(px * scale_factor)
}

fn split_unit(t: &str) -> Option<(f32, &str)> {
    let idx = t
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+'))
        .map(|(i, _)| i)
        .unwrap_or(t.len());
    let num: f32 = t[..idx].parse().ok()?;
    Some((num, t[idx..].trim()))
}
