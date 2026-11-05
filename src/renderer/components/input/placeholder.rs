//! 占位符样式：`placeholder-style` 与 `placeholder-class` 的 color / font-size / font-weight。
//!
//! 官方（component/input.html、textarea.html）：
//! - `placeholder-style`：指定 placeholder 的样式（textarea 写明「目前仅支持 color, font-size, font-weight, line-height」）；
//!   Skyline 下 textarea 的 `placeholder-style` 「需传入对象 `{ fontSize: number, fontWeight: string, color: string }`」——
//!   两种写法都认（对象经模板求值后是 JSON 串）。
//! - `placeholder-class`：input 默认 `input-placeholder`，textarea 默认 `textarea-placeholder`。
//!
//! 优先级：`placeholder-style` > `placeholder-class` 里的规则 > 默认（#BFBFBF、与输入框同字号同字重）。
//! line-height 不接：占位符与值共用一个行盒（引擎的 input 高度只按值的字号推）。
use super::*;
use crate::parser::wxss::{LengthUnit, StyleValue};

/// 占位符自己的字号（逻辑 px）：与值的字号不同时才写，绘制端按它画占位符
pub const PLACEHOLDER_FONT_SIZE: &str = "__placeholder_fs";
/// 占位符加粗（`font-weight: bold` / ≥600）
pub const PLACEHOLDER_BOLD: &str = "__placeholder_bold";

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct PlaceholderStyle {
    pub color: Option<Color>,
    /// 逻辑 px
    pub font_size: Option<f32>,
    pub bold: Option<bool>,
}

fn weight_is_bold(v: &str) -> bool {
    let v = v.trim().trim_matches('"');
    v.eq_ignore_ascii_case("bold") || v.eq_ignore_ascii_case("bolder") || v.parse::<u32>().is_ok_and(|n| n >= 600)
}

impl PlaceholderStyle {
    /// 解析 `placeholder-style`：CSS 声明串（`color:red; font-size:28rpx`）或 Skyline 的对象（JSON 串）
    pub fn parse(s: &str, screen_width: f32) -> Self {
        let s = s.trim();
        let mut out = Self::default();
        if s.starts_with('{') {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(s) else { return out };
            out.color = v.get("color").and_then(|c| c.as_str()).and_then(super::parse_color_value);
            out.font_size = v.get("fontSize").and_then(|n| n.as_f64().map(|n| n as f32).or_else(|| {
                n.as_str().and_then(|s| crate::renderer::components::attr_len_px(s, screen_width))
            }));
            out.bold = v.get("fontWeight").map(|w| match w {
                serde_json::Value::Number(n) => n.as_f64().is_some_and(|n| n >= 600.0),
                other => weight_is_bold(other.as_str().unwrap_or("")),
            });
            return out;
        }
        for part in s.split(';') {
            let Some((k, v)) = part.split_once(':') else { continue };
            match k.trim() {
                "color" => out.color = super::parse_color_value(v.trim()),
                "font-size" => out.font_size = crate::renderer::components::attr_len_px(v, screen_width).filter(|n| *n > 0.0),
                "font-weight" => out.bold = Some(weight_is_bold(v)),
                _ => {}
            }
        }
        out
    }

    /// `placeholder-class` 在页面样式表里命中的规则
    pub fn from_class(cls: &str, ctx: &ComponentContext) -> Self {
        let names: Vec<&str> = cls.split_whitespace().collect();
        if names.is_empty() {
            return Self::default();
        }
        let styles = ctx.stylesheet.get_styles(&names, "");
        let mut out = Self::default();
        if let Some(StyleValue::Color(c)) = styles.get("color") {
            out.color = Some(*c);
        }
        match styles.get("font-size") {
            Some(StyleValue::Length(px, LengthUnit::Px)) if *px > 0.0 => out.font_size = Some(*px),
            Some(StyleValue::Length(r, LengthUnit::Rpx)) if *r > 0.0 => out.font_size = Some(r * ctx.screen_width / 750.0),
            Some(StyleValue::Number(px)) if *px > 0.0 => out.font_size = Some(*px),
            Some(StyleValue::String(s)) => out.font_size = crate::renderer::components::attr_len_px(s, ctx.screen_width),
            _ => {}
        }
        match styles.get("font-weight") {
            Some(StyleValue::String(s)) => out.bold = Some(weight_is_bold(s)),
            Some(StyleValue::Number(n)) => out.bold = Some(*n >= 600.0),
            _ => {}
        }
        out
    }

    /// 前者没写的项用后者补
    pub fn or(self, other: Self) -> Self {
        Self {
            color: self.color.or(other.color),
            font_size: self.font_size.or(other.font_size),
            bold: self.bold.or(other.bold),
        }
    }

    /// 节点上的占位符样式：属性 > 类 > 不写
    pub fn of_node(node: &WxmlNode, ctx: &ComponentContext) -> Self {
        let default_class = if node.tag_name == "textarea" { "textarea-placeholder" } else { "input-placeholder" };
        let cls = node.get_attr("placeholder-class").unwrap_or(default_class);
        let attr = node.get_attr("placeholder-style").map(|s| Self::parse(s, ctx.screen_width)).unwrap_or_default();
        attr.or(Self::from_class(cls, ctx))
    }
}

/// 绘制端：画占位符时用的字号（物理 px）与是否加粗。没写的跟值一样
pub(super) fn placeholder_font(node: &RenderNode, sf: f32) -> (f32, bool) {
    let fs = node
        .attrs
        .get(PLACEHOLDER_FONT_SIZE)
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(node.style.font_size);
    let value_bold = matches!(
        node.style.font_weight,
        FontWeight::Bold | FontWeight::W600 | FontWeight::W700 | FontWeight::W800 | FontWeight::W900
    );
    let bold = node.attrs.get(PLACEHOLDER_BOLD).map(|s| s == "1").unwrap_or(value_bold);
    (fs * sf, bold)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 声明串与对象两种写法() {
        let p = PlaceholderStyle::parse("color:#ff0000; font-size: 28rpx; font-weight: bold", 375.0);
        assert_eq!(p.color, Some(Color::from_hex(0xff0000)));
        assert_eq!(p.font_size, Some(14.0));
        assert_eq!(p.bold, Some(true));
        let o = PlaceholderStyle::parse(r##"{"fontSize":20,"fontWeight":"600","color":"#00ff00"}"##, 375.0);
        assert_eq!((o.font_size, o.bold, o.color), (Some(20.0), Some(true), Some(Color::from_hex(0x00ff00))));
        let n = PlaceholderStyle::parse("font-weight: 400", 375.0);
        assert_eq!((n.bold, n.color, n.font_size), (Some(false), None, None));
    }
}
