//! CSS 颜色取值解析：`#rgb` / `#rgba` / `#rrggbb` / `#rrggbbaa`、
//! `rgb()` / `rgba()`（逗号式与空格 + `/` 的新语法都支持）、命名颜色。
//!
//! 单独成文件的原因：颜色是全项目唯一一处「同一份语法有四个入口共享」的取值——
//! WXSS 声明、内联 `style`、渐变色标、canvas `fillStyle`。之前各写一份，
//! 其中两份**直接把 alpha 丢掉**（`rgba(249,245,238,.55)` 当成不透明色），
//! 于是所有半透明遮罩/淡出/描边都糊成实色：tea-app 首页 banner 上那道横线，
//! 就是「顶部 55% → 底部 0%」的淡出被解析成一整块实色后留下的硬边。
//! 这里是唯一实现，别处只许调用不许再写第二份。

use crate::Color;

/// 解析任意 CSS 颜色字符串。无法识别时返回 `None`（调用方各自兜底）。
///
/// 支持：
/// - 十六进制：`#f0a`、`#f0a8`、`#ff00aa`、`#ff00aa80`
/// - 函数式：`rgb(255,0,0)`、`rgba(255,0,0,.5)`、`rgb(100% 0% 0% / 50%)`
/// - 命名色：`white`、`transparent` 等常用集合
pub fn parse_color_str(s: &str) -> Option<Color> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex_color(hex);
    }
    if let Some(open) = s.find('(') {
        // 只认 rgb/rgba；linear-gradient(...) 之类在这里必须返回 None，
        // 否则会被当成纯色吃掉，渐变就画不出来了。
        let func = s[..open].trim().to_ascii_lowercase();
        if func != "rgb" && func != "rgba" {
            return None;
        }
        let inner = s[open + 1..].trim_end().strip_suffix(')')?;
        return parse_rgb_func(inner);
    }
    parse_named_color(s)
}

/// 十六进制颜色，长度 3/4/6/8（4 位与 8 位带 alpha）。
fn parse_hex_color(hex: &str) -> Option<Color> {
    let h = hex.trim();
    if h.is_empty() || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    // `#abc` 每位要展开成两位（a -> aa），即乘 17。
    let nib = |i: usize| -> Option<u8> { u8::from_str_radix(&h[i..i + 1], 16).ok().map(|v| v * 17) };
    let byte = |i: usize| -> Option<u8> { u8::from_str_radix(&h[i..i + 2], 16).ok() };
    match h.len() {
        3 => Some(Color::new(nib(0)?, nib(1)?, nib(2)?, 255)),
        4 => Some(Color::new(nib(0)?, nib(1)?, nib(2)?, nib(3)?)),
        6 => Some(Color::new(byte(0)?, byte(2)?, byte(4)?, 255)),
        8 => Some(Color::new(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
        _ => None,
    }
}

/// `rgb()/rgba()` 的括号内部分。
fn parse_rgb_func(inner: &str) -> Option<Color> {
    let args = split_args(inner);
    if args.len() < 3 {
        return None;
    }
    let r = channel_u8(args[0])?;
    let g = channel_u8(args[1])?;
    let b = channel_u8(args[2])?;
    let a = match args.get(3) {
        Some(t) => alpha_u8(t)?,
        None => 255,
    };
    Some(Color::new(r, g, b, a))
}

/// 逗号、斜杠、空白都当分隔符：一次覆盖 `1,2,3,.5` 与 `1 2 3 / 50%` 两种写法。
fn split_args(inner: &str) -> Vec<&str> {
    inner
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect()
}

/// 颜色通道：`255`、`254.6`、`100%` 都接受。
fn channel_u8(tok: &str) -> Option<u8> {
    let t = tok.trim();
    let v = match t.strip_suffix('%') {
        Some(p) => p.trim().parse::<f32>().ok()? / 100.0 * 255.0,
        None => t.parse::<f32>().ok()?,
    };
    Some(v.round().clamp(0.0, 255.0) as u8)
}

/// alpha：`0.55`、`.55`、`55%`、`1` 都接受，结果落到 0~255。
fn alpha_u8(tok: &str) -> Option<u8> {
    let t = tok.trim();
    let v = match t.strip_suffix('%') {
        Some(p) => p.trim().parse::<f32>().ok()? / 100.0,
        None => t.parse::<f32>().ok()?,
    };
    Some((v.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// 命名颜色（常用集合 + `transparent`）。
pub fn parse_named_color(name: &str) -> Option<Color> {
    let n = name.trim();
    // 绝大多数取值是 hex/rgba，命名色少见；先按原样比一遍避免无谓的分配。
    if let Some(c) = named_lookup(n) {
        return Some(c);
    }
    let lower = n.to_ascii_lowercase();
    if lower == n {
        return None;
    }
    named_lookup(&lower)
}

fn named_lookup(n: &str) -> Option<Color> {
    let c = match n {
        "transparent" => Color::new(0, 0, 0, 0),
        "black" => Color::BLACK,
        "white" => Color::WHITE,
        "red" => Color::new(255, 0, 0, 255),
        "green" => Color::new(0, 128, 0, 255),
        "blue" => Color::new(0, 0, 255, 255),
        "yellow" => Color::new(255, 255, 0, 255),
        "orange" => Color::new(255, 165, 0, 255),
        "purple" => Color::new(128, 0, 128, 255),
        "pink" => Color::new(255, 192, 203, 255),
        "gray" | "grey" => Color::new(128, 128, 128, 255),
        "lightgray" | "lightgrey" => Color::new(211, 211, 211, 255),
        "darkgray" | "darkgrey" => Color::new(169, 169, 169, 255),
        "cyan" | "aqua" => Color::new(0, 255, 255, 255),
        "magenta" | "fuchsia" => Color::new(255, 0, 255, 255),
        "brown" => Color::new(165, 42, 42, 255),
        "navy" => Color::new(0, 0, 128, 255),
        "teal" => Color::new(0, 128, 128, 255),
        "olive" => Color::new(128, 128, 0, 255),
        "maroon" => Color::new(128, 0, 0, 255),
        "silver" => Color::new(192, 192, 192, 255),
        "lime" => Color::new(0, 255, 0, 255),
        "gold" => Color::new(255, 215, 0, 255),
        "whitesmoke" => Color::new(245, 245, 245, 255),
        _ => return None,
    };
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_forms() {
        assert_eq!(parse_color_str("#f0a"), Some(Color::new(255, 0, 170, 255)));
        assert_eq!(parse_color_str("#ff00aa"), Some(Color::new(255, 0, 170, 255)));
        // 8 位与 4 位十六进制带 alpha
        assert_eq!(parse_color_str("#ff00aa80"), Some(Color::new(255, 0, 170, 128)));
        assert_eq!(parse_color_str("#f0a8"), Some(Color::new(255, 0, 170, 136)));
        // 非法长度/非法字符不能蒙对一个颜色出来
        assert_eq!(parse_color_str("#ff00a"), None);
        assert_eq!(parse_color_str("#gg0011"), None);
    }

    /// alpha 必须真的生效。这是本文件存在的理由：以前 `rgba(...)` 的第四个分量
    /// 被丢掉，所有半透明层都画成实色。
    #[test]
    fn rgba_alpha_is_kept() {
        let c = parse_color_str("rgba(249, 245, 238, 0.55)").unwrap();
        assert_eq!((c.r, c.g, c.b), (249, 245, 238));
        assert_eq!(c.a, 140, "0.55*255 应为 140，实际 {}", c.a);
        assert_eq!(parse_color_str("rgba(0,0,0,0)").unwrap().a, 0);
        assert_eq!(parse_color_str("rgba(0,0,0,.5)").unwrap().a, 128);
        assert_eq!(parse_color_str("rgb(1,2,3)").unwrap().a, 255);
    }

    #[test]
    fn modern_space_and_percent_syntax() {
        assert_eq!(parse_color_str("rgb(100% 0% 0%)"), Some(Color::new(255, 0, 0, 255)));
        assert_eq!(parse_color_str("rgb(255 0 0 / 50%)"), Some(Color::new(255, 0, 0, 128)));
    }

    /// 被空白切碎的残片（`border: 1px solid rgba(0,` 这种）必须解析失败，
    /// 不能吐出一个「看起来能用」的颜色，否则边框颜色会莫名其妙。
    #[test]
    fn rejects_truncated_and_non_color_functions() {
        assert_eq!(parse_color_str("rgba(0,"), None);
        assert_eq!(parse_color_str("linear-gradient(to bottom, #fff, #000)"), None);
        assert_eq!(parse_color_str("inherit"), None);
        assert_eq!(parse_color_str(""), None);
    }

    #[test]
    fn named_colors_and_transparent() {
        assert_eq!(parse_color_str("transparent"), Some(Color::new(0, 0, 0, 0)));
        assert_eq!(parse_color_str("WHITE"), Some(Color::WHITE));
        assert_eq!(parse_color_str(" grey "), Some(Color::new(128, 128, 128, 255)));
    }
}
