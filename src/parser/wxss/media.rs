//! `@media` 条件求值（CSS Media Queries 4 的常用子集）。
//!
//! 与 `env()`、rpx 一样在**解析期**求值：条件成立的块里的规则按书写位置并进样式表，不成立的整块丢掉。
//! 环境量取本线程的 [`super::env_context`]（屏宽、屏高、深色模式），由宿主在解析前放进去；
//! 尺寸 / 主题变了要重新解析样式表（SDK 的 `set_safe_area` / `resize` 本来就会重解析）。
//!
//! 认得的：
//! - 媒体类型 `all` / `screen`（成立）、`print`（不成立），前缀 `only` 忽略、`not` 取反整条；
//! - 特性 `width` / `min-width` / `max-width`、`height` / `min-height` / `max-height`（px / rpx / 无单位按 px），
//!   `orientation: portrait | landscape`、`prefers-color-scheme: light | dark`、
//!   `aspect-ratio` / `min-aspect-ratio` / `max-aspect-ratio`（`w/h`）；
//! - `and` 连接、逗号分隔的多条（任一成立即成立）。
//!
//! 认不出的特性按**不成立**（浏览器同样把未知特性当成 not all）。

use super::env_context;

/// 整条媒体查询列表是否成立
pub fn media_matches(query: &str) -> bool {
    query.split(',').any(|q| single(q.trim()))
}

fn single(q: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    let lower = q.to_ascii_lowercase();
    let (negate, rest) = match lower.strip_prefix("not ") {
        Some(r) => (true, r.trim().to_string()),
        None => (false, lower.strip_prefix("only ").unwrap_or(&lower).trim().to_string()),
    };
    let ok = rest.split(" and ").map(str::trim).filter(|p| !p.is_empty()).all(part);
    ok != negate
}

fn part(p: &str) -> bool {
    match p {
        "all" | "screen" => return true,
        "print" | "speech" => return false,
        _ => {}
    }
    let Some(inner) = p.strip_prefix('(').and_then(|s| s.strip_suffix(')')) else { return false };
    let (name, value) = match inner.split_once(':') {
        Some((n, v)) => (n.trim(), Some(v.trim())),
        None => (inner.trim(), None),
    };
    let env = env_context();
    let (w, h) = (env.screen_width, env.screen_height);
    let len = |v: &str| -> Option<f32> {
        if let Some(n) = v.strip_suffix("rpx") {
            return n.trim().parse::<f32>().ok().map(|n| n * w / 750.0);
        }
        v.strip_suffix("px").unwrap_or(v).trim().parse::<f32>().ok()
    };
    let ratio = |v: &str| -> Option<f32> {
        let (a, b) = v.split_once('/')?;
        let (a, b) = (a.trim().parse::<f32>().ok()?, b.trim().parse::<f32>().ok()?);
        (b > 0.0).then_some(a / b)
    };
    let Some(v) = value else {
        // `(orientation)` 这种布尔写法：特性有非零值就成立
        return matches!(name, "width" | "height" | "orientation" | "prefers-color-scheme" | "color");
    };
    match name {
        "width" => len(v).map(|x| (w - x).abs() < 0.5).unwrap_or(false),
        "min-width" => len(v).map(|x| w >= x).unwrap_or(false),
        "max-width" => len(v).map(|x| w <= x).unwrap_or(false),
        "height" => len(v).map(|x| (h - x).abs() < 0.5).unwrap_or(false),
        "min-height" => len(v).map(|x| h >= x).unwrap_or(false),
        "max-height" => len(v).map(|x| h <= x).unwrap_or(false),
        "orientation" => match v {
            "portrait" => h >= w,
            "landscape" => w > h,
            _ => false,
        },
        "prefers-color-scheme" => match v {
            "dark" => env.dark,
            "light" => !env.dark,
            _ => false,
        },
        "aspect-ratio" => ratio(v).map(|r| (w / h.max(1.0) - r).abs() < 0.01).unwrap_or(false),
        "min-aspect-ratio" => ratio(v).map(|r| w / h.max(1.0) >= r).unwrap_or(false),
        "max-aspect-ratio" => ratio(v).map(|r| w / h.max(1.0) <= r).unwrap_or(false),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::{set_env_context, EnvContext};
    use super::*;

    fn with_env(w: f32, h: f32, dark: bool, f: impl FnOnce()) {
        set_env_context(EnvContext { screen_width: w, screen_height: h, dark, ..Default::default() });
        f();
        set_env_context(EnvContext::default());
    }

    #[test]
    fn width_orientation_and_theme() {
        with_env(375.0, 667.0, false, || {
            assert!(media_matches("screen and (max-width: 400px)"));
            assert!(!media_matches("(min-width: 768px)"));
            assert!(media_matches("(min-width: 768px), (orientation: portrait)"));
            assert!(!media_matches("(orientation: landscape)"));
            assert!(media_matches("(prefers-color-scheme: light)"));
            assert!(!media_matches("(prefers-color-scheme: dark)"));
            assert!(media_matches("not print"));
            assert!(!media_matches("print"));
            assert!(media_matches("only screen and (max-width: 750rpx)"));
            assert!(!media_matches("(hover: hover)"), "认不出的特性不成立");
        });
        with_env(800.0, 600.0, true, || {
            assert!(media_matches("(min-width: 768px) and (orientation: landscape)"));
            assert!(media_matches("(prefers-color-scheme: dark)"));
            assert!(media_matches("(min-aspect-ratio: 4/3)"));
        });
    }
}
