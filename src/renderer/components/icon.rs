//! icon 组件 - 图标
//!
//! 图形数据全部来自 WeUI（见 `icon_data.rs`），也就是微信 `<icon>` 自己用的那套
//! 字形；这里只负责「把字形放进组件盒」和样式解析。
//!
//! 微信官方 type：success / success_no_circle / info / warn / waiting / cancel /
//! download / search / clear。其余（close / back / arrow_* / plus / minus /
//! star / heart / chat / *_no_circle）是引擎扩展，仍尽量取 WeUI 的同名字形。
//!
//! 属性与 CSS：
//! - `size`：图标边长（px，默认 23），CSS 的 width/height 优先；
//! - `color`：WXML 属性优先于 CSS `color`，都没有时用微信默认色；
//! - 支持 opacity、box-shadow。

use super::base::*;
use super::icon_data::{icon_default_color, icon_glyph, IconGlyph};
use super::svg_path::{append_svg_path, Xf};
use crate::parser::wxml::WxmlNode;
use crate::{Canvas, Color, Paint, PaintStyle, Path};
use taffy::prelude::*;

pub struct IconComponent;

impl IconComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;

        let icon_type = node.get_attr("type").unwrap_or("success");
        let icon_size = node
            .get_attr("size")
            .and_then(|s| s.trim().parse::<f32>().ok())
            .unwrap_or(23.0);
        let icon_color = node.get_attr("color").and_then(parse_color_str);

        // CSS 定了尺寸就听 CSS 的，否则用 size 属性
        let has_custom_size = !dim_is_auto(ts.size.width) || !dim_is_auto(ts.size.height);
        if !has_custom_size {
            let size = icon_size * sf;
            ts.size = Size { width: length(size), height: length(size) };
        }
        // 图标不参与 flex 压缩/拉伸（与 HTML 端 .wxicon{flex:none} 一致），
        // 否则同列的兄弟节点一有溢出就会把图标压扁。
        ts.flex_shrink = 0.0;
        ts.flex_grow = 0.0;

        // 颜色优先级：WXML color 属性 > CSS color > 微信默认色
        if let Some(color) = icon_color {
            ns.text_color = Some(color);
        } else if ns.text_color.is_none() {
            ns.text_color = Some(Color::from_hex(icon_default_color(icon_type)));
        }

        ns.font_size = icon_size;

        let tn = ctx.taffy.new_leaf(ts).unwrap();

        Some(RenderNode {
            tag: "icon".into(),
            text: icon_type.into(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }

    pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, _sf: f32) {
        let style = &node.style;

        if let Some(shadow) = &style.box_shadow {
            draw_box_shadow(canvas, shadow, x, y, w, h, 0.0);
        }

        let icon_type = node.text.as_str();
        let Some(glyph) = icon_glyph(icon_type) else {
            // 微信对未知 type 不画东西
            return;
        };
        if w <= 0.0 || h <= 0.0 {
            return;
        }

        let base = style
            .text_color
            .unwrap_or_else(|| Color::from_hex(icon_default_color(icon_type)));
        let color = if style.opacity < 1.0 {
            Color::new(base.r, base.g, base.b, (base.a as f32 * style.opacity) as u8)
        } else {
            base
        };

        let mut path = Path::new();
        append_svg_path(glyph.d, &fit_transform(&glyph, x, y, w, h), &mut path);
        // even-odd 填充：子路径反向绕出来的洞（对勾/叉/指针）保持透明，
        // 与微信的 mask 效果一致 —— 放在有色背景上会透出背景色。
        let paint = Paint::new()
            .with_color(color)
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        canvas.draw_path(&path, &paint);
    }
}

/// 把字形视图盒按 `xMidYMid meet`（等比缩放取小 + 居中）放进组件盒，
/// 需要时先按 `glyph.rot` 转向。
fn fit_transform(glyph: &IconGlyph, x: f32, y: f32, w: f32, h: f32) -> Xf {
    // 第一步：旋转，并把结果平移回第一象限（转 90° 时 (x,y) -> (vh - y, x)）
    let spin = match glyph.rot as i32 {
        90 => Xf::rotate(90.0).then(&Xf::translate(glyph.vh, 0.0)),
        180 => Xf::rotate(180.0).then(&Xf::translate(glyph.vw, glyph.vh)),
        270 => Xf::rotate(-90.0).then(&Xf::translate(0.0, glyph.vw)),
        _ => Xf::IDENTITY,
    };
    // 第二步：等比缩放 + 居中
    let (vw, vh) = glyph.view_size();
    let s = (w / vw).min(h / vh);
    let ox = x + (w - vw * s) / 2.0;
    let oy = y + (h - vh * s) / 2.0;
    spin.then(&Xf::scale_translate(s, s, ox, oy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::components::icon_data::ICON_TYPES;

    fn glyph_ink(icon_type: &str, x: f32, y: f32, w: f32, h: f32) -> (f32, f32, f32, f32) {
        let g = icon_glyph(icon_type).unwrap();
        let mut p = Path::new();
        append_svg_path(g.d, &fit_transform(&g, x, y, w, h), &mut p);
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for c in p.flatten(0.1) {
            for pt in c {
                x0 = x0.min(pt.x);
                y0 = y0.min(pt.y);
                x1 = x1.max(pt.x);
                y1 = y1.max(pt.y);
            }
        }
        (x0, y0, x1, y1)
    }

    #[test]
    fn square_glyph_fills_box_and_stays_centered() {
        // 23px 盒子里 success 的圆直径应为 23 * 5/6 ≈ 19.17（WeUI 字面自带留白）
        let (x0, y0, x1, y1) = glyph_ink("success", 10.0, 20.0, 23.0, 23.0);
        let d = ((x1 - x0) + (y1 - y0)) / 2.0;
        assert!((d - 23.0 * 5.0 / 6.0).abs() < 0.2, "直径 {d}");
        assert!(((x0 + x1) / 2.0 - 21.5).abs() < 0.1, "水平中心 {}", (x0 + x1) / 2.0);
        assert!(((y0 + y1) / 2.0 - 31.5).abs() < 0.1, "垂直中心 {}", (y0 + y1) / 2.0);
    }

    #[test]
    fn non_square_glyph_keeps_ratio_and_centers() {
        // 12x24 的右箭头放进 24x24 的盒子：等比缩放取 min(24/12, 24/24) = 1，
        // 视图盒被摆在 x ∈ [6, 18]（居中），墨迹只能落在这一段里，不会被横向拉宽。
        // 注意 WeUI 的箭头字形本身在 12 宽的盒里偏右，所以校验的是视图盒的落位，
        // 不是墨迹的中心。
        let (x0, y0, x1, y1) = glyph_ink("arrow_right", 0.0, 0.0, 24.0, 24.0);
        assert!(x0 >= 6.0 && x1 <= 18.0, "水平范围 {x0}..{x1} 应落在居中的 [6,18] 视图盒内");
        assert!(y0 >= 5.0 && y1 <= 19.0, "竖直范围 {y0}..{y1}"); // 字形本身上下有留白
        assert!(x1 - x0 <= 12.0, "宽度 {}", x1 - x0);
    }

    #[test]
    fn rotated_arrow_spans_horizontally() {
        // 向下箭头旋转后应变成「宽 > 高」
        let (x0, y0, x1, y1) = glyph_ink("arrow_down", 0.0, 0.0, 24.0, 24.0);
        assert!(x1 - x0 > y1 - y0, "旋转后 {}x{}", x1 - x0, y1 - y0);
    }

    #[test]
    fn all_types_render_inside_the_box() {
        for t in ICON_TYPES {
            let (x0, y0, x1, y1) = glyph_ink(t, 5.0, 7.0, 40.0, 40.0);
            assert!(x0 >= 4.9 && x1 <= 45.1, "{t} 水平越界 {x0}..{x1}");
            assert!(y0 >= 6.9 && y1 <= 47.1, "{t} 垂直越界 {y0}..{y1}");
        }
    }

    #[test]
    fn zero_sized_box_draws_nothing() {
        // 布局还没算出尺寸时不能崩
        let g = icon_glyph("success").unwrap();
        let xf = fit_transform(&g, 0.0, 0.0, 0.0, 0.0);
        let mut p = Path::new();
        append_svg_path(g.d, &xf, &mut p);
        for c in p.flatten(0.2) {
            for pt in c {
                assert!(pt.x.is_finite() && pt.y.is_finite());
            }
        }
    }
}
