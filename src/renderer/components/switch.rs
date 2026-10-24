//! switch 组件 - 开关选择器
//! 
//! 支持完整的 CSS 样式，同时保留微信默认样式作为 fallback
//! 属性：
//! - checked: 是否选中
//! - disabled: 是否禁用
//! - type: switch(默认) / checkbox
//! - color: 选中时的颜色
//! 
//! CSS 支持：
//! - width/height: 自定义尺寸
//! - background-color: 自定义轨道背景色
//! - border-radius: 自定义圆角
//! - opacity: 透明度
//! - box-shadow: 阴影

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::{Canvas, Color, Paint, PaintStyle, Path};
use taffy::prelude::*;

/// 微信 switch 默认轨道尺寸（weui：52x32，内边距 2，滑块 28）
pub const SWITCH_WIDTH: f32 = 52.0;
pub const SWITCH_HEIGHT: f32 = 32.0;
/// 关态轨道：WeUI 的 `--weui-FG-3` = `rgba(0,0,0,.1)`（压在白底上是 #E6E6E6）
const TRACK_OFF_ALPHA: u8 = 26;
/// 滑块与轨道之间的内边距（`padding: 2px`）
const KNOB_INSET: f32 = 2.0;
/// 滑块投影 `0 2px 3px 0 rgba(0,0,0,.06)`
const KNOB_SHADOW_ALPHA: u8 = 15;
/// disabled 的整体透明度（与 H5 导出的 `.wx-switch-disabled` 同值）
const DISABLED_OPACITY: f32 = 0.45;
/// switch 默认开启色（微信文档默认值）
const DEFAULT_ON: u32 = 0x04BE02;

pub struct SwitchComponent;

impl SwitchComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let checked = node.get_attr("checked").map(|s| crate::renderer::components::attr_is_true(s)).unwrap_or(false);
        let switch_type = node.get_attr("type").unwrap_or("switch");
        
        // 检查 CSS 是否定义了尺寸和颜色
        let has_custom_size = !dim_is_auto(ts.size.width) || 
                              !dim_is_auto(ts.size.height);
        let has_custom_radius = ns.border_radius > 0.0;
        
        // 微信（weui）官方 switch 度量：轨道 52x32、内边距 2、滑块直径 28（见 draw_switch）
        let (default_width, default_height) = if switch_type == "checkbox" {
            (24.0, 24.0)
        } else {
            (SWITCH_WIDTH, SWITCH_HEIGHT)
        };
        
        if !has_custom_size {
            ts.size = Size { width: length(default_width * sf), height: length(default_height * sf) };
        }
        // 开关不参与 flex 伸缩（对齐 HTML .wx-switch{flex:none}），否则会被兄弟节点压扁
        ts.flex_shrink = 0.0;
        ts.flex_grow = 0.0;
        
        // 只在 CSS 没有定义时使用默认圆角
        if !has_custom_radius {
            ns.border_radius = if switch_type == "checkbox" { 
                4.0 * sf 
            } else { 
                default_height * sf / 2.0 
            };
        }
        
        // custom_data 存「开启进度」：0=关、1=开，中间值用于点击后的滑动过渡。
        // 轨道/滑块颜色都在绘制期按进度求值，这样运行时切换也能拿到正确配色
        // （此前一旦交互层改写 background_color，关态就变成纯白而不是微信的浅灰）。
        ns.custom_data = if checked { 1.0 } else { 0.0 };
        
        let text = switch_type.to_string();
        
        // 按压态样式是建树前半段（build_base_style）的快照，上面填的选中进度、圆角都不在里面：
        // 按住一个开着的开关，它会先跳成关、抬手再拨。微信的 switch 没有点击态，丢掉
        // （同 checkbox.rs / slider.rs）。
        ns.pressed_style = None;
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "switch".into(),
            text,
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        let style = &node.style;
        let switch_type = node.text.as_str();
        
        // 绘制盒子阴影
        if let Some(shadow) = &style.box_shadow {
            draw_box_shadow(canvas, shadow, x, y, w, h, style.border_radius);
        }
        
        // `color` 对两种形态都生效：switch 是开启轨道色，checkbox 是选中底色
        let on_color = node.attrs.get("color").and_then(|c| parse_color_str(c))
            .unwrap_or(Color::from_hex(DEFAULT_ON));
        if switch_type == "checkbox" {
            Self::draw_checkbox(canvas, style, style.custom_data > 0.5, on_color, x, y, w, h, sf);
        } else {
            let progress = style.custom_data.clamp(0.0, 1.0);
            let disabled = node.attrs.get("disabled")
                .map(|s| crate::renderer::components::attr_is_true(s)).unwrap_or(false);
            Self::draw_switch(canvas, style, progress, on_color, disabled, x, y, w, h, sf);
        }
    }
    
    /// 按微信当前的开关画（WeUI master 的 `weui-switch.less`，微信 7.0 起的新版组件样式）：
    /// 52×32 实心轨道（关 = FG-3 `rgba(0,0,0,.1)`，开 = `color`）、**没有边框也没有内层白胶囊**，
    /// 内边距 2 里一个 28 的白色滑块，投影 `0 2px 3px rgba(0,0,0,.06)`，开启时右移 20。
    ///
    /// 从前画的是 WeUI 1.x 那一版（1px 灰边 + 内层 #FDFDFD 白胶囊 + 30 的滑块、投影 `.4`）：
    /// 关态看起来是「白底细灰框」，和现在微信的实心浅灰轨道差得最明显。
    ///
    /// `progress` 为 0..1 的开启进度，点击后由交互层按缓动推进，形成滑动手感。
    /// disabled 是**整体**半透明（CSS `opacity` 的语义）：先画进离屏再按透明度贴回 ——
    /// 逐个颜色乘 alpha 的话，半透明的滑块会透出底下的绿色轨道。
    fn draw_switch(
        canvas: &mut Canvas,
        style: &NodeStyle,
        progress: f32,
        on_color: Color,
        disabled: bool,
        x: f32, y: f32, w: f32, h: f32,
        sf: f32,
    ) {
        let opacity = style.opacity * if disabled { DISABLED_OPACITY } else { 1.0 };
        if opacity >= 0.999 {
            Self::paint_switch(canvas, style, progress, on_color, x, y, w, h, sf);
            return;
        }
        // 整组透明：投影往下、往外各多 2+3 像素，离屏留够边
        let pad = (6.0 * sf).ceil();
        let (ox, oy) = ((x - pad).floor(), (y - pad).floor());
        let (tw, th) = ((w + pad * 2.0).ceil() as u32 + 1, (h + pad * 2.0).ceil() as u32 + 1);
        if tw == 0 || th == 0 || tw > 2048 || th > 2048 {
            return;
        }
        let mut off = Canvas::new(tw, th);
        off.clear(Color::new(0, 0, 0, 0));
        Self::paint_switch(&mut off, style, progress, on_color, x - ox, y - oy, w, h, sf);
        canvas.blend_pixels_opacity(ox as i32, oy as i32, off.pixels(), tw as usize, opacity);
    }

    /// 不带透明度地画一个开关（轨道 + 滑块），见 [`Self::draw_switch`]
    #[allow(clippy::too_many_arguments)]
    fn paint_switch(
        canvas: &mut Canvas,
        style: &NodeStyle,
        progress: f32,
        on_color: Color,
        x: f32, y: f32, w: f32, h: f32,
        sf: f32,
    ) {
        let lerp = |a: u8, b: u8| -> u8 {
            (a as f32 + (b as f32 - a as f32) * progress).round().clamp(0.0, 255.0) as u8
        };
        let radius = if style.border_radius > 0.0 { style.border_radius.min(h / 2.0) } else { h / 2.0 };
        // 轨道：CSS 指定过背景就尊重 CSS，否则关态 FG-3 → 开态主色（颜色与透明度一起插值）
        let off_track = Color::new(0, 0, 0, TRACK_OFF_ALPHA);
        let track = style.background_color.unwrap_or_else(|| {
            Color::new(
                lerp(off_track.r, on_color.r),
                lerp(off_track.g, on_color.g),
                lerp(off_track.b, on_color.b),
                lerp(off_track.a, on_color.a),
            )
        });
        if w > 0.0 && h > 0.0 {
            let paint = Paint::new().with_color(track).with_style(PaintStyle::Fill).with_anti_alias(true);
            let mut path = Path::new();
            path.add_round_rect(x, y, w, h, radius.min(w / 2.0));
            canvas.draw_path(&path, &paint);
        }

        // 滑块：轨道内边距 2，直径 = 轨道高 - 4（52×32 时 28，行程 20）
        let inset = KNOB_INSET * sf;
        let d = (h - inset * 2.0).max(0.0);
        let left = x + inset;
        let right = x + w - inset - d;
        let kx = left + (right - left) * progress;
        let ky = y + inset;
        crate::renderer::components::shadow::draw_outer_shadow(
            canvas,
            Color::new(0, 0, 0, KNOB_SHADOW_ALPHA),
            0.0,
            2.0 * sf,
            3.0 * sf,
            0.0,
            kx,
            ky,
            d,
            d,
            d / 2.0,
        );
        let knob_paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill).with_anti_alias(true);
        canvas.draw_circle(kx + d / 2.0, ky + d / 2.0, d / 2.0, &knob_paint);
    }
    
    /// `type="checkbox"` 形态：与 `<checkbox>` 同一套外观（选中 = 色块 + 白勾，未选中 = 白底灰边）。
    ///
    /// 选中底色 = CSS `background-color`，没写就用 `color` 属性。从前只有 CSS 写了背景才填底，
    /// 而交互层并不替 switch 改写背景色（开关配色归组件按进度求值，见 interactive.rs），
    /// 于是选中后只剩一个白勾压在白卡片上 —— 看着像点了没反应。
    #[allow(clippy::too_many_arguments)]
    fn draw_checkbox(canvas: &mut Canvas, style: &NodeStyle, checked: bool, on_color: Color, x: f32, y: f32, w: f32, h: f32, _sf: f32) {
        // 应用透明度
        let apply_opacity = |color: Color| -> Color {
            if style.opacity < 1.0 {
                Color::new(color.r, color.g, color.b, (color.a as f32 * style.opacity) as u8)
            } else {
                color
            }
        };
        
        // 获取圆角值
        let radius_tl = style.border_radius_tl.unwrap_or(style.border_radius);
        let radius_tr = style.border_radius_tr.unwrap_or(style.border_radius);
        let radius_br = style.border_radius_br.unwrap_or(style.border_radius);
        let radius_bl = style.border_radius_bl.unwrap_or(style.border_radius);
        let uniform_radius = radius_tl == radius_tr && radius_tr == radius_br && radius_br == radius_bl;
        let radius = radius_tl;
        
        // 绘制背景 - 使用抗锯齿（未选中时背景随后被白底灰边盖住，与原来一致）
        let fill = if checked { Some(style.background_color.unwrap_or(on_color)) } else { style.background_color };
        if let Some(bg) = fill {
            let paint = Paint::new().with_color(apply_opacity(bg)).with_style(PaintStyle::Fill).with_anti_alias(true);
            let mut path = Path::new();
            if uniform_radius {
                path.add_round_rect(x, y, w, h, radius);
            } else {
                path.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
            }
            canvas.draw_path(&path, &paint);
        }
        
        // 绘制边框
        if !checked {
            let border_width = 2.0;
            let border_paint = Paint::new()
                .with_color(Color::from_hex(0xD1D1D1))
                .with_style(PaintStyle::Fill)
                .with_anti_alias(true);
            let mut border = Path::new();
            if uniform_radius {
                border.add_round_rect(x, y, w, h, radius);
            } else {
                border.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
            }
            canvas.draw_path(&border, &border_paint);
            
            // 内部白色
            let inner_paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill).with_anti_alias(true);
            let mut inner = Path::new();
            let inner_radius = |r: f32| (r - border_width).max(0.0);
            if uniform_radius {
                inner.add_round_rect(
                    x + border_width, 
                    y + border_width, 
                    w - border_width * 2.0, 
                    h - border_width * 2.0, 
                    inner_radius(radius)
                );
            } else {
                inner.add_round_rect_varying(
                    x + border_width, 
                    y + border_width, 
                    w - border_width * 2.0, 
                    h - border_width * 2.0, 
                    inner_radius(radius_tl), inner_radius(radius_tr),
                    inner_radius(radius_br), inner_radius(radius_bl)
                );
            }
            canvas.draw_path(&inner, &inner_paint);
        }
        
        // 绘制对勾 - 使用粗线条
        if checked {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let thickness = w * 0.12;
            
            let p1 = (cx - w * 0.25, cy);
            let p2 = (cx - w * 0.05, cy + h * 0.2);
            let p3 = (cx + w * 0.25, cy - h * 0.2);
            
            let paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill).with_anti_alias(true);
            let half = thickness / 2.0;
            
            // 第一段
            let angle1 = ((p2.1 - p1.1) / (p2.0 - p1.0)).atan();
            let dx1 = half * angle1.sin();
            let dy1 = half * angle1.cos();
            
            let mut seg1 = Path::new();
            seg1.move_to(p1.0 - dx1, p1.1 + dy1);
            seg1.line_to(p1.0 + dx1, p1.1 - dy1);
            seg1.line_to(p2.0 + dx1, p2.1 - dy1);
            seg1.line_to(p2.0 - dx1, p2.1 + dy1);
            seg1.close();
            canvas.draw_path(&seg1, &paint);
            
            // 第二段
            let angle2 = ((p3.1 - p2.1) / (p3.0 - p2.0)).atan();
            let dx2 = half * angle2.sin();
            let dy2 = half * angle2.cos();
            
            let mut seg2 = Path::new();
            seg2.move_to(p2.0 - dx2, p2.1 + dy2);
            seg2.line_to(p2.0 + dx2, p2.1 - dy2);
            seg2.line_to(p3.0 + dx2, p3.1 - dy2);
            seg2.line_to(p3.0 - dx2, p3.1 + dy2);
            seg2.close();
            canvas.draw_path(&seg2, &paint);
            
            // 拐点圆形
            canvas.draw_circle(p2.0, p2.1, half, &paint);
        }
    }
}
