//! `<picker-view>` 的选中框与蒙层的样式：`indicator-style` / `indicator-class` / `mask-style` / `mask-class`。
//!
//! 官方（component/picker-view.html）：`indicator-style`「设置选择器中间选中框的样式」（Skyline 3.3.4 起支持
//! height、border、background-color）、`indicator-class`（WebView）、`mask-class`「设置蒙层的类名」、
//! `mask-style`（WebView）。认的属性：
//! - 选中框：`height`（px / rpx）、`background(-color)`、`border` / `border-top` / `border-bottom` / `border-width` / `border-color`；
//!   写了任何 border 就按它画上下两条边，替掉默认的 1px #e5e5e5 细线；
//! - 蒙层：`background(-color)` 纯色 → 上下两片都画这个颜色；`background: none / transparent`、
//!   `background-image: none` → 不画蒙层；渐变（`linear-gradient`）按默认蒙层画（近似）。
//!
//! 写了选中框高度时，列里每一项的高度也设成它（WebView 版按选中框的高度当项高，见 picker_view_draw.rs）。
//! 建树时解析一次（类要查样式表），结果写在节点属性上，绘制时只读这几个短串。
use crate::color::Color;
use crate::parser::wxss::{LengthUnit, StyleSheet, StyleValue};
use std::collections::HashMap;

const IND_H: &str = "__pv_ind_h";
const IND_BG: &str = "__pv_ind_bg";
const IND_BORDER: &str = "__pv_ind_border";
const MASK: &str = "__pv_mask";

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum MaskStyle {
    /// 白色渐隐（微信默认）
    #[default]
    Default,
    /// 不画
    None,
    /// 纯色
    Solid(Color),
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PickerDecor {
    /// 选中框高（逻辑 px）
    pub ind_h: Option<f32>,
    pub ind_bg: Option<Color>,
    /// 上下边（宽 逻辑 px，颜色）
    pub ind_border: Option<(f32, Color)>,
    pub mask: MaskStyle,
}

fn color_str(c: Color) -> String {
    format!("#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a)
}

fn parse_hex8(s: &str) -> Option<Color> {
    let h = s.strip_prefix('#').filter(|h| h.len() == 8)?;
    let v = u32::from_str_radix(h, 16).ok()?;
    Some(Color::new((v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8))
}

fn len(v: &str, sw: f32) -> Option<f32> {
    super::attr_len_px(v, sw)
}

/// `1px solid #f00`：宽与色（缺宽按 1px，缺色按 #e5e5e5）
fn border_of(v: &str, sw: f32) -> Option<(f32, Color)> {
    let v = v.trim();
    if v == "none" || v == "0" {
        return Some((0.0, Color::from_hex(0xE5E5E5)));
    }
    let mut w = None;
    let mut c = None;
    for t in v.split_whitespace() {
        if w.is_none() {
            if let Some(n) = len(t, sw) {
                w = Some(n);
                continue;
            }
        }
        if let Some(col) = super::parse_color_str(t) {
            c = Some(col);
        }
    }
    Some((w.unwrap_or(1.0), c.unwrap_or(Color::from_hex(0xE5E5E5))))
}

/// 声明串 `k: v; k: v`
fn decls(s: &str) -> Vec<(String, String)> {
    s.split(';').filter_map(|d| d.split_once(':')).map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string())).collect()
}

/// 样式表里一个类的声明（转回声明串的形状）
fn class_decls(cls: &str, sheet: &StyleSheet) -> Vec<(String, String)> {
    let names: Vec<&str> = cls.split_whitespace().collect();
    if names.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<(String, String)> = sheet
        .get_styles(&names, "")
        .into_iter()
        .filter_map(|(k, v)| {
            let s = match v {
                StyleValue::Length(n, LengthUnit::Px) => format!("{n}px"),
                StyleValue::Length(n, LengthUnit::Rpx) => format!("{n}rpx"),
                StyleValue::Color(c) => color_str(c),
                StyleValue::String(s) => s,
                StyleValue::Number(n) => n.to_string(),
                StyleValue::None => "none".into(),
                _ => return None,
            };
            Some((k, s))
        })
        .collect();
    // 先写的先生效、后写的盖住：这里没有顺序，按键名排一下保证结果稳定
    out.sort();
    out
}

fn apply_indicator(d: &mut PickerDecor, list: &[(String, String)], sw: f32) {
    let mut border: Option<(f32, Color)> = d.ind_border;
    for (k, v) in list {
        match k.as_str() {
            "height" => d.ind_h = len(v, sw).filter(|h| *h > 0.0).or(d.ind_h),
            "background" | "background-color" => {
                d.ind_bg = parse_hex8(v).or_else(|| super::parse_color_str(v)).or(d.ind_bg);
            }
            "border" | "border-top" | "border-bottom" => border = border_of(v, sw).or(border),
            "border-width" => {
                if let Some(w) = len(v, sw) {
                    border = Some((w, border.map(|b| b.1).unwrap_or(Color::from_hex(0xE5E5E5))));
                }
            }
            "border-color" => {
                if let Some(c) = parse_hex8(v).or_else(|| super::parse_color_str(v)) {
                    border = Some((border.map(|b| b.0).unwrap_or(1.0), c));
                }
            }
            _ => {}
        }
    }
    d.ind_border = border;
}

fn apply_mask(d: &mut PickerDecor, list: &[(String, String)]) {
    for (k, v) in list {
        let v = v.trim();
        match k.as_str() {
            "background" | "background-color" | "background-image" => {
                if v == "none" || v == "transparent" {
                    d.mask = MaskStyle::None;
                } else if let Some(c) = parse_hex8(v).or_else(|| super::parse_color_str(v)).filter(|_| k != "background-image") {
                    d.mask = if c.a == 0 { MaskStyle::None } else { MaskStyle::Solid(c) };
                }
            }
            _ => {}
        }
    }
}

impl PickerDecor {
    /// 建树：类 < 行内样式（与 CSS 一致）
    pub fn resolve(attrs: &HashMap<String, String>, sheet: &StyleSheet, screen_width: f32) -> Self {
        let mut d = Self::default();
        let get = |k: &str| attrs.get(k).map(String::as_str).filter(|s| !s.trim().is_empty());
        if let Some(c) = get("indicator-class") {
            apply_indicator(&mut d, &class_decls(c, sheet), screen_width);
        }
        if let Some(s) = get("indicator-style") {
            apply_indicator(&mut d, &decls(s), screen_width);
        }
        if let Some(c) = get("mask-class") {
            apply_mask(&mut d, &class_decls(c, sheet));
        }
        if let Some(s) = get("mask-style") {
            apply_mask(&mut d, &decls(s));
        }
        d
    }

    /// 写进节点属性（绘制端 [`from_attrs`](Self::from_attrs) 读回）
    pub fn write(&self, attrs: &mut HashMap<String, String>) {
        if let Some(h) = self.ind_h {
            attrs.insert(IND_H.into(), h.to_string());
        }
        if let Some(c) = self.ind_bg {
            attrs.insert(IND_BG.into(), color_str(c));
        }
        if let Some((w, c)) = self.ind_border {
            attrs.insert(IND_BORDER.into(), format!("{w} {}", color_str(c)));
        }
        match self.mask {
            MaskStyle::Default => {}
            MaskStyle::None => {
                attrs.insert(MASK.into(), "none".into());
            }
            MaskStyle::Solid(c) => {
                attrs.insert(MASK.into(), color_str(c));
            }
        }
    }

    pub fn from_attrs(attrs: &HashMap<String, String>) -> Self {
        let ind_border = attrs.get(IND_BORDER).and_then(|s| {
            let (w, c) = s.split_once(' ')?;
            Some((w.parse().ok()?, parse_hex8(c)?))
        });
        let mask = match attrs.get(MASK).map(String::as_str) {
            Some("none") => MaskStyle::None,
            Some(c) => parse_hex8(c).map(MaskStyle::Solid).unwrap_or_default(),
            None => MaskStyle::Default,
        };
        Self {
            ind_h: attrs.get(IND_H).and_then(|s| s.parse().ok()),
            ind_bg: attrs.get(IND_BG).and_then(|s| parse_hex8(s)),
            ind_border,
            mask,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 行内样式_rpx_border_mask() {
        let sheet = StyleSheet::default();
        let mut a = HashMap::new();
        a.insert("indicator-style".to_string(), "height: 100rpx; border-top: 2px solid #ff0000; background-color: rgba(0,0,255,0.2)".to_string());
        a.insert("mask-style".to_string(), "background-image: none".to_string());
        let d = PickerDecor::resolve(&a, &sheet, 375.0);
        assert_eq!(d.ind_h, Some(50.0));
        assert_eq!(d.ind_border, Some((2.0, Color::from_hex(0xff0000))));
        assert_eq!(d.ind_bg.map(|c| (c.b, c.a)), Some((255, 51)));
        assert_eq!(d.mask, MaskStyle::None);
        let mut out = HashMap::new();
        d.write(&mut out);
        assert_eq!(PickerDecor::from_attrs(&out), d, "写进属性再读回来不变");
    }
}
