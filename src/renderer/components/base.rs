//! 组件基础定义

use crate::parser::wxml::WxmlNode;
use crate::parser::wxss::{StyleSheet, StyleValue, LengthUnit, rpx_to_px, ElementDesc};
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
    /// 元素自身内边距（逻辑像素），供文本组件在内容区内定位文字
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,
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

/// 可继承的文本样式（CSS 继承语义：color / font-size / font-weight / text-align /
/// line-height / letter-spacing 会从父元素传递给子元素，直到被显式覆盖）。
#[derive(Clone)]
pub struct InheritedText {
    pub font_size: f32,
    pub color: Option<Color>,
    pub weight: FontWeight,
    pub align: TextAlign,
    pub line_height: Option<f32>,
    pub letter_spacing: f32,
}

impl Default for InheritedText {
    fn default() -> Self {
        Self {
            font_size: 16.0, // 根默认字号（对齐移动端常见默认 16px）
            color: None,
            weight: FontWeight::Normal,
            align: TextAlign::Left,
            line_height: None,
            letter_spacing: 0.0,
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
    /// 祖先元素链（根 -> 父），用于后代/子选择器匹配。缺省为空。
    pub ancestors: Vec<ElementDesc>,
    /// 从父元素继承下来的文本样式。
    pub inherited: InheritedText,
}

/// 组件 trait
pub trait Component {
    fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode>;
    fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32);
}

// CSS 取值解析（parse_color_str / parse_box_shadow / parse_transform /
// parse_border_shorthand / parse_length_simple）已拆分到 style_parse 模块，
// 这里重新导出以保持 `use super::base::*` 的调用点不变。
pub use super::style_parse::{
    parse_color_str, parse_box_shadow, parse_transform, parse_border_shorthand, parse_length_simple,
};

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
    let id = node.get_attr("id");
    // 构建「祖先链 + 当前元素」，支持 #id、[attr]、*、后代/子选择器
    let mut chain = ctx.ancestors.clone();
    chain.push(ElementDesc::new(&node.tag_name, id, &classes, &node.attributes));
    let css = ctx.stylesheet.get_styles_chain(&chain);
    
    // 先用继承的文本样式做默认值，再用 CSS/内联覆盖（CSS 继承语义）
    let mut ns = NodeStyle {
        font_size: ctx.inherited.font_size,
        text_color: ctx.inherited.color,
        font_weight: ctx.inherited.weight,
        text_align: ctx.inherited.align,
        line_height: ctx.inherited.line_height,
        letter_spacing: ctx.inherited.letter_spacing,
        opacity: 1.0,
        ..Default::default()
    };
    
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

/// 解析盒模型简写（margin/padding），返回 [top, right, bottom, left]（像素，未乘 sf）。
///
/// 支持 CSS 1-4 值语法：
/// - 1 值：四边相同
/// - 2 值：上下 / 左右
/// - 3 值：上 / 左右 / 下
/// - 4 值：上 / 右 / 下 / 左
///
/// 之前只处理单值，`padding: 10rpx 20rpx` 这类会被静默丢弃。
fn box_sides_px(value: &StyleValue, ctx: &ComponentContext) -> Option<[f32; 4]> {
    match value {
        StyleValue::String(s) => {
            let vals: Vec<f32> = s
                .split_whitespace()
                .filter_map(|p| to_px(&parse_inline_value(p), ctx.screen_width, ctx.screen_height))
                .collect();
            match vals.len() {
                1 => Some([vals[0], vals[0], vals[0], vals[0]]),
                2 => Some([vals[0], vals[1], vals[0], vals[1]]),
                3 => Some([vals[0], vals[1], vals[2], vals[1]]),
                4 => Some([vals[0], vals[1], vals[2], vals[3]]),
                _ => None,
            }
        }
        _ => to_px(value, ctx.screen_width, ctx.screen_height).map(|v| [v, v, v, v]),
    }
}

/// 解析内联样式值
fn parse_inline_value(value: &str) -> StyleValue {
    let value = value.trim();
    if value == "auto" { return StyleValue::Auto; }
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
            "padding" => if let Some([t, r, b, l]) = box_sides_px(value, ctx) {
                ts.padding = Rect {
                    top: length(t * sf), right: length(r * sf),
                    bottom: length(b * sf), left: length(l * sf),
                };
                ns.padding_top = t; ns.padding_right = r; ns.padding_bottom = b; ns.padding_left = l;
            }
            "padding-top" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.padding.top = length(v * sf); ns.padding_top = v; }
            "padding-right" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.padding.right = length(v * sf); ns.padding_right = v; }
            "padding-bottom" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.padding.bottom = length(v * sf); ns.padding_bottom = v; }
            "padding-left" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.padding.left = length(v * sf); ns.padding_left = v; }
            "margin" => {
                if matches!(value, StyleValue::Auto) {
                    // margin: auto -> 居中
                    ts.margin = Rect { top: auto(), right: auto(), bottom: auto(), left: auto() };
                } else if let Some([t, r, b, l]) = box_sides_px(value, ctx) {
                    ts.margin = Rect {
                        top: length(t * sf), right: length(r * sf),
                        bottom: length(b * sf), left: length(l * sf),
                    };
                }
            }
            "margin-top" => if matches!(value, StyleValue::Auto) { ts.margin.top = auto(); } else if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.margin.top = length(v * sf); }
            "margin-right" => if matches!(value, StyleValue::Auto) { ts.margin.right = auto(); } else if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.margin.right = length(v * sf); }
            "margin-bottom" => if matches!(value, StyleValue::Auto) { ts.margin.bottom = auto(); } else if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.margin.bottom = length(v * sf); }
            "margin-left" => if matches!(value, StyleValue::Auto) { ts.margin.left = auto(); } else if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.margin.left = length(v * sf); }
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
    
    // 绘制边框（宽度精确 + 抗锯齿的环形填充）
    if style.border_width > 0.0 {
        if let Some(bc) = style.border_color {
            let bc = if style.opacity < 1.0 {
                Color::new(bc.r, bc.g, bc.b, (bc.a as f32 * style.opacity) as u8)
            } else { bc };
            stroke_round_rect_ring(canvas, x, y, w, h, radii, style.border_width, bc);
        }
    }
}

/// 以指定宽度绘制（可带圆角的）边框环，抗锯齿。
///
/// 之前边框用 `PaintStyle::Stroke` 走 `draw_line`（Wu 1px 线），既忽略 `border-width`
/// 又在圆角处产生锯齿。这里改为「外圆角矩形 - 内圆角矩形」组成的环形，用扫描线
/// even-odd 填充（自带 4x 超采样抗锯齿），边框宽度精确、边缘平滑。
pub fn stroke_round_rect_ring(
    canvas: &mut Canvas,
    x: f32, y: f32, w: f32, h: f32,
    radii: [f32; 4],
    width: f32,
    color: Color,
) {
    if width <= 0.0 || w <= 0.0 || h <= 0.0 { return; }
    let bw = width.min(w / 2.0).min(h / 2.0);
    let [tl, tr, br, bl] = radii;

    let mut path = Path::new();
    // 外圈
    add_round_rect_with_radii(&mut path, x, y, w, h, radii);
    // 内圈（内缩 border-width，圆角相应减小），形成挖空的环
    let iw = (w - 2.0 * bw).max(0.0);
    let ih = (h - 2.0 * bw).max(0.0);
    let ir = |r: f32| (r - bw).max(0.0);
    add_round_rect_with_radii(&mut path, x + bw, y + bw, iw, ih, [ir(tl), ir(tr), ir(br), ir(bl)]);

    let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
    canvas.draw_path(&path, &paint);
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
