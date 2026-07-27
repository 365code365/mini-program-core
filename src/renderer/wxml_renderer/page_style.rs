//! `page { … }` 选择器的支持。
//!
//! 小程序里 `page` 是页面的根节点，`page { background-color / color / font-family /
//! font-size }` 是**整页的基线样式**：背景色铺满视口，文字属性向下继承给所有节点。
//!
//! 之前引擎完全忽略这条选择器 —— 页面背景固定用宿主写死的 `#F5F5F5`，
//! 文字继承从内置默认值开始。于是像 tea-app 这种在 `page` 上定义暖米色底
//! （`#f7f3ec`）+ 宋体的应用，整体色调和字形全对不上，视觉差距一眼就能看出来。

use super::*;
use crate::parser::wxss::ElementDesc;

/// 从 `page { … }` 解析出的整页基线
#[derive(Clone, Default)]
pub struct PageStyle {
    /// 整页背景色（宿主用它清屏）
    pub background: Option<Color>,
    /// 继承给所有节点的文字样式
    pub inherited: InheritedText,
    /// `page` 上显式写的高度（`100%` / `100vh` 这类），无则由内容与视口决定
    pub explicit_height: Option<f32>,
}

impl WxmlRenderer {
    /// 解析当前样式表里的 `page` 规则
    pub fn page_style(&self) -> PageStyle {
        let attrs = std::collections::HashMap::new();
        let desc = ElementDesc::new("page", None, &[], &attrs);
        let css = self.stylesheet.get_styles_chain(&[desc]);
        if css.is_empty() {
            return PageStyle::default();
        }

        let sf = self.scale_factor;
        let (sw, sh) = (self.screen_width, self.screen_height);
        let mut out = PageStyle::default();
        // 文字基线先取引擎默认，再让 page 的声明覆盖
        out.inherited = InheritedText::default();
        let px = |v: &crate::parser::wxss::StyleValue| {
            crate::renderer::components::to_px(v, sw, sh)
        };

        for (name, value) in css.iter() {
            match name.as_str() {
                "background-color" | "background" => {
                    if let crate::parser::wxss::StyleValue::Color(c) = value {
                        out.background = Some(*c);
                    }
                }
                "color" => {
                    if let crate::parser::wxss::StyleValue::Color(c) = value {
                        out.inherited.color = Some(*c);
                    }
                }
                // 注意：`InheritedText` 里的文字度量是**逻辑像素**（默认 16.0 就是 16 逻辑 px），
                // 缩放在绘制/度量时才乘。这里曾经写成 `v * sf`，于是在 @2x 的真机上
                // `page{font-size:28rpx}`（= 14 逻辑 px）继承下去变成 28 逻辑 px ——
                // 所有「没写字号、靠继承」的文字整整大一倍（tea-app 登录页的
                // 「微信一键登录」就是这么变成 28px 的，设计稿是 15px）。
                // 单测当年是用 scale_factor=1 建的渲染器，正好把这个 bug 盖住了。
                "font-size" => {
                    if let Some(v) = px(value) {
                        out.inherited.font_size = v;
                    }
                }
                "font-weight" => {
                    let bold = match value {
                        crate::parser::wxss::StyleValue::String(s) => s.trim() == "bold",
                        crate::parser::wxss::StyleValue::Number(n) => *n >= 600.0,
                        _ => false,
                    };
                    if bold {
                        out.inherited.weight = crate::renderer::components::FontWeight::Bold;
                    }
                }
                "text-align" => {
                    if let crate::parser::wxss::StyleValue::String(s) = value {
                        out.inherited.align = match s.trim() {
                            "center" => TextAlign::Center,
                            "right" => TextAlign::Right,
                            _ => TextAlign::Left,
                        };
                    }
                }
                // 同上，行高与字距也是逻辑像素
                "line-height" => {
                    if let Some(v) = px(value) {
                        out.inherited.line_height = Some(v);
                    }
                }
                "letter-spacing" => {
                    if let Some(v) = px(value) {
                        out.inherited.letter_spacing = v;
                    }
                }
                // 整页的字体栈：在 `page` 上写宋体的应用，全页文字都该是宋体
                "font-family" => {
                    if let crate::parser::wxss::StyleValue::String(s) = value {
                        let v = s.trim();
                        if !v.is_empty() {
                            out.inherited.font_family = Some(std::sync::Arc::from(v));
                        }
                    }
                }
                // `height: 100%` / `100vh`：整页高度以视口为准。
                // 百分比要在这里单独处理 —— `to_px` 没有参照物，而 page 的参照物
                // 恰恰就是视口本身。
                "height" => {
                    use crate::parser::wxss::{LengthUnit, StyleValue};
                    let resolved = match value {
                        StyleValue::Length(v, LengthUnit::Percent) => Some(sh * v / 100.0),
                        other => px(other),
                    };
                    if let Some(v) = resolved {
                        out.explicit_height = Some(v * sf);
                    }
                }
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::WxssParser;
    use crate::renderer::WxmlRenderer;

    fn renderer(css: &str) -> WxmlRenderer {
        let ss = WxssParser::new(css).parse().unwrap_or_default();
        WxmlRenderer::new(ss, 375.0, 667.0)
    }

    #[test]
    fn reads_page_background_and_text_baseline() {
        let r = renderer("page { background-color: #f7f3ec; color: #2d251e; font-size: 28rpx; }");
        let ps = r.page_style();
        let bg = ps.background.expect("page 的背景色要能读出来");
        assert_eq!((bg.r, bg.g, bg.b), (0xf7, 0xf3, 0xec));
        let color = ps.inherited.color.expect("page 的文字色要继承下去");
        assert_eq!((color.r, color.g, color.b), (0x2d, 0x25, 0x1e));
        // 28rpx 在 375 宽下 = 14px
        let fs = ps.inherited.font_size;
        assert!((fs - 14.0).abs() < 0.51, "28rpx 应约等于 14px，实际 {}", fs);
    }

    /// 关键回归：继承下去的文字度量是**逻辑像素**，不能跟着 scale_factor 放大。
    /// 这条用例必须用 @2x 的渲染器 —— 原来只测 scale_factor=1，`* sf` 的 bug 被盖住了。
    #[test]
    fn inherited_text_metrics_are_logical_px_at_2x() {
        let ss = WxssParser::new("page { font-size: 28rpx; line-height: 40rpx; letter-spacing: 4rpx; }")
            .parse()
            .unwrap_or_default();
        let r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 2.0);
        let ps = r.page_style();
        assert!((ps.inherited.font_size - 14.0).abs() < 0.51, "28rpx 应是 14 逻辑 px，实际 {}", ps.inherited.font_size);
        assert!((ps.inherited.line_height.unwrap() - 20.0).abs() < 0.51, "40rpx 应是 20 逻辑 px");
        assert!((ps.inherited.letter_spacing - 2.0).abs() < 0.51, "4rpx 应是 2 逻辑 px");
    }

    #[test]
    fn no_page_rule_yields_empty_style() {
        let r = renderer(".foo { color: red; }");
        let ps = r.page_style();
        assert!(ps.background.is_none());
        assert!(ps.explicit_height.is_none());
    }

    #[test]
    fn page_height_percent_maps_to_viewport() {
        let r = renderer("page { height: 100%; }");
        let ps = r.page_style();
        let h = ps.explicit_height.expect("page 的高度要能读出来");
        // 100% 相对视口高 667（scale_factor 默认 1）
        assert!((h - 667.0).abs() < 1.0, "应解析成视口高，实际 {}", h);
    }
}
