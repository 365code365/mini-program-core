//! WXML 渲染器 - 使用组件系统渲染微信小程序

use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::parser::wxss::{StyleSheet, ElementDesc};
use crate::text::TextRenderer;
use crate::ui::interaction::{InteractionManager, InteractiveElement, InteractionType};
use crate::ui::scroll_cache::ScrollCacheManager;
use crate::{Canvas, Color, Rect as GeoRect};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use taffy::prelude::*;

use crate::renderer::components::{
    RenderNode, NodeStyle, ComponentContext, InheritedText, TextAlign,
    ViewComponent, TextComponent, ButtonComponent, IconComponent,
    ProgressComponent, SwitchComponent, CheckboxComponent, RadioComponent,
    SliderComponent, InputComponent, ImageComponent, VideoComponent,
    CanvasComponent, SwiperComponent, SwiperItemComponent, RichTextComponent,
    PickerComponent, PickerViewComponent, PickerViewColumnComponent,
    CheckboxGroupComponent, RadioGroupComponent,
    build_base_style, Tree, TextMeasure, measure_text_node, draw_background,
};

#[derive(Debug, Clone)]
pub struct EventBinding {
    pub event_type: String,
    pub handler: String,
    /// 声明这条绑定的标签名（`view` / `checkbox-group` / `input` …）。
    /// 事件语义与标签有关：`checkbox-group` 的 `change` 要发**组内选中项数组**，
    /// 而 `switch` 的 `change` 是个布尔值。
    pub tag: String,
    /// 节点的 `id` 属性（事件对象里的 `target.id` / `currentTarget.id`）。
    /// 没写 `id` 的节点这里是空串 —— 微信里 `e.target.id` 也确实是空串。
    pub id: String,
    /// 引擎内部句柄，与 `get_component_id` 一致（有 `id` 属性用它，否则 `tag_x_y`）。
    ///
    /// 存在的理由：宿主侧的输入框状态（焦点/文本）是按这个句柄索引的
    /// （`InteractionResult::Input*` 里的 `id`），而 `id` 属性绝大多数节点是空的。
    /// 没有它就只能「取第一条 event_type=="input" 的绑定」，一个页面有两个输入框时
    /// 所有输入都会打到第一个。
    pub component_id: String,
    pub data: HashMap<String, String>,
    pub bounds: GeoRect,
    /// 是否是 catch 事件（阻止继续传播）
    pub is_catch: bool,
    /// 传播阶段（捕获先于冒泡）
    pub phase: crate::renderer::components::EventPhase,
    /// `mut-bind:*`：互斥绑定，同一次传播里只触发最内层那一条
    pub mut_bind: bool,
    /// 声明这条绑定的自定义组件标签名（页面模板里声明的是空串）。
    /// 派发时优先送给该组件实例 —— 组件与页面的方法很容易同名。
    pub owner: String,
    /// 是否来自 `position: fixed` 覆盖层。
    /// 覆盖层的坐标是视口坐标（不含滚动偏移），而且它在页面之上 ——
    /// 命中判定必须先只看覆盖层，命中就到此为止，否则点击会穿透到下层。
    pub is_fixed: bool,
}

/// 一个 `<picker>` 在本帧的可点区域与它的选择器配置。
///
/// picker 的「点一下弹出选择面板」是宿主行为（微信是原生浮层），渲染器只负责
/// 把「哪里有一个 picker、它有哪些选项、当前选到第几个、变更回调叫什么」
/// 记录下来交给宿主。`bindchange` 本身不是可命中事件，光靠 `event_bindings` 找不到它。
#[derive(Debug, Clone)]
pub struct PickerBinding {
    /// 命中/按压用的组件 id（与 `get_component_id` 一致）
    pub id: String,
    /// 逻辑坐标包围盒（fixed 的是视口坐标，其余是内容坐标）
    pub bounds: GeoRect,
    /// selector / multiSelector / time / date / region
    pub mode: String,
    /// 单列选项（selector）
    pub range: Vec<String>,
    /// 多列选项（multiSelector）
    pub multi_range: Vec<Vec<String>>,
    /// 当前值：selector 是下标，time/date 是字符串，multiSelector/region 是下标数组
    pub value: String,
    /// `date` 模式的 `fields`（year / month / day）
    pub fields: String,
    /// `time`/`date` 模式的 `start` / `end`
    pub start: String,
    pub end: String,
    /// `bindchange` / `bind:change` 的处理函数名
    pub change_handler: Option<String>,
    pub disabled: bool,
    pub is_fixed: bool,
}

pub struct CachedLayout {
    pub render_nodes: Vec<RenderNode>,
    pub taffy: Tree,
    pub content_height: f32,
    pub data: JsonValue,
}

/// 在两个可选颜色之间插值：任一侧为 None 时按「另一侧的透明版本」处理，
/// 这样「常态无背景 → 按压态有背景」会淡入而不是硬切。
fn lerp_color_opt(a: Option<Color>, b: Option<Color>, t: f32) -> Option<Color> {
    match (a, b) {
        (Some(x), Some(y)) => Some(crate::renderer::anim::lerp_color(x, y, t)),
        (None, Some(y)) => Some(crate::renderer::anim::lerp_color(Color::new(y.r, y.g, y.b, 0), y, t)),
        (Some(x), None) => Some(crate::renderer::anim::lerp_color(x, Color::new(x.r, x.g, x.b, 0), t)),
        (None, None) => None,
    }
}

/// `wx.createAnimation` 求值出的样式覆盖值
#[derive(Clone)]
struct JsAnimValues {
    transform: crate::renderer::components::Transform,
    has_transform: bool,
    opacity: Option<f32>,
    background_color: Option<Color>,
}

/// 视口裁剪的余量（物理像素）：视口外这个距离内的节点仍然绘制，
/// 兜住阴影、溢出内容与滚动到来前的一小段预取。
///
/// 宿主据此只清理/只使用画布上的这一条带 —— 整页画布可能有上万像素高，
/// 每帧全量 clear 是纯浪费（首页那张 750x6870 的画布相当于每帧 20MB memset）。
pub const VIEWPORT_CULL_MARGIN_PX: f32 = 400.0;

/// 绘制上下文种类：顶层节点与子节点的绘制入口签名不同，
/// transform/动画处理需要在两者间复用同一套逻辑。
#[derive(Clone, Copy)]
enum DrawKind {
    Top { scroll_offset: f32, viewport_height: f32 },
    Child { inherited: Color, scroll_offset: f32, viewport_height: f32 },
}

pub struct WxmlRenderer {
    stylesheet: StyleSheet,
    /// 页面 json `usingComponents` 声明的自定义组件模板（标签名 → 组件 WXML）。
    /// 渲染时把 `<tab-bar/>` 这类标签展开成组件自己的模板 + 组件实例数据。
    component_templates: crate::parser::template::ComponentTemplates,
    screen_width: f32,
    screen_height: f32,
    event_bindings: Vec<EventBinding>,
    text_renderer: Option<std::sync::Arc<TextRenderer>>,
    scale_factor: f32,
    cache: Option<CachedLayout>,
    /// Scroll-view 离屏缓存管理器
    scroll_cache: ScrollCacheManager,
    /// 当前视口信息 (scroll_offset, viewport_height) - 用于虚拟列表
    current_viewport: Option<(f32, f32)>,
    /// `@keyframes` 时间轴缓存（按动画名，None 表示样式表里没有该动画）
    timelines: HashMap<String, Option<crate::renderer::anim::Timeline>>,
    /// 本帧是否有仍在推进的 CSS 动画（宿主据此决定是否继续出帧）
    animations_active: bool,
    /// 「标记过动画」的次数。用来判断**具体某个节点**有没有动画 ——
    /// 只看 `animations_active` 的话，第一个动画节点置真之后，
    /// 后面的节点全都看不出自己在动，损伤区就会漏掉它们（表现为只有第一个动画在跑）。
    anim_marks: u64,
    /// 动画时钟（秒）。宿主每帧写入，未设置时用全局时钟。
    anim_time: Option<f32>,
    /// 正在往 transform 离屏画布上绘制：此时坐标是画布局部坐标，
    /// 与屏幕/滚动空间无关，视口裁剪必须停用（否则整棵子树会被误剔除）。
    drawing_offscreen: bool,
    /// 本帧「有动画在跑」的节点包围盒（物理像素，画布坐标）。
    ///
    /// 宿主用它做损伤区重绘：一个 `infinite` 的小徽标不该逼着整屏每帧重新光栅化
    /// （浏览器是靠图层合成避免这件事的）。
    animated_bounds: Vec<GeoRect>,
    /// 本帧只重绘这个矩形（其余像素保持上一帧）。None 表示整条带都画。
    damage_clip: Option<GeoRect>,
    /// `wx.createAnimation` 载荷指纹 -> 播放起始时刻（秒）
    js_animations: HashMap<u64, f32>,
    /// 正在绘制 `position: fixed` 覆盖层（此期间注册的事件绑定标记为 fixed）
    registering_fixed: bool,
    /// 覆盖层各子树根在视口中的逻辑包围盒。
    /// 落在这些区域内的点击一律由覆盖层消费 —— 即使那里没有事件处理器
    /// （比如只写了半透明遮罩），也不能穿透到下层页面。
    fixed_hit_regions: Vec<GeoRect>,
    /// 本帧登记的 `<picker>`（宿主据此弹出原生选择面板）
    picker_regions: Vec<PickerBinding>,
}


mod animation;
mod draw;
mod draw_interactive;
mod entry;
mod fixed_layer;
mod hit_test;
mod interactions;
mod invalidate;
pub use invalidate::FramePlan;
pub(super) mod layout;
mod page_style;
pub use page_style::PageStyle;
mod pressed;
/// 第二遍布局修正：按实际几何收拾溢出（文本换行、绝对定位高度、滚动容器）
mod reflow;

impl WxmlRenderer {
    pub fn new(stylesheet: StyleSheet, screen_width: f32, screen_height: f32) -> Self {
        Self::new_with_scale(stylesheet, screen_width, screen_height, 1.0)
    }
    
    pub fn new_with_scale(stylesheet: StyleSheet, screen_width: f32, screen_height: f32, scale_factor: f32) -> Self {
        // 共享进程内唯一的字体实例：避免每次创建渲染器都重新加载上百 MB 字体
        let text_renderer = crate::text::shared_fonts();
        
        Self { 
            stylesheet, 
            component_templates: Default::default(),
            screen_width,
            screen_height,
            event_bindings: Vec::new(),
            text_renderer,
            scale_factor,
            cache: None,
            scroll_cache: ScrollCacheManager::new(),
            current_viewport: None,
            timelines: HashMap::new(),
            animations_active: false,
            anim_marks: 0,
            anim_time: None,
            drawing_offscreen: false,
            animated_bounds: Vec::new(),
            damage_clip: None,
            js_animations: HashMap::new(),
            registering_fixed: false,
            fixed_hit_regions: Vec::new(),
            picker_regions: Vec::new(),
        }
    }

}
