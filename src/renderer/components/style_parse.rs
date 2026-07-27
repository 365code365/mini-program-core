//! CSS 取值解析（纯函数，无布局/上下文依赖）。
//!
//! 从 `base.rs` 拆出，集中放置颜色、`box-shadow`、`transform`、`border` 简写、
//! 长度单位等的解析逻辑，便于独立测试、复用与维护。这些函数不触碰 Taffy 布局树，
//! 只负责把字符串解析为结构化的样式值。

use super::base::{NodeStyle, BoxShadow, Transform, LinearGradientBg};
use super::color_parse::parse_color_str;
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

/// 顶层空白切分（保留 `rgba(0, 0, 0, .06)` 这类带内部空格的函数值）。
///
/// `border` / `box-shadow` 这些简写以前直接用 `split_whitespace()`，
/// 于是 `2rpx solid rgba(0, 0, 0, 0.04)` 被切成 `rgba(0,` `0,` `0,` `0.04)` 四段残片，
/// 颜色永远解析失败：边框退回默认灰、阴影退回 50% 纯黑。
pub fn split_top_ws(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for ch in s.chars() {
        match ch {
            '(' => { depth += 1; cur.push(ch); }
            ')' => { depth -= 1; cur.push(ch); }
            c if c.is_whitespace() && depth == 0 => {
                if !cur.is_empty() { out.push(std::mem::take(&mut cur)); }
            }
            _ => cur.push(ch),
        }
    }
    if !cur.is_empty() { out.push(cur); }
    out
}

/// 顶层逗号切分（保留 rgba(...) / rgb(...) 内部逗号）
pub fn split_top_commas(s: &str) -> Vec<String> {
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

/// 解析「属性里的 JSON」。
///
/// 模板引擎把数组/对象插值成**单引号** JSON（`['a','b']`），因为属性值本身用双引号
/// 包裹，直接塞双引号 JSON 会把属性截断。这里先按标准 JSON 试，失败再把单引号
/// 换回双引号重试 —— 与 H5 运行时 `__callPageMethod` 里的还原方式一致。
///
/// 解析不出来时返回 `Null`，调用方自行降级。
pub fn parse_attr_json(s: &str) -> serde_json::Value {
    let s = s.trim();
    if s.is_empty() {
        return serde_json::Value::Null;
    }
    if let Ok(v) = serde_json::from_str(s) {
        return v;
    }
    serde_json::from_str(&s.replace('\'', "\"")).unwrap_or(serde_json::Value::Null)
}

// 颜色解析统一在 `color_parse` 模块（见那里的文件头注释：alpha 曾被丢掉）。

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

    let parts = split_top_ws(s);
    let mut num_idx = 0;

    for part in parts.iter().map(|p| p.as_str()) {
        if part == "inset" {
            shadow.inset = true;
        } else if let Some(color) = parse_color_str(part) {
            shadow.color = color;
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
    for part in split_top_ws(s).iter().map(|p| p.as_str()) {
        if let Some(color) = parse_color_str(part) {
            ns.border_color = Some(color);
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
    for part in split_top_ws(s).iter().map(|p| p.as_str()) {
        if let Some(c) = parse_color_str(part) {
            color = Some(c);
        } else if let Some((num, unit)) = parse_length_simple(part) {
            let px = match unit { "rpx" => num * screen_width / 750.0, _ => num };
            width = Some(px * sf);
        }
    }
    (width, color)
}

#[cfg(test)]
mod gradient_tests {
    use super::*;

    /// `background-image: linear-gradient(to bottom, rgba(…,.55), rgba(…,0))`
    /// 是「顶部半透明 → 底部全透明」的淡出。方向与**每个 stop 的 alpha** 都必须解析对，
    /// 否则淡出会变成一块平铺的半透明色，在它结束的地方留下一条硬边
    /// （tea-app 首页 banner 上那道横线就是这么来的）。
    #[test]
    fn parses_to_bottom_fade_with_alpha_stops() {
        let g = parse_linear_gradient(
            "linear-gradient(to bottom, rgba(249, 245, 238, 0.55), rgba(249, 245, 238, 0))",
        )
        .expect("应能解析 to bottom 的 rgba 渐变");
        assert!(
            (g.angle_deg - 180.0).abs() < 0.01,
            "`to bottom` 应等于 180deg，实际 {}",
            g.angle_deg
        );
        assert_eq!(g.stops.len(), 2, "应有两个色标");
        let a0 = g.stops[0].1.a;
        let a1 = g.stops[1].1.a;
        assert!(a0 > 120 && a0 < 160, "首个色标 alpha 应约 140(0.55)，实际 {}", a0);
        assert_eq!(a1, 0, "末尾色标应完全透明，实际 {}", a1);
    }
}
