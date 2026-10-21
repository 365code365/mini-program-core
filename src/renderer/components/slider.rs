//! slider 组件 - 滑动选择器
//!
//! 属性（微信 component/slider.html）：
//! - min / max / step / value：默认 0 / 100 / 1 / 0
//! - activeColor（`selected-color` 是它的旧名）：已选择部分颜色，默认 #1aad19
//! - backgroundColor（`color` 是它的旧名）：背景条颜色，默认 #e9e9e9
//! - block-size：滑块大小，取值 12~28，默认 28
//! - block-color：滑块颜色，默认 #ffffff
//! - show-value：是否显示当前值
//! - disabled：禁用（已选部分变灰 #ccc，与微信一致）
//!
//! 颜色**只认属性**，不认 CSS：微信里 slider 的轨道色与 CSS `color` / `background-color`
//! 无关。从前是「CSS 有就用 CSS」—— 而 `text_color` 是从祖先继承下来的，于是页面上
//! 随便哪层写了 `color`，`active-color` 就被顶掉、轨道变成文字色。
//!
//! 绘制几何（轨道两端各让出半个滑块、show-value 预留数值宽）与命中换算共用
//! [`slider_track_span`]：从前两边各算各的，指针按整个盒子换算、绘制却缩进了一截，
//! 于是滑块跟不上手指，show-value 时永远拖不到头。

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path};
use std::collections::HashMap;
use taffy::prelude::*;

/// show-value 时右侧给数值预留的宽度（逻辑像素）
pub const SLIDER_VALUE_WIDTH: f32 = 50.0;

/// slider 的数值属性。绘制、命中换算、表单取值共用这一份解析。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderAttrs {
    pub min: f64,
    pub max: f64,
    pub step: f64,
    /// WXML 里写的 `value`（已夹到 [min, max] 并按 step 取整）
    pub value: f64,
    /// 滑块直径（逻辑像素，已夹到 12~28）
    pub block_size: f32,
    pub show_value: bool,
}

impl SliderAttrs {
    pub fn from_attrs(attrs: &HashMap<String, String>) -> Self {
        let num = |k: &str| attrs.get(k).and_then(|s| s.trim().parse::<f64>().ok()).filter(|v| v.is_finite());
        let min = num("min").unwrap_or(0.0);
        let max = num("max").unwrap_or(100.0);
        let step = num("step").filter(|s| *s > 0.0).unwrap_or(1.0);
        let block_size = num("block-size").map(|v| v as f32).unwrap_or(28.0).clamp(12.0, 28.0);
        let show_value = attrs.get("show-value").map(|s| attr_truthy(s)).unwrap_or(false);
        let mut a = Self { min, max, step, value: min, block_size, show_value };
        a.value = a.snap(num("value").unwrap_or(0.0));
        a
    }

    /// 夹到 [min, max] 并吸附到 step 的整数倍（与微信 `_filterValue` 同一算式）
    pub fn snap(&self, raw: f64) -> f64 {
        if self.max <= self.min {
            return self.min;
        }
        if raw <= self.min {
            return self.min;
        }
        if raw >= self.max {
            // max - min 不是 step 的整数倍时最后一格落不到 max 上：取能到的最大格
            let n = ((self.max - self.min) / self.step + 1e-9).floor();
            return self.tidy(self.min + n * self.step);
        }
        let n = ((raw - self.min) / self.step).round();
        self.tidy((self.min + n * self.step).min(self.max))
    }

    /// 去掉浮点噪声（0.1 步长累出来的 0.30000000000000004），位数按 step / min 的小数位
    fn tidy(&self, v: f64) -> f64 {
        let digits = decimals(self.step).max(decimals(self.min)).min(6);
        let p = 10f64.powi(digits as i32);
        (v * p).round() / p
    }

    /// 值 → 进度 [0, 1]
    pub fn progress(&self, v: f64) -> f32 {
        if self.max <= self.min {
            return 0.0;
        }
        (((v - self.min) / (self.max - self.min)) as f32).clamp(0.0, 1.0)
    }

    /// 轨道上的横坐标 → 吸附后的值（`track_x` / `track_w` 与 `x` 同一坐标系）
    pub fn value_at(&self, x: f32, track_x: f32, track_w: f32) -> f64 {
        if track_w <= 0.0 {
            return self.min;
        }
        let p = ((x - track_x) / track_w).clamp(0.0, 1.0) as f64;
        self.snap(self.min + p * (self.max - self.min))
    }

    /// 数值标签（与 JS 的 `String(number)` 一致：整数不带小数点）
    pub fn format(v: f64) -> String {
        format!("{}", v)
    }
}

/// 小数位数（`0.25` → 2，`1` → 0）
fn decimals(v: f64) -> usize {
    let s = format!("{}", v);
    s.split_once('.').map(|(_, f)| f.len()).unwrap_or(0)
}

/// 布尔属性：写了且不是 `false` / `0` 就是真（`<slider show-value>` 不带值也算开）
fn attr_truthy(s: &str) -> bool {
    let s = s.trim();
    s != "false" && s != "0" && s != "{{false}}"
}

/// 轨道左端与长度：两端各让出半个滑块（滑块圆心落在轨道端点上），show-value 时右侧再让出数值宽。
///
/// 单位跟着入参走（物理像素进、物理像素出；逻辑进、逻辑出）。绘制与命中都必须用它。
pub fn slider_track_span(x: f32, w: f32, block: f32, value_w: f32) -> (f32, f32) {
    (x + block / 2.0, (w - value_w - block).max(0.0))
}

pub struct SliderComponent;

impl SliderComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        let a = SliderAttrs::from_attrs(&attrs);
        let disabled = attrs.get("disabled").map(|s| attr_truthy(s)).unwrap_or(false);
        let color = |names: &[&str], default: u32| {
            names
                .iter()
                .find_map(|n| node.get_attr(n).and_then(parse_color_str))
                .unwrap_or(Color::from_hex(default))
        };
        // 下面三个字段被借来存 slider 的三种颜色：轨道底色 / 已选部分 / 滑块
        ns.background_color = Some(color(&["backgroundColor", "background-color", "color"], 0xE9E9E9));
        ns.text_color = Some(if disabled {
            Color::from_hex(0xCCCCCC)
        } else {
            color(&["activeColor", "active-color", "selected-color", "selectedColor"], 0x1AAD19)
        });
        ns.border_color = Some(color(&["block-color", "blockColor"], 0xFFFFFF));
        // 按压态（`:active` / hover-class）对 slider 没有意义，而且有害：按压态样式在
        // build_base_style 里就算好了，那时上面这些借来的字段还没填。按住滑块时整份样式
        // 被换成按压态，轨道色变成 None、滑块直径变成 0 —— 拖动的全程滑块是**隐形**的，
        // 只剩右边的数值在跳（只要页面样式表里有任何一条 `:active` 规则就会中招）。
        ns.pressed_style = None;

        if dim_is_auto(ts.size.height) {
            ts.size.height = length(a.block_size * sf);
        }
        if dim_is_auto(ts.size.width) {
            ts.size.width = percent(1.0_f32);
        }
        ts.flex_direction = FlexDirection::Row;
        if ts.align_items.is_none() {
            ts.align_items = Some(AlignItems::CENTER);
        }

        ns.custom_data = a.progress(a.value);
        ns.border_width = a.block_size;
        ns.font_size = a.value as f32;

        let text = if a.show_value { SliderAttrs::format(a.value) } else { String::new() };
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        Some(RenderNode {
            tag: "slider".into(),
            text,
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        node: &RenderNode,
        canvas: &mut Canvas,
        text_renderer: Option<&TextRenderer>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        sf: f32,
    ) {
        let style = &node.style;
        let progress = style.custom_data;
        let block_size = style.border_width * sf;
        let show_value = !node.text.is_empty();

        if let Some(shadow) = &style.box_shadow {
            draw_box_shadow(canvas, shadow, x, y, w, h, style.border_radius);
        }
        let apply_opacity = |color: Color| -> Color {
            if style.opacity < 1.0 {
                Color::new(color.r, color.g, color.b, (color.a as f32 * style.opacity) as u8)
            } else {
                color
            }
        };

        let value_width = if show_value { SLIDER_VALUE_WIDTH * sf } else { 0.0 };
        let (track_x, track_width) = slider_track_span(x, w, block_size, value_width);
        let track_height = 4.0 * sf;
        let track_y = y + (h - track_height) / 2.0;
        let track_radius = if style.border_radius > 0.0 {
            style.border_radius.min(track_height / 2.0)
        } else {
            track_height / 2.0
        };

        if let Some(bg) = style.background_color {
            let paint = Paint::new().with_color(apply_opacity(bg)).with_style(PaintStyle::Fill).with_anti_alias(true);
            let mut path = Path::new();
            path.add_round_rect(track_x, track_y, track_width, track_height, track_radius);
            canvas.draw_path(&path, &paint);
        }

        let active_width = track_width * progress;
        if active_width > 0.0 {
            if let Some(active) = style.text_color {
                let paint = Paint::new().with_color(apply_opacity(active)).with_style(PaintStyle::Fill).with_anti_alias(true);
                let mut path = Path::new();
                path.add_round_rect(track_x, track_y, active_width, track_height, track_radius);
                canvas.draw_path(&path, &paint);
            }
        }

        let knob_x = track_x + active_width;
        let knob_y = y + h / 2.0;
        let knob_radius = block_size / 2.0;

        let shadow_paint = Paint::new()
            .with_color(Color::new(0, 0, 0, 40))
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        canvas.draw_circle(knob_x, knob_y + 2.0 * sf, knob_radius, &shadow_paint);

        let block_color = style.border_color.unwrap_or(Color::WHITE);
        let knob_paint = Paint::new().with_color(apply_opacity(block_color)).with_style(PaintStyle::Fill).with_anti_alias(true);
        canvas.draw_circle(knob_x, knob_y, knob_radius, &knob_paint);

        // 滑块边框：先画一圈浅灰，再用滑块色盖住内圈
        let border_width = 1.0 * sf;
        let border_paint = Paint::new()
            .with_color(Color::from_hex(0xE9E9E9))
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        canvas.draw_circle(knob_x, knob_y, knob_radius, &border_paint);
        let inner_paint = Paint::new().with_color(apply_opacity(block_color)).with_style(PaintStyle::Fill).with_anti_alias(true);
        canvas.draw_circle(knob_x, knob_y, knob_radius - border_width, &inner_paint);

        if show_value {
            if let Some(tr) = text_renderer {
                let font_size = 14.0 * sf;
                let text_width = tr.measure_text(&node.text, font_size);
                // 右对齐数值，确保不超出容器
                let text_x = x + w - text_width - 4.0 * sf;
                let text_y = y + (h + font_size) / 2.0 - 2.0 * sf;
                let paint = Paint::new().with_color(Color::from_hex(0x888888)).with_style(PaintStyle::Fill);
                tr.draw_text(canvas, &node.text, text_x, text_y, font_size, &paint);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs(pairs: &[(&str, &str)]) -> SliderAttrs {
        SliderAttrs::from_attrs(&pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
    }

    #[test]
    fn 默认值与夹取() {
        let a = attrs(&[]);
        assert_eq!((a.min, a.max, a.step, a.value), (0.0, 100.0, 1.0, 0.0));
        assert_eq!(a.block_size, 28.0);
        assert!(!a.show_value);
        // block-size 超出 12~28 夹回去；value 超出范围夹回去
        let a = attrs(&[("block-size", "40"), ("min", "10"), ("max", "60"), ("value", "99")]);
        assert_eq!(a.block_size, 28.0);
        assert_eq!(a.value, 60.0);
        assert_eq!(attrs(&[("block-size", "4")]).block_size, 12.0);
    }

    #[test]
    fn 按步长吸附且没有浮点噪声() {
        let a = attrs(&[("min", "0"), ("max", "1"), ("step", "0.1")]);
        assert_eq!(a.snap(0.31), 0.3);
        assert_eq!(SliderAttrs::format(a.snap(0.31)), "0.3");
        let a = attrs(&[("min", "1"), ("max", "5")]);
        assert_eq!(a.snap(3.4), 3.0);
        assert_eq!(SliderAttrs::format(a.snap(3.6)), "4");
        // max-min 不是 step 的整数倍：拉到头也只能到最后一格
        let a = attrs(&[("min", "0"), ("max", "10"), ("step", "3")]);
        assert_eq!(a.snap(10.0), 9.0);
    }

    #[test]
    fn 坐标换算与绘制几何一致() {
        let a = attrs(&[("min", "1"), ("max", "5"), ("show-value", "")]);
        assert!(a.show_value, "不带值的 show-value 也算开");
        // 盒子 x=100 宽 300：轨道 114..236（左右各让 14，右侧再让 50）
        let (tx, tw) = slider_track_span(100.0, 300.0, a.block_size, SLIDER_VALUE_WIDTH);
        assert_eq!((tx, tw), (114.0, 222.0));
        assert_eq!(a.value_at(tx, tx, tw), 1.0);
        assert_eq!(a.value_at(tx + tw, tx, tw), 5.0);
        assert_eq!(a.value_at(tx + tw * 0.75, tx, tw), 4.0);
        // 数值区与盒子外都夹到端点
        assert_eq!(a.value_at(390.0, tx, tw), 5.0);
        assert_eq!(a.value_at(0.0, tx, tw), 1.0);
        assert_eq!(a.progress(4.0), 0.75);
    }
}
