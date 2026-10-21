//! swiper 的指示点：尺寸 / 位置属性与 Skyline 的 `indicator-type` 动画。
//!
//! 依据 Skyline swiper 增强特性（`indicator-type`、`indicator-margin` / `-spacing` / `-radius` /
//! `-width` / `-height` / `-alignment` / `-offset`）。**没写这些属性时外观与从前一字不差**
//! （7px 圆点、间距 6、距边 8 —— 与 H5 导出一致，画廊与双端基线不动）；写了的按属性画。
//!
//! 动画都由**连续位置** `pos`（第几页的浮点数）驱动，拖动与翻页过程中逐帧变化：
//! - `normal`：选中点跳到最近的一页；
//! - `slide` / `swap` / `swapYRotation` / `scroll`：选中点随位置连续滑动；
//! - `worm` / `wormThin`：选中点像虫子一样先伸头再收尾；`*Underground` 把选中点画在底下；
//! - `expand`：选中点变宽（宽度在相邻两点间插值）；`scale`：选中点放大；
//! - `jump` / `jumpWithOffset`：选中点沿弧线跳到下一点；`color`：颜色在相邻两点间渐变；
//! - `scrollFixedCenter`：整排点滚动，选中点固定在中间。
//!
//! 取舍：`swapYRotation` 的绕 Y 轴翻转没有 3D，按 `swap`（平移）画。

use crate::renderer::components::base::RenderNode;
use crate::renderer::components::color_parse::parse_color_str;
use crate::{Canvas, Color, Paint, PaintStyle};

struct Geo {
    w: f32,
    h: f32,
    radius: f32,
    spacing: f32,
    margin: f32,
    align: Option<(f32, f32)>,
    offset: (f32, f32),
}

fn num(node: &RenderNode, k: &str) -> Option<f32> {
    node.attrs.get(k).and_then(|s| s.trim().trim_end_matches("px").trim().parse::<f32>().ok())
}

fn pair(node: &RenderNode, k: &str) -> Option<(f32, f32)> {
    let v = crate::renderer::components::parse_attr_json(node.attrs.get(k)?);
    let a = v.as_array()?;
    Some((a.first()?.as_f64()? as f32, a.get(1)?.as_f64()? as f32))
}

fn geo(node: &RenderNode) -> Geo {
    let w = num(node, "indicator-width").unwrap_or(7.0).max(0.0);
    let h = num(node, "indicator-height").unwrap_or(7.0).max(0.0);
    Geo {
        w,
        h,
        radius: num(node, "indicator-radius").unwrap_or(w.min(h) / 2.0).max(0.0),
        spacing: num(node, "indicator-spacing").unwrap_or(6.0).max(0.0),
        margin: num(node, "indicator-margin").unwrap_or(8.0).max(0.0),
        align: pair(node, "indicator-alignment"),
        offset: pair(node, "indicator-offset").unwrap_or((0.0, 0.0)),
    }
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color::new(l(a.r, b.r), l(a.g, b.g), l(a.b, b.b), l(a.a, b.a))
}

#[allow(clippy::too_many_arguments)]
fn pill(canvas: &mut Canvas, cx: f32, cy: f32, w: f32, h: f32, r: f32, color: Color) {
    if (w - h).abs() < 0.01 && (r * 2.0 - w).abs() < 0.01 {
        // 圆点：与从前同一个画法（逐像素基线）
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
        canvas.draw_circle(cx, cy, w / 2.0, &paint);
    } else {
        canvas.fill_round_rect(cx - w / 2.0, cy - h / 2.0, w, h, r, color);
    }
}

/// 画指示点。`pos` 是连续位置（页），`current` 是当前页
#[allow(clippy::too_many_arguments)]
pub fn draw(node: &RenderNode, canvas: &mut Canvas, (x, y, w, h): (f32, f32, f32, f32), sf: f32, pos: f32, current: usize, pages: usize, vertical: bool, circular: bool) {
    let inactive = node.attrs.get("indicator-color").and_then(|s| parse_color_str(s)).unwrap_or(Color::new(0, 0, 0, 76));
    let active = node.attrs.get("indicator-active-color").and_then(|s| parse_color_str(s)).unwrap_or(Color::BLACK);
    let g = geo(node);
    let kind = node.attrs.get("indicator-type").map(|s| s.trim().to_string()).unwrap_or_default();
    let (dw, dh, gap, margin) = (g.w * sf, g.h * sf, g.spacing * sf, g.margin * sf);
    let r = g.radius * sf;
    let step = if vertical { dh + gap } else { dw + gap };
    let total = pages as f32 * if vertical { dh } else { dw } + (pages - 1) as f32 * gap;
    // 连续位置归一到 [0, pages)：衔接模式从末页往首页走时也连续
    let p = if circular { pos.rem_euclid(pages as f32) } else { pos.clamp(0.0, (pages - 1) as f32) };
    let i0 = p.floor() as usize % pages;
    let f = p - p.floor();
    let i1 = if circular { (i0 + 1) % pages } else { (i0 + 1).min(pages - 1) };
    // 一排点的中心
    let (mut gx, mut gy) = match g.align {
        Some((ax, ay)) => {
            let (half_w, half_h) = if vertical { (dw / 2.0, total / 2.0) } else { (total / 2.0, dh / 2.0) };
            (x + w / 2.0 + ax * (w / 2.0 - margin - half_w), y + h / 2.0 + ay * (h / 2.0 - margin - half_h))
        }
        None if vertical => (x + w - margin - dw / 2.0, y + h / 2.0),
        None => (x + w / 2.0, y + h - margin - dh / 2.0),
    };
    gx += g.offset.0 * sf;
    gy += g.offset.1 * sf;
    if kind == "scrollFixedCenter" {
        // 整排平移：选中点（连续位置）始终在正中
        let shift = (p - (pages as f32 - 1.0) / 2.0) * step;
        if vertical { gy -= shift } else { gx -= shift }
    }
    // 没写对齐、没开整排滚动：沿用从前的算式（浮点运算顺序都不改，默认外观逐字节不变）
    let legacy = g.align.is_none() && kind != "scrollFixedCenter";
    let (ox, oy) = (g.offset.0 * sf, g.offset.1 * sf);
    let center = |i: f32| -> (f32, f32) {
        if legacy {
            return if vertical {
                (gx, y + (h - total) / 2.0 + i * step + dh / 2.0 + oy)
            } else {
                (x + (w - total) / 2.0 + i * step + dw / 2.0 + ox, gy)
            };
        }
        let o = -total / 2.0 + i * step + if vertical { dh / 2.0 } else { dw / 2.0 };
        if vertical { (gx, gy + o) } else { (gx + o, gy) }
    };
    let thin = kind.starts_with("wormThin");
    let underground = kind.ends_with("Underground");
    let draw_inactive = |canvas: &mut Canvas| {
        for i in 0..pages {
            let (cx, cy) = center(i as f32);
            let (iw, ih) = if thin { if vertical { (dw * 0.5, dh) } else { (dw, dh * 0.5) } } else { (dw, dh) };
            let color = match kind.as_str() {
                "color" if i == i0 => mix(active, inactive, f),
                "color" if i == i1 && i1 != i0 => mix(inactive, active, f),
                "" | "normal" if i == current => active,
                _ => inactive,
            };
            pill(canvas, cx, cy, iw, ih, r, color);
        }
    };
    if !underground {
        draw_inactive(canvas);
    }
    match kind.as_str() {
        "" | "normal" | "color" => {}
        "worm" | "wormThin" | "wormUnderground" | "wormThinUnderground" => {
            // 头先走（前半程），尾后跟（后半程）
            let head = i0 as f32 + (2.0 * f).min(1.0);
            let tail = i0 as f32 + (2.0 * f - 1.0).max(0.0);
            let (a, b) = (center(tail), center(head));
            let (cx, cy) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
            let len = (head - tail) * step;
            if vertical { pill(canvas, cx, cy, dw, dh + len, r, active) } else { pill(canvas, cx, cy, dw + len, dh, r, active) }
        }
        "expand" => {
            let (cx, cy) = center(i0 as f32 + f);
            let grow = 1.0 + (1.0 - (2.0 * f - 1.0).abs());
            if vertical { pill(canvas, cx, cy, dw, dh * grow.max(1.0) * 1.5, r, active) } else { pill(canvas, cx, cy, dw * grow.max(1.0) * 1.5, dh, r, active) }
        }
        "scale" => {
            // 静止时选中点放大 1.4 倍，翻页途中缩回原大再在下一点放大
            let near = (i0 as f32 + f).round();
            let s = 1.0 + 0.4 * ((f - 0.5).abs() * 2.0);
            let (cx, cy) = center(near);
            pill(canvas, cx, cy, dw * s, dh * s, r * s, active);
        }
        "jump" | "jumpWithOffset" => {
            let (cx, cy) = center(i0 as f32 + f);
            let lift = (f * std::f32::consts::PI).sin() * if kind == "jump" { dh } else { dh * 1.6 };
            if vertical { pill(canvas, cx - lift, cy, dw, dh, r, active) } else { pill(canvas, cx, cy - lift, dw, dh, r, active) }
        }
        // slide / slideUnderground / swap / swapYRotation / scroll / scrollFixedCenter：连续滑动
        _ => {
            let (cx, cy) = center(i0 as f32 + f);
            pill(canvas, cx, cy, dw, dh, r, active);
        }
    }
    if underground {
        draw_inactive(canvas);
    }
}
