//! progress 组件 - 进度条
//! 
//! 支持完整的 CSS 样式，同时保留微信默认样式作为 fallback
//! 属性（官方 component/progress.html；长度属性认 px / rpx，见 [`attr_len_px`]）：
//! - percent: 进度百分比 0-100
//! - stroke-width: 进度条宽度，默认 6px
//! - border-radius: 圆角，默认 0（直角）
//! - font-size: 右侧百分比字号，默认 16
//! - activeColor（旧名 color）: 已选择的进度条颜色
//! - backgroundColor: 未选择的进度条颜色
//! - active / active-mode / duration / bindactiveend: 进度动画（`progress_anim.rs`）
//! - show-info: 在进度条右侧显示百分比
//! 
//! CSS 支持：
//! - width/height: 自定义尺寸
//! - background-color: 自定义轨道背景色（覆盖 backgroundColor 属性）
//! - color: 自定义进度颜色（覆盖 activeColor 属性）
//! - border-radius: 自定义圆角
//! - opacity: 透明度
//! - box-shadow: 阴影

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use taffy::prelude::*;

pub struct ProgressComponent;

/// 组件的长度属性（progress 的 stroke-width / font-size / border-radius、icon 的 size）：
/// 纯数字与 `px` 按 px，`rpx` 按 750 设计宽换算成逻辑 px（`screen_width` 是逻辑宽）
pub fn attr_len_px(v: &str, screen_width: f32) -> Option<f32> {
    let v = v.trim();
    let n = |s: &str| s.trim().parse::<f32>().ok().filter(|n| n.is_finite());
    if let Some(r) = v.strip_suffix("rpx") {
        return n(r).map(|r| r * screen_width / 750.0);
    }
    n(v.strip_suffix("px").unwrap_or(v))
}

impl ProgressComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let pct = node.get_attr("percent").and_then(|s| s.trim().parse::<f32>().ok()).unwrap_or(0.0);
        // 长度属性认 px / rpx（官方：2.4.0 起支持传入单位，默认 px）
        let len = |k: &str| node.get_attr(k).and_then(|s| attr_len_px(s, ctx.screen_width));
        let stroke_width = len("stroke-width").filter(|v| *v >= 0.0).unwrap_or(6.0);
        let show_info = node.get_attr("show-info").map(|s| crate::renderer::components::attr_is_true(s)).unwrap_or(false);
        // 右侧百分比的字号：属性 `font-size`，默认 16（官方默认值）。从前写死 12
        let info_font = len("font-size").filter(|v| *v > 0.0).unwrap_or(16.0);
        
        // 检查 CSS 是否定义了颜色
        let has_custom_bg = ns.background_color.is_some();
        // 只认写在 progress 自己身上的 color：继承来的（`page { color: #111 }`）不算 ——
        // 从前继承的字色把 activeColor 与默认绿一起盖掉，进度条画成黑的（马拉松名额进度）
        let has_custom_active = ns.text_color.is_some() && ns.text_color != ctx.inherited.color;
        let has_custom_height = !dim_is_auto(ts.size.height);
        let has_custom_radius = ns.border_radius > 0.0;
        
        // 只在 CSS 没有定义时使用属性值或默认值
        if !has_custom_bg {
            let bg_color = node.get_attr("backgroundColor").or(node.get_attr("background-color"))
                .and_then(|c| parse_color_str(c)).unwrap_or(Color::from_hex(0xEBEBEB));
            ns.background_color = Some(bg_color);
        }
        
        if !has_custom_active {
            // `activeColor` 优先；`color` 是它的旧名（官方：「进度条颜色（请使用 activeColor）」）
            let active_color = node.get_attr("activeColor").or(node.get_attr("active-color")).or(node.get_attr("color"))
                .and_then(|c| parse_color_str(c)).unwrap_or(Color::from_hex(0x09BB07));
            ns.text_color = Some(active_color);
        }
        
        // 容器高度 = stroke-width（微信的 wx-progress 是 flex 行，条高就是 stroke-width，上下不留白）。
        // 从前是 stroke+4 再加 4px 的 margin-bottom：news 热榜页进度条下面那行字比 iOS WebKit 参考低 7px。
        // 带百分比文字时行高由那行字撑（flex 行 align-items:center，取两者里高的那个）
        if !has_custom_height {
            let info_h = info_font * crate::text::LATIN_LINE_HEIGHT_FACTOR;
            let container_height = if show_info { stroke_width.max(info_h) } else { stroke_width } * sf;
            ts.size.height = length(container_height);
        }
        ns.font_size = info_font;
        
        // 默认宽度 100%
        if dim_is_auto(ts.size.width) {
            ts.size.width = percent(1.0_f32);
        }
        
        ts.flex_direction = FlexDirection::Row;
        if ts.align_items.is_none() {
            ts.align_items = Some(AlignItems::CENTER);
        }
        
        ns.custom_data = pct.clamp(0.0, 100.0);
        ns.border_width = stroke_width; // 存储实际的进度条高度
        
        // 圆角：属性 `border-radius`（px / rpx），默认 0 —— 直角（官方默认值）。
        // 从前默认画成半高的胶囊，与微信不一致；页面 CSS 写了 border-radius 的照旧用 CSS
        if !has_custom_radius {
            ns.border_radius = len("border-radius").unwrap_or(0.0).max(0.0) * sf;
        }
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        // 存储 show_info 状态
        let text = if show_info { format!("{}%", pct as i32) } else { String::new() };
        
        Some(RenderNode {
            tag: "progress".into(),
            text,
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        sf: f32
    ) {
        Self::draw_pct(node, canvas, text_renderer, (x, y, w, h), sf, None);
    }

    /// 同上，`shown`：`active` 动画此刻画到的百分比（`progress_anim::frame`，`None` = 按属性 percent）。
    /// 动画中右侧百分比跟着走（WebView 版 wx-progress 的 `curPercent`）
    pub fn draw_pct(
        node: &RenderNode,
        canvas: &mut Canvas,
        text_renderer: Option<&TextRenderer>,
        (x, y, w, h): (f32, f32, f32, f32),
        sf: f32,
        shown: Option<f32>,
    ) {
        let style = &node.style;
        let percent = shown.unwrap_or(style.custom_data).clamp(0.0, 100.0) / 100.0;
        // 只有动画中的 progress 才分配这一串
        let anim_text = match shown {
            Some(p) if !node.text.is_empty() => Some(format!("{}%", p as i32)),
            _ => None,
        };
        let info_text = anim_text.as_deref().unwrap_or(&node.text);
        let stroke_width = style.border_width * sf; // 实际进度条高度
        
        // 绘制盒子阴影
        if let Some(shadow) = &style.box_shadow {
            draw_box_shadow(canvas, shadow, x, y, w, h, style.border_radius);
        }
        
        // 应用透明度
        let apply_opacity = |color: Color| -> Color {
            if style.opacity < 1.0 {
                Color::new(color.r, color.g, color.b, (color.a as f32 * style.opacity) as u8)
            } else {
                color
            }
        };
        
        // 获取圆角值（支持四个角独立设置）
        let radius_tl = style.border_radius_tl.unwrap_or(style.border_radius);
        let radius_tr = style.border_radius_tr.unwrap_or(style.border_radius);
        let radius_br = style.border_radius_br.unwrap_or(style.border_radius);
        let radius_bl = style.border_radius_bl.unwrap_or(style.border_radius);
        let uniform_radius = radius_tl == radius_tr && radius_tr == radius_br && radius_br == radius_bl;
        let radius = radius_tl.min(stroke_width / 2.0);
        
        // 计算进度条实际宽度（如果显示百分比，需要减去文字宽度）。
        // 微信的 `.wx-progress-info`：margin-left 15px、min-width 2em，字号按 `font-size` 属性
        let show_info = !node.text.is_empty();
        let info_font = style.font_size * sf;
        let info_text_w = match (show_info, text_renderer) {
            (true, Some(tr)) => tr.measure_text(&node.text, info_font),
            _ => 0.0,
        };
        let info_width = if show_info { 15.0 * sf + info_text_w.max(2.0 * info_font) } else { 0.0 };
        let bar_width = (w - info_width).max(0.0);
        
        // 垂直居中
        let bar_y = y + (h - stroke_width) / 2.0;
        
        // 绘制背景轨道
        if let Some(bg) = style.background_color {
            let paint = Paint::new().with_color(apply_opacity(bg)).with_style(PaintStyle::Fill).with_anti_alias(true);
            if radius > 0.0 {
                let mut path = Path::new();
                if uniform_radius {
                    path.add_round_rect(x, bar_y, bar_width, stroke_width, radius);
                } else {
                    let r = radius.min(stroke_width / 2.0);
                    path.add_round_rect_varying(x, bar_y, bar_width, stroke_width, 
                        radius_tl.min(r), radius_tr.min(r), radius_br.min(r), radius_bl.min(r));
                }
                canvas.draw_path(&path, &paint);
            } else {
                canvas.draw_rect(&GeoRect::new(x, bar_y, bar_width, stroke_width), &paint);
            }
        }
        
        // 绘制进度
        if percent > 0.0 {
            if let Some(active) = style.text_color {
                let paint = Paint::new().with_color(apply_opacity(active)).with_style(PaintStyle::Fill).with_anti_alias(true);
                // 有圆角时最小宽度给够两个圆角（否则圆角路径画歪）；直角（默认）按百分比原样
                let progress_width = (bar_width * percent).max(radius * 2.0).min(bar_width.max(radius * 2.0));
                if radius > 0.0 {
                    let mut path = Path::new();
                    if uniform_radius {
                        path.add_round_rect(x, bar_y, progress_width, stroke_width, radius);
                    } else {
                        let r = radius.min(stroke_width / 2.0);
                        path.add_round_rect_varying(x, bar_y, progress_width, stroke_width,
                            radius_tl.min(r), radius_tr.min(r), radius_br.min(r), radius_bl.min(r));
                    }
                    canvas.draw_path(&path, &paint);
                } else {
                    canvas.draw_rect(&GeoRect::new(x, bar_y, progress_width, stroke_width), &paint);
                }
            }
        }
        
        // 绘制百分比文字
        if show_info {
            if let Some(tr) = text_renderer {
                let font_size = info_font;
                let text_x = x + bar_width + 15.0 * sf;
                // 与进度条同一条中线（flex 行 align-items:center）
                let text_y = y + (h + font_size) / 2.0 - font_size * 0.12;
                let paint = Paint::new().with_color(Color::from_hex(0x999999)).with_style(PaintStyle::Fill);
                tr.draw_text(canvas, info_text, text_x, text_y, font_size, &paint);
            }
        }
    }
}
