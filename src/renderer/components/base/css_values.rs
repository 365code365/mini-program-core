//! CSS 时间与缓动函数、transition 简写解析
//!
//! 由 `base/mod.rs` 组合。**纯搬迁**：从 1925 行的 base.rs 按职责切开，一行代码没改。
use super::*;

/// 解析 CSS 时间值（`.25s` / `250ms`）为秒
pub fn parse_css_time(s: &str) -> f32 {
    let s = s.trim();
    if let Some(ms) = s.strip_suffix("ms") {
        ms.trim().parse::<f32>().unwrap_or(0.0) / 1000.0
    } else if let Some(sec) = s.strip_suffix('s') {
        sec.trim().parse::<f32>().unwrap_or(0.0)
    } else {
        s.parse::<f32>().unwrap_or(0.0)
    }
}

/// 解析缓动函数名/`cubic-bezier(...)` 为三次贝塞尔控制点
pub fn parse_timing_function(s: &str) -> (f32, f32, f32, f32) {
    let s = s.trim();
    if let Some(args) = s.strip_prefix("cubic-bezier(").and_then(|v| v.strip_suffix(')')) {
        let nums: Vec<f32> = args
            .split(',')
            .filter_map(|v| v.trim().parse::<f32>().ok())
            .collect();
        if nums.len() == 4 {
            return (nums[0], nums[1], nums[2], nums[3]);
        }
    }
    match s {
        "linear" => (0.0, 0.0, 1.0, 1.0),
        "ease-in" => (0.42, 0.0, 1.0, 1.0),
        "ease-out" => (0.0, 0.0, 0.58, 1.0),
        "ease-in-out" => (0.42, 0.0, 0.58, 1.0),
        _ => (0.25, 0.1, 0.25, 1.0), // ease
    }
}

/// 解析 `transition` 简写。属性名被忽略：按压态是整套样式切换，统一插值。
/// 形如 `background .25s ease, transform .15s` 时取第一段的时长/缓动。
pub fn parse_transition_shorthand(s: &str) -> Option<TransitionSpec> {
    let first = s.split(',').next()?.trim();
    if first.is_empty() || first == "none" {
        return None;
    }
    let mut spec = TransitionSpec::default();
    let mut times = Vec::new();
    for tok in first.split_whitespace() {
        if tok.ends_with("ms") || tok.ends_with('s') {
            times.push(parse_css_time(tok));
        } else if tok.starts_with("cubic-bezier(")
            || matches!(tok, "linear" | "ease" | "ease-in" | "ease-out" | "ease-in-out")
        {
            spec.curve = parse_timing_function(tok);
        }
    }
    // CSS 规定：第一个时间是 duration，第二个是 delay
    spec.duration = times.first().copied().unwrap_or(0.0);
    spec.delay = times.get(1).copied().unwrap_or(0.0);
    if spec.duration <= 0.0 {
        return None;
    }
    Some(spec)
}
