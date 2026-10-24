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
    RenderNode, NodeStyle, ComponentContext, InheritedText, TextAlign, ButtonComponent, InputComponent, SwiperComponent,
    build_base_style, Tree, TextMeasure, measure_text_node, draw_background,
};

/// `<image>` 加载事件的 `detail`（与微信一致）
#[derive(Debug, Clone, Copy)]
pub enum ImageEventDetail {
    /// `bindload`：`{ width, height }` 是图片的**原始**像素尺寸
    Load { width: u32, height: u32 },
    /// `binderror`：`{ errMsg }`
    Error,
}

/// 一条待派发的 `<image>` 加载事件
#[derive(Debug, Clone)]
pub struct ImageEvent {
    /// `"load"` 或 `"error"`
    pub event_type: &'static str,
    pub handler: String,
    /// 声明它的自定义组件标签（页面模板里是空串）
    pub owner: String,
    /// 节点的 `id` 属性
    pub id: String,
    /// 节点的 `data-*`
    pub data: std::collections::HashMap<String, String>,
    pub detail: ImageEventDetail,
}

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
    /// `bindcolumnchange`（multiSelector 某一列滚动后）的处理函数名
    pub column_handler: Option<String>,
    /// `bindcancel`（取消 / 点遮罩收起）的处理函数名
    pub cancel_handler: Option<String>,
    /// `header-text`：面板头部「取消」「确定」之间的标题（官方 component/picker.html）
    pub header_text: String,
    pub disabled: bool,
    pub is_fixed: bool,
}

pub struct CachedLayout {
    pub render_nodes: Vec<RenderNode>,
    pub taffy: Tree,
    pub content_height: f32,
    pub data: JsonValue,
    /// 建这棵树用的模板展开结果（下一次 setData 拿它比，没变的子树原样留着，见 layout_incremental.rs）
    pub rendered: Vec<WxmlNode>,
    /// 页面根在 `taffy` 里的节点（增量重建时换它的孩子，不重建）
    pub root: NodeId,
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
    /// 本帧攒下的 `<image>` 加载事件，由宿主 `take_image_events()` 取走派发
    image_events: Vec<ImageEvent>,
    /// 已经发过的 `(组件 id, src, 事件名)`，保证每张图只发一次 load/error
    image_events_sent: std::collections::HashSet<(String, String, &'static str)>,
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
    /// 正在把 `opacity < 1` 的子树画进离屏缓冲（组合成）。
    /// 这一层自己的绘制用完全不透明，整体透明度在贴回时乘一次。
    flattening_opacity: bool,
    /// 离屏缓冲装不下时，退回「逐颜色乘 alpha」而不要再进组合成（否则递归）
    bypass_opacity_group: bool,
    /// 离屏绘制期间，事件绑定与命中盒要加回的逻辑偏移（让登记坐标仍是页面坐标）
    binding_shift: (f32, f32),
    /// 本帧「有动画在跑」的节点包围盒（物理像素，画布坐标）。
    ///
    /// 宿主用它做损伤区重绘：一个 `infinite` 的小徽标不该逼着整屏每帧重新光栅化
    /// （浏览器是靠图层合成避免这件事的）。
    animated_bounds: Vec<GeoRect>,
    /// 页面画布上各张动图画的是第几帧（跨帧保留，见 animated_image.rs）
    gif_marks: Vec<animated_image::AnimatedImageMark>,
    /// 这一趟走到的动图（趟末并进 `gif_marks`）
    gif_pass: Vec<animated_image::AnimatedImageMark>,
    /// 这一趟走的纵向窗口（物理像素，页面画布坐标）
    gif_window: (f32, f32),
    /// 布局树整棵重建过几次（宿主据此知道节点可能变了，见 `layout_generation`）
    layout_generation: u64,
    /// 正在进行的那串触摸锁定的派发目标（touch_capture.rs）。派发时只拿得到 `&self`，所以包一层锁
    touch_capture: std::sync::Mutex<Option<touch_capture::TouchCapture>>,
    /// fixed 子树根钉在视口上的原点（物理像素）与登记时的布局代数：SelectorQuery 按它报 fixed 节点的位置
    /// （fixed_layer.rs 登记、query.rs 读；布局重建过就作废）
    fixed_origins: (u64, HashMap<NodeId, (f32, f32)>),
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
    /// 本帧登记的输入框属性（宿主据此弹对应类型的软键盘，见 host_regions.rs）
    input_regions: Vec<InputRegion>,
    /// 本帧画到的、由宿主承载的原生组件
    native_regions: Vec<NativeRegion>,
    /// 宿主承载的原生组件标签（创建选项 `nativeComponents`）
    native_tags: Vec<String>,
    /// 本帧画到的 scroll-view（属性、阈值、下拉刷新配置，见 scroll_views.rs）
    scroll_views: Vec<ScrollViewInfo>,
    /// 本帧画到的 swiper 盒子（手势仲裁的候选，见 swiper_draw.rs）
    swiper_regions: Vec<SwiperRegion>,
    /// 画到过的每个 swiper 最近一次的盒子（跨帧留着：局部重画帧里没画到的也在）。
    /// SDK 的 swiper 换页 / 自动播放只重画这块（`host/engine_swiper.rs`）
    swiper_known: HashMap<String, SwiperRegion>,
    /// 上一次整棵重建各段的耗时（毫秒：模板 / 建树+样式 / 布局 / 换行修正 / 比对），
    /// 宿主取走接在 `⏱ [SDK] pump` 行尾（`take_rebuild_laps`）。时钟本来就在取，记下来不多花
    rebuild_laps: Option<[f32; 5]>,
    /// 布局之后才改的样式（换行补高、包含块修正、覆盖层重排）改之前的原样：增量重建时先还原，
    /// 留下来的子树才与「重新建」时的样式一致（layout_incremental.rs）。只记缓存那棵树上的
    layout_touched: std::cell::RefCell<HashMap<NodeId, Style>>,
    /// 正在改的是缓存那棵树（要记进 `layout_touched`）
    record_touch: std::cell::Cell<bool>,
    /// 上一次重建留下了几个子树、重建了几个节点（诊断与测试，见 `last_reuse`）
    reuse_stats: (usize, usize),
}


/// 动图（GIF）换帧才重画：画布上是第几帧、哪几张该换了
mod animated_image;
mod animation;
/// `wx.createAnimation()` 的逐步插值（从 animation.rs 拆出）
mod js_animation;
/// 触摸序列锁定在 touchstart 的节点上（touchmove / touchend / touchcancel 都归它）
mod touch_capture;
mod draw;
mod draw_interactive;
mod entry;
mod fixed_layer;
mod hit_test;
/// 交给宿主的区域：输入框属性、原生组件摆放
mod host_regions;
pub use host_regions::{InputRegion, NativeRegion};
/// `<scroll-view>` 的属性、受控位置与下拉刷新配置
pub mod scroll_views;
/// scroll-view 下拉刷新的默认样式（背景 + 三个点）
mod refresher;
/// 按选择器找节点与位置（`wx.pageScrollTo({selector})`、SelectorQuery）
mod query;
pub use query::QueryHit;
/// `<form>`：form-type 按钮所在的表单与表单里的控件（提交 / 重置）
mod form;
pub use form::{FormAction, FormControl, FormField, FormHit};
/// `<navigator>` 的跳转与 `<label>` 的控件转发（点击命中，走布局树）
mod builtin_hits;
pub use builtin_hits::NavigatorHit;
/// swiper 的分页绘制（三条绘制路径共用）与手势候选登记
mod swiper_draw;
/// `<picker-view>` 的分列偏移绘制、遮罩与指示条、列的手势候选
mod picker_view_draw;
pub use swiper_draw::SwiperRegion;
/// 有 swiper 的顶层 scroll-view 不走离屏缓存
mod scroll_live;
pub use scroll_views::{RefresherConfig, ScrollViewInfo};
mod interactions;
/// 有状态组件的状态落地与带交互的绘制（唯一一份，三条绘制路径共用）
mod interactive;
mod invalidate;
/// 内容变高（列表追加）时的失效范围：从最高的变化往下整宽重画
mod invalidate_grow;
/// 失效分析的孩子配对：fixed 的单独配、内容变高时配不上的两段各走一遍
mod invalidate_pair;
/// 带静态 transform 的子树：按绘制几何算旧、新两次画到哪
mod invalidate_transform;
pub use invalidate::FramePlan;
pub(super) mod layout;
/// setData 的增量布局：没变的子树（连同 taffy 节点与它的布局缓存）原样留着，只重建变了的
mod layout_incremental;
/// 排版那几步（根样式、排一遍 + 修正、文字度量、量内容高），整棵 / 增量 / 校验共用
mod layout_pass;
/// 失效分析读布局的两种来源：一棵 taffy 树，或增量重建前拍下的旧布局
mod layout_snapshot;
/// `MINI_LAYOUT_VERIFY=1`：增量重建之后再整棵重建一遍逐节点比（测试用）
mod layout_verify;
pub use layout_verify::layout_verify_failures;
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
        let mut stylesheet = stylesheet;
        stylesheet.ensure_button_hover();

        Self { 
            stylesheet, 
            component_templates: Default::default(),
            screen_width,
            screen_height,
            event_bindings: Vec::new(),
            image_events: Vec::new(),
            image_events_sent: Default::default(),
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
            flattening_opacity: false,
            bypass_opacity_group: false,
            binding_shift: (0.0, 0.0),
            animated_bounds: Vec::new(),
            gif_marks: Vec::new(),
            gif_pass: Vec::new(),
            gif_window: (f32::NEG_INFINITY, f32::INFINITY),
            layout_generation: 0,
            touch_capture: std::sync::Mutex::new(None),
            fixed_origins: (0, HashMap::new()),
            layout_touched: Default::default(),
            record_touch: Default::default(),
            reuse_stats: (0, 0),
            damage_clip: None,
            js_animations: HashMap::new(),
            registering_fixed: false,
            fixed_hit_regions: Vec::new(),
            picker_regions: Vec::new(),
            input_regions: Vec::new(),
            native_regions: Vec::new(),
            native_tags: Vec::new(),
            scroll_views: Vec::new(),
            swiper_regions: Vec::new(),
            swiper_known: HashMap::new(),
            rebuild_laps: None,
        }
    }

    /// 取走上一次整棵重建的分段耗时（模板 / 建树+样式 / 布局 / 换行修正 / 比对，毫秒）
    pub fn take_rebuild_laps(&mut self) -> Option<[f32; 5]> {
        self.rebuild_laps.take()
    }

    /// 布局树整棵重建过几次。没变就说明节点（及其属性）都没变 —— 宿主不必每帧遍历树
    /// 找它关心的节点（同层渲染的视频列表，`host/engine_video_feed.rs`）
    pub fn layout_generation(&self) -> u64 {
        self.layout_generation
    }
}
