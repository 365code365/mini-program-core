//! Canvas 的 `font` 简写（CSS `font`）与按字族 / 字重画字、量字（小游戏用，game-doc/03 §2.2、05 GT-F2/F4）。
//!
//! 小程序 `<canvas>` 的老路径（text2d.rs）只认字号、用默认字体、基线按 0.8/0.35 em 近似 —— 那条路已经有像素基线，不动。
//! 小游戏走这里：
//! - `font` 按 CSS 简写解析：`[style] [variant] [weight] [stretch] size[/line-height] family[, family…]`，
//!   size 认 px / pt / em / rem / %（em / rem / % 按 HTML 画布的默认 10px 换算）；解析不了返回 None（HTML：保留旧值）；
//! - 字族按 `text_family::renderer_for_family` 解析（`wx.loadFont` 登记的字族也在里面）；字重 ≥ 600 画粗体；
//! - 基线按字体真实的 ascent / descent 对（top / hanging / middle / alphabetic / ideographic / bottom）；
//! - 旋转 / 斜切、`maxWidth`（横向压缩，与浏览器一样字高不变）、`strokeText`（真描轮廓）走离屏光栅再按矩阵贴回（text_raster.rs）。

use super::{Canvas2DContext, TextAlign, TextBaseline};
use crate::text::TextRenderer;
use crate::{Color, Paint, PaintStyle};
use std::sync::Arc;

/// 解析好的 `font`
#[derive(Clone, Debug, PartialEq)]
pub struct FontSpec {
    pub italic: bool,
    /// 100~900
    pub weight: u16,
    /// 逻辑像素
    pub size: f32,
    /// 原样的字族栈（`"Helvetica Neue", Arial, sans-serif`）
    pub family: String,
}

impl Default for FontSpec {
    fn default() -> Self {
        Self { italic: false, weight: 400, size: 10.0, family: "sans-serif".into() }
    }
}

impl FontSpec {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let mut spec = FontSpec { italic: false, weight: 400, size: 0.0, family: String::new() };
        let mut rest = s;
        // 字号之前的那些关键字（顺序不限）
        loop {
            let (word, tail) = match rest.find(char::is_whitespace) {
                Some(i) => (&rest[..i], rest[i..].trim_start()),
                None => return None,
            };
            let w = word.to_ascii_lowercase();
            match w.as_str() {
                "normal" | "small-caps" | "ultra-condensed" | "extra-condensed" | "condensed" | "semi-condensed" | "semi-expanded" | "expanded" | "extra-expanded" | "ultra-expanded" => {}
                "italic" | "oblique" => spec.italic = true,
                "bold" | "bolder" => spec.weight = 700,
                "lighter" => spec.weight = 300,
                _ if w.len() == 3 && w.ends_with("00") && ('1'..='9').contains(&w.chars().next().unwrap()) => spec.weight = w.parse().ok()?,
                _ => {
                    // 字号（可带 /line-height）
                    let size_part = word.split('/').next().unwrap_or(word);
                    spec.size = parse_size(size_part)?;
                    spec.family = tail.trim().to_string();
                    break;
                }
            }
            rest = tail;
        }
        if spec.family.is_empty() || !(spec.size > 0.0 && spec.size.is_finite()) {
            return None;
        }
        Some(spec)
    }

    /// 读回值（与 Chrome 的序列化一致：normal 省略、700 写 bold、其它数字照写，字号 px）
    pub fn to_css(&self) -> String {
        let mut out = String::new();
        if self.italic {
            out.push_str("italic ");
        }
        match self.weight {
            400 => {}
            700 => out.push_str("bold "),
            w => out.push_str(&format!("{w} ")),
        }
        let size = if self.size.fract() == 0.0 { format!("{}", self.size as i64) } else { format!("{}", self.size) };
        out.push_str(&format!("{size}px {}", self.family));
        out
    }

    pub fn bold(&self) -> bool {
        self.weight >= 600
    }

    /// 这个字族栈的渲染器（找不到任何一项时用画布默认字体）
    pub fn renderer(&self) -> Option<Arc<TextRenderer>> {
        crate::text_family::renderer_for_family(Some(&self.family)).or_else(|| super::CANVAS_FONT.clone())
    }
}

fn parse_size(s: &str) -> Option<f32> {
    let s = s.trim().to_ascii_lowercase();
    let num = |t: &str| t.trim().parse::<f32>().ok().filter(|v| v.is_finite() && *v > 0.0);
    if let Some(v) = s.strip_suffix("px") {
        num(v)
    } else if let Some(v) = s.strip_suffix("pt") {
        num(v).map(|v| v * 4.0 / 3.0)
    } else if let Some(v) = s.strip_suffix("rem") {
        num(v).map(|v| v * 10.0)
    } else if let Some(v) = s.strip_suffix("em") {
        num(v).map(|v| v * 10.0)
    } else if let Some(v) = s.strip_suffix('%') {
        num(v).map(|v| v / 10.0)
    } else {
        None
    }
}

/// `measureText` 的结果（HTML TextMetrics 的子集，单位逻辑像素）
#[derive(Clone, Copy, Debug, Default)]
pub struct TextMetrics2D {
    pub width: f32,
    pub left: f32,
    pub right: f32,
    pub ascent: f32,
    pub descent: f32,
    pub font_ascent: f32,
    pub font_descent: f32,
}

impl Canvas2DContext {
    /// 基线相对 y 的下移量（用户坐标）
    fn baseline_shift(&self, tr: &TextRenderer, size: f32) -> f32 {
        let (asc, desc) = tr.line_metrics(size);
        // top / middle 按 em 框（ascent : descent 按比例缩到正好 1em），与 Chrome 一致：按字体的 ascent 对 top，
        // 20px Arial 会比 Chrome 低 2px、middle 低 1px（tools/game-compare.py g-text 格 7 / 8 量出来的）
        let em_asc = if asc + desc > 0.0 { size * asc / (asc + desc) } else { size * 0.8 };
        let em_desc = size - em_asc;
        match self.text_baseline {
            TextBaseline::Top => em_asc,
            TextBaseline::Hanging => asc * 0.8,
            TextBaseline::Middle => (em_asc - em_desc) / 2.0,
            TextBaseline::Alphabetic => 0.0,
            TextBaseline::Ideographic | TextBaseline::Bottom => -desc,
        }
    }

    fn align_shift(&self, width: f32) -> f32 {
        match self.text_align {
            TextAlign::Center => -width / 2.0,
            TextAlign::Right => -width,
            _ => 0.0,
        }
    }

    /// 按 `font` 画字（`stroke` 用描边色，近似描边）。`max_width` 见文件头
    pub fn fill_text_font(&mut self, font: &FontSpec, text: &str, x: f32, y: f32, max_width: Option<f32>, stroke: bool) {
        let Some(tr) = font.renderer() else { return };
        let size = font.size;
        let width = tr.measure_text_weighted(text, size, 0.0, font.bold());
        let mut draw_width = width;
        if let Some(mw) = max_width.filter(|m| m.is_finite()) {
            if mw <= 0.0 {
                return;
            }
            if width > mw && width > 0.0 {
                draw_width = mw;
            }
        }
        let ax = x + self.align_shift(draw_width);
        let ay = y + self.baseline_shift(&tr, size);
        // 旋转 / 斜切、maxWidth 横向压缩、描边：离屏光栅再按矩阵贴回（text_raster.rs）。
        // 合成模式 / 阴影 / 渐变填充仍走下面的蒙版路径（不旋转、不压缩，记在 game-doc/验收/G4.md）
        if (self.has_rotation() || draw_width < width || stroke) && self.fx.trivial(stroke) {
            let base = if stroke { self.stroke_style } else { self.fill_style };
            let lw = stroke.then_some(self.line_width);
            let job = super::text_raster::TextJob { tr: &tr, text, size, bold: font.bold(), x: ax, y: ay, width, draw_width, stroke: lw };
            return self.draw_text_offscreen(job, base);
        }
        let sc = self.avg_scale();
        let (px, py) = self.tp(ax, ay);
        let dsize = size * sc;
        let bold = font.bold();
        let saved = (self.fill_style, self.fx.fill.clone());
        if stroke {
            self.fill_style = self.stroke_style;
            self.fx.fill = self.fx.stroke.clone();
        }
        if !self.fx.trivial(false) {
            let text = text.to_string();
            self.paint_with_fx(false, |mask| {
                let paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill);
                tr.draw_text_weighted(mask, &text, px, py, dsize, 0.0, bold, &paint);
            });
        } else {
            let color = self.apply_alpha(self.fill_style);
            if let Ok(mut canvas) = self.canvas.lock() {
                let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
                tr.draw_text_weighted(&mut canvas, text, px, py, dsize, 0.0, bold, &paint);
            }
        }
        (self.fill_style, self.fx.fill) = saved;
    }

    /// `measureText`：宽度与墨迹 / 字体包围盒（相对当前 textAlign / textBaseline 的锚点，HTML 的定义）
    pub fn measure_text_font(&self, font: &FontSpec, text: &str) -> TextMetrics2D {
        let Some(tr) = font.renderer() else { return TextMetrics2D::default() };
        let size = font.size;
        let width = tr.measure_text_weighted(text, size, 0.0, font.bold());
        let (l, r, a, d) = tr.ink_bounds(text, size, font.bold());
        let (fa, fd) = tr.line_metrics(size);
        let dx = self.align_shift(width);
        let dy = self.baseline_shift(&tr, size);
        TextMetrics2D { width, left: l - dx, right: r + dx, ascent: a - dy, descent: d + dy, font_ascent: fa - dy, font_descent: fd + dy }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GT-F2：font 简写 10 种写法
    #[test]
    fn font_简写解析() {
        let p = |s: &str| FontSpec::parse(s);
        let f = p("16px sans-serif").unwrap();
        assert_eq!((f.size, f.weight, f.italic, f.family.as_str()), (16.0, 400, false, "sans-serif"));
        assert_eq!(p("bold 20px Arial").unwrap().weight, 700);
        assert_eq!(p("italic 600 12px \"Helvetica Neue\", Arial").unwrap().family, "\"Helvetica Neue\", Arial");
        assert!(p("italic 600 12px serif").unwrap().italic);
        assert_eq!(p("12pt serif").unwrap().size, 16.0);
        assert_eq!(p("2em serif").unwrap().size, 20.0);
        assert_eq!(p("normal normal 300 14px/20px PingFang SC").unwrap().size, 14.0);
        assert_eq!(p("normal normal 300 14px/20px PingFang SC").unwrap().weight, 300);
        assert_eq!(p("150% monospace").unwrap().size, 15.0);
        assert!(p("16px").is_none(), "没有字族");
        assert!(p("bold sans-serif").is_none(), "没有字号");
        assert!(p("abc 16px serif").is_none(), "认不出的词在字号之前");
        assert!(p("-4px serif").is_none());
        assert_eq!(p("bold 16px Arial").unwrap().to_css(), "bold 16px Arial");
        assert_eq!(p("italic 300 12.5px serif").unwrap().to_css(), "italic 300 12.5px serif");
    }
}
