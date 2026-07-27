//! 组件基础定义

use crate::parser::wxml::WxmlNode;
use crate::parser::wxss::{StyleSheet, StyleValue, LengthUnit, rpx_to_px, ElementDesc};
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use std::collections::HashMap;
use taffy::prelude::*;

/// 与渲染器同款的共享度量字体：build 阶段按真实字形宽度测量文本盒宽度，
/// 避免用粗糙估算导致按钮/标签等叶子组件盒子偏窄或塌缩。
static MEASURE_FONT: once_cell::sync::Lazy<Option<std::sync::Arc<TextRenderer>>> =
    once_cell::sync::Lazy::new(crate::text::shared_fonts);

/// 用真实字体度量文本宽度（物理像素）。取不到字体时按中文全宽/西文 0.6 估算回退。
pub fn intrinsic_text_width(text: &str, font_px: f32, letter_spacing_px: f32) -> f32 {
    intrinsic_text_width_weighted(text, font_px, letter_spacing_px, false)
}

/// 同 [`intrinsic_text_width`]，但按字重度量（粗体字面通常更宽，必须与绘制一致）。
pub fn intrinsic_text_width_weighted(text: &str, font_px: f32, letter_spacing_px: f32, bold: bool) -> f32 {
    if let Some(tr) = MEASURE_FONT.as_ref() {
        tr.measure_text_weighted(text, font_px, letter_spacing_px, bold)
    } else {
        text.chars()
            .map(|c| if c.is_ascii() { font_px * 0.6 } else { font_px } + letter_spacing_px)
            .sum()
    }
}

/// 给定物理字号的 CSS `line-height:normal` 行高（按含 CJK 处理）。
pub fn natural_line_height_px(font_px: f32) -> f32 {
    MEASURE_FONT
        .as_ref()
        .map(|tr| tr.natural_line_height(font_px))
        .unwrap_or(font_px * crate::text::NORMAL_LINE_HEIGHT_FACTOR)
}

/// 按文本内容选择的 `line-height:normal` 行高（含 CJK 与纯西文系数不同，见 text 模块）。
pub fn natural_line_height_px_for(text: &str, font_px: f32) -> f32 {
    MEASURE_FONT
        .as_ref()
        .map(|tr| tr.natural_line_height_for(text, font_px))
        .unwrap_or_else(|| {
            let factor = if crate::text::text_has_cjk(text) {
                crate::text::CJK_LINE_HEIGHT_FACTOR
            } else {
                crate::text::LATIN_LINE_HEIGHT_FACTOR
            };
            font_px * factor
        })
}

// ── taffy 尺寸值的读取 ──
//
// taffy 0.12 起 `Dimension` / `LengthPercentage(Auto)` 不再是枚举，而是包着
// `CompactLength`（把 tag 塞进 f32 低位的位压缩表示）的不透明结构 —— 省内存、
// 但也意味着**不能再 match**。这里把「读值」收成三个判据，免得每个调用点各写一遍
// tag 比较，将来再换表示也只改这里。

/// `Dimension` 是不是 `auto`
#[inline]
pub fn dim_is_auto(d: Dimension) -> bool {
    d.is_auto()
}

/// `Dimension` 是绝对长度时取出像素值
#[inline]
pub fn dim_length(d: Dimension) -> Option<f32> {
    (d.tag() == CompactLength::LENGTH_TAG).then(|| d.value())
}

/// `Dimension` 是百分比时取出比例（0~1，注意不是 0~100）
#[inline]
pub fn dim_percent(d: Dimension) -> Option<f32> {
    (d.tag() == CompactLength::PERCENT_TAG).then(|| d.value())
}

/// `LengthPercentageAuto` 是绝对长度时取出像素值
#[inline]
pub fn lpa_length(v: LengthPercentageAuto) -> Option<f32> {
    let raw = v.into_raw();
    (raw.tag() == CompactLength::LENGTH_TAG).then(|| raw.value())
}

/// 从 taffy 的 LengthPercentage 取出长度像素（百分比/auto 记 0）。
pub fn length_px(v: LengthPercentage) -> f32 {
    if v.into_raw().tag() == CompactLength::LENGTH_TAG {
        v.into_raw().value()
    } else {
        0.0
    }
}

/// 渲染节点
#[derive(Clone)]
pub struct RenderNode {
    pub tag: String,
    pub text: String,
    pub attrs: HashMap<String, String>,
    pub taffy_node: NodeId,
    pub style: NodeStyle,
    pub children: Vec<RenderNode>,
    /// 事件绑定（含捕获/冒泡、catch、mut-bind）
    pub events: Vec<EventBind>,
}

/// 文本叶子的 taffy 度量上下文：让 taffy 在布局时按「可用宽度」正确解析文本的
/// 最小/最大内容宽与换行高度。这样 block 文本在定宽父级里按容器宽换行，在收缩型
/// （auto 宽）父级里又能把父级撑到内容宽——两种 CSS 语义都成立，无需宽度 hack。
#[derive(Clone)]
pub struct TextMeasure {
    pub text: String,
    pub font_px: f32,
    pub letter_spacing_px: f32,
    pub line_height_px: f32,
    pub pad_l: f32,
    pub pad_r: f32,
    pub pad_t: f32,
    pub pad_b: f32,
    pub nowrap: bool,
    /// 是否用粗体字面度量（与绘制端一致）
    pub bold: bool,
    /// 显式换行符决定的最少行数
    pub min_lines: usize,
    /// 不换行时的单行内容宽（多段取最宽），即 max-content 宽
    pub max_line_width: f32,
    /// 最小不可拆分单元宽（CJK 单字 / 最长西文词），即 min-content 宽
    pub min_unit_width: f32,
    /// 这段文字的字体栈：taffy 的度量闭包据此解析出与绘制**同一个**字体
    pub font_family: Option<std::sync::Arc<str>>,
}

/// 节点的 taffy 上下文（目前仅文本需要度量；其它叶子用显式尺寸）。
pub type NodeContext = TextMeasure;
/// 带文本度量上下文的 taffy 树类型别名。
pub type Tree = taffy::TaffyTree<NodeContext>;

/// 节点样式
/// `transition` 规格：时长、延迟与缓动曲线
#[derive(Clone, Copy, Debug)]
pub struct TransitionSpec {
    pub duration: f32,
    pub delay: f32,
    /// 三次贝塞尔的四个控制点（ease / linear / ease-in-out 都归一到这里）
    pub curve: (f32, f32, f32, f32),
}

impl Default for TransitionSpec {
    fn default() -> Self {
        // CSS 默认 ease
        Self { duration: 0.0, delay: 0.0, curve: (0.25, 0.1, 0.25, 1.0) }
    }
}

#[derive(Clone, Default)]
pub struct NodeStyle {
    /// 按压态（`:active` 命中或小程序的 `hover-class`）下的整套样式。
    /// 在建树期算好，绘制期按需切换 —— 按压不触发重新布局，代价是
    /// 只有绘制类属性会生效（背景/颜色/透明度/transform）。
    pub pressed_style: Option<Box<NodeStyle>>,
    /// 是否显式声明了 `position`（relative / absolute / fixed）。
    /// 用来判断「定位祖先」——taffy 的默认 position 就是 Relative，
    /// 光看它分不出 CSS 的 static 与 relative。
    pub is_positioned: bool,
    /// `transition` 的时长与缓动（秒 / 三次贝塞尔控制点）。
    /// 只在「常态 ↔ 按压态」之间做插值，覆盖 CSS transition 的主要用法。
    pub transition: Option<TransitionSpec>,
    pub background_color: Option<Color>,
    /// 线性渐变背景（优先于 background_color 绘制）
    pub background_gradient: Option<LinearGradientBg>,
    pub text_color: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    /// 各边独立边框宽度（逻辑像素 * sf；0 表示该边无独立边框）
    pub border_top_width: f32,
    pub border_right_width: f32,
    pub border_bottom_width: f32,
    pub border_left_width: f32,
    /// 各边独立边框颜色（None 时回退到 border_color）
    pub border_top_color: Option<Color>,
    pub border_right_color: Option<Color>,
    pub border_bottom_color: Option<Color>,
    pub border_left_color: Option<Color>,
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
    /// CSS `font-family` 原样保留的字体栈（解析成具体字体见 `text_family`）。
    /// 用 `Arc<str>`：NodeStyle 每个节点都要克隆一份，字符串共享比复制便宜。
    pub font_family: Option<std::sync::Arc<str>>,
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
    /// CSS 动画（`animation` 简写或分项），由渲染器在绘制阶段按全局时钟求值
    pub animation: Option<crate::renderer::anim::AnimationSpec>,
    /// 无单位 line-height（倍数）。在全部声明应用完后再乘以最终字号，
    /// 避免受 CSS 声明遍历顺序影响（HashMap 无序，line-height 可能先于 font-size 生效）。
    pub line_height_scale: Option<f32>,
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

/// 线性渐变背景
#[derive(Clone)]
pub struct LinearGradientBg {
    /// CSS 角度：0deg=向上(to top)，90deg=向右，180deg=向下，顺时针
    pub angle_deg: f32,
    /// 颜色停靠点 (位置 0~1, 颜色)，按位置升序
    pub stops: Vec<(f32, Color)>,
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
#[derive(Debug, Clone, Copy, Default, PartialEq)]
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
    /// 继承下来的字体栈（`font-family` 是继承属性）
    pub font_family: Option<std::sync::Arc<str>>,
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
            font_family: None,
        }
    }
}

/// 组件上下文
pub struct ComponentContext<'a> {
    pub scale_factor: f32,
    pub screen_width: f32,
    pub screen_height: f32,
    pub stylesheet: &'a StyleSheet,
    pub taffy: &'a mut Tree,
    /// 祖先元素链（根 -> 父），用于后代/子选择器匹配。缺省为空。
    pub ancestors: Vec<ElementDesc>,
    /// 从父元素继承下来的文本样式。
    pub inherited: InheritedText,
    /// 当前元素在兄弟中的位置（0 起）与兄弟总数，用于结构性伪类匹配。
    pub sibling_index: usize,
    pub sibling_count: usize,
    /// 祖先链里是否存在**定位祖先**（`position` 为 relative/absolute/fixed）。
    ///
    /// CSS 规则：`position:absolute` 元素的包含块是最近的定位祖先；一个都没有时
    /// 是「初始包含块」，也就是**视口**。而布局引擎只会按父节点解析百分比，
    /// 于是 `position:absolute; height:100%` 的铺底元素挂在 auto 高度的父节点下时
    /// 会塌成 0（tea-app 闪屏的整屏背景图就是这么消失的）。
    /// 有了这个标记，就能在没有定位祖先时把百分比按视口折算成确定像素。
    pub has_positioned_ancestor: bool,
}

/// 组件 trait
pub trait Component {
    fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode>;
    fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32);
}

// CSS 取值解析（parse_color_str / parse_box_shadow / parse_transform /
// parse_border_shorthand / parse_length_simple）已拆分到 style_parse 模块，
// 这里重新导出以保持 `use super::base::*` 的调用点不变。
pub use super::color_parse::{parse_color_str, parse_named_color};
pub use super::style_parse::{
    parse_box_shadow, parse_transform, parse_border_shorthand, parse_border_side,
    parse_length_simple, parse_linear_gradient, parse_attr_json,
    split_top_commas, split_top_ws,
};

/// taffy 文本度量：给定已知尺寸与可用空间，按 CSS 语义算出文本盒尺寸。
///
/// - 两维都已知：直接返回。
/// - 宽度未知：按 available 决定（Definite=定宽换行；MaxContent=单行内容宽；MinContent=最小单元宽）。
/// - 高度按解析出的行数 * 行高 + 上下内边距。
pub fn measure_text_node(
    known: taffy::geometry::Size<Option<f32>>,
    available: taffy::geometry::Size<taffy::style::AvailableSpace>,
    tm: &TextMeasure,
    tr: Option<&crate::text::TextRenderer>,
) -> taffy::geometry::Size<f32> {
    use taffy::style::AvailableSpace;
    // 指定了 font-family 就用那一族的字体来数换行：与绘制端同一把尺子
    let family = crate::text_family::renderer_for_family(tm.font_family.as_deref());
    let tr = family.as_deref().or(tr);
    let pad_w = tm.pad_l + tm.pad_r;
    let pad_h = tm.pad_t + tm.pad_b;
    let content_w = tm.max_line_width + pad_w;

    // 解析可用宽度 → 内容区宽
    let box_w = match known.width {
        Some(w) => w,
        None => match available.width {
            AvailableSpace::Definite(w) => w.min(content_w).max(tm.min_unit_width + pad_w),
            AvailableSpace::MaxContent => content_w,
            AvailableSpace::MinContent => tm.min_unit_width + pad_w,
        },
    };
    let inner_w = (box_w - pad_w).max(1.0);

    // 行数：nowrap 只按显式换行；否则按内容宽/可用宽估算，再取真实度量
    let lines = if tm.nowrap {
        tm.min_lines.max(1)
    } else {
        let wrap = if let Some(tr) = tr {
            count_wrapped_text_lines(tr, &tm.text, inner_w, tm.font_px, tm.letter_spacing_px, tm.bold)
        } else if inner_w + WRAP_TOLERANCE_PX < tm.max_line_width {
            (tm.max_line_width / inner_w).ceil() as usize
        } else {
            1
        };
        tm.min_lines.max(wrap).max(1)
    };

    let height = known
        .height
        .unwrap_or(lines as f32 * tm.line_height_px + pad_h);
    taffy::geometry::Size { width: box_w, height }
}

/// 文本的最小内容宽（CSS min-content）：最宽的不可拆分单元。
/// CJK 逐字可断，故为单字宽；连续 ASCII 视作整词不可断，取最长词宽。
pub fn min_unit_width(
    text: &str,
    tr: Option<&crate::text::TextRenderer>,
    font_px: f32,
    letter_spacing_px: f32,
    bold: bool,
) -> f32 {
    let measure = |s: &str| -> f32 {
        if let Some(tr) = tr {
            tr.measure_text_weighted(s, font_px, letter_spacing_px, bold)
        } else {
            s.chars()
                .map(|c| if c.is_ascii() { font_px * 0.62 } else { font_px } + letter_spacing_px)
                .sum()
        }
    };
    let mut widest = 0.0f32;
    let mut word = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            word.push(ch);
        } else {
            if !word.is_empty() {
                widest = widest.max(measure(&word));
                word.clear();
            }
            if !ch.is_whitespace() {
                widest = widest.max(measure(&ch.to_string()));
            }
        }
    }
    if !word.is_empty() {
        widest = widest.max(measure(&word));
    }
    widest.max(font_px)
}

/// 换行判定的亚像素容差（物理像素）。
///
/// 文本盒宽度来自字形度量求和，绘制时逐字累加同样的度量，理论上正好放得下；
/// 但两处的浮点累加顺序不同，末字可能因 1e-3 级误差被判为"超出"而换行。
/// 用半像素容差吸收这个误差 —— 而不是把每个文本盒都加宽几个像素
/// （后者会让所有按内容定宽的元素比浏览器宽一圈）。
pub const WRAP_TOLERANCE_PX: f32 = 0.5;

/// 解析 CSS 时间值（`.25s` / `250ms`）为秒
pub fn parse_css_time(s: &str) -> f32 {
    let s = s.trim();
    if let Some(ms) = s.strip_suffix("ms") {
        ms.trim().parse::<f32>().unwrap_or(0.0) / 1000.0
    } else if let Some(sec) = s.strip_suffix('s') {
        sec.trim().parse::<f32>().unwrap_or(0.0)
    } else {
        s.parse::<f32>().unwrap_or(0.0)
    }
}

/// 解析缓动函数名/`cubic-bezier(...)` 为三次贝塞尔控制点
pub fn parse_timing_function(s: &str) -> (f32, f32, f32, f32) {
    let s = s.trim();
    if let Some(args) = s.strip_prefix("cubic-bezier(").and_then(|v| v.strip_suffix(')')) {
        let nums: Vec<f32> = args
            .split(',')
            .filter_map(|v| v.trim().parse::<f32>().ok())
            .collect();
        if nums.len() == 4 {
            return (nums[0], nums[1], nums[2], nums[3]);
        }
    }
    match s {
        "linear" => (0.0, 0.0, 1.0, 1.0),
        "ease-in" => (0.42, 0.0, 1.0, 1.0),
        "ease-out" => (0.0, 0.0, 0.58, 1.0),
        "ease-in-out" => (0.42, 0.0, 0.58, 1.0),
        _ => (0.25, 0.1, 0.25, 1.0), // ease
    }
}

/// 解析 `transition` 简写。属性名被忽略：按压态是整套样式切换，统一插值。
/// 形如 `background .25s ease, transform .15s` 时取第一段的时长/缓动。
fn parse_transition_shorthand(s: &str) -> Option<TransitionSpec> {
    let first = s.split(',').next()?.trim();
    if first.is_empty() || first == "none" {
        return None;
    }
    let mut spec = TransitionSpec::default();
    let mut times = Vec::new();
    for tok in first.split_whitespace() {
        if tok.ends_with("ms") || tok.ends_with('s') {
            times.push(parse_css_time(tok));
        } else if tok.starts_with("cubic-bezier(")
            || matches!(tok, "linear" | "ease" | "ease-in" | "ease-out" | "ease-in-out")
        {
            spec.curve = parse_timing_function(tok);
        }
    }
    // CSS 规定：第一个时间是 duration，第二个是 delay
    spec.duration = times.first().copied().unwrap_or(0.0);
    spec.delay = times.get(1).copied().unwrap_or(0.0);
    if spec.duration <= 0.0 {
        return None;
    }
    Some(spec)
}

/// 一个字符是否属于「不可拆分的西文词」。与 [`min_unit_width`] 同一套规则，
/// 保证 min-content 宽度和实际断行位置不会互相打架。
pub fn is_wrap_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'
}

/// 段落断行：把一段（不含 `\n` 的）文本切成若干行，返回每行的字符下标区间 `[start, end)`。
///
/// 与浏览器 `white-space: normal` + `overflow-wrap: normal` 对齐的三条规则：
/// 1. 连续西文词是不可拆分单元：放不下也不从词中间切断，让它独占一行横向溢出
///    （Chrome 就是这么处理 `HOT` 挤在 32px 盒子里的情形）；CJK 逐字可断。
/// 2. 行尾空白不参与「放不下」的判断，也不进入行区间 —— 否则右/居中对齐会被空格推偏。
/// 3. 换行处的空白被吃掉，不会跑到下一行行首。
///
/// `measure` 由调用方提供（同一套字形度量），确保度量、行数统计、绘制三处结果一致。
pub fn wrap_paragraph_lines(
    chars: &[char],
    max_width: f32,
    mut measure: impl FnMut(&[char]) -> f32,
) -> Vec<(usize, usize)> {
    if chars.is_empty() {
        return vec![(0, 0)];
    }
    // 去掉区间末尾的空白（规则 2）
    let trim_end = |end: usize| -> usize {
        let mut e = end;
        while e > 0 && chars[e - 1].is_whitespace() {
            e -= 1;
        }
        e
    };

    let mut lines: Vec<(usize, usize)> = Vec::new();
    let mut line_start = 0usize;
    let mut line_w = 0.0f32; // 本行已确定的宽度，不含挂起的行尾空白
    let mut pending_ws = 0.0f32;
    let mut i = 0usize;

    while i < chars.len() {
        let unit_start = i;
        if chars[i].is_whitespace() {
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            pending_ws += measure(&chars[unit_start..i]);
            continue;
        }
        if is_wrap_word_char(chars[i]) {
            while i < chars.len() && is_wrap_word_char(chars[i]) {
                i += 1;
            }
        } else {
            i += 1;
        }
        let unit_w = measure(&chars[unit_start..i]);
        if line_w > 0.0 && line_w + pending_ws + unit_w > max_width + WRAP_TOLERANCE_PX {
            lines.push((line_start, trim_end(unit_start)));
            line_start = unit_start;
            line_w = unit_w;
        } else {
            line_w += pending_ws + unit_w;
        }
        pending_ws = 0.0;
    }
    lines.push((line_start, trim_end(chars.len())));
    lines
}

/// 统计文本在给定宽度下的换行行数（含显式换行符），供度量使用。
fn count_wrapped_text_lines(tr: &crate::text::TextRenderer, text: &str, max_width: f32, size: f32, ls: f32, bold: bool) -> usize {
    if max_width <= 0.0 { return text.split('\n').count().max(1); }
    let mut total = 0usize;
    for para in text.split('\n') {
        if para.is_empty() { total += 1; continue; }
        let chars: Vec<char> = para.chars().collect();
        let measure = |s: &[char]| -> f32 {
            s.iter().map(|c| tr.measure_char_weighted(*c, size, bold) + ls).sum()
        };
        total += wrap_paragraph_lines(&chars, max_width, measure).len();
    }
    total.max(1)
}

/// 事件的传播阶段（微信的 `capture-bind:` / `capture-catch:` 走捕获）
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EventPhase {
    /// 冒泡阶段：由内向外
    #[default]
    Bubble,
    /// 捕获阶段：由外向内，先于冒泡
    Capture,
}

/// 一条事件绑定（节点上的 `bind*` / `catch*` / `mut-bind:*` / `capture-*`）
#[derive(Clone, Debug)]
pub struct EventBind {
    /// 事件名（`bindtap` → `tap`，`bind:touchstart` → `touchstart`）
    pub event_type: String,
    /// 处理函数名（页面或组件实例上的方法）
    pub handler: String,
    /// `data-*` 数据集（外加 input 类组件的若干属性）
    pub data: HashMap<String, String>,
    /// `catch*`：阻止继续传播
    pub is_catch: bool,
    /// 捕获还是冒泡
    pub phase: EventPhase,
    /// `mut-bind:*`：互斥绑定，一条触发后其它 mut-bind 不再触发（`bind`/`catch` 不受影响）
    pub mut_bind: bool,
}

/// 解析事件属性名 → (事件名, 是否 catch, 阶段, 是否互斥绑定)。
///
/// 微信的写法有六种前缀，冒号可省：`bind` / `catch` / `mut-bind` /
/// `capture-bind` / `capture-catch`（`bindtap` 与 `bind:tap` 等价）。
/// 之前引擎是**按固定属性名白名单**匹配的（只认 bindtap/catchtap/bindchange…），
/// 于是 `bindtouchstart`、`bind:tap`、`catchtouchmove`、`capture-bind:tap`
/// 这些全部被无声忽略 —— 自定义手势、遮罩阻止滚动、事件捕获统统失效。
pub fn parse_event_attr(attr: &str) -> Option<(&str, bool, EventPhase, bool)> {
    // 长前缀必须先匹配（`capture-bind` 也以 `bind` 结尾之外的形式出现）
    const PREFIXES: &[(&str, bool, EventPhase, bool)] = &[
        ("capture-catch:", true, EventPhase::Capture, false),
        ("capture-catch", true, EventPhase::Capture, false),
        ("capture-bind:", false, EventPhase::Capture, false),
        ("capture-bind", false, EventPhase::Capture, false),
        ("mut-bind:", false, EventPhase::Bubble, true),
        ("mut-bind", false, EventPhase::Bubble, true),
        ("catch:", true, EventPhase::Bubble, false),
        ("catch", true, EventPhase::Bubble, false),
        ("bind:", false, EventPhase::Bubble, false),
        ("bind", false, EventPhase::Bubble, false),
    ];
    for (prefix, is_catch, phase, mut_bind) in PREFIXES {
        if let Some(rest) = attr.strip_prefix(prefix) {
            let name = rest.trim();
            if name.is_empty() {
                return None;
            }
            return Some((name, *is_catch, *phase, *mut_bind));
        }
    }
    None
}

/// 提取节点上的全部事件绑定
pub fn extract_events(node: &WxmlNode) -> Vec<EventBind> {
    let mut events = vec![];
    // 数据集只算一次：同一节点上的多个绑定共享 `data-*`
    let mut data = HashMap::new();
    for (k, v) in &node.attributes {
        if let Some(name) = k.strip_prefix("data-") {
            data.insert(name.to_string(), v.clone());
        }
    }
    // input/textarea：把宿主侧编辑需要的属性一并带上（maxlength/type/password）
    if node.tag_name == "input" || node.tag_name == "textarea" {
        for attr in ["maxlength", "type", "password"] {
            if let Some(v) = node.get_attr(attr) {
                data.insert(attr.to_string(), v.to_string());
            }
        }
    }
    for (attr, handler) in &node.attributes {
        let Some((event_type, is_catch, phase, mut_bind)) = parse_event_attr(attr) else {
            continue;
        };
        if handler.trim().is_empty() {
            continue;
        }
        events.push(EventBind {
            event_type: event_type.to_string(),
            handler: handler.clone(),
            data: data.clone(),
            is_catch,
            phase,
            mut_bind,
        });
    }
    // 属性表是 HashMap，顺序不定；排序让绑定顺序稳定（快照/测试要可复现）
    events.sort_by(|a, b| (&a.event_type, &a.handler).cmp(&(&b.event_type, &b.handler)));
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
/// 定位偏移所在的轴（决定百分比在「相对视口」场景下参照宽还是高）
#[derive(Clone, Copy, PartialEq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// 写入一条 `top/right/bottom/left`。
///
/// - 百分比 → `LengthPercentageAuto::Percent`，由布局引擎按包含块解析（CSS 语义）
/// - `auto` → `LengthPercentageAuto::auto()`
/// - 其它长度 → 像素
///
/// 同时记录一份「相对视口」的像素值给 `position:fixed` 的固定层使用。
fn set_inset(
    value: &StyleValue,
    ctx: &ComponentContext,
    sf: f32,
    axis: Axis,
    slot: &mut LengthPercentageAuto,
    fixed_slot: &mut Option<f32>,
) {
    let viewport = if axis == Axis::Horizontal { ctx.screen_width } else { ctx.screen_height };
    match value {
        StyleValue::Auto => {
            *slot = LengthPercentageAuto::auto();
            *fixed_slot = None;
        }
        StyleValue::Length(n, LengthUnit::Percent) => {
            *slot = LengthPercentageAuto::percent(*n / 100.0);
            *fixed_slot = Some(*n / 100.0 * viewport * sf);
        }
        _ => {
            let px = match value {
                StyleValue::Number(n) => Some(*n),
                other => to_px(other, ctx.screen_width, viewport),
            };
            if let Some(px) = px {
                *slot = LengthPercentageAuto::length(px * sf);
                *fixed_slot = Some(px * sf);
            }
        }
    }
}

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
        StyleValue::Auto => Some(Dimension::auto()),
        StyleValue::Length(n, LengthUnit::Percent) => Some(percent(*n / 100.0)),
        _ => to_px(v, screen_width, screen_height).map(|px| length(px * sf)),
    }
}

/// 一条声明的落地优先序：**简写在前，细项在后**。
///
/// 级联把所有中选规则合并成一张 map，简写与细项的先后关系在这一步就丢了。
/// 于是 `.dot{border:2rpx solid #ccc}` + `.dot.on{border-color:#FF6B35}` 两条规则
/// 合并后，map 里同时躺着 `border` 和 `border-color` —— 谁后落地谁赢。
/// map 的遍历顺序在 Rust 里是随机的（每进程一个 hash 种子），
/// 所以同一份源码**每次运行渲染结果都可能不同**（那个圆点时橙时灰）。
///
/// 真正的 CSS 模型是解析期就把简写展开成细项，这里用更小的改动达到同样效果：
/// 按「简写 → 方向细项 → 单属性细项」的固定档位排序，档位内按属性名排序，
/// 保证细项永远覆盖简写，且结果与运行次数无关。
fn declaration_order(name: &str) -> u8 {
    match name {
        // 全能简写
        "font" | "background" | "border" | "border-radius" | "margin" | "padding"
        | "flex" | "transition" | "animation" | "grid-area" | "inset" => 0,
        // 方向/边简写（仍是简写，但比 `border` 更具体）
        "border-top" | "border-right" | "border-bottom" | "border-left"
        | "border-width" | "border-color" | "border-style"
        | "margin-block" | "margin-inline" | "padding-block" | "padding-inline"
        | "background-position" | "background-size" | "flex-flow" => 1,
        // 其余都是细项
        _ => 2,
    }
}

/// 把级联后的声明按确定性顺序取出（见 [`declaration_order`]）
fn sorted_declarations(css: &HashMap<String, StyleValue>) -> Vec<(&str, &StyleValue)> {
    let mut out: Vec<(&str, &StyleValue)> = css.iter().map(|(k, v)| (k.as_str(), v)).collect();
    out.sort_by(|a, b| declaration_order(a.0).cmp(&declaration_order(b.0)).then(a.0.cmp(b.0)));
    out
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
    // 属性表只有 `[attr]` 选择器用得到；没有这类规则就别带 —— 它会随祖先链克隆被深拷很多遍
    static EMPTY_ATTRS: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
    let desc_attrs = if ctx.stylesheet.has_attr_selectors() {
        &node.attributes
    } else {
        EMPTY_ATTRS.get_or_init(HashMap::new)
    };
    chain.push(ElementDesc::new(&node.tag_name, id, &classes, desc_attrs)
        .with_position(ctx.sibling_index, ctx.sibling_count));
    let css = ctx.stylesheet.get_styles_chain(&chain);
    
    // 先用继承的文本样式做默认值，再用 CSS/内联覆盖（CSS 继承语义）
    let mut ns = NodeStyle {
        font_size: ctx.inherited.font_size,
        text_color: ctx.inherited.color,
        font_weight: ctx.inherited.weight,
        text_align: ctx.inherited.align,
        line_height: ctx.inherited.line_height,
        letter_spacing: ctx.inherited.letter_spacing,
        font_family: ctx.inherited.font_family.clone(),
        opacity: 1.0,
        ..Default::default()
    };
    
    // 默认样式：flex 布局，列方向
    let mut ts = Style { 
        display: Display::Flex, 
        flex_direction: FlexDirection::Column,
        ..Default::default() 
    };

    // 应用类样式（顺序见 `declaration_order`：简写必须先落地）
    for (name, value) in sorted_declarations(&css) {
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
    
    // 无单位 line-height 在所有声明落地后统一按最终字号换算
    if let Some(scale) = ns.line_height_scale {
        ns.line_height = Some(ns.font_size * scale);
    }

    // ── 绝对定位的包含块修正（CSS 语义）──
    //
    // `position:absolute` 的包含块是「最近的定位祖先」，一个都没有时是**初始包含块**
    // （视口）。布局引擎只会按父节点解析百分比，于是
    // `.background{position:absolute;left:0;top:0;width:100%;height:100%}` 挂在
    // 一个 auto 高度的父 view 下时，高度会塌成 0 —— 整屏铺底的背景图就此消失。
    //
    // 没有定位祖先时，这里把百分比尺寸按视口折算成确定像素，等价于把包含块换成视口。
    // 只修**高度**：宽度按父节点解析本来就是对的（父节点通常就是整宽），
    // 而且元素的 left/top 偏移仍然是相对父节点的 —— 把宽度也换成视口宽会让
    // 「窄父节点里的绝对定位元素」既变宽又不移位，反而画错（实测 canvas/组件页
    // 与 H5 的差异从 5.4%/3.8% 恶化到 12.5%/10.2%）。
    // 高度不一样：父节点高度是 auto 时百分比没有参照物，会直接塌成 0。
    if ts.position == Position::Absolute && !ctx.has_positioned_ancestor {
        if let Some(p) = dim_percent(ts.size.height) {
            ts.size.height = Dimension::length(ctx.screen_height * ctx.scale_factor * p);
        }
    }

    // ── 按压态样式 ──
    // 同一条祖先链，把目标元素标记为 pressed 并补上 `hover-class` 的类名，
    // 再取一次 CSS 声明 —— `:active` 与小程序的 `hover-class` 因此走同一条路径。
    // 只覆盖绘制类属性（taffy 布局结果丢弃）：按压不触发重新布局，
    // 这是刻意的取舍，按一下就重排整页在纯软件光栅上代价太高。
    let hover_class = node
        .get_attr("hover-class")
        .filter(|s| !s.trim().is_empty() && s.trim() != "none")
        .map(|s| s.to_string());
    if ctx.stylesheet.has_active_rules() || hover_class.is_some() {
        let mut pressed_chain = chain;
        if let Some(last) = pressed_chain.last_mut() {
            last.pressed = true;
            if let Some(hc) = &hover_class {
                for c in hc.split_whitespace() {
                    last.classes.push(c.to_string());
                }
            }
        }
        let pressed_css = ctx.stylesheet.get_styles_chain(&pressed_chain);
        let mut pressed_ns = ns.clone();
        let mut throwaway_ts = ts.clone();
        for (name, value) in sorted_declarations(&pressed_css) {
            apply_style_property(name, value, &mut throwaway_ts, &mut pressed_ns, ctx);
        }
        // 内联 style 优先级高于类样式，按压态同样要重放一遍
        if let Some(style_str) = node.get_attr("style") {
            for part in style_str.split(';') {
                let part = part.trim();
                if part.is_empty() { continue; }
                if let Some(colon_pos) = part.find(':') {
                    let name = part[..colon_pos].trim();
                    let value = parse_inline_value(part[colon_pos + 1..].trim());
                    apply_style_property(name, &value, &mut throwaway_ts, &mut pressed_ns, ctx);
                }
            }
        }
        if let Some(scale) = pressed_ns.line_height_scale {
            pressed_ns.line_height = Some(pressed_ns.font_size * scale);
        }
        pressed_ns.pressed_style = None; // 不递归
        ns.pressed_style = Some(Box::new(pressed_ns));
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

/// 从 StyleValue 解析颜色（Color 直取，String 解析）
fn color_value(v: &StyleValue) -> Option<Color> {
    match v {
        StyleValue::Color(c) => Some(*c),
        StyleValue::String(s) => parse_color_str(s),
        _ => None,
    }
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
                        ts.display = Display::Block;
                        ns.is_block = true;
                    }
                    "flex" => ts.display = Display::Flex,
                    "grid" => ts.display = Display::Grid,
                    _ => ts.display = Display::Flex,
                };
            }
            // `box-sizing`：决定 width/height 算的是内容盒还是边框盒。
            //
            // 从前这条属性被整条忽略（布局引擎里没有这个概念），于是写
            // `box-sizing: content-box` 的元素会比浏览器窄一圈内边距 + 边框。
            // 两端的缺省都是 `border-box`（编译出的 H5 里有 `*{box-sizing:border-box}`，
            // 布局引擎的缺省也是它），所以只有显式写 content-box 的地方会变。
            "box-sizing" => if let StyleValue::String(s) = value {
                ts.box_sizing = match s.as_str() {
                    "content-box" => BoxSizing::ContentBox,
                    _ => BoxSizing::BorderBox,
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
                ts.flex_basis = Dimension::length(0.0);
            }
            "flex-shrink" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.flex_shrink = v; }
            "flex-basis" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.flex_basis = v; }
            "justify-content" => if let StyleValue::String(s) = value {
                ts.justify_content = Some(match s.as_str() {
                    "center" => JustifyContent::CENTER,
                    "space-between" => JustifyContent::SPACE_BETWEEN,
                    "space-around" => JustifyContent::SPACE_AROUND,
                    "space-evenly" => JustifyContent::SPACE_EVENLY,
                    "flex-end" | "end" => JustifyContent::FLEX_END,
                    "flex-start" | "start" => JustifyContent::FLEX_START,
                    _ => JustifyContent::FLEX_START,
                });
            }
            "align-items" => if let StyleValue::String(s) = value {
                let align = match s.as_str() {
                    "center" => AlignItems::CENTER,
                    "flex-end" | "end" => AlignItems::FLEX_END,
                    "flex-start" | "start" => AlignItems::FLEX_START,
                    "stretch" => AlignItems::STRETCH,
                    "baseline" => AlignItems::BASELINE,
                    _ => AlignItems::FLEX_START,
                };
                ts.align_items = Some(align);
            }
            "align-self" => if let StyleValue::String(s) = value {
                ts.align_self = Some(match s.as_str() {
                    "center" => AlignSelf::CENTER,
                    "flex-end" | "end" => AlignSelf::FLEX_END,
                    "flex-start" | "start" => AlignSelf::FLEX_START,
                    "stretch" => AlignSelf::STRETCH,
                    "baseline" => AlignSelf::BASELINE,
                    _ => AlignSelf::START,
                });
            }
            "align-content" => if let StyleValue::String(s) = value {
                ts.align_content = Some(match s.as_str() {
                    "center" => AlignContent::CENTER,
                    "flex-end" | "end" => AlignContent::FLEX_END,
                    "flex-start" | "start" => AlignContent::FLEX_START,
                    "stretch" => AlignContent::STRETCH,
                    "space-between" => AlignContent::SPACE_BETWEEN,
                    "space-around" => AlignContent::SPACE_AROUND,
                    "space-evenly" => AlignContent::SPACE_EVENLY,
                    _ => AlignContent::FLEX_START,
                });
            }
            "gap" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { 
                let sv = v * sf;
                ts.gap = Size { width: length(sv), height: length(sv) }; 
            }
            "row-gap" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.gap.height = length(v * sf); }
            "column-gap" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.gap.width = length(v * sf); }
            "background-color" | "background" | "background-image" => {
                match value {
                    StyleValue::Color(c) => ns.background_color = Some(*c),
                    StyleValue::String(s) if s.contains("linear-gradient(") => {
                        if let Some(g) = parse_linear_gradient(s) {
                            // 兜底纯色（渐变未绘制处使用）取首个停靠点
                            ns.background_color = g.stops.first().map(|(_, c)| *c);
                            ns.background_gradient = Some(g);
                        }
                    }
                    StyleValue::String(s) => {
                        if let Some(c) = parse_color_str(s) { ns.background_color = Some(c); }
                    }
                    _ => {}
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
            "border-top" | "border-right" | "border-bottom" | "border-left" => {
                if let StyleValue::String(s) = value {
                    let (w, c) = parse_border_side(s, ctx.screen_width, sf);
                    match name {
                        "border-top" => { if let Some(w) = w { ns.border_top_width = w; } ns.border_top_color = c.or(ns.border_top_color); }
                        "border-right" => { if let Some(w) = w { ns.border_right_width = w; } ns.border_right_color = c.or(ns.border_right_color); }
                        "border-bottom" => { if let Some(w) = w { ns.border_bottom_width = w; } ns.border_bottom_color = c.or(ns.border_bottom_color); }
                        _ => { if let Some(w) = w { ns.border_left_width = w; } ns.border_left_color = c.or(ns.border_left_color); }
                    }
                }
            }
            "border-top-width" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_top_width = v * sf; }
            "border-right-width" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_right_width = v * sf; }
            "border-bottom-width" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_bottom_width = v * sf; }
            "border-left-width" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_left_width = v * sf; }
            "border-top-color" => { if let Some(c) = color_value(value) { ns.border_top_color = Some(c); } }
            "border-right-color" => { if let Some(c) = color_value(value) { ns.border_right_color = Some(c); } }
            "border-bottom-color" => { if let Some(c) = color_value(value) { ns.border_bottom_color = Some(c); } }
            "border-left-color" => { if let Some(c) = color_value(value) { ns.border_left_color = Some(c); } }
            "font-size" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.font_size = v; }
            "font-weight" => {
                // 数字字重（400/700）与关键字（normal/bold）都要支持
                let text = match value {
                    StyleValue::String(s) => s.clone(),
                    StyleValue::Number(n) => format!("{}", *n as i32),
                    StyleValue::Length(v, _) => format!("{}", *v as i32),
                    _ => String::new(),
                };
                ns.font_weight = match text.as_str() {
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
                // 顺序很关键：无单位值是「倍数」，必须先判数字。
                // 之前先走 to_px，`line-height:1.9` 被当成 1.9 像素，行距直接被压成一条线。
                if let StyleValue::Number(n) = value {
                    ns.line_height_scale = Some(*n);
                    ns.line_height = None;
                } else if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) {
                    ns.line_height = Some(v);
                    ns.line_height_scale = None;
                }
            }
            "letter-spacing" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.letter_spacing = v; }
            // 字体栈原样留到绘制/度量期解析（见 `text_family::renderer_for_family`）：
            // 哪个字族可用取决于本机装了什么字体，样式解析期不该下这个判断。
            "font-family" => {
                if let StyleValue::String(s) = value {
                    let v = s.trim();
                    if !v.is_empty() && v != "inherit" {
                        ns.font_family = Some(std::sync::Arc::from(v));
                    }
                }
            }
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
            "z-index" => if let Some(n) = unitless_number(value) { ns.z_index = n as i32; }
            "opacity" => if let Some(n) = unitless_number(value) { ns.opacity = n.clamp(0.0, 1.0); }
            "box-shadow" => if let StyleValue::String(s) = value {
                if let Some(shadow) = parse_box_shadow(s, ctx.screen_width) {
                    ns.box_shadow = Some(shadow);
                }
            }
            "transform" => if let StyleValue::String(s) = value {
                // 用带单位换算的解析：rpx 位移必须按 750 设计宽折算，否则位移量翻倍
                if let Some(transform) = crate::renderer::anim::parse_transform_px(s, ctx.screen_width, sf) {
                    ns.transform = Some(transform);
                } else if let Some(transform) = parse_transform(s) {
                    ns.transform = Some(transform);
                }
            }
            // ── CSS 动画：animation 简写 + 各分项 ──
            // transition 简写：只取时长/延迟/缓动，属性列表忽略（在常态↔按压态之间整体插值）
            "transition" => if let StyleValue::String(s) = value {
                ns.transition = parse_transition_shorthand(s);
            }
            "transition-duration" => if let StyleValue::String(s) = value {
                let mut t = ns.transition.unwrap_or_default();
                t.duration = parse_css_time(s.split(',').next().unwrap_or("0"));
                ns.transition = Some(t);
            }
            "transition-delay" => if let StyleValue::String(s) = value {
                let mut t = ns.transition.unwrap_or_default();
                t.delay = parse_css_time(s.split(',').next().unwrap_or("0"));
                ns.transition = Some(t);
            }
            "transition-timing-function" => if let StyleValue::String(s) = value {
                let mut t = ns.transition.unwrap_or_default();
                t.curve = parse_timing_function(s.split(',').next().unwrap_or("ease"));
                ns.transition = Some(t);
            }
            "animation" => if let StyleValue::String(s) = value {
                ns.animation = crate::renderer::anim::AnimationSpec::parse_shorthand(s);
            }
            "animation-name" => if let StyleValue::String(s) = value {
                let name = s.trim().split(',').next().unwrap_or("").trim().to_string();
                if name.is_empty() || name == "none" {
                    ns.animation = None;
                } else {
                    let mut spec = ns.animation.clone().unwrap_or_default();
                    spec.name = name;
                    ns.animation = Some(spec);
                }
            }
            "animation-duration" | "animation-delay" => {
                let text = match value {
                    StyleValue::String(s) => s.clone(),
                    StyleValue::Length(v, _) => format!("{}s", v),
                    StyleValue::Number(n) => format!("{}s", n),
                    _ => String::new(),
                };
                if let Some(secs) = crate::renderer::anim::parse_time(text.split(',').next().unwrap_or("")) {
                    let mut spec = ns.animation.clone().unwrap_or_default();
                    if name == "animation-duration" { spec.duration = secs; } else { spec.delay = secs; }
                    ns.animation = Some(spec);
                }
            }
            "animation-timing-function" => if let StyleValue::String(s) = value {
                if let Some(timing) = crate::renderer::anim::Timing::parse(s.split(',').next().unwrap_or("").trim()) {
                    let mut spec = ns.animation.clone().unwrap_or_default();
                    spec.timing = timing;
                    ns.animation = Some(spec);
                }
            }
            "animation-iteration-count" => {
                let mut spec = ns.animation.clone().unwrap_or_default();
                spec.iterations = match value {
                    StyleValue::Number(n) => *n,
                    StyleValue::String(s) if s.trim() == "infinite" => f32::INFINITY,
                    StyleValue::String(s) => s.trim().parse().unwrap_or(1.0),
                    _ => 1.0,
                };
                ns.animation = Some(spec);
            }
            "animation-direction" => if let StyleValue::String(s) = value {
                use crate::renderer::anim::Direction;
                let mut spec = ns.animation.clone().unwrap_or_default();
                spec.direction = match s.trim() {
                    "reverse" => Direction::Reverse,
                    "alternate" => Direction::Alternate,
                    "alternate-reverse" => Direction::AlternateReverse,
                    _ => Direction::Normal,
                };
                ns.animation = Some(spec);
            }
            "animation-fill-mode" => if let StyleValue::String(s) = value {
                use crate::renderer::anim::FillMode;
                let mut spec = ns.animation.clone().unwrap_or_default();
                spec.fill = match s.trim() {
                    "forwards" => FillMode::Forwards,
                    "backwards" => FillMode::Backwards,
                    "both" => FillMode::Both,
                    _ => FillMode::None,
                };
                ns.animation = Some(spec);
            }
            "position" => if let StyleValue::String(s) = value {
                match s.as_str() {
                    "absolute" => {
                        ts.position = Position::Absolute;
                        ns.is_positioned = true;
                    }
                    "fixed" => {
                        // fixed 定位：使用 absolute 让 Taffy 处理，但标记为 fixed
                        ts.position = Position::Absolute;
                        ns.is_fixed = true;
                        ns.is_positioned = true;
                    }
                    "relative" => {
                        ts.position = Position::Relative;
                        ns.is_positioned = true;
                    }
                    _ => ts.position = Position::Relative,
                };
            }
            // ── 定位偏移 top/right/bottom/left ──
            //
            // 百分比必须交给布局引擎按「包含块」解析，不能在这里换算成像素：
            // `to_px` 对百分比一律乘屏幕宽度，于是居中老写法
            // `left:50%; top:50%; margin:-44rpx` 里的 `top:50%` 变成 187.5px，
            // 元素被推到父容器下方（视频页封面上的播放按钮就这样被裁掉一半）。
            // `position:fixed` 另算：固定层用 ns.fixed_* 的像素值，
            // 相对视口解析（水平轴用视口宽、垂直轴用视口高）。
            "top" => set_inset(value, ctx, sf, Axis::Vertical, &mut ts.inset.top, &mut ns.fixed_top),
            "left" => set_inset(value, ctx, sf, Axis::Horizontal, &mut ts.inset.left, &mut ns.fixed_left),
            "right" => set_inset(value, ctx, sf, Axis::Horizontal, &mut ts.inset.right, &mut ns.fixed_right),
            "bottom" => set_inset(value, ctx, sf, Axis::Vertical, &mut ts.inset.bottom, &mut ns.fixed_bottom),
            _ => {}
    }
}

// 线性渐变的绘制搬到了 `components::gradient`（那里有轴对齐快路径与一致性测试），
// 这里重新导出以保持 `use super::base::*` 的调用点不变。
pub use super::gradient::draw_linear_gradient;

/// 绘制盒子阴影
pub fn draw_box_shadow(canvas: &mut Canvas, shadow: &BoxShadow, x: f32, y: f32, w: f32, h: f32, border_radius: f32) {
    if shadow.inset {
        return; // 暂不支持内阴影
    }
    // 模糊掩膜 + 一次混合（实现与理由见 components::shadow）
    super::shadow::draw_outer_shadow(
        canvas, shadow.color,
        shadow.offset_x, shadow.offset_y, shadow.blur, shadow.spread,
        x, y, w, h, border_radius,
    );
}

/// 获取有效的边框圆角（未按盒子尺寸裁剪）
pub fn get_border_radii(style: &NodeStyle) -> [f32; 4] {
    [
        style.border_radius_tl.unwrap_or(style.border_radius),
        style.border_radius_tr.unwrap_or(style.border_radius),
        style.border_radius_br.unwrap_or(style.border_radius),
        style.border_radius_bl.unwrap_or(style.border_radius),
    ]
}

/// 获取按盒子尺寸裁剪后的圆角（与 CSS 一致：每个角最多为对应边的一半）。
///
/// 关键修复：`border-radius:50%` 在解析期被换算成 `50% * screen_width`（≈187px），
/// 对小元素（如关闭按钮圆圈）会产生远超盒子的半径，圆角路径的控制点飞到盒外，画出
/// 巨大杂散曲线。这里在绘制期按 min(w,h)/2 夹紧，既修复杂散曲线又让 50% 得到正确圆形。
pub fn get_border_radii_clamped(style: &NodeStyle, w: f32, h: f32) -> [f32; 4] {
    let max_r = (w.min(h) / 2.0).max(0.0);
    let clamp = |r: f32| r.max(0.0).min(max_r);
    let [tl, tr, br, bl] = get_border_radii(style);
    [clamp(tl), clamp(tr), clamp(br), clamp(bl)]
}

/// 绘制背景和边框
pub fn draw_background(canvas: &mut Canvas, style: &NodeStyle, x: f32, y: f32, w: f32, h: f32) {
    use crate::renderer::draw_profile::Timer;
    // 绘制阴影（在背景之前）
    if let Some(shadow) = &style.box_shadow {
        let _t = Timer::start("背景:阴影");
        draw_box_shadow(canvas, shadow, x, y, w, h, style.border_radius);
    }
    
    let radii = get_border_radii_clamped(style, w, h);
    let has_different_radii = radii[0] != radii[1] || radii[1] != radii[2] || radii[2] != radii[3];
    let uniform_radius = radii[0];
    
    // 绘制背景：线性渐变优先，否则纯色
    if let Some(grad) = &style.background_gradient {
        let _t = Timer::start("背景:渐变");
        draw_linear_gradient(canvas, grad, x, y, w, h, radii, style.opacity);
    } else if let Some(bg) = style.background_color {
        let _t = Timer::start(if uniform_radius > 0.0 || has_different_radii { "背景:圆角填充" } else { "背景:纯色" });
        let mut paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
        if style.opacity < 1.0 { 
            paint.color.a = (paint.color.a as f32 * style.opacity) as u8; 
        }
        
        if has_different_radii {
            let mut path = Path::new();
            add_round_rect_with_radii(&mut path, x, y, w, h, radii);
            canvas.draw_path(&path, &paint);
        } else if uniform_radius > 0.0 {
            // 快速圆角填充（实心内部 + 抗锯齿角），避免整块 4x 扫描线
            canvas.fill_round_rect(x, y, w, h, uniform_radius, paint.color);
        } else {
            canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
        }
    }
    
    // 绘制边框（宽度精确 + 抗锯齿的环形填充）
    if style.border_width > 0.0 {
        let _t = Timer::start("背景:边框环");
        if let Some(bc) = style.border_color {
            let fade = |c: Color| if style.opacity < 1.0 {
                Color::new(c.r, c.g, c.b, (c.a as f32 * style.opacity) as u8)
            } else { c };
            // border-top-color 这类「只改某一边颜色」的写法：CSS 里宽度来自 border 简写，
            // 单边宽度为 0，所以不能走 draw_side_borders；必须把环按对角线分成四段着色
            // （加载动画 `border-top-color` 转圈就依赖这个）。
            let sides = [
                style.border_top_color.unwrap_or(bc),
                style.border_right_color.unwrap_or(bc),
                style.border_bottom_color.unwrap_or(bc),
                style.border_left_color.unwrap_or(bc),
            ];
            if sides.iter().any(|c| *c != bc) {
                stroke_round_rect_ring_sides(
                    canvas, x, y, w, h, radii, style.border_width,
                    [fade(sides[0]), fade(sides[1]), fade(sides[2]), fade(sides[3])],
                );
            } else {
                stroke_round_rect_ring(canvas, x, y, w, h, radii, style.border_width, fade(bc));
            }
        }
    }
    
    // 各边独立边框（常用于列表分割线 border-bottom 等），画为轴对齐细矩形
    {
        let _t = Timer::start("背景:单边框");
        draw_side_borders(canvas, style, x, y, w, h);
    }
}

/// 绘制各边独立边框（border-top/right/bottom/left）。
fn draw_side_borders(canvas: &mut Canvas, style: &NodeStyle, x: f32, y: f32, w: f32, h: f32) {
    let fallback = style.border_color.unwrap_or(Color::from_hex(0xE5E5E5));
    let alpha = |c: Color| if style.opacity < 1.0 {
        Color::new(c.r, c.g, c.b, (c.a as f32 * style.opacity) as u8)
    } else { c };
    let mut fill = |rx: f32, ry: f32, rw: f32, rh: f32, c: Color| {
        if rw <= 0.0 || rh <= 0.0 { return; }
        let paint = Paint::new().with_color(alpha(c)).with_style(PaintStyle::Fill);
        canvas.draw_rect(&GeoRect::new(rx, ry, rw, rh), &paint);
    };
    if style.border_top_width > 0.0 {
        fill(x, y, w, style.border_top_width, style.border_top_color.unwrap_or(fallback));
    }
    if style.border_bottom_width > 0.0 {
        fill(x, y + h - style.border_bottom_width, w, style.border_bottom_width, style.border_bottom_color.unwrap_or(fallback));
    }
    if style.border_left_width > 0.0 {
        fill(x, y, style.border_left_width, h, style.border_left_color.unwrap_or(fallback));
    }
    if style.border_right_width > 0.0 {
        fill(x + w - style.border_right_width, y, style.border_right_width, h, style.border_right_color.unwrap_or(fallback));
    }
}

/// 取「无单位数值」：解析器对 `opacity:.5` 这类值历史上会给出 `Length(_, Px)`，
/// 两种形态都接受，避免声明被静默忽略。
fn unitless_number(value: &StyleValue) -> Option<f32> {
    match value {
        StyleValue::Number(n) => Some(*n),
        StyleValue::Length(v, _) => Some(*v),
        StyleValue::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// 圆角矩形的有符号距离（负数在内部），按象限取对应圆角半径。
fn round_rect_sdf(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, radii: [f32; 4]) -> f32 {
    let (hw, hh) = (w / 2.0, h / 2.0);
    let (cx, cy) = (x + hw, y + hh);
    let (dx, dy) = (px - cx, py - cy);
    // radii 顺序：左上、右上、右下、左下
    let r = match (dx >= 0.0, dy >= 0.0) {
        (false, false) => radii[0],
        (true, false) => radii[1],
        (true, true) => radii[2],
        (false, true) => radii[3],
    };
    let r = r.min(hw).min(hh).max(0.0);
    let qx = dx.abs() - (hw - r);
    let qy = dy.abs() - (hh - r);
    let outside = (qx.max(0.0) * qx.max(0.0) + qy.max(0.0) * qy.max(0.0)).sqrt();
    outside + qx.max(qy).min(0.0) - r
}

/// 逐边着色的边框环：按盒子对角线把环分成上/右/下/左四段，各段用各自颜色。
///
/// 与浏览器一致的分界方式（对角线斜接）。用有符号距离场做 1px 抗锯齿带，
/// 因此对 `border-radius:50%` 的圆环同样正确。
pub fn stroke_round_rect_ring_sides(
    canvas: &mut Canvas,
    x: f32, y: f32, w: f32, h: f32,
    radii: [f32; 4],
    width: f32,
    colors: [Color; 4],
) {
    if width <= 0.0 || w <= 0.0 || h <= 0.0 { return; }
    let bw = width.min(w / 2.0).min(h / 2.0);
    let inner_radii = [
        (radii[0] - bw).max(0.0),
        (radii[1] - bw).max(0.0),
        (radii[2] - bw).max(0.0),
        (radii[3] - bw).max(0.0),
    ];
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = (x + w).ceil() as i32;
    let y1 = (y + h).ceil() as i32;

    for py in y0..y1 {
        for px in x0..x1 {
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            let d_out = round_rect_sdf(fx, fy, x, y, w, h, radii);
            let d_in = round_rect_sdf(fx, fy, x + bw, y + bw, w - bw * 2.0, h - bw * 2.0, inner_radii);
            // 在外轮廓内 且 在内轮廓外
            let cov_out = (0.5 - d_out).clamp(0.0, 1.0);
            let cov_in = (0.5 - d_in).clamp(0.0, 1.0);
            let coverage = cov_out * (1.0 - cov_in);
            if coverage <= 0.002 { continue; }

            // 归一化方向决定归属哪一边（对角线分界）
            let ndx = (fx - cx) / (w / 2.0).max(0.001);
            let ndy = (fy - cy) / (h / 2.0).max(0.001);
            let color = if ndy.abs() >= ndx.abs() {
                if ndy < 0.0 { colors[0] } else { colors[2] }
            } else if ndx > 0.0 {
                colors[1]
            } else {
                colors[3]
            };
            let alpha = (color.a as f32 * coverage).round().clamp(0.0, 255.0) as u8;
            if alpha > 0 {
                canvas.set_pixel(px, py, Color::new(color.r, color.g, color.b, alpha));
            }
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

/// 添加带有不同圆角的圆角矩形路径。
///
/// 圆角用三次贝塞尔逼近四分之一圆（控制点系数 0.5523），而不是「控制点落在角点」的
/// 单段二次贝塞尔——后者明显比真正的圆弧更方，`border-radius:50%` 会画成方角化的
/// 「squircle」而非圆形。
fn add_round_rect_with_radii(path: &mut Path, x: f32, y: f32, w: f32, h: f32, radii: [f32; 4]) {
    // 四分之一圆的三次贝塞尔控制点比例
    const K: f32 = 0.552_284_75;
    let max_r = (w.min(h) / 2.0).max(0.0);
    let clamp = |r: f32| r.max(0.0).min(max_r);
    let [tl, tr, br, bl] = [clamp(radii[0]), clamp(radii[1]), clamp(radii[2]), clamp(radii[3])];

    // 从左上角圆弧终点开始，顺时针
    path.move_to(x + tl, y);

    // 上边 → 右上角
    path.line_to(x + w - tr, y);
    if tr > 0.0 {
        path.cubic_to(
            x + w - tr + tr * K, y,
            x + w, y + tr - tr * K,
            x + w, y + tr,
        );
    }

    // 右边 → 右下角
    path.line_to(x + w, y + h - br);
    if br > 0.0 {
        path.cubic_to(
            x + w, y + h - br + br * K,
            x + w - br + br * K, y + h,
            x + w - br, y + h,
        );
    }

    // 下边 → 左下角
    path.line_to(x + bl, y + h);
    if bl > 0.0 {
        path.cubic_to(
            x + bl - bl * K, y + h,
            x, y + h - bl + bl * K,
            x, y + h - bl,
        );
    }

    // 左边 → 左上角
    path.line_to(x, y + tl);
    if tl > 0.0 {
        path.cubic_to(
            x, y + tl - tl * K,
            x + tl - tl * K, y,
            x + tl, y,
        );
    }

    path.close();
}
