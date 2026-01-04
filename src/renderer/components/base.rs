//! 组件基础定义

use crate::parser::wxml::WxmlNode;
use crate::parser::wxss::{StyleSheet, StyleValue, LengthUnit, rpx_to_px};
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use std::collections::HashMap;
use taffy::prelude::*;

/// 渲染节点
#[derive(Clone)]
pub struct RenderNode {
    pub tag: String,
    pub text: String,
    pub attrs: HashMap<String, String>,
    pub taffy_node: NodeId,
    pub style: NodeStyle,
    pub children: Vec<RenderNode>,
    /// 事件绑定: (event_type, handler, data, is_catch)
    pub events: Vec<(String, String, HashMap<String, String>, bool)>,
}

/// 节点样式
#[derive(Clone, Default)]
pub struct NodeStyle {
    pub background_color: Option<Color>,
    pub text_color: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub border_radius: f32,
    pub border_radius_tl: Option<f32>,
    pub border_radius_tr: Option<f32>,
    pub border_radius_br: Option<f32>,
    pub border_radius_bl: Option<f32>,
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub opacity: f32,
    pub text_align: TextAlign,
    pub text_decoration: TextDecoration,
    pub line_height: Option<f32>,
    pub letter_spacing: f32,
    pub white_space: WhiteSpace,
    pub text_overflow: TextOverflow,
    pub overflow: Overflow,
    pub box_shadow: Option<BoxShadow>,
    pub transform: Option<Transform>,
    pub z_index: i32,
    pub vertical_align: VerticalAlign,
    pub word_break: WordBreak,
    /// 组件特定数据（如 progress 的百分比、switch 的选中状态等）
    pub custom_data: f32,
    /// 是否是 fixed 定位（相对于视口固定）
    pub is_fixed: bool,
    /// fixed 定位的 bottom 值
    pub fixed_bottom: Option<f32>,
    /// fixed 定位的 top 值
    pub fixed_top: Option<f32>,
    /// fixed 定位的 left 值
    pub fixed_left: Option<f32>,
    /// fixed 定位的 right 值
    pub fixed_right: Option<f32>,
    /// 是否是 block 显示（占满整行）
    pub is_block: bool,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum FontWeight {
    #[default]
    Normal,
    Bold,
    W100,
    W200,
    W300,
    W400,
    W500,
    W600,
    W700,
    W800,
    W900,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum TextDecoration {
    #[default]
    None,
    Underline,
    LineThrough,
    Overline,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum WhiteSpace {
    #[default]
    Normal,
    NoWrap,
    Pre,
    PreWrap,
    PreLine,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum TextOverflow {
    #[default]
    Clip,
    Ellipsis,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
    Auto,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum VerticalAlign {
    #[default]
    Baseline,
    Top,
    Middle,
    Bottom,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum WordBreak {
    #[default]
    Normal,
    BreakAll,
    KeepAll,
    BreakWord,
}

/// 盒子阴影
#[derive(Clone, Copy, Default)]
pub struct BoxShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Color,
    pub inset: bool,
}

/// 变换
#[derive(Clone, Copy, Default)]
pub struct Transform {
    pub translate_x: f32,
    pub translate_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotate: f32, // 角度
    pub skew_x: f32,
    pub skew_y: f32,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            scale_x: 1.0,
            scale_y: 1.0,
            ..Default::default()
        }
    }
}

/// 组件上下文
pub struct ComponentContext<'a> {
    pub scale_factor: f32,
    pub screen_width: f32,
    pub screen_height: f32,
    pub stylesheet: &'a StyleSheet,
    pub taffy: &'a mut TaffyTree,
}

/// 组件 trait
pub trait Component {
    fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode>;
    fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32);
}

/// 解析颜色字符串
pub fn parse_color_str(s: &str) -> Option<Color> {
    let s = s.trim();
    if s.starts_with('#') {
        let hex = s.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::new(r, g, b, 255));
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            return Some(Color::new(r, g, b, 255));
        }
    } else if s.starts_with("rgb") {
        let nums: Vec<u8> = s.chars()
            .filter(|c| c.is_ascii_digit() || *c == ',')
            .collect::<String>()
            .split(',')
            .filter_map(|n| n.trim().parse().ok())
            .collect();
        if nums.len() >= 3 {
            return Some(Color::new(nums[0], nums[1], nums[2], 255));
        }
    }
    None
}

/// 提取事件绑定
/// 返回 (event_type, handler, data, is_catch)
pub fn extract_events(node: &WxmlNode) -> Vec<(String, String, HashMap<String, String>, bool)> {
    let mut events = vec![];
    for attr in ["bindtap", "catchtap", "bindchange", "bindinput", "bindblur", "bindfocus", "bindconfirm", "bindlinechange"] {
        if let Some(h) = node.get_attr(attr) {
            let mut d = HashMap::new();
            // 添加 data-* 属性
            for (k, v) in &node.attributes {
                if k.starts_with("data-") { 
                    d.insert(k[5..].into(), v.clone()); 
                }
            }
            // 对于 input/textarea，添加相关属性到事件数据
            if node.tag_name == "input" || node.tag_name == "textarea" {
                if let Some(v) = node.get_attr("maxlength") {
                    d.insert("maxlength".into(), v.into());
                }
                if let Some(v) = node.get_attr("type") {
                    d.insert("type".into(), v.into());
                }
                if let Some(v) = node.get_attr("password") {
                    d.insert("password".into(), v.into());
                }
            }
            let is_catch = attr.starts_with("catch");
            let event_type = attr.trim_start_matches("bind").trim_start_matches("catch");
            events.push((event_type.into(), h.into(), d, is_catch));
        }
    }
    events
}

/// 获取节点的 class 列表
pub fn get_classes(node: &WxmlNode) -> Vec<&str> {
    node.get_attr("class").map(|s| s.split_whitespace().collect()).unwrap_or_default()
}

/// 获取节点的文本内容
pub fn get_text_content(node: &WxmlNode) -> String {
    use crate::parser::wxml::WxmlNodeType;
    let mut s = String::new();
    for c in &node.children {
        if c.node_type == WxmlNodeType::Text { 
            s.push_str(&c.text_content); 
        } else { 
            s.push_str(&get_text_content(c)); 
        }
    }
    s.trim().into()
}

/// 将 StyleValue 转换为像素值
pub fn to_px(v: &StyleValue, screen_width: f32, screen_height: f32) -> Option<f32> {
    match v {
        StyleValue::Length(n, u) => Some(match u {
            LengthUnit::Px => *n,
            LengthUnit::Rpx => rpx_to_px(*n, screen_width),
            LengthUnit::Percent => *n / 100.0 * screen_width,
            LengthUnit::Em | LengthUnit::Rem => *n * 16.0,
            LengthUnit::Vw => *n / 100.0 * screen_width,
            LengthUnit::Vh => *n / 100.0 * screen_height,
        }),
        StyleValue::Number(n) => Some(*n),
        _ => None,
    }
}

/// 将 StyleValue 转换为 Dimension
pub fn to_dimension(v: &StyleValue, screen_width: f32, screen_height: f32, sf: f32) -> Option<Dimension> {
    match v {
        StyleValue::Auto => Some(Dimension::Auto),
        StyleValue::Length(n, LengthUnit::Percent) => Some(percent(*n / 100.0)),
        _ => to_px(v, screen_width, screen_height).map(|px| length(px * sf)),
    }
}

/// 构建基础 Taffy 样式
pub fn build_base_style(
    node: &WxmlNode,
    ctx: &mut ComponentContext,
) -> (Style, NodeStyle) {
    let classes = get_classes(node);
    let css = ctx.stylesheet.get_styles(&classes, &node.tag_name);
    
    let mut ns = NodeStyle { font_size: 14.0, opacity: 1.0, ..Default::default() };
    
    // 默认样式：flex 布局，列方向
    let mut ts = Style { 
        display: Display::Flex, 
        flex_direction: FlexDirection::Column,
        ..Default::default() 
    };

    // 应用类样式
    for (name, value) in &css {
        apply_style_property(name, value, &mut ts, &mut ns, ctx);
    }

    // 应用内联样式
    if let Some(style_str) = node.get_attr("style") {
        for part in style_str.split(';') {
            let part = part.trim();
            if part.is_empty() { continue; }
            
            if let Some(colon_pos) = part.find(':') {
                let name = part[..colon_pos].trim();
                let value_str = part[colon_pos + 1..].trim();
                let value = parse_inline_value(value_str);
                apply_style_property(name, &value, &mut ts, &mut ns, ctx);
            }
        }
    }
    
    (ts, ns)
}

/// 解析内联样式值
fn parse_inline_value(value: &str) -> StyleValue {
    let value = value.trim();
    if value.ends_with("rpx") {
        if let Ok(n) = value.trim_end_matches("rpx").parse() { return StyleValue::Length(n, LengthUnit::Rpx); }
    }
    if value.ends_with("px") {
        if let Ok(n) = value.trim_end_matches("px").parse() { return StyleValue::Length(n, LengthUnit::Px); }
    }
    if value.ends_with("%") {
        if let Ok(n) = value.trim_end_matches("%").parse() { return StyleValue::Length(n, LengthUnit::Percent); }
    }
    if let Ok(n) = value.parse() { return StyleValue::Number(n); }
    
    // Color
    if value.starts_with('#') || value.starts_with("rgb") {
        if let Some(c) = parse_color_str(value) { return StyleValue::Color(c); }
    }
    
    StyleValue::String(value.to_string())
}

/// 应用单个样式属性
fn apply_style_property(
    name: &str,
    value: &StyleValue,
    ts: &mut Style,
    ns: &mut NodeStyle,
    ctx: &mut ComponentContext
) {
    let sf = ctx.scale_factor;
    match name {
            "width" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.size.width = v; }
            "height" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.size.height = v; }
            "min-width" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.min_size.width = v; }
            "min-height" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.min_size.height = v; }
            "max-width" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.max_size.width = v; }
            "max-height" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.max_size.height = v; }
            "padding" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) {
                let sv = v * sf;
                ts.padding = Rect { top: length(sv), right: length(sv), bottom: length(sv), left: length(sv) };
            }
            "padding-top" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.padding.top = length(v * sf); }
            "padding-right" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.padding.right = length(v * sf); }
            "padding-bottom" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.padding.bottom = length(v * sf); }
            "padding-left" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.padding.left = length(v * sf); }
            "margin" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) {
                let sv = v * sf;
                ts.margin = Rect { top: length(sv), right: length(sv), bottom: length(sv), left: length(sv) };
            }
            "margin-top" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.margin.top = length(v * sf); }
            "margin-right" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.margin.right = length(v * sf); }
            "margin-bottom" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.margin.bottom = length(v * sf); }
            "margin-left" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.margin.left = length(v * sf); }
            "display" => if let StyleValue::String(s) = value {
                match s.as_str() {
                    "none" => ts.display = Display::None,
                    "block" => {
                        ts.display = Display::Flex;
                        ns.is_block = true;
                    }
                    "flex" => ts.display = Display::Flex,
                    "grid" => ts.display = Display::Grid,
                    _ => ts.display = Display::Flex,
                };
            }
            "flex-direction" => if let StyleValue::String(s) = value {
                ts.flex_direction = match s.as_str() {
                    "row" => FlexDirection::Row,
                    "row-reverse" => FlexDirection::RowReverse,
                    "column-reverse" => FlexDirection::ColumnReverse,
                    "column" => FlexDirection::Column,
                    _ => FlexDirection::Column,
                };
            }
            "flex-wrap" => if let StyleValue::String(s) = value {
                ts.flex_wrap = match s.as_str() {
                    "wrap" => FlexWrap::Wrap,
                    "wrap-reverse" => FlexWrap::WrapReverse,
                    _ => FlexWrap::NoWrap,
                };
            }
            "flex-grow" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.flex_grow = v; }
            "flex" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { 
                ts.flex_grow = v;
                // flex: <number> implies flex-grow: <number>, flex-shrink: 1, flex-basis: 0
                ts.flex_shrink = 1.0;
                ts.flex_basis = Dimension::Length(0.0);
            }
            "flex-shrink" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.flex_shrink = v; }
            "flex-basis" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.flex_basis = v; }
            "justify-content" => if let StyleValue::String(s) = value {
                ts.justify_content = Some(match s.as_str() {
                    "center" => JustifyContent::Center,
                    "space-between" => JustifyContent::SpaceBetween,
                    "space-around" => JustifyContent::SpaceAround,
                    "space-evenly" => JustifyContent::SpaceEvenly,
                    "flex-end" | "end" => JustifyContent::FlexEnd,
                    "flex-start" | "start" => JustifyContent::FlexStart,
                    _ => JustifyContent::FlexStart,
                });
            }
            "align-items" => if let StyleValue::String(s) = value {
                let align = match s.as_str() {
                    "center" => AlignItems::Center,
                    "flex-end" | "end" => AlignItems::FlexEnd,
                    "flex-start" | "start" => AlignItems::FlexStart,
                    "stretch" => AlignItems::Stretch,
                    "baseline" => AlignItems::Baseline,
                    _ => AlignItems::FlexStart,
                };
                ts.align_items = Some(align);
            }
            "align-self" => if let StyleValue::String(s) = value {
                ts.align_self = Some(match s.as_str() {
                    "center" => AlignSelf::Center,
                    "flex-end" | "end" => AlignSelf::FlexEnd,
                    "flex-start" | "start" => AlignSelf::FlexStart,
                    "stretch" => AlignSelf::Stretch,
                    "baseline" => AlignSelf::Baseline,
                    _ => AlignSelf::Start,
                });
            }
            "align-content" => if let StyleValue::String(s) = value {
                ts.align_content = Some(match s.as_str() {
                    "center" => AlignContent::Center,
                    "flex-end" | "end" => AlignContent::FlexEnd,
                    "flex-start" | "start" => AlignContent::FlexStart,
                    "stretch" => AlignContent::Stretch,
                    "space-between" => AlignContent::SpaceBetween,
                    "space-around" => AlignContent::SpaceAround,
                    "space-evenly" => AlignContent::SpaceEvenly,
                    _ => AlignContent::FlexStart,
                });
            }
            "gap" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { 
                let sv = v * sf;
                ts.gap = Size { width: length(sv), height: length(sv) }; 
            }
            "row-gap" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.gap.height = length(v * sf); }
            "column-gap" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.gap.width = length(v * sf); }
            "background-color" | "background" => {
                if let StyleValue::Color(c) = value { 
                    ns.background_color = Some(*c); 
                } else if let StyleValue::String(s) = value {
                    // 尝试从字符串解析颜色
                    if let Some(c) = parse_color_str(s) {
                        ns.background_color = Some(c);
                    }
                }
            }
            "color" => {
                if let StyleValue::Color(c) = value { 
                    ns.text_color = Some(*c); 
                } else if let StyleValue::String(s) = value {
                    if let Some(c) = parse_color_str(s) {
                        ns.text_color = Some(c);
                    }
                }
            }
            "border-color" => {
                if let StyleValue::Color(c) = value { 
                    ns.border_color = Some(*c); 
                } else if let StyleValue::String(s) = value {
                    if let Some(c) = parse_color_str(s) {
                        ns.border_color = Some(c);
                    }
                }
            }
            "border-width" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_width = v * sf; }
            "border-radius" => {
                // 支持 border-radius 简写：1-4 个值
                if let StyleValue::String(s) = value {
                    let parts: Vec<&str> = s.split_whitespace().collect();
                    let values: Vec<f32> = parts.iter()
                        .filter_map(|p| {
                            let sv = parse_inline_value(p);
                            to_px(&sv, ctx.screen_width, ctx.screen_height)
                        })
                        .map(|v| v * sf)
                        .collect();
                    
                    match values.len() {
                        1 => {
                            ns.border_radius = values[0];
                        }
                        2 => {
                            // top-left/bottom-right, top-right/bottom-left
                            ns.border_radius_tl = Some(values[0]);
                            ns.border_radius_br = Some(values[0]);
                            ns.border_radius_tr = Some(values[1]);
                            ns.border_radius_bl = Some(values[1]);
                        }
                        3 => {
                            // top-left, top-right/bottom-left, bottom-right
                            ns.border_radius_tl = Some(values[0]);
                            ns.border_radius_tr = Some(values[1]);
                            ns.border_radius_bl = Some(values[1]);
                            ns.border_radius_br = Some(values[2]);
                        }
                        4 => {
                            // top-left, top-right, bottom-right, bottom-left
                            ns.border_radius_tl = Some(values[0]);
                            ns.border_radius_tr = Some(values[1]);
                            ns.border_radius_br = Some(values[2]);
                            ns.border_radius_bl = Some(values[3]);
                        }
                        _ => {}
                    }
                } else if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) {
                    ns.border_radius = v * sf;
                }
            }
            "border-top-left-radius" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_radius_tl = Some(v * sf); }
            "border-top-right-radius" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_radius_tr = Some(v * sf); }
            "border-bottom-right-radius" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_radius_br = Some(v * sf); }
            "border-bottom-left-radius" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_radius_bl = Some(v * sf); }
            "border" => {
                // border: 1px solid #000
                if let StyleValue::String(s) = value {
                    parse_border_shorthand(s, ns, ctx.screen_width, sf);
                }
            }
            "font-size" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.font_size = v; }
            "font-weight" => if let StyleValue::String(s) = value {
                ns.font_weight = match s.as_str() {
                    "100" => FontWeight::W100,
                    "200" => FontWeight::W200,
                    "300" | "light" => FontWeight::W300,
                    "400" | "normal" => FontWeight::Normal,
                    "500" | "medium" => FontWeight::W500,
                    "600" | "semibold" => FontWeight::W600,
                    "700" | "bold" => FontWeight::Bold,
                    "800" => FontWeight::W800,
                    "900" | "black" => FontWeight::W900,
                    _ => FontWeight::Normal,
                };
            }
            "text-align" => if let StyleValue::String(s) = value {
                ns.text_align = match s.as_str() {
                    "center" => TextAlign::Center,
                    "right" => TextAlign::Right,
                    "justify" => TextAlign::Justify,
                    _ => TextAlign::Left,
                };
            }
            "text-decoration" | "text-decoration-line" => if let StyleValue::String(s) = value {
                ns.text_decoration = match s.as_str() {
                    "underline" => TextDecoration::Underline,
                    "line-through" => TextDecoration::LineThrough,
                    "overline" => TextDecoration::Overline,
                    _ => TextDecoration::None,
                };
            }
            "line-height" => {
                if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { 
                    ns.line_height = Some(v); 
                } else if let StyleValue::Number(n) = value {
                    ns.line_height = Some(ns.font_size * n);
                }
            }
            "letter-spacing" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.letter_spacing = v; }
            "white-space" => if let StyleValue::String(s) = value {
                ns.white_space = match s.as_str() {
                    "nowrap" => WhiteSpace::NoWrap,
                    "pre" => WhiteSpace::Pre,
                    "pre-wrap" => WhiteSpace::PreWrap,
                    "pre-line" => WhiteSpace::PreLine,
                    _ => WhiteSpace::Normal,
                };
            }
            "text-overflow" => if let StyleValue::String(s) = value {
                ns.text_overflow = match s.as_str() {
                    "ellipsis" => TextOverflow::Ellipsis,
                    _ => TextOverflow::Clip,
                };
            }
            "overflow" | "overflow-x" | "overflow-y" => if let StyleValue::String(s) = value {
                ns.overflow = match s.as_str() {
                    "hidden" => Overflow::Hidden,
                    "scroll" => Overflow::Scroll,
                    "auto" => Overflow::Auto,
                    _ => Overflow::Visible,
                };
                let taffy_overflow = match s.as_str() {
                    "hidden" | "scroll" | "auto" => taffy::style::Overflow::Hidden,
                    _ => taffy::style::Overflow::Visible,
                };
                ts.overflow.x = taffy_overflow;
                ts.overflow.y = taffy_overflow;
            }
            "vertical-align" => if let StyleValue::String(s) = value {
                ns.vertical_align = match s.as_str() {
                    "top" => VerticalAlign::Top,
                    "middle" => VerticalAlign::Middle,
                    "bottom" => VerticalAlign::Bottom,
                    _ => VerticalAlign::Baseline,
                };
            }
            "word-break" => if let StyleValue::String(s) = value {
                ns.word_break = match s.as_str() {
                    "break-all" => WordBreak::BreakAll,
                    "keep-all" => WordBreak::KeepAll,
                    "break-word" => WordBreak::BreakWord,
                    _ => WordBreak::Normal,
                };
            }
            "z-index" => if let StyleValue::Number(n) = value { ns.z_index = *n as i32; }
            "opacity" => if let StyleValue::Number(n) = value { ns.opacity = *n; }
            "box-shadow" => if let StyleValue::String(s) = value {
                if let Some(shadow) = parse_box_shadow(s, ctx.screen_width) {
                    ns.box_shadow = Some(shadow);
                }
            }
            "transform" => if let StyleValue::String(s) = value {
                if let Some(transform) = parse_transform(s) {
                    ns.transform = Some(transform);
                }
            }
            "position" => if let StyleValue::String(s) = value {
                match s.as_str() {
                    "absolute" => {
                        ts.position = Position::Absolute;
                    }
                    "fixed" => {
                        // fixed 定位：使用 absolute 让 Taffy 处理，但标记为 fixed
                        ts.position = Position::Absolute;
                        ns.is_fixed = true;
                    }
                    "relative" => {
                        ts.position = Position::Relative;
                    }
                    _ => ts.position = Position::Relative,
                };
            }
            "top" => {
                if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { 
                    ts.inset.top = LengthPercentageAuto::Length(v * sf);
                    ns.fixed_top = Some(v * sf);
                } else if let StyleValue::Number(n) = value {
                    // 处理纯数字（如 top: 0）
                    ts.inset.top = LengthPercentageAuto::Length(*n * sf);
                    ns.fixed_top = Some(*n * sf);
                }
            }
            "left" => {
                if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { 
                    ts.inset.left = LengthPercentageAuto::Length(v * sf);
                    ns.fixed_left = Some(v * sf);
                } else if let StyleValue::Number(n) = value {
                    ts.inset.left = LengthPercentageAuto::Length(*n * sf);
                    ns.fixed_left = Some(*n * sf);
                }
            }
            "right" => {
                if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { 
                    ts.inset.right = LengthPercentageAuto::Length(v * sf);
                    ns.fixed_right = Some(v * sf);
                } else if let StyleValue::Number(n) = value {
                    ts.inset.right = LengthPercentageAuto::Length(*n * sf);
                    ns.fixed_right = Some(*n * sf);
                }
            }
            "bottom" => {
                if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { 
                    ts.inset.bottom = LengthPercentageAuto::Length(v * sf);
                    ns.fixed_bottom = Some(v * sf);
                } else if let StyleValue::Number(n) = value {
                    ts.inset.bottom = LengthPercentageAuto::Length(*n * sf);
                    ns.fixed_bottom = Some(*n * sf);
                }
            }
            _ => {}
    }
}

/// 解析 box-shadow
fn parse_box_shadow(s: &str, screen_width: f32) -> Option<BoxShadow> {
    let s = s.trim();
    if s == "none" { return None; }
    
    let mut shadow = BoxShadow::default();
    shadow.color = Color::new(0, 0, 0, 128);
    
    let parts: Vec<&str> = s.split_whitespace().collect();
    let mut num_idx = 0;
    
    for part in parts {
        if part == "inset" {
            shadow.inset = true;
        } else if part.starts_with('#') || part.starts_with("rgb") {
            if let Some(color) = parse_color_str(part) {
                shadow.color = color;
            }
        } else if let Some((num, unit)) = parse_length_simple(part) {
            let px = match unit {
                "rpx" => num * screen_width / 750.0,
                _ => num,
            };
            match num_idx {
                0 => shadow.offset_x = px,
                1 => shadow.offset_y = px,
                2 => shadow.blur = px,
                3 => shadow.spread = px,
                _ => {}
            }
            num_idx += 1;
        }
    }
    
    Some(shadow)
}

/// 解析 transform
fn parse_transform(s: &str) -> Option<Transform> {
    let mut transform = Transform::new();
    let mut remaining = s.trim();
    
    while !remaining.is_empty() {
        if let Some(paren_start) = remaining.find('(') {
            let func_name = remaining[..paren_start].trim();
            if let Some(paren_end) = remaining.find(')') {
                let args = &remaining[paren_start + 1..paren_end];
                
                match func_name {
                    "translate" | "translateX" | "translateY" => {
                        let values: Vec<f32> = args.split(',')
                            .filter_map(|v| parse_length_simple(v.trim()).map(|(n, _)| n))
                            .collect();
                        if func_name == "translateX" && !values.is_empty() {
                            transform.translate_x = values[0];
                        } else if func_name == "translateY" && !values.is_empty() {
                            transform.translate_y = values[0];
                        } else if !values.is_empty() {
                            transform.translate_x = values[0];
                            if values.len() > 1 { transform.translate_y = values[1]; }
                        }
                    }
                    "scale" | "scaleX" | "scaleY" => {
                        let values: Vec<f32> = args.split(',')
                            .filter_map(|v| v.trim().parse().ok())
                            .collect();
                        if func_name == "scaleX" && !values.is_empty() {
                            transform.scale_x = values[0];
                        } else if func_name == "scaleY" && !values.is_empty() {
                            transform.scale_y = values[0];
                        } else if !values.is_empty() {
                            transform.scale_x = values[0];
                            transform.scale_y = if values.len() > 1 { values[1] } else { values[0] };
                        }
                    }
                    "rotate" => {
                        let angle = args.trim().trim_end_matches("deg");
                        if let Ok(deg) = angle.parse::<f32>() { transform.rotate = deg; }
                    }
                    "skew" | "skewX" | "skewY" => {
                        let values: Vec<f32> = args.split(',')
                            .filter_map(|v| v.trim().trim_end_matches("deg").parse().ok())
                            .collect();
                        if func_name == "skewX" && !values.is_empty() {
                            transform.skew_x = values[0];
                        } else if func_name == "skewY" && !values.is_empty() {
                            transform.skew_y = values[0];
                        } else if !values.is_empty() {
                            transform.skew_x = values[0];
                            if values.len() > 1 { transform.skew_y = values[1]; }
                        }
                    }
                    _ => {}
                }
                remaining = remaining[paren_end + 1..].trim();
            } else { break; }
        } else { break; }
    }
    Some(transform)
}

/// 解析 border 简写
fn parse_border_shorthand(s: &str, ns: &mut NodeStyle, screen_width: f32, sf: f32) {
    for part in s.split_whitespace() {
        if part.starts_with('#') || part.starts_with("rgb") {
            if let Some(color) = parse_color_str(part) { ns.border_color = Some(color); }
        } else if let Some((num, unit)) = parse_length_simple(part) {
            let px = match unit { "rpx" => num * screen_width / 750.0, _ => num };
            ns.border_width = px * sf;
        }
    }
}

/// 简单长度解析
fn parse_length_simple(s: &str) -> Option<(f32, &str)> {
    let s = s.trim();
    for unit in ["rpx", "px", "em", "rem", "vh", "vw", "%"] {
        if s.ends_with(unit) {
            if let Ok(num) = s.trim_end_matches(unit).parse::<f32>() {
                return Some((num, unit));
            }
        }
    }
    s.parse::<f32>().ok().map(|n| (n, "px"))
}

/// 绘制盒子阴影
pub fn draw_box_shadow(canvas: &mut Canvas, shadow: &BoxShadow, x: f32, y: f32, w: f32, h: f32, border_radius: f32) {
    if shadow.inset {
        return; // 暂不支持内阴影
    }
    
    let shadow_x = x + shadow.offset_x;
    let shadow_y = y + shadow.offset_y;
    let shadow_w = w + shadow.spread * 2.0;
    let shadow_h = h + shadow.spread * 2.0;
    let adjusted_x = shadow_x - shadow.spread;
    let adjusted_y = shadow_y - shadow.spread;
    
    // 简化的阴影绘制：使用多层半透明矩形模拟模糊
    let blur_steps = (shadow.blur / 2.0).max(1.0) as i32;
    let base_alpha = shadow.color.a as f32 / blur_steps as f32;
    
    for i in 0..blur_steps {
        let expand = i as f32 * 2.0;
        let alpha = (base_alpha * (1.0 - i as f32 / blur_steps as f32)) as u8;
        if alpha == 0 { continue; }
        
        let color = Color::new(shadow.color.r, shadow.color.g, shadow.color.b, alpha);
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
        
        let sx = adjusted_x - expand;
        let sy = adjusted_y - expand;
        let sw = shadow_w + expand * 2.0;
        let sh = shadow_h + expand * 2.0;
        
        if border_radius > 0.0 {
            let mut path = Path::new();
            path.add_round_rect(sx, sy, sw, sh, border_radius + expand);
            canvas.draw_path(&path, &paint);
        } else {
            canvas.draw_rect(&GeoRect::new(sx, sy, sw, sh), &paint);
        }
    }
}

/// 获取有效的边框圆角
pub fn get_border_radii(style: &NodeStyle) -> [f32; 4] {
    [
        style.border_radius_tl.unwrap_or(style.border_radius),
        style.border_radius_tr.unwrap_or(style.border_radius),
        style.border_radius_br.unwrap_or(style.border_radius),
        style.border_radius_bl.unwrap_or(style.border_radius),
    ]
}

/// 绘制背景和边框
pub fn draw_background(canvas: &mut Canvas, style: &NodeStyle, x: f32, y: f32, w: f32, h: f32) {
    // 绘制阴影（在背景之前）
    if let Some(shadow) = &style.box_shadow {
        draw_box_shadow(canvas, shadow, x, y, w, h, style.border_radius);
    }
    
    let radii = get_border_radii(style);
    let has_different_radii = radii[0] != radii[1] || radii[1] != radii[2] || radii[2] != radii[3];
    
    // 绘制背景
    if let Some(bg) = style.background_color {
        let mut paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
        if style.opacity < 1.0 { 
            paint.color.a = (paint.color.a as f32 * style.opacity) as u8; 
        }
        
        if has_different_radii {
            let mut path = Path::new();
            add_round_rect_with_radii(&mut path, x, y, w, h, radii);
            canvas.draw_path(&path, &paint);
        } else if style.border_radius > 0.0 {
            let mut path = Path::new();
            path.add_round_rect(x, y, w, h, style.border_radius);
            canvas.draw_path(&path, &paint);
        } else {
            canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
        }
    }
    
    // 绘制边框
    if style.border_width > 0.0 {
        if let Some(bc) = style.border_color {
            let paint = Paint::new().with_color(bc).with_style(PaintStyle::Stroke);
            if has_different_radii {
                let mut path = Path::new();
                add_round_rect_with_radii(&mut path, x, y, w, h, radii);
                canvas.draw_path(&path, &paint);
            } else if style.border_radius > 0.0 {
                let mut path = Path::new();
                path.add_round_rect(x, y, w, h, style.border_radius);
                canvas.draw_path(&path, &paint);
            } else {
                canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
            }
        }
    }
}

/// 添加带有不同圆角的圆角矩形路径
fn add_round_rect_with_radii(path: &mut Path, x: f32, y: f32, w: f32, h: f32, radii: [f32; 4]) {
    let [tl, tr, br, bl] = radii;
    
    // 从左上角开始，顺时针绘制
    path.move_to(x + tl, y);
    
    // 上边 + 右上角
    path.line_to(x + w - tr, y);
    if tr > 0.0 {
        path.quad_to(x + w, y, x + w, y + tr);
    }
    
    // 右边 + 右下角
    path.line_to(x + w, y + h - br);
    if br > 0.0 {
        path.quad_to(x + w, y + h, x + w - br, y + h);
    }
    
    // 下边 + 左下角
    path.line_to(x + bl, y + h);
    if bl > 0.0 {
        path.quad_to(x, y + h, x, y + h - bl);
    }
    
    // 左边 + 左上角
    path.line_to(x, y + tl);
    if tl > 0.0 {
        path.quad_to(x, y, x + tl, y);
    }
    
    path.close();
}
