//! 类型定义：RenderNode / NodeStyle / 各枚举 / ComponentContext
//!
//! 由 `base/mod.rs` 组合。**纯搬迁**：从 1925 行的 base.rs 按职责切开，一行代码没改。
use super::*;

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
pub use crate::renderer::components::color_parse::{parse_color_str, parse_named_color};
pub use crate::renderer::components::style_parse::{
    parse_box_shadow, parse_transform, parse_border_shorthand, parse_border_side,
    parse_length_simple, parse_linear_gradient, parse_attr_json,
    split_top_commas, split_top_ws,
};
