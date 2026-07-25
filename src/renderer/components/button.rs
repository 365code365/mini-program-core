//! button 组件 - 按钮
//! 
//! 支持完整的 CSS 样式，同时保留微信默认样式作为 fallback
//! - type: default(灰色) / primary(绿色) / warn(红色)
//! - size: default / mini
//! - plain: 镂空按钮
//! - disabled: 禁用状态

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use taffy::prelude::*;

pub struct ButtonComponent;

impl ButtonComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        // 首先使用 base 的样式解析，获取 CSS 定义的样式
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let text = get_text_content(node);
        let btn_type = node.get_attr("type").unwrap_or("default");
        let btn_size = node.get_attr("size").unwrap_or("default");
        let plain = node.get_attr("plain").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let disabled = node.get_attr("disabled").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        
        // 只有在 CSS 没有定义时才使用微信默认样式
        let has_custom_bg = ns.background_color.is_some();
        let has_custom_color = ns.text_color.is_some();
        let has_custom_border = ns.border_color.is_some() || ns.border_width > 0.0;
        
        // 微信默认配色（仅在没有自定义样式时使用）
        if !has_custom_bg || !has_custom_color {
            let (default_bg, default_fg, default_border) = match btn_type {
                "primary" => {
                    if plain {
                        (Color::WHITE, Color::from_hex(0x07C160), Some(Color::from_hex(0x07C160)))
                    } else {
                        (Color::from_hex(0x07C160), Color::WHITE, None)
                    }
                }
                "warn" => {
                    if plain {
                        (Color::WHITE, Color::from_hex(0xE64340), Some(Color::from_hex(0xE64340)))
                    } else {
                        (Color::from_hex(0xE64340), Color::WHITE, None)
                    }
                }
                _ => { // default
                    if plain {
                        (Color::WHITE, Color::from_hex(0x353535), Some(Color::from_hex(0x353535)))
                    } else {
                        (Color::from_hex(0xF8F8F8), Color::BLACK, Some(Color::from_hex(0xD9D9D9)))
                    }
                }
            };
            
            if !has_custom_bg {
                ns.background_color = Some(default_bg);
            }
            if !has_custom_color {
                ns.text_color = Some(default_fg);
            }
            if !has_custom_border {
                ns.border_color = default_border;
                if default_border.is_some() {
                    ns.border_width = 1.0 * sf;
                }
            }
        }
        
        // 禁用状态覆盖颜色
        if disabled {
            ns.background_color = Some(Color::from_hex(0xF7F7F7));
            ns.text_color = Some(Color::from_hex(0xB2B2B2));
        }
        
        // 各项默认值相互独立地应用（此前它们被绑在同一个 if 里：页面只要写了
        // `width:100%` 就会让 has_custom_size 为真，连默认内边距/最小高度/字号一起丢掉，
        // 于是按钮只有文字高，明显比微信和 HTML 端的 46px 矮）。
        let has_custom_width = !matches!(ts.size.width, Dimension::Auto);
        let has_custom_height = !matches!(ts.size.height, Dimension::Auto);
        let has_custom_padding = !matches!(ts.padding.top, LengthPercentage::Length(0.0)) ||
                                  !matches!(ts.padding.left, LengthPercentage::Length(0.0));
        
        // 微信默认度量：字号 / 垂直内边距 / 水平内边距 / 圆角 / 最小高度
        let (font_size, padding_v, padding_h, radius, min_height) = match btn_size {
            "mini" => (13.0, 4.0, 12.0, 3.0, 30.0),
            _ => (18.0, 12.0, 24.0, 5.0, 46.0),
        };
        
        // 只在没有自定义 font-size 时使用默认值（14.0 是 NodeStyle 的默认值）
        if ns.font_size == 14.0 {
            ns.font_size = font_size;
        }
        // 只在没有自定义 border-radius 时使用默认值
        if ns.border_radius == 0.0 {
            ns.border_radius = radius * sf;
        }
        if !has_custom_padding {
            ts.padding = Rect { 
                top: length(padding_v * sf), 
                right: length(padding_h * sf), 
                bottom: length(padding_v * sf), 
                left: length(padding_h * sf) 
            };
        }
        // 宽度：mini 按内容宽，其余占满一行（与微信 block 按钮一致）
        if !has_custom_width {
            ts.size.width = if btn_size == "mini" {
                let tw = intrinsic_text_width(&text, ns.font_size * sf, ns.letter_spacing * sf);
                length(tw + padding_h * 2.0 * sf)
            } else {
                percent(1.0)
            };
        }
        // 最小高度：按 CSS 级联语义，控件默认样式表里的 min-height 不会被页面的
        // height 覆盖（min-height 优先级高于 height），因此只要页面没有显式写
        // min-height 就应用微信默认值——与浏览器端 .wx-button{min-height:46px} 一致。
        let _ = has_custom_height;
        if matches!(ts.min_size.height, Dimension::Auto) {
            let content_min = (ns.font_size + padding_v * 2.0) * sf;
            ts.min_size.height = length(content_min.max(min_height * sf));
        }
        
        // 默认 margin（如果没有自定义）
        let has_custom_margin = !matches!(ts.margin.top, LengthPercentageAuto::Length(0.0));
        if !has_custom_margin {
            ts.margin = Rect { 
                top: length(5.0 * sf), 
                right: length(0.0), 
                bottom: length(5.0 * sf), 
                left: length(0.0) 
            };
        }
        
        // 默认居中对齐
        if ts.align_items.is_none() {
            ts.align_items = Some(AlignItems::Center);
        }
        if ts.justify_content.is_none() {
            ts.justify_content = Some(JustifyContent::Center);
        }
        
        // 按钮文字默认水平居中（微信与 HTML 端 .wx-button 均为 text-align:center）。
        // build_base_style 用继承值作为初值，若 CSS/内联未改写 text-align（仍等于继承值），
        // 说明页面没有显式指定，此时取居中而不是继承来的左对齐。
        if ns.text_align == ctx.inherited.align {
            ns.text_align = TextAlign::Center;
        }
        
        // 关键修复：button 是叶子节点（文本存在 node.text，taffy 无法据此推断内容宽）。
        // 当宽度为 auto 时，用「真实文本宽 + 左右内边距」作为 min-width，避免在 flex-row
        // 里塌缩成小圆块；同时保留 flex-column 下 align:stretch 撑满整行的默认全宽行为。
        if matches!(ts.size.width, Dimension::Auto) && !text.is_empty() {
            let pad_w = length_px(ts.padding.left) + length_px(ts.padding.right);
            let text_w = intrinsic_text_width(&text, ns.font_size * sf, ns.letter_spacing * sf);
            let min_w = text_w + pad_w + 2.0 * sf;
            let keep = match ts.min_size.width {
                Dimension::Length(px) => px.max(min_w),
                _ => min_w,
            };
            ts.min_size.width = length(keep);
        }
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "button".into(),
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
        Self::draw_with_state(node, canvas, text_renderer, x, y, w, h, sf, false);
    }
    
    pub fn draw_with_state(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        sf: f32,
        pressed: bool,
    ) {
        let style = &node.style;
        let disabled = node.attrs.get("disabled")
            .map(|s| s == "true" || s == "{{true}}")
            .unwrap_or(false);
        
        // 获取圆角值（支持四个角独立设置），并按盒子尺寸夹紧（含 border-radius:50% 情况）
        let [radius_tl, radius_tr, radius_br, radius_bl] = get_border_radii_clamped(style, w, h);
        let has_radius = radius_tl > 0.0 || radius_tr > 0.0 || radius_br > 0.0 || radius_bl > 0.0;
        let uniform_radius = radius_tl == radius_tr && radius_tr == radius_br && radius_br == radius_bl;
        
        // 绘制盒子阴影
        if let Some(shadow) = &style.box_shadow {
            draw_box_shadow(canvas, shadow, x, y, w, h, style.border_radius);
        }
        
        // 获取背景色，按下时变暗
        let bg = if let Some(bg) = style.background_color {
            if pressed && !disabled {
                Self::darken_color(bg, 0.1)
            } else {
                bg
            }
        } else {
            Color::WHITE
        };
        
        // 应用透明度
        let bg = if style.opacity < 1.0 {
            Color::new(bg.r, bg.g, bg.b, (bg.a as f32 * style.opacity) as u8)
        } else {
            bg
        };
        
        // 绘制背景
        let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
        if has_radius {
            let mut path = Path::new();
            if uniform_radius {
                path.add_round_rect(x, y, w, h, radius_tl);
            } else {
                path.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
            }
            canvas.draw_path(&path, &paint);
        } else {
            canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
        }
        
        // 绘制边框
        if style.border_width > 0.0 {
            if let Some(bc) = style.border_color {
                let border_color = if pressed && !disabled {
                    Self::darken_color(bc, 0.1)
                } else {
                    bc
                };
                let paint = Paint::new().with_color(border_color).with_style(PaintStyle::Stroke);
                if has_radius {
                    let mut path = Path::new();
                    if uniform_radius {
                        path.add_round_rect(x, y, w, h, radius_tl);
                    } else {
                        path.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
                    }
                    canvas.draw_path(&path, &paint);
                } else {
                    canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
                }
            }
        }
        
        // 按下时绘制半透明遮罩
        if pressed && !disabled {
            let overlay = Paint::new()
                .with_color(Color::new(0, 0, 0, 25))
                .with_style(PaintStyle::Fill);
            if has_radius {
                let mut path = Path::new();
                if uniform_radius {
                    path.add_round_rect(x, y, w, h, radius_tl);
                } else {
                    path.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
                }
                canvas.draw_path(&path, &overlay);
            } else {
                canvas.draw_rect(&GeoRect::new(x, y, w, h), &overlay);
            }
        }
        
        // 绘制文本
        if let Some(tr) = text_renderer {
            let color = style.text_color.unwrap_or(Color::WHITE);
            let size = style.font_size * sf;
            let tw = tr.measure_text(&node.text, size);
            
            // 根据 text-align 计算 x 位置
            let tx = match style.text_align {
                TextAlign::Center => x + (w - tw) / 2.0,
                TextAlign::Right => x + w - tw - 8.0 * sf, // 右边留点 padding
                TextAlign::Left | TextAlign::Justify => x + 8.0 * sf, // 左边留点 padding
            };
            
            // 垂直居中
            let ty = y + (h - size) / 2.0 + size;
            
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
            tr.draw_text(canvas, &node.text, tx, ty, size, &paint);
        }
    }
    
    /// 使颜色变暗
    fn darken_color(color: Color, amount: f32) -> Color {
        let factor = 1.0 - amount;
        Color::new(
            (color.r as f32 * factor) as u8,
            (color.g as f32 * factor) as u8,
            (color.b as f32 * factor) as u8,
            color.a,
        )
    }
}
