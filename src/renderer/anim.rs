//! CSS 动画运行时：`@keyframes` 时间轴求值 + 缓动函数 + 全局动画时钟。
//!
//! 设计取舍：只在**绘制阶段**求值，不重建布局。
//! 关键帧里最常见的属性（`transform` / `opacity` / 颜色）都只影响绘制，不影响
//! 盒模型；把它们放在绘制期求值意味着动画每帧只重画、不重排，60/144Hz 下开销
//! 与静态页面几乎相同。代价是 `@keyframes` 里改 `width/height/margin` 这类会
//! 触发重排的属性不生效（如需支持要走「按帧失效布局缓存」的另一条路）。

use crate::parser::wxss::{KeyframesRule, LengthUnit, StyleValue};
use crate::renderer::components::{parse_color_str, Transform};
use crate::Color;
use std::sync::OnceLock;
use std::time::Instant;

static CLOCK: OnceLock<Instant> = OnceLock::new();

/// 全局动画时钟（秒）。进程内所有页面共用一个起点，保证同一 `@keyframes`
/// 在不同组件上的相位一致（浏览器行为亦然）。
pub fn now_secs() -> f32 {
    CLOCK.get_or_init(Instant::now).elapsed().as_secs_f32()
}

/// 缓动函数
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Timing {
    Linear,
    CubicBezier(f32, f32, f32, f32),
    /// steps(n, jump-start?)
    Steps(u32, bool),
}

impl Timing {
    pub fn parse(name: &str) -> Option<Timing> {
        let name = name.trim();
        match name {
            "linear" => Some(Timing::Linear),
            "ease" => Some(Timing::CubicBezier(0.25, 0.1, 0.25, 1.0)),
            "ease-in" => Some(Timing::CubicBezier(0.42, 0.0, 1.0, 1.0)),
            "ease-out" => Some(Timing::CubicBezier(0.0, 0.0, 0.58, 1.0)),
            "ease-in-out" => Some(Timing::CubicBezier(0.42, 0.0, 0.58, 1.0)),
            "step-start" => Some(Timing::Steps(1, true)),
            "step-end" => Some(Timing::Steps(1, false)),
            _ => {
                if let Some(args) = name.strip_prefix("cubic-bezier(").and_then(|s| s.strip_suffix(')')) {
                    let nums: Vec<f32> = args.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                    if nums.len() == 4 {
                        return Some(Timing::CubicBezier(nums[0], nums[1], nums[2], nums[3]));
                    }
                }
                if let Some(args) = name.strip_prefix("steps(").and_then(|s| s.strip_suffix(')')) {
                    let mut parts = args.split(',');
                    let n: u32 = parts.next()?.trim().parse().ok()?;
                    let jump_start = parts.next().map(|s| s.trim() == "start" || s.trim() == "jump-start").unwrap_or(false);
                    return Some(Timing::Steps(n.max(1), jump_start));
                }
                None
            }
        }
    }

    /// 输入线性进度 0..1，输出缓动后的进度
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match *self {
            Timing::Linear => t,
            Timing::Steps(n, jump_start) => {
                let n = n as f32;
                let step = (t * n).floor();
                let step = if jump_start { step + 1.0 } else { step };
                (step / n).clamp(0.0, 1.0)
            }
            Timing::CubicBezier(x1, y1, x2, y2) => cubic_bezier(t, x1, y1, x2, y2),
        }
    }
}

/// 三次贝塞尔缓动：给定 x 求 y（牛顿迭代 + 二分兜底，与浏览器实现同思路）
pub fn cubic_bezier(x: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    fn curve(t: f32, a: f32, b: f32) -> f32 {
        let mt = 1.0 - t;
        3.0 * mt * mt * t * a + 3.0 * mt * t * t * b + t * t * t
    }
    fn slope(t: f32, a: f32, b: f32) -> f32 {
        let mt = 1.0 - t;
        3.0 * mt * mt * a + 6.0 * mt * t * (b - a) + 3.0 * t * t * (1.0 - b)
    }
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let mut t = x;
    for _ in 0..8 {
        let err = curve(t, x1, x2) - x;
        if err.abs() < 1e-4 {
            return curve(t, y1, y2);
        }
        let d = slope(t, x1, x2);
        if d.abs() < 1e-6 {
            break;
        }
        t -= err / d;
    }
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    t = x;
    for _ in 0..20 {
        let v = curve(t, x1, x2);
        if (v - x).abs() < 1e-4 {
            break;
        }
        if v > x {
            hi = t;
        } else {
            lo = t;
        }
        t = (lo + hi) / 2.0;
    }
    curve(t, y1, y2)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FillMode {
    None,
    Forwards,
    Backwards,
    Both,
}

/// `animation` 简写（或各分项）解析结果
#[derive(Debug, Clone)]
pub struct AnimationSpec {
    pub name: String,
    /// 秒
    pub duration: f32,
    pub delay: f32,
    pub timing: Timing,
    /// 迭代次数，`f32::INFINITY` 表示 infinite
    pub iterations: f32,
    pub direction: Direction,
    pub fill: FillMode,
}

impl Default for AnimationSpec {
    fn default() -> Self {
        Self {
            name: String::new(),
            duration: 0.0,
            delay: 0.0,
            timing: Timing::CubicBezier(0.25, 0.1, 0.25, 1.0), // ease
            iterations: 1.0,
            direction: Direction::Normal,
            fill: FillMode::None,
        }
    }
}

impl AnimationSpec {
    /// 解析 `animation` 简写，如 `spin 0.9s linear infinite`、`fadeIn .6s ease-out`。
    pub fn parse_shorthand(value: &str) -> Option<AnimationSpec> {
        // 多组动画（逗号分隔）只取第一组：原生端一个节点同时跑多条时间轴的收益很低
        let value = value.split(',').next()?.trim();
        if value.is_empty() || value == "none" {
            return None;
        }
        let mut spec = AnimationSpec::default();
        let mut times: Vec<f32> = Vec::new();
        for token in value.split_whitespace() {
            if let Some(secs) = parse_time(token) {
                times.push(secs);
                continue;
            }
            if token == "infinite" {
                spec.iterations = f32::INFINITY;
                continue;
            }
            if let Ok(n) = token.parse::<f32>() {
                spec.iterations = n;
                continue;
            }
            if let Some(t) = Timing::parse(token) {
                spec.timing = t;
                continue;
            }
            match token {
                "normal" => spec.direction = Direction::Normal,
                "reverse" => spec.direction = Direction::Reverse,
                "alternate" => spec.direction = Direction::Alternate,
                "alternate-reverse" => spec.direction = Direction::AlternateReverse,
                "none" => spec.fill = FillMode::None,
                "forwards" => spec.fill = FillMode::Forwards,
                "backwards" => spec.fill = FillMode::Backwards,
                "both" => spec.fill = FillMode::Both,
                "running" | "paused" => {}
                other => {
                    if spec.name.is_empty() {
                        spec.name = other.to_string();
                    }
                }
            }
        }
        // CSS 简写里第一个时间是 duration，第二个是 delay
        if let Some(d) = times.first() {
            spec.duration = *d;
        }
        if let Some(d) = times.get(1) {
            spec.delay = *d;
        }
        if spec.name.is_empty() {
            return None;
        }
        Some(spec)
    }

    /// 求当前时刻在时间轴上的位置（已应用 delay / 迭代 / direction / 缓动）。
    /// 返回 None 表示此刻不应用动画值（延迟期且无 backwards 填充，或已结束且无 forwards 填充）。
    pub fn progress_at(&self, now: f32) -> Option<f32> {
        if self.duration <= 0.0 {
            return None;
        }
        let elapsed = now - self.delay;
        if elapsed < 0.0 {
            return match self.fill {
                FillMode::Backwards | FillMode::Both => Some(self.timing.apply(self.direction_map(0, 0.0))),
                _ => None,
            };
        }
        let raw_iter = elapsed / self.duration;
        let finished = raw_iter >= self.iterations;
        let (iter_index, mut frac) = if finished {
            // 停在最后一帧
            let last = if self.iterations.is_finite() { self.iterations } else { raw_iter };
            let idx = (last.ceil() as u32).saturating_sub(1);
            (idx, 1.0)
        } else {
            (raw_iter.floor() as u32, raw_iter.fract())
        };
        if finished && !matches!(self.fill, FillMode::Forwards | FillMode::Both) {
            return None;
        }
        frac = frac.clamp(0.0, 1.0);
        Some(self.timing.apply(self.direction_map(iter_index, frac)))
    }

    fn direction_map(&self, iteration: u32, frac: f32) -> f32 {
        let odd = iteration % 2 == 1;
        match self.direction {
            Direction::Normal => frac,
            Direction::Reverse => 1.0 - frac,
            Direction::Alternate => if odd { 1.0 - frac } else { frac },
            Direction::AlternateReverse => if odd { frac } else { 1.0 - frac },
        }
    }
}

/// `0.9s` / `900ms` / `.6s` → 秒
pub fn parse_time(token: &str) -> Option<f32> {
    let t = token.trim();
    if let Some(ms) = t.strip_suffix("ms") {
        return ms.parse::<f32>().ok().map(|v| v / 1000.0);
    }
    if let Some(s) = t.strip_suffix('s') {
        return s.parse::<f32>().ok();
    }
    None
}

/// 关键帧求值结果：只覆盖会被动画改写的绘制属性
#[derive(Debug, Clone, Default)]
pub struct AnimatedValues {
    pub opacity: Option<f32>,
    pub transform: Option<Transform>,
    pub background_color: Option<Color>,
    pub text_color: Option<Color>,
}

impl AnimatedValues {
    pub fn is_empty(&self) -> bool {
        self.opacity.is_none()
            && self.transform.is_none()
            && self.background_color.is_none()
            && self.text_color.is_none()
    }
}

/// 单个关键帧上被支持的属性（已按屏宽换算成物理像素）
#[derive(Debug, Clone, Default)]
pub struct FrameValues {
    pub offset: f32,
    pub opacity: Option<f32>,
    pub transform: Option<Transform>,
    pub background_color: Option<Color>,
    pub text_color: Option<Color>,
}

/// 已解析好的动画时间轴（按名字缓存，避免每帧重新解析字符串）
#[derive(Debug, Clone)]
pub struct Timeline {
    frames: Vec<FrameValues>,
}

impl Timeline {
    /// 从解析出的 `@keyframes` 规则构建时间轴。
    /// `screen_width` / `scale_factor` 用于把 rpx / px 换算成物理像素。
    pub fn from_rule(rule: &KeyframesRule, screen_width: f32, scale_factor: f32) -> Timeline {
        let mut frames: Vec<FrameValues> = rule
            .steps
            .iter()
            .map(|step| {
                let mut f = FrameValues { offset: step.offset, ..Default::default() };
                for (name, value) in &step.properties {
                    match name.as_str() {
                        "opacity" => f.opacity = value_to_f32(value),
                        "transform" => {
                            if let StyleValue::String(s) = value {
                                f.transform = parse_transform_px(s, screen_width, scale_factor);
                            }
                        }
                        "background" | "background-color" => {
                            f.background_color = value_to_color(value);
                        }
                        "color" => f.text_color = value_to_color(value),
                        _ => {}
                    }
                }
                f
            })
            .collect();
        frames.sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap_or(std::cmp::Ordering::Equal));
        Timeline { frames }
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// 在时间轴位置 `t`（0..1，已缓动）上求值。
    pub fn sample(&self, t: f32) -> AnimatedValues {
        self.sample_with_base(t, &FrameValues::default())
    }

    /// 按元素自身的计算样式作为「隐式关键帧」求值。
    ///
    /// CSS 规定：`@keyframes` 里缺失的 `0%` / `100%` 关键帧，用元素自身的计算值补上。
    /// 少了这条，`@keyframes spin { to { transform: rotate(360deg) } }` 这种只写 `to`
    /// 的写法会在整个周期里恒等于 360°（看着就是永远不转）—— 而它恰好是最常见的写法。
    pub fn sample_with_base(&self, t: f32, base: &FrameValues) -> AnimatedValues {
        let mut out = AnimatedValues::default();
        if self.frames.is_empty() {
            return out;
        }
        out.opacity = self.interpolate(t, |f| f.opacity, lerp_f32, base.opacity);
        out.transform = self.interpolate(t, |f| f.transform, lerp_transform, base.transform);
        out.background_color =
            self.interpolate(t, |f| f.background_color, lerp_color, base.background_color);
        out.text_color = self.interpolate(t, |f| f.text_color, lerp_color, base.text_color);
        out
    }

    /// 对单个属性求插值：只在**声明了该属性**的关键帧之间插值
    /// （CSS 语义：某属性缺失的关键帧不参与该属性的时间轴）。
    /// `base` 是元素自身的计算值，用来补出缺失的首/末关键帧。
    fn interpolate<T: Copy>(
        &self,
        t: f32,
        pick: impl Fn(&FrameValues) -> Option<T>,
        lerp: impl Fn(T, T, f32) -> T,
        base: Option<T>,
    ) -> Option<T> {
        let mut prev: Option<(f32, T)> = None;
        let mut next: Option<(f32, T)> = None;
        for frame in &self.frames {
            if let Some(v) = pick(frame) {
                if frame.offset <= t {
                    prev = Some((frame.offset, v));
                } else if next.is_none() {
                    next = Some((frame.offset, v));
                }
            }
        }
        match (prev, next) {
            (Some((p_off, p_val)), Some((n_off, n_val))) => {
                let span = n_off - p_off;
                let local = if span > 1e-6 { (t - p_off) / span } else { 0.0 };
                Some(lerp(p_val, n_val, local.clamp(0.0, 1.0)))
            }
            // 没有后续关键帧：末尾缺 100%，从最后一帧插值回元素自身值
            (Some((p_off, p_val)), None) => match base {
                Some(b) if p_off < 1.0 - 1e-6 => {
                    let local = ((t - p_off) / (1.0 - p_off)).clamp(0.0, 1.0);
                    Some(lerp(p_val, b, local))
                }
                _ => Some(p_val),
            },
            // 没有前置关键帧：开头缺 0%，从元素自身值插值到第一帧
            (None, Some((n_off, n_val))) => match base {
                Some(b) if n_off > 1e-6 => {
                    let local = (t / n_off).clamp(0.0, 1.0);
                    Some(lerp(b, n_val, local))
                }
                _ => Some(n_val),
            },
            (None, None) => None,
        }
    }
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round().clamp(0.0, 255.0) as u8
}

pub fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        lerp_u8(a.r, b.r, t),
        lerp_u8(a.g, b.g, t),
        lerp_u8(a.b, b.b, t),
        lerp_u8(a.a, b.a, t),
    )
}

pub fn lerp_transform(a: Transform, b: Transform, t: f32) -> Transform {
    Transform {
        translate_x: lerp_f32(a.translate_x, b.translate_x, t),
        translate_y: lerp_f32(a.translate_y, b.translate_y, t),
        scale_x: lerp_f32(a.scale_x, b.scale_x, t),
        scale_y: lerp_f32(a.scale_y, b.scale_y, t),
        rotate: lerp_f32(a.rotate, b.rotate, t),
        skew_x: lerp_f32(a.skew_x, b.skew_x, t),
        skew_y: lerp_f32(a.skew_y, b.skew_y, t),
    }
}

fn value_to_f32(value: &StyleValue) -> Option<f32> {
    match value {
        StyleValue::Number(n) => Some(*n),
        StyleValue::Length(v, LengthUnit::Percent) => Some(v / 100.0),
        StyleValue::Length(v, _) => Some(*v),
        StyleValue::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn value_to_color(value: &StyleValue) -> Option<Color> {
    match value {
        StyleValue::Color(c) => Some(*c),
        StyleValue::String(s) => parse_color_str(s),
        _ => None,
    }
}

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
