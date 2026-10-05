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
        
        // `<button>` 里写了元素子节点（典型 `<button><text class="…">…</text></button>`）时，
        // 文字由子树自己按各自的 CSS 画（见 layout.rs 的 button_as_container）。
        // 这里就不能再把收集到的文本当按钮标签用了 —— 否则同一段文字会被按钮
        // 用「继承来的」样式画一遍，页面写在子 text 上的颜色/字号全看不到。
        let has_element_children = node
            .children
            .iter()
            .any(|c| c.node_type == crate::parser::wxml::WxmlNodeType::Element);
        let text = if has_element_children { String::new() } else { get_text_content(node) };
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
        let has_custom_width = !dim_is_auto(ts.size.width);
        let has_custom_height = !dim_is_auto(ts.size.height);
        let has_custom_padding = !(length_px(ts.padding.top) == 0.0) ||
                                  !(length_px(ts.padding.left) == 0.0);
        
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
        // 宽度：交给父容器的对齐语义决定，只保证不小于内容宽。
        //
        // 微信里 button 是 display:block、width:auto —— 在普通块/列容器里（align-items 默认
        // stretch）会撑满一行，但父级写了 `align-items:center` 时就只有内容宽。之前这里
        // 无条件写 `width:100%`，于是「空购物车」那种居中容器里的按钮被拉成整行，和 H5 不一致。
        if !has_custom_width {
            let tw = intrinsic_text_width(&text, ns.font_size * sf, ns.letter_spacing * sf);
            let content_width = tw + padding_h * 2.0 * sf;
            if btn_size == "mini" {
                ts.size.width = length(content_width);
            } else {
                ts.size.width = Dimension::auto();
                ts.min_size.width = length(content_width);
            }
        }
        // 高度：页面没写 height 时给出「微信默认按钮高度」。
        //
        // 这里用显式 height 而不是 min-height：按钮是叶子节点，taffy 无法从内容推出
        // 高度（内容高按 0 处理），若只给 min-height，flex 基准尺寸 0 与最小高度之间的
        // 差值会被当作溢出，进而把同一列 flex 容器里的兄弟节点（如 icon）压缩变形。
        // 单行标签下 height == max(默认高, 字号+上下内边距)，与 HTML 端 min-height 等价。
        if !has_custom_height {
            let content_min = (ns.font_size + padding_v * 2.0) * sf;
            ts.size.height = length(content_min.max(min_height * sf));
        }
        
        // 不给 button 任何默认 margin。
        //
        // 微信与 HTML 端（`*{margin:0}` + `.wx-button` 无 margin）都没有默认外边距，
        // 这里原先无条件写入 `margin:5px 0`：一来每个按钮凭空多出 10px 垂直间距，
        // 二来判定「是否自定义」只看 margin.top，页面写 `margin-left:auto`（靠右对齐）
        // 时整个 margin 会被这段默认值覆盖掉，按钮永远靠不到右边。
        
        // 默认居中对齐
        if ts.align_items.is_none() {
            ts.align_items = Some(AlignItems::CENTER);
        }
        if ts.justify_content.is_none() {
            ts.justify_content = Some(JustifyContent::CENTER);
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
        if dim_is_auto(ts.size.width) && !text.is_empty() {
            let pad_w = length_px(ts.padding.left) + length_px(ts.padding.right);
            let text_w = intrinsic_text_width(&text, ns.font_size * sf, ns.letter_spacing * sf);
            let min_w = text_w + pad_w + 2.0 * sf;
            let keep = dim_length(ts.min_size.width).map(|px| px.max(min_w)).unwrap_or(min_w);
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
        _pressed: bool,
    ) {
        let style = &node.style;
        // 点击态不再在这里另外压暗、盖一层黑。微信的按压就是 `hover-class`
        // （默认 `button-hover`：整颗按钮 opacity 0.7），由按压样式和组合成负责。
        let _disabled = node.attrs.get("disabled")
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
        
        let bg = style.background_color.unwrap_or(Color::WHITE);
        
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
                let border_color = bc;
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
}
