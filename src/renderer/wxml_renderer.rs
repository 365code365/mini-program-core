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

use super::components::{
    RenderNode, NodeStyle, ComponentContext, InheritedText, TextAlign, WhiteSpace,
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
    pub data: HashMap<String, String>,
    pub bounds: GeoRect,
    /// 是否是 catch 事件（阻止冒泡）
    pub is_catch: bool,
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
    transform: super::components::Transform,
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
}

impl WxmlRenderer {
    pub fn new(stylesheet: StyleSheet, screen_width: f32, screen_height: f32) -> Self {
        Self::new_with_scale(stylesheet, screen_width, screen_height, 1.0)
    }
    
    pub fn new_with_scale(stylesheet: StyleSheet, screen_width: f32, screen_height: f32, scale_factor: f32) -> Self {
        // 共享进程内唯一的字体实例：避免每次创建渲染器都重新加载上百 MB 字体
        let text_renderer = crate::text::shared_fonts();
        
        Self { 
            stylesheet, 
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
        }
    }

    /// 取走本帧记录的动画节点包围盒（物理像素，画布坐标）
    pub fn take_animated_bounds(&mut self) -> Vec<GeoRect> {
        std::mem::take(&mut self.animated_bounds)
    }

    /// 标记「本帧有动画在跑」，同时推进计数器
    fn mark_animating(&mut self) {
        self.animations_active = true;
        self.anim_marks += 1;
    }

    /// 直接设置「本帧有动画」标记。
    ///
    /// `render_fixed_elements` 是接着页面那一趟累加的，宿主要分别知道
    /// 「页面在动」和「fixed 覆盖层在动」（后者不能走局部重绘），
    /// 于是需要在两趟之间清零再合并回来。
    pub fn set_animations_active(&mut self, active: bool) {
        self.animations_active = active;
    }

    /// 限定本帧只重绘这个矩形（物理像素，画布坐标）。每帧渲染前设置，渲染后自动清空。
    pub fn set_damage_clip(&mut self, rect: Option<GeoRect>) {
        self.damage_clip = rect;
    }

    /// 本帧是否存在仍在推进的 CSS 动画。宿主用它决定「继续按刷新率出帧」还是「空闲休眠」。
    pub fn has_active_animations(&self) -> bool {
        self.animations_active
    }

    /// 开关类组件（switch）的状态过渡进度：点击后在 `TOGGLE_DURATION` 内从旧值滑到新值。
    ///
    /// 微信的开关滑块是带缓动的位移（weui 用 cubic-bezier(.4,.4,.25,1.35) 略带回弹），
    /// 直接跳变会失去"拨动"手感。
    fn toggle_progress(
        &mut self,
        interaction: &InteractionManager,
        id: &str,
        checked: bool,
    ) -> f32 {
        const TOGGLE_DURATION: f32 = 0.3;
        let target = if checked { 1.0 } else { 0.0 };
        let Some(started) = interaction.transitions.get(id).copied() else { return target };
        let elapsed = self.animation_time() - started;
        if elapsed < 0.0 || elapsed >= TOGGLE_DURATION {
            return target;
        }
        self.mark_animating();
        let eased = crate::renderer::anim::cubic_bezier(elapsed / TOGGLE_DURATION, 0.4, 0.4, 0.25, 1.35);
        if checked { eased.clamp(0.0, 1.0) } else { (1.0 - eased).clamp(0.0, 1.0) }
    }

    /// 把按压态样式应用到待绘制节点上。
    ///
    /// 有 `transition` 时在常态与按压态之间按缓动插值（只插值绘制类属性：
    /// 背景色、文字色、边框色、透明度）；没有就直接切换。
    /// 插值期间标记「本帧有动画」，宿主才会继续出帧把过渡走完。
    fn apply_pressed_style(
        &mut self,
        node_to_draw: &mut RenderNode,
        interaction: &InteractionManager,
        component_id: &str,
    ) {
        let Some(pressed) = node_to_draw.style.pressed_style.clone() else { return };
        let is_pressed = interaction.is_button_pressed(component_id);
        let spec = node_to_draw.style.transition;

        // 过渡进度：0 = 常态，1 = 按压态
        let progress = match spec {
            Some(t) if t.duration > 0.0 => {
                let started = interaction.transitions.get(component_id).copied();
                match started {
                    Some(start) => {
                        let elapsed = self.animation_time() - start - t.delay;
                        let raw = (elapsed / t.duration).clamp(0.0, 1.0);
                        if elapsed < t.duration {
                            self.mark_animating();
                        }
                        let eased =
                            crate::renderer::anim::cubic_bezier(raw, t.curve.0, t.curve.1, t.curve.2, t.curve.3);
                        if is_pressed { eased } else { 1.0 - eased }
                    }
                    // 没有记录过起始时刻：直接给终态
                    None => if is_pressed { 1.0 } else { 0.0 },
                }
            }
            _ => if is_pressed { 1.0 } else { 0.0 },
        };

        if progress <= 0.001 {
            return;
        }
        if progress >= 0.999 {
            let keep = node_to_draw.style.pressed_style.take();
            let transition = node_to_draw.style.transition;
            node_to_draw.style = *pressed;
            node_to_draw.style.pressed_style = keep;
            node_to_draw.style.transition = transition;
            return;
        }
        // 中间态：只插值绘制类属性
        let base = &mut node_to_draw.style;
        base.background_color = lerp_color_opt(base.background_color, pressed.background_color, progress);
        base.text_color = lerp_color_opt(base.text_color, pressed.text_color, progress);
        base.border_color = lerp_color_opt(base.border_color, pressed.border_color, progress);
        base.opacity += (pressed.opacity - base.opacity) * progress;
        if pressed.transform.is_some() || base.transform.is_some() {
            let from = base.transform.unwrap_or_else(super::components::Transform::new);
            let to = pressed.transform.unwrap_or_else(super::components::Transform::new);
            base.transform = Some(crate::renderer::anim::lerp_transform(from, to, progress));
        }
    }

    /// 显式设置动画时钟（秒）。用于静态截图/测试等需要确定性时间的场景；
    /// 不设置时使用进程内全局时钟。
    pub fn set_animation_time(&mut self, seconds: f32) {
        self.anim_time = Some(seconds);
    }

    fn animation_time(&self) -> f32 {
        self.anim_time.unwrap_or_else(crate::renderer::anim::now_secs)
    }

    /// 取（并缓存）动画名对应的时间轴
    fn timeline_for(&mut self, name: &str) -> Option<&crate::renderer::anim::Timeline> {
        if !self.timelines.contains_key(name) {
            let built = self
                .stylesheet
                .keyframes_named(name)
                .map(|rule| {
                    crate::renderer::anim::Timeline::from_rule(rule, self.screen_width, self.scale_factor)
                })
                .filter(|tl| !tl.is_empty());
            self.timelines.insert(name.to_string(), built);
        }
        self.timelines.get(name).and_then(|t| t.as_ref())
    }

    /// 求节点此刻的 CSS 动画覆盖值；顺带标记「本帧还有动画在跑」。
    fn animated_values(&mut self, node: &RenderNode) -> Option<crate::renderer::anim::AnimatedValues> {
        let spec = node.style.animation.clone()?;
        let now = self.animation_time();
        // 无限循环 / 未结束的动画都要求宿主继续出帧
        let still_running = !spec.iterations.is_finite()
            || now < spec.delay + spec.duration * spec.iterations;
        if still_running {
            self.mark_animating();
        }
        let progress = spec.progress_at(now)?;
        // 元素自身的计算值作为隐式 0%/100% 关键帧（CSS 语义）
        let base = crate::renderer::anim::FrameValues {
            offset: 0.0,
            opacity: Some(node.style.opacity),
            transform: Some(node.style.transform.unwrap_or_else(super::components::Transform::new)),
            background_color: node.style.background_color,
            text_color: node.style.text_color,
        };
        let timeline = self.timeline_for(&spec.name)?;
        let values = timeline.sample_with_base(progress, &base);
        if values.is_empty() {
            None
        } else {
            Some(values)
        }
    }

    /// 求 `animation="{{animData}}"`（`wx.createAnimation().export()`）此刻的样式覆盖值。
    ///
    /// 载荷形如 `{actions:[{animates:[{type,args}],option:{transition:{duration,delay,timingFunction}}}]}`，
    /// 每个 action 是一「步」：按 delay/duration 顺序播放，步与步之间对目标值做插值。
    /// 起始时刻按载荷指纹记忆 —— 同一份 export 在后续帧里继续播，换了新的 export 就重新开始。
    fn js_animated_values(&mut self, node: &RenderNode) -> Option<JsAnimValues> {
        let raw = node.attrs.get("animation")?;
        if !raw.contains("actions") {
            return None;
        }
        // 属性里的对象被序列化成单引号 JSON（见 expr::render_value），这里还原
        let parsed: JsonValue = serde_json::from_str(&raw.replace('\'', "\"")).ok()?;
        let actions = parsed.get("actions")?.as_array()?;
        if actions.is_empty() {
            return None;
        }

        let fingerprint = {
            // FNV-1a：只用来判断「还是不是同一份 export」
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for b in raw.as_bytes() {
                h ^= *b as u64;
                h = h.wrapping_mul(0x1000_0000_01b3);
            }
            h
        };
        let now = self.animation_time();
        let start = match self.js_animations.get(&fingerprint) {
            Some(s) => *s,
            None => {
                if self.js_animations.len() > 64 {
                    self.js_animations.clear();
                }
                self.js_animations.insert(fingerprint, now);
                now
            }
        };
        let elapsed = now - start;

        let sf = self.scale_factor;
        let mut state = JsAnimValues {
            transform: super::components::Transform::new(),
            has_transform: false,
            opacity: None,
            background_color: None,
        };
        let mut cursor = 0.0f32;
        let mut finished = true;
        for action in actions {
            let tr = action.get("option").and_then(|o| o.get("transition"));
            let ms = |k: &str, d: f32| {
                tr.and_then(|t| t.get(k)).and_then(|v| v.as_f64()).unwrap_or(d as f64) as f32 / 1000.0
            };
            let duration = ms("duration", 400.0).max(0.0);
            let delay = ms("delay", 0.0).max(0.0);
            let curve = tr
                .and_then(|t| t.get("timingFunction"))
                .and_then(|v| v.as_str())
                .map(super::components::parse_timing_function)
                .unwrap_or((0.0, 0.0, 1.0, 1.0)); // createAnimation 默认 linear

            let target = Self::apply_js_animates(&state, action.get("animates"), sf);
            let step_start = cursor + delay;
            let step_end = step_start + duration;
            if elapsed >= step_end || duration <= 0.0 {
                state = target;
                cursor = step_end;
                continue;
            }
            finished = false;
            let p = if elapsed <= step_start {
                0.0
            } else {
                (elapsed - step_start) / duration
            };
            let eased = crate::renderer::anim::cubic_bezier(p, curve.0, curve.1, curve.2, curve.3);
            state = JsAnimValues {
                transform: crate::renderer::anim::lerp_transform(state.transform, target.transform, eased),
                has_transform: state.has_transform || target.has_transform,
                opacity: match (state.opacity, target.opacity) {
                    (Some(a), Some(b)) => Some(a + (b - a) * eased),
                    (None, Some(b)) => Some(1.0 + (b - 1.0) * eased),
                    (a, None) => a,
                },
                background_color: match (state.background_color, target.background_color) {
                    (Some(a), Some(b)) => Some(crate::renderer::anim::lerp_color(a, b, eased)),
                    (None, Some(b)) => Some(b),
                    (a, None) => a,
                },
            };
            break;
        }
        if !finished {
            self.mark_animating();
        }
        Some(state)
    }

    /// 把一步里的 animates 累积到状态上（每步给的是绝对目标值，未提及的属性沿用上一步）
    fn apply_js_animates(
        base: &JsAnimValues,
        animates: Option<&JsonValue>,
        sf: f32,
    ) -> JsAnimValues {
        let mut out = base.clone();
        let Some(list) = animates.and_then(|a| a.as_array()) else { return out };
        for item in list {
            let ty = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let args = item.get("args").and_then(|v| v.as_array());
            let num = |i: usize, d: f32| {
                args.and_then(|a| a.get(i))
                    .and_then(|v| v.as_f64())
                    .map(|v| v as f32)
                    .unwrap_or(d)
            };
            match ty {
                // translate 的参数是逻辑 px，Transform 用物理 px
                "translate" => {
                    out.transform.translate_x = num(0, 0.0) * sf;
                    out.transform.translate_y = num(1, 0.0) * sf;
                    out.has_transform = true;
                }
                "translateX" => { out.transform.translate_x = num(0, 0.0) * sf; out.has_transform = true; }
                "translateY" => { out.transform.translate_y = num(0, 0.0) * sf; out.has_transform = true; }
                "rotate" => { out.transform.rotate = num(0, 0.0); out.has_transform = true; }
                "scale" => {
                    let sx = num(0, 1.0);
                    out.transform.scale_x = sx;
                    out.transform.scale_y = num(1, sx);
                    out.has_transform = true;
                }
                "scaleX" => { out.transform.scale_x = num(0, 1.0); out.has_transform = true; }
                "scaleY" => { out.transform.scale_y = num(0, 1.0); out.has_transform = true; }
                "skew" => {
                    out.transform.skew_x = num(0, 0.0);
                    out.transform.skew_y = num(1, 0.0);
                    out.has_transform = true;
                }
                "opacity" => out.opacity = Some(num(0, 1.0).clamp(0.0, 1.0)),
                "backgroundColor" => {
                    if let Some(c) = args
                        .and_then(|a| a.first())
                        .and_then(|v| v.as_str())
                        .and_then(super::components::parse_color_str)
                    {
                        out.background_color = Some(c);
                    }
                }
                // width/height 会引发重排，与 CSS 动画同样的取舍：不支持
                _ => {}
            }
        }
        out
    }

    /// 把动画求值结果 + 静态 transform 合成成「本帧要用的节点」。
    /// 返回 None 表示该节点本帧无需特殊处理（走普通绘制路径）。
    fn resolve_animated_node(
        &mut self,
        node: &RenderNode,
    ) -> Option<(RenderNode, super::components::Transform)> {
        let values = self.animated_values(node);
        let js = self.js_animated_values(node);
        let has_values = values.is_some() || js.is_some();
        let mut transform = values
            .as_ref()
            .and_then(|v| v.transform)
            .or(node.style.transform);
        if let Some(j) = &js {
            if j.has_transform {
                transform = Some(j.transform);
            }
        }
        if !has_values && transform.is_none() {
            return None;
        }
        let mut resolved = node.clone();
        // 动画值已在此处落地，避免离屏重绘时再次求值（否则会无限递归）。
        // `animation` 属性（wx.createAnimation 的载荷）同样要摘掉 —— 只清 style
        // 不清属性的话，重入这条路径时又会解析出动画，直接栈溢出。
        resolved.style.animation = None;
        resolved.style.transform = None;
        resolved.attrs.remove("animation");
        if let Some(v) = &values {
            if let Some(op) = v.opacity {
                resolved.style.opacity = (node.style.opacity * op).clamp(0.0, 1.0);
            }
            if let Some(bg) = v.background_color {
                resolved.style.background_color = Some(bg);
                resolved.style.background_gradient = None;
            }
            if let Some(color) = v.text_color {
                resolved.style.text_color = Some(color);
            }
        }
        // JS 动画（wx.createAnimation）优先级高于 CSS 动画：它是逻辑层刚下发的目标态
        if let Some(j) = &js {
            if let Some(op) = j.opacity {
                resolved.style.opacity = (node.style.opacity * op).clamp(0.0, 1.0);
            }
            if let Some(bg) = j.background_color {
                resolved.style.background_color = Some(bg);
                resolved.style.background_gradient = None;
            }
        }
        Some((resolved, transform.unwrap_or_else(super::components::Transform::new)))
    }

    /// 绘制期的**浅拷贝**：只复制这一个节点自身（标签/文本/属性/样式），不复制子树。
    ///
    /// 绘制路径为了写入继承色、勾选态等要改样式，原来直接 `node.clone()`。
    /// 而 `RenderNode` 自己持有 `children: Vec<RenderNode>`，于是「每个节点都克隆一遍
    /// 自己的整棵子树」—— 越靠近根越贵，整体是 O(n²)。首页那种 6800px 长页面每帧
    /// 光克隆就吃掉十几毫秒（页面有动画时每帧都发生），这是"卡"的主因之一。
    ///
    /// 组件绘制只用到自身的 tag/text/attrs/style；需要知道"有没有子节点"的
    /// rich-text / picker 已改为读属性标记，swiper 走独立容器路径拿原节点。
    fn shallow_for_draw(node: &RenderNode) -> RenderNode {
        RenderNode {
            tag: node.tag.clone(),
            text: node.text.clone(),
            attrs: node.attrs.clone(),
            taffy_node: node.taffy_node,
            style: node.style.clone(),
            children: Vec::new(),
            events: Vec::new(),
        }
    }

    /// 视口裁剪：子树完全落在可见区之外时跳过整棵绘制。
    ///
    /// 宿主把整页内容画进一张「内容高」的长画布，再按滚动位置 blit 可见的一屏。
    /// 页面有 CSS 动画时每帧都要重绘，于是首页那种 6800px 高的长页面每帧都在
    /// 光栅化 5M 像素（实测 49.6ms/帧 ≈ 20FPS，肉眼就是卡）。
    /// 可见区之外的内容 blit 时根本读不到，绘制它纯属浪费。
    ///
    /// 留一段余量并跳过带 transform/动画的节点：它们的实际绘制范围可能超出布局盒
    /// （离屏仿射、位移），不能只按盒子判断。
    fn cull_outside_viewport(
        &self,
        node: &RenderNode,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        scroll_offset: f32,
        viewport_height: f32,
    ) -> bool {
        // 离屏合成时坐标是画布局部坐标，和滚动空间没有对应关系，不能用它裁剪
        if self.drawing_offscreen {
            return false;
        }
        if node.style.transform.is_some() || node.style.animation.is_some() {
            return false;
        }
        const MARGIN: f32 = VIEWPORT_CULL_MARGIN_PX;
        // 横向：画布宽就是屏宽，横滑列表里被推到屏幕外的项同样不可见
        let canvas_w = self.screen_width * self.scale_factor;
        if x + w < -MARGIN || x > canvas_w + MARGIN {
            return true;
        }
        if viewport_height <= 0.0 {
            return false;
        }
        let visible_top = scroll_offset * self.scale_factor - MARGIN;
        let visible_bottom = scroll_offset * self.scale_factor + viewport_height + MARGIN;
        y + h < visible_top || y > visible_bottom
    }

    /// 绘制 swiper 容器：裁剪到自身盒子 → 整行按当前页横向偏移 → 逐项走普通绘制 → 指示点。
    ///
    /// 只有"当前页在哪儿"是 swiper 特有的，item 内容（图片、浮层、flex、动画、
    /// 绝对定位）全部复用通用绘制路径，不再自成一套。
    fn draw_swiper_container(
        &mut self,
        canvas: &mut Canvas,
        taffy: &Tree,
        node: &RenderNode,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        interaction: &mut InteractionManager,
        kind: DrawKind,
    ) {
        let sf = self.scale_factor;
        draw_background(canvas, &node.style, x, y, w, h);
        if node.children.is_empty() || w <= 0.0 || h <= 0.0 {
            return;
        }
        let current = SwiperComponent::current_index(node, x, y);
        // 换页是滑过去的（与微信一致）：拿到 prev→current 之间的浮点页号。
        // 只有滑动进行中才要求继续出帧 —— 从前是「有多于一项就每帧重绘」，
        // 而 swiper 平时根本不动，等于让带轮播的页面永远整屏重绘。
        let (page_pos, sliding) = SwiperComponent::slide_position(node, x, y)
            .unwrap_or((current as f32, false));
        if sliding {
            self.mark_animating();
        }

        canvas.save();
        canvas.clip_rect(GeoRect::new(x, y, w, h));
        // 子项在 taffy 里排成一行/一列（每项一屏），整体沿主轴移动 page_pos 屏
        let vertical = SwiperComponent::is_vertical(node);
        let (dx, dy) = if vertical {
            (0.0, -page_pos * h)
        } else {
            (-page_pos * w, 0.0)
        };
        // 滑动中要画相邻两页（否则中间过程一侧是空白）；静止时只画当前页 ——
        // 非当前页被裁剪后完全不可见，逐帧重采样它们的图片纯属浪费。
        let first = page_pos.floor().max(0.0) as usize;
        let last = if sliding { page_pos.ceil().max(0.0) as usize } else { first };
        for idx in first..=last.min(node.children.len().saturating_sub(1)) {
            if let Some(child) = node.children.get(idx) {
                self.dispatch_draw(canvas, taffy, child, x + dx, y + dy, interaction, kind);
            }
        }
        canvas.restore();

        SwiperComponent::draw_indicators(node, canvas, x, y, w, h, sf, current);
    }

    /// 分发到对应的绘制路径（顶层节点 / 子节点两套上下文）
    fn dispatch_draw(
        &mut self,
        canvas: &mut Canvas,
        taffy: &Tree,
        node: &RenderNode,
        ox: f32,
        oy: f32,
        interaction: &mut InteractionManager,
        kind: DrawKind,
    ) {
        match kind {
            DrawKind::Top { scroll_offset, viewport_height } => {
                self.draw_with_interaction(canvas, taffy, node, ox, oy, interaction, scroll_offset, viewport_height)
            }
            DrawKind::Child { inherited, scroll_offset, viewport_height } => self
                .draw_child_with_interaction(canvas, taffy, node, ox, oy, inherited, interaction, scroll_offset, viewport_height),
        }
    }

    /// 处理节点的 CSS 动画与 `transform`。返回 true 表示本节点（含子树）已绘制完成。
    ///
    /// 三条路径：
    /// 1. 只有颜色/透明度在动 → 直接用求值后的节点走普通绘制；
    /// 2. 纯平移 → 加偏移绘制（子树、命中区都自然跟随）；
    /// 3. 含缩放/旋转/倾斜 → 子树画到离屏画布再仿射贴回。
    fn draw_with_transform(
        &mut self,
        canvas: &mut Canvas,
        taffy: &Tree,
        node: &RenderNode,
        ox: f32,
        oy: f32,
        interaction: &mut InteractionManager,
        kind: DrawKind,
    ) -> bool {
        let marks_before = self.anim_marks;
        let Some((resolved, transform)) = self.resolve_animated_node(node) else { return false };
        // 这个节点自己有动画在跑的话，记下它的包围盒（含 transform 溢出余量），
        // 宿主据此只重绘这些小块而不是整屏。按计数器判断而不是看全局标记 ——
        // 后者被前一个动画节点置真后，后续节点就无法自证「我在动」。
        if self.anim_marks != marks_before && !self.drawing_offscreen {
            if let Ok(layout) = taffy.layout(node.taffy_node) {
                let (x, y) = (ox + layout.location.x, oy + layout.location.y);
                let (w, h) = (layout.size.width, layout.size.height);
                let pad = super::compose::transform_padding(w, h, &transform).max(2.0);
                self.animated_bounds.push(GeoRect::new(
                    x - pad,
                    y - pad,
                    w + pad * 2.0,
                    h + pad * 2.0,
                ));
            }
        }

        if super::compose::is_identity(&transform) {
            self.dispatch_draw(canvas, taffy, &resolved, ox, oy, interaction, kind);
            return true;
        }
        if super::compose::is_translate_only(&transform) {
            self.dispatch_draw(
                canvas,
                taffy,
                &resolved,
                ox + transform.translate_x,
                oy + transform.translate_y,
                interaction,
                kind,
            );
            return true;
        }

        let layout = taffy.layout(node.taffy_node).unwrap();
        let (x, y) = (ox + layout.location.x, oy + layout.location.y);
        let (w, h) = (layout.size.width, layout.size.height);
        let pad = super::compose::transform_padding(w, h, &transform);
        let tw = (w + pad * 2.0).ceil() as u32;
        let th = (h + pad * 2.0).ceil() as u32;
        if w <= 0.0 || h <= 0.0 || tw == 0 || th == 0 || tw > 4096 || th > 4096 {
            // 尺寸异常（含 0 尺寸叶子）时退回不变换绘制，避免分配巨大离屏画布
            self.dispatch_draw(canvas, taffy, &resolved, ox, oy, interaction, kind);
            return true;
        }

        let mut offscreen = Canvas::new(tw, th);
        offscreen.clear(Color::TRANSPARENT);
        // 变换后的子树坐标与屏幕不再一一对应：命中区与事件绑定用一次性容器承接，
        // 不污染真实的交互命中表（旋转/缩放子树内的点击是已知限制）。
        let mut scratch = InteractionManager::new();
        let bindings_before = self.event_bindings.len();
        let was_offscreen = self.drawing_offscreen;
        self.drawing_offscreen = true;
        self.dispatch_draw(
            &mut offscreen,
            taffy,
            &resolved,
            pad - layout.location.x,
            pad - layout.location.y,
            &mut scratch,
            kind,
        );
        self.drawing_offscreen = was_offscreen;
        self.event_bindings.truncate(bindings_before);

        super::compose::blit_transformed(
            canvas,
            &offscreen,
            (x - pad, y - pad),
            (x + w / 2.0, y + h / 2.0),
            &transform,
            1.0,
        );
        true
    }

    fn update_layout_if_needed(
        &mut self,
        nodes: &[WxmlNode],
        data: &JsonValue,
        viewport: Option<(f32, f32)>,
    ) {
        // 检查视口是否变化（用于虚拟列表）
        let viewport_changed = self.current_viewport != viewport;
        
        if let Some(cache) = &self.cache {
            if cache.data == *data && !viewport_changed {
                return; // Cache hit!
            }
        }
        
        // 更新当前视口
        self.current_viewport = viewport;
        
        // 数据变化，标记所有 scroll-view 缓存为脏
        self.scroll_cache.mark_all_dirty();
        
        // 重建耗时诊断（`MINI_LAYOUT_LOG=1`）：一次 setData 会走完整条
        // 「模板求值 → 建树/样式 → 布局」流水线，是交互卡顿的主要来源，
        // 分段计时能直接指出该优化哪一段。
        let log_timing = std::env::var("MINI_LAYOUT_LOG").is_ok();
        let t_start = std::time::Instant::now();
        let rendered = crate::parser::TemplateEngine::render_with_virtual_list(nodes, data, viewport);
        let t_template = std::time::Instant::now();
        let mut taffy = Tree::new();
        
        let mut render_nodes = Vec::new();
        
        for (sib_i, node) in rendered.iter().enumerate() {
            if let Some(rn) = self.build_tree(&mut taffy, node, &[], &InheritedText::default(), sib_i, rendered.len()) {
                render_nodes.push(rn);
            }
        }
        let t_build = std::time::Instant::now();
        
        // 构建正常布局树（包含所有节点，fixed 元素也参与布局计算）
        let child_ids: Vec<NodeId> = render_nodes.iter().map(|n| n.taffy_node).collect();
        let root = taffy.new_with_children(
            Style {
                size: Size { width: length(self.screen_width * self.scale_factor), height: auto() },
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &child_ids,
        ).unwrap();
        
        self.compute_with_text(&mut taffy, root, Size::MAX_CONTENT);
        let t_layout1 = std::time::Instant::now();
        // 第二遍：按实际宽度修正换行文本高度后重新布局
        if self.correct_wrapped_text_heights(&mut taffy, &render_nodes) {
            self.compute_with_text(&mut taffy, root, Size::MAX_CONTENT);
        }
        if log_timing {
            let ms = |a: std::time::Instant, b: std::time::Instant| (b - a).as_secs_f32() * 1000.0;
            eprintln!(
                "⏱  重建布局：模板 {:.1}ms  建树+样式 {:.1}ms  布局一遍 {:.1}ms  换行修正 {:.1}ms",
                ms(t_start, t_template),
                ms(t_template, t_build),
                ms(t_build, t_layout1),
                ms(t_layout1, std::time::Instant::now()),
            );
        }
        
        // 获取实际内容高度
        let root_layout = taffy.layout(root).unwrap();
        let content_height = root_layout.size.height / self.scale_factor;
        
        self.cache = Some(CachedLayout {
            render_nodes,
            taffy,
            content_height,
            data: data.clone(),
        });
    }

    /// 渲染 WXML 节点，使用交互管理器处理状态
    pub fn render_with_interaction(
        &mut self, 
        canvas: &mut Canvas, 
        nodes: &[WxmlNode], 
        data: &JsonValue,
        interaction: &mut InteractionManager,
    ) {
        self.render_with_scroll_and_viewport(canvas, nodes, data, interaction, 0.0, self.screen_height);
    }
    
    /// 渲染 WXML 节点，支持滚动偏移（用于 fixed 定位）
    pub fn render_with_scroll(
        &mut self, 
        canvas: &mut Canvas, 
        nodes: &[WxmlNode], 
        data: &JsonValue,
        interaction: &mut InteractionManager,
        scroll_offset: f32,
    ) {
        self.render_with_scroll_and_viewport(canvas, nodes, data, interaction, scroll_offset, self.screen_height);
    }
    
    /// 渲染 WXML 节点，支持滚动偏移和自定义视口高度（用于 fixed 定位）
    /// 返回实际内容高度
    pub fn render_with_scroll_and_viewport(
        &mut self, 
        canvas: &mut Canvas, 
        nodes: &[WxmlNode], 
        data: &JsonValue,
        interaction: &mut InteractionManager,
        scroll_offset: f32,
        viewport_height: f32,
    ) -> f32 {
        // 传递视口信息给模板引擎，用于虚拟列表优化
        self.update_layout_if_needed(nodes, data, Some((scroll_offset, viewport_height)));
        
        self.animations_active = false;
        self.anim_marks = 0;
        self.animated_bounds.clear();
        self.event_bindings.clear();
        // 不清除交互元素，保留 scroll controller 状态
        // interaction.clear_elements();  // 移除这行，避免每帧重建
        
        if let Some(cache) = self.cache.take() {
            let content_height = cache.content_height;
            // 损伤区重绘：只有这块矩形内的像素会被改写，其余保留上一帧
            let damaged = self.damage_clip.take();
            if let Some(rect) = damaged {
                canvas.save();
                canvas.clip_rect(rect);
            }
            // 渲染所有元素（fixed 元素会在 draw_with_interaction 中被跳过）
            // 不使用滚动偏移渲染，滚动在 present_to_buffer 中处理
            for rn in &cache.render_nodes {
                self.draw_with_interaction(canvas, &cache.taffy, rn, 0.0, 0.0, interaction, scroll_offset, viewport_height * self.scale_factor);
            }
            if damaged.is_some() {
                canvas.restore();
            }
            self.cache = Some(cache);
            return content_height;
        }
        
        0.0
    }
    
    /// 单独渲染 fixed 元素到指定的 canvas（覆盖在主内容之上的透明层）。
    ///
    /// 与静态渲染走**同一套** `draw_fixed_layer`：先按视口约束（left+right → 宽、
    /// top+bottom → 高）对 fixed 子树重排，再钉到视口坐标。
    /// 此前窗体自己实现了一套简化版，直接拿「相对根容器」的布局尺寸当 fixed 尺寸 ——
    /// `top:0;bottom:0` 的全屏遮罩会被撑到整页内容高（近 2000px），于是遮罩铺满屏幕
    /// 而居中的弹窗被推到视口下方看不见（首页新人券弹窗就是这么"消失"的）。
    pub fn render_fixed_elements(
        &mut self,
        canvas: &mut Canvas,
        nodes: &[WxmlNode],
        data: &JsonValue,
        interaction: &mut InteractionManager,
        _viewport_height: f32, // fixed 层以画布高度为视口高
    ) {
        // 使用已缓存的视口信息，不重新计算布局
        self.update_layout_if_needed(nodes, data, self.current_viewport);
        
        if let Some(mut cache) = self.cache.take() {
            let viewport_h = canvas.height() as f32;
            let roots = std::mem::take(&mut cache.render_nodes);
            self.draw_fixed_layer_inner(canvas, &mut cache.taffy, &roots, viewport_h, Some(interaction));
            cache.render_nodes = roots;
            self.cache = Some(cache);
        }
    }
    
    /// 使用原始 taffy 布局绘制 fixed 元素
    fn draw_fixed_element_original(
        &mut self,
        taffy: &Tree,
        canvas: &mut Canvas,
        node: &RenderNode,
        fixed_x: f32,
        fixed_y: f32,
        fixed_w: f32,
        fixed_h: f32,
        interaction: &mut InteractionManager,
        viewport_height: f32,
    ) {
        let sf = self.scale_factor;
        let logical_bounds = GeoRect::new(fixed_x / sf, fixed_y / sf, fixed_w / sf, fixed_h / sf);
        
        // 绘制 fixed 元素的背景
        self.draw_component(canvas, node, fixed_x, fixed_y, fixed_w, fixed_h, sf);
        
        // 注册交互元素
        self.register_interactive_element(node, node, &logical_bounds, interaction, taffy, true);

        // 绘制子节点 - 子节点位置相对于 fixed 元素
        if !Self::is_leaf_component(&node.tag) {
            let text_color = node.style.text_color.unwrap_or(Color::BLACK);
            for child in &node.children {
                // 获取子节点在原始布局中相对于父节点的位置
                let child_layout = taffy.layout(child.taffy_node).unwrap();
                let child_x = fixed_x + child_layout.location.x;
                let child_y = fixed_y + child_layout.location.y;
                let child_w = child_layout.size.width;
                let child_h = child_layout.size.height;
                
                self.draw_fixed_child_recursive(taffy, canvas, child, child_x, child_y, child_w, child_h, text_color, interaction, viewport_height);
            }
        }
        
        // 记录事件绑定
        for (et, handler, data, is_catch) in &node.events {
            self.event_bindings.push(EventBinding {
                event_type: et.clone(),
                handler: handler.clone(),
                data: data.clone(),
                bounds: logical_bounds,
                is_catch: *is_catch,
            });
        }
    }
    
    /// 递归绘制 fixed 元素的子节点
    fn draw_fixed_child_recursive(
        &mut self,
        taffy: &Tree,
        canvas: &mut Canvas,
        node: &RenderNode,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        inherited_color: Color,
        interaction: &mut InteractionManager,
        viewport_height: f32,
    ) {
        // Viewport culling
        if y > viewport_height || y + h < 0.0 {
            return;
        }

        let sf = self.scale_factor;
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);
        
        let text_color = node.style.text_color.unwrap_or(inherited_color);
        let mut node_to_draw = Self::shallow_for_draw(node);
        if node_to_draw.style.text_color.is_none() {
            node_to_draw.style.text_color = Some(text_color);
        }
        
        // 注册交互元素
        self.register_interactive_element(node, &node_to_draw, &logical_bounds, interaction, taffy, true);

        // 绘制组件 - 特殊处理 button 以支持按下状态
        let component_id = Self::get_component_id(node, &logical_bounds);
        match node.tag.as_str() {
            "button" => {
                let pressed = interaction.is_button_pressed(&component_id);
                ButtonComponent::draw_with_state(
                    &node_to_draw, canvas, self.text_renderer.as_deref(),
                    x, y, w, h, sf, pressed
                );
            }
            _ => {
                self.draw_component(canvas, &node_to_draw, x, y, w, h, sf);
            }
        }
        
        // 递归绘制子节点
        if !Self::is_leaf_component(&node.tag) {
            let is_scroll_view = node.tag == "scroll-view";
            let mut child_offset_y = 0.0;
            let scroll_position: f32;
            
            if is_scroll_view {
                canvas.save();
                canvas.clip_rect(GeoRect::new(x, y, w, h));
                
                scroll_position = if let Some(controller) = interaction.get_scroll_controller(&component_id) {
                    let pos = controller.get_position();
                    child_offset_y = -pos * sf; // 转换为物理像素
                    pos
                } else {
                    0.0
                };
            } else {
                scroll_position = 0.0;
            }

            // 对于 scroll-view，只渲染可见区域内的子元素
            if is_scroll_view {
                let viewport_top = scroll_position * sf;
                let viewport_bottom = viewport_top + h;
                
                for child in &node.children {
                    let child_layout = taffy.layout(child.taffy_node).unwrap();
                    let child_top = child_layout.location.y;
                    let child_bottom = child_top + child_layout.size.height;
                    
                    // 只渲染与视口相交的子元素
                    if child_bottom >= viewport_top && child_top <= viewport_bottom {
                        let child_x = x + child_layout.location.x;
                        let child_y = y + child_layout.location.y + child_offset_y;
                        let child_w = child_layout.size.width;
                        let child_h = child_layout.size.height;
                        
                        self.draw_fixed_child_recursive(taffy, canvas, child, child_x, child_y, child_w, child_h, text_color, interaction, viewport_height);
                    }
                }
            } else {
                for child in &node.children {
                    let child_layout = taffy.layout(child.taffy_node).unwrap();
                    let child_x = x + child_layout.location.x;
                    let child_y = y + child_layout.location.y + child_offset_y;
                    let child_w = child_layout.size.width;
                    let child_h = child_layout.size.height;
                    
                    self.draw_fixed_child_recursive(taffy, canvas, child, child_x, child_y, child_w, child_h, text_color, interaction, viewport_height);
                }
            }

            if is_scroll_view {
                canvas.restore();
            }
        }
        
        // 记录事件绑定
        for (et, handler, data, is_catch) in &node.events {
            self.event_bindings.push(EventBinding {
                event_type: et.clone(),
                handler: handler.clone(),
                data: data.clone(),
                bounds: logical_bounds,
                is_catch: *is_catch,
            });
        }
    }
    
    /// 兼容旧接口
    pub fn render(&mut self, canvas: &mut Canvas, nodes: &[WxmlNode], data: &JsonValue) {
        self.render_with_shell(canvas, nodes, data, |_| {});
    }

    /// 渲染页面，并在「正常流」与「fixed 覆盖层」之间插入宿主外壳绘制（如 tabBar）。
    ///
    /// 层叠顺序与浏览器一致：页面背景/内容 → 宿主外壳(tabBar) → 页面内 position:fixed
    /// 覆盖层（遮罩/弹窗）。这样全屏遮罩会同时压暗 tabBar，而 tabBar 又不会被页面背景覆盖。
    pub fn render_with_shell(
        &mut self,
        canvas: &mut Canvas,
        nodes: &[WxmlNode],
        data: &JsonValue,
        draw_shell: impl FnOnce(&mut Canvas),
    ) {
        self.animations_active = false;
        self.event_bindings.clear();
        let rendered = crate::parser::TemplateEngine::render(nodes, data);
        let mut taffy = Tree::new();
        
        let mut render_nodes = Vec::new();
        for (sib_i, node) in rendered.iter().enumerate() {
            if let Some(rn) = self.build_tree(&mut taffy, node, &[], &InheritedText::default(), sib_i, rendered.len()) {
                render_nodes.push(rn);
            }
        }
        
        let child_ids: Vec<NodeId> = render_nodes.iter().map(|n| n.taffy_node).collect();
        let root = taffy.new_with_children(
            Style {
                size: Size { width: length(self.screen_width * self.scale_factor), height: auto() },
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &child_ids,
        ).unwrap();
        
        self.compute_with_text(&mut taffy, root, Size::MAX_CONTENT);
        if self.correct_wrapped_text_heights(&mut taffy, &render_nodes) {
            self.compute_with_text(&mut taffy, root, Size::MAX_CONTENT);
        }
        
        // 正常流：跳过 fixed 子树（顶层与嵌套均由固定层处理）
        for rn in &render_nodes {
            if rn.style.is_fixed { continue; }
            self.draw(canvas, &taffy, rn, 0.0, 0.0);
        }
        // 宿主外壳（tabBar 等）：位于页面内容之上、页面 fixed 覆盖层之下
        draw_shell(canvas);
        // 固定层：把 position:fixed 元素钉在视口（画布高度即视口）
        let viewport_h = canvas.height() as f32;
        self.draw_fixed_layer(canvas, &mut taffy, &render_nodes, viewport_h);
    }

    /// 测量给定 WXML+数据的内容总高度（逻辑像素），用于自适应画布尺寸。
    pub fn measure_content_height(&self, nodes: &[WxmlNode], data: &JsonValue) -> f32 {
        let rendered = crate::parser::TemplateEngine::render(nodes, data);
        let mut taffy = Tree::new();
        let mut render_nodes = Vec::new();
        for (sib_i, node) in rendered.iter().enumerate() {
            if let Some(rn) = self.build_tree(&mut taffy, node, &[], &InheritedText::default(), sib_i, rendered.len()) {
                render_nodes.push(rn);
            }
        }
        let child_ids: Vec<NodeId> = render_nodes.iter().map(|n| n.taffy_node).collect();
        let root = taffy.new_with_children(
            Style {
                size: Size { width: length(self.screen_width * self.scale_factor), height: auto() },
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &child_ids,
        ).unwrap();
        self.compute_with_text(&mut taffy, root, Size::MAX_CONTENT);
        if self.correct_wrapped_text_heights(&mut taffy, &render_nodes) {
            self.compute_with_text(&mut taffy, root, Size::MAX_CONTENT);
        }
        taffy.layout(root).unwrap().size.height / self.scale_factor
    }
    
    /// 第二遍布局修正：首遍 `compute_layout` 后文本节点的实际宽度已知，
    /// 对「宽度为百分比/100%」等在 build 阶段无法预知换行的文本重新计算换行行数，
    /// 据此修正其盒子高度，避免多行文本被压成一行高而与后续兄弟节点重叠。
    /// 返回是否有节点高度被修改（需要重新 compute_layout）。
    ///
    /// 这是对标准 CSS「文本按可用宽度自动换行、盒子高度随行数增长」语义的补齐。
    /// 用文本度量闭包计算布局：仅带 TextMeasure 上下文的（block/auto 宽）文本按可用
    /// 宽度解析换行与高度；其它叶子沿用各自 Style 里的显式尺寸（known dimensions）。
    fn compute_with_text(&self, taffy: &mut Tree, root: NodeId, available: Size<AvailableSpace>) {
        let tr = self.text_renderer.as_deref();
        let _ = taffy.compute_layout_with_measure(
            root,
            available,
            |known, avail, _id, ctx: Option<&mut TextMeasure>| match ctx {
                Some(tm) => measure_text_node(known, avail, tm, tr),
                None => Size {
                    width: known.width.unwrap_or(0.0),
                    height: known.height.unwrap_or(0.0),
                },
            },
        );
    }

    fn correct_wrapped_text_heights(&self, taffy: &mut Tree, nodes: &[RenderNode]) -> bool {
        let tr = match self.text_renderer.as_deref() { Some(t) => t, None => return false };
        let sf = self.scale_factor;
        let mut changed = false;
        for node in nodes {
            if node.tag == "text" && !node.text.is_empty() {
                let should_wrap = !matches!(node.style.white_space, WhiteSpace::NoWrap | WhiteSpace::Pre);
                if should_wrap {
                    if let Ok(layout) = taffy.layout(node.taffy_node) {
                        let box_w = layout.size.width;
                        let box_h = layout.size.height;
                        let pl = node.style.padding_left * sf;
                        let pr = node.style.padding_right * sf;
                        let pt = node.style.padding_top * sf;
                        let pb = node.style.padding_bottom * sf;
                        let avail = (box_w - pl - pr).max(1.0);
                        let size = node.style.font_size * sf;
                        let ls = node.style.letter_spacing * sf;
                        let line_height = node.style.line_height.map(|lh| lh * sf)
                            .unwrap_or_else(|| tr.natural_line_height_for(&node.text, size)).max(size);
                        let bold = matches!(
                            node.style.font_weight,
                            super::components::FontWeight::Bold | super::components::FontWeight::W600
                                | super::components::FontWeight::W700 | super::components::FontWeight::W800
                                | super::components::FontWeight::W900
                        ) && tr.has_bold_face();
                        let lines = count_wrapped_lines(tr, &node.text, avail, size, ls, bold);
                        let needed_h = lines as f32 * line_height + pt + pb;
                        if needed_h > box_h + 0.5 {
                            if let Ok(mut st) = taffy.style(node.taffy_node).cloned() {
                                st.size.height = length(needed_h);
                                st.min_size.height = length(needed_h);
                                taffy.set_style(node.taffy_node, st).ok();
                                changed = true;
                            }
                        }
                    }
                }
            }
            if self.correct_wrapped_text_heights(taffy, &node.children) {
                changed = true;
            }
        }
        changed
    }

    fn build_tree(&self, taffy: &mut Tree, node: &WxmlNode, ancestors: &[ElementDesc], inherited: &InheritedText, sib_index: usize, sib_count: usize) -> Option<RenderNode> {
        let sf = self.scale_factor;
        
        if node.node_type == WxmlNodeType::Text {
            let text = node.text_content.trim();
            if text.is_empty() { return None; }
            // 原始文本节点继承父级的字号/颜色/字重/对齐/行高
            let fs = inherited.font_size;
            // 默认行高取字体自然行高（≈浏览器 normal），无字体时回退 1.2 倍
            let natural_lh = self.text_renderer.as_deref()
                .map(|tr| tr.natural_line_height_for(text, fs * sf) / sf)
                .unwrap_or(fs * crate::text::NORMAL_LINE_HEIGHT_FACTOR);
            let line_h = inherited.line_height.unwrap_or(natural_lh);
            let tw = self.measure_text(text, fs * sf);
            // 居中/右对齐的文本撑满可用宽度，绘制时再按对齐做偏移（否则无法居中）
            let width_dim: Dimension = if matches!(inherited.align, TextAlign::Center | TextAlign::Right) {
                percent(1.0)
            } else {
                // 取整到整像素即可（换行判定另有亚像素容差），不额外加宽
                length(tw.ceil())
            };
            let tn = taffy.new_leaf(Style {
                size: Size { width: width_dim, height: length(line_h * sf) },
                ..Default::default()
            }).unwrap();
            return Some(RenderNode {
                tag: "#text".into(), 
                text: text.into(), 
                attrs: HashMap::new(),
                taffy_node: tn,
                style: NodeStyle {
                    font_size: fs,
                    text_color: inherited.color,
                    font_weight: inherited.weight,
                    text_align: inherited.align,
                    line_height: inherited.line_height,
                    letter_spacing: inherited.letter_spacing,
                    opacity: 1.0,
                    ..Default::default()
                },
                children: vec![], 
                events: vec![],
            });
        }
        
        if node.node_type != WxmlNodeType::Element { return None; }

        let tag = node.tag_name.as_str();
        let mut ctx = ComponentContext {
            scale_factor: sf,
            screen_width: self.screen_width,
            screen_height: self.screen_height,
            stylesheet: &self.stylesheet,
            taffy,
            ancestors: ancestors.to_vec(),
            inherited: inherited.clone(),
            sibling_index: sib_index,
            sibling_count: sib_count,
        };
        
        let mut render_node = match tag {
            "text" => TextComponent::build(node, &mut ctx),
            "button" => ButtonComponent::build(node, &mut ctx),
            "icon" => IconComponent::build(node, &mut ctx),
            "progress" => ProgressComponent::build(node, &mut ctx),
            "switch" => SwitchComponent::build(node, &mut ctx),
            "checkbox" => CheckboxComponent::build(node, &mut ctx),
            "checkbox-group" => CheckboxGroupComponent::build(node, &mut ctx),
            "radio" => RadioComponent::build(node, &mut ctx),
            "radio-group" => RadioGroupComponent::build(node, &mut ctx),
            "slider" => SliderComponent::build(node, &mut ctx),
            "input" | "textarea" => InputComponent::build(node, &mut ctx),
            "image" => ImageComponent::build(node, &mut ctx),
            "video" => VideoComponent::build(node, &mut ctx),
            "canvas" => CanvasComponent::build(node, &mut ctx),
            "swiper" => SwiperComponent::build(node, &mut ctx),
            "swiper-item" => SwiperItemComponent::build(node, &mut ctx),
            "rich-text" => RichTextComponent::build(node, &mut ctx),
            "picker" => PickerComponent::build(node, &mut ctx),
            "picker-view" => PickerViewComponent::build(node, &mut ctx),
            "picker-view-column" => PickerViewColumnComponent::build(node, &mut ctx),
            _ => ViewComponent::build(node, &mut ctx),
        };
        
        if let Some(ref mut rn) = render_node {
            if !Self::is_leaf_component(tag) {
                // 扩展祖先链：当前节点作为子节点的父级，用于后代/子选择器匹配
                let mut child_ancestors = ancestors.to_vec();
                let node_classes: Vec<&str> = node.get_attr("class")
                    .map(|s| s.split_whitespace().collect())
                    .unwrap_or_default();
                child_ancestors.push(ElementDesc::new(
                    &node.tag_name,
                    node.get_attr("id"),
                    &node_classes,
                    &node.attributes,
                ));
                
                // 计算传递给子节点的继承文本样式（来自当前节点的计算样式）
                let child_inherited = InheritedText {
                    font_size: rn.style.font_size,
                    color: rn.style.text_color,
                    weight: rn.style.font_weight,
                    align: rn.style.text_align,
                    line_height: rn.style.line_height,
                    letter_spacing: rn.style.letter_spacing,
                };
                
                let mut children = vec![];
                for (sib_ci, c) in node.children.iter().enumerate() {
                    if let Some(cr) = self.build_tree(ctx.taffy, c, &child_ancestors, &child_inherited, sib_ci, node.children.len()) { 
                        children.push(cr); 
                    }
                }
                
                if !children.is_empty() {
                    let child_ids: Vec<NodeId> = children.iter().map(|c| c.taffy_node).collect();
                    let (mut ts, ns) = build_base_style(node, &mut ctx);
                    
                    // 这里重新算了一遍基础样式，会覆盖组件 build 里设的布局，
                    // 所以需要容器语义的组件必须在这里再补一次（swiper / swiper-item）。
                    if tag == "swiper" {
                        let vertical = node.get_attr("vertical")
                            .map(|v| v == "true" || v == "{{true}}")
                            .unwrap_or(false);
                        ts.flex_direction = if vertical { FlexDirection::Column } else { FlexDirection::Row };
                        ts.flex_wrap = FlexWrap::NoWrap;
                        if matches!(ts.size.width, Dimension::Auto) {
                            ts.size.width = percent(1.0);
                        }
                        if matches!(ts.size.height, Dimension::Auto) {
                            ts.size.height = length(SwiperComponent::DEFAULT_HEIGHT * ctx.scale_factor);
                        }
                        // 每个 item 占满一屏且不收缩（对齐 HTML .wx-swiper-item{flex:0 0 100%}）
                        for child in &children {
                            if let Ok(mut style) = ctx.taffy.style(child.taffy_node).cloned() {
                                style.size = taffy::geometry::Size { width: percent(1.0), height: percent(1.0) };
                                style.min_size.width = percent(1.0);
                                style.flex_shrink = 0.0;
                                style.flex_grow = 0.0;
                                ctx.taffy.set_style(child.taffy_node, style).ok();
                            }
                        }
                    }
                    
                    // 对于 scroll-view，使用 Overflow::Visible 让子节点能够正确布局
                    // 裁剪在渲染时通过 canvas.clip_rect 处理
                    if tag == "scroll-view" {
                        ts.overflow.x = taffy::style::Overflow::Visible;
                        ts.overflow.y = taffy::style::Overflow::Visible;
                        
                        // 检查是否是横向滚动
                        let scroll_x = node.get_attr("scroll-x")
                            .map(|s| s == "true" || s == "{{true}}")
                            .unwrap_or(false);
                        
                        if scroll_x {
                            // 横向滚动：子元素横向排列
                            ts.flex_direction = FlexDirection::Row;
                            ts.flex_wrap = FlexWrap::NoWrap;
                        }
                        
                        // 为 scroll-view 的子元素设置 flex-shrink: 0，防止被压缩
                        for child in &children {
                            if let Ok(mut style) = ctx.taffy.style(child.taffy_node).cloned() {
                                style.flex_shrink = 0.0;
                                ctx.taffy.set_style(child.taffy_node, style).ok();
                            }
                        }
                    }
                    
                    let new_tn = ctx.taffy.new_with_children(ts, &child_ids).unwrap();
                    
                    rn.taffy_node = new_tn;
                    rn.children = children;
                    // 更新样式（保留原有样式中已设置的值，但用新样式覆盖）
                    rn.style = ns;
                }
            }
        }
        
        render_node
    }
    
    /// 取输入类组件的初始值：`value` 或双向绑定写法 `model:value`。
    fn input_value_attr(node: &RenderNode) -> String {
        node.attrs
            .get("value")
            .or_else(|| node.attrs.get("model:value"))
            .cloned()
            .unwrap_or_default()
    }

    fn is_leaf_component(tag: &str) -> bool {
        // rich-text / picker 不再是叶子：rich-text 自建带样式的文本片段子树；
        // picker 渲染其子元素（触发视图，如“当前选择：xxx”）而非合成占位 UI。
        matches!(tag, 
            "text" | "button" | "icon" | "progress" | "switch" | 
            "checkbox" | "radio" | "slider" | "input" | "textarea" | "image" | "video" | "canvas" |
            "picker-view-column"
        )
    }
    
    fn get_component_id(node: &RenderNode, bounds: &GeoRect) -> String {
        if let Some(id) = node.attrs.get("id") {
            if !id.is_empty() {
                return id.clone();
            }
        }
        // 用整数格式化：`{:.0}` 走浮点转十进制，逐节点逐帧调用时开销不可忽略
        format!("{}_{}_{}", node.tag, bounds.x as i32, bounds.y as i32)
    }

    fn draw_with_interaction(
        &mut self, 
        canvas: &mut Canvas, 
        taffy: &Tree, 
        node: &RenderNode, 
        ox: f32, 
        oy: f32,
        interaction: &mut InteractionManager,
        scroll_offset: f32,
        viewport_height: f32,
    ) {
        // 跳过 fixed 元素，它们会单独渲染
        if node.style.is_fixed {
            return;
        }
        // CSS 动画 / transform：由专门路径接管（含离屏仿射合成）
        if self.draw_with_transform(
            canvas,
            taffy,
            node,
            ox,
            oy,
            interaction,
            DrawKind::Top { scroll_offset, viewport_height },
        ) {
            return;
        }
        
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        // 渲染整个内容到 canvas，滚动在 present_to_buffer 中处理
        
        if self.cull_outside_viewport(node, x, y, w, h, scroll_offset, viewport_height) {
            return;
        }
        
        if node.tag == "swiper" {
            self.draw_swiper_container(canvas, taffy, node, x, y, w, h, interaction,
                DrawKind::Top { scroll_offset, viewport_height });
            return;
        }
        
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);
        
        let component_id = Self::get_component_id(node, &logical_bounds);
        
        // 应用交互状态
        let mut node_to_draw = Self::shallow_for_draw(node);
        // 按压态：`:active` / `hover-class` 在建树期已算好整套样式，这里按需切换，
        // 有 `transition` 就在常态与按压态之间插值
        self.apply_pressed_style(&mut node_to_draw, interaction, &component_id);
        let state_checked = interaction.get_state(&component_id).map(|s| s.checked);
        let switch_progress = if node.tag == "switch" {
            state_checked.map(|checked| self.toggle_progress(interaction, &component_id, checked))
        } else {
            None
        };
        if let Some(state) = interaction.get_state(&component_id) {
            match node.tag.as_str() {
                // switch 的配色由组件自己按进度求值（轨道关态是微信的浅灰，不是纯白）
                "switch" => {
                    node_to_draw.style.custom_data = switch_progress.unwrap_or(if state.checked { 1.0 } else { 0.0 });
                }
                "checkbox" => {
                    node_to_draw.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                    // 更新颜色
                    let checkbox_color = node.attrs.get("color")
                        .and_then(|c| super::components::parse_color_str(c))
                        .unwrap_or(Color::from_hex(0x09BB07));
                    if state.checked {
                        node_to_draw.style.background_color = Some(checkbox_color);
                        node_to_draw.style.border_color = Some(checkbox_color);
                    } else {
                        node_to_draw.style.background_color = Some(Color::WHITE);
                        node_to_draw.style.border_color = Some(Color::from_hex(0xD1D1D1));
                    }
                }
                "radio" => {
                    node_to_draw.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                    // 更新颜色
                    let radio_color = node.attrs.get("color")
                        .and_then(|c| super::components::parse_color_str(c))
                        .unwrap_or(Color::from_hex(0x09BB07));
                    if state.checked {
                        node_to_draw.style.background_color = Some(radio_color);
                        node_to_draw.style.border_color = Some(radio_color);
                    } else {
                        node_to_draw.style.background_color = Some(Color::WHITE);
                        node_to_draw.style.border_color = Some(Color::from_hex(0xD1D1D1));
                    }
                }
                "slider" => {
                    if let Ok(v) = state.value.parse::<f32>() {
                        node_to_draw.style.custom_data = v / 100.0;
                        if !node_to_draw.text.is_empty() {
                            node_to_draw.text = format!("{}", v as i32);
                        }
                    }
                }
                "input" | "textarea" => {
                    // 获取 placeholder
                    let placeholder = node.attrs.get("placeholder").cloned().unwrap_or_default();
                    
                    // 检查是否聚焦
                    let is_focused = interaction.focused_input.as_ref()
                        .map(|f| f.id == component_id)
                        .unwrap_or(false);
                    
                    if state.value.is_empty() && !is_focused {
                        // 没有输入值且未聚焦时显示 placeholder
                        node_to_draw.text = placeholder;
                        node_to_draw.style.text_color = Some(Color::from_hex(0xBFBFBF));
                    } else {
                        // 有输入值或聚焦时显示实际值（聚焦时即使为空也不显示 placeholder）
                        node_to_draw.text = state.value.clone();
                        node_to_draw.style.text_color = Some(Color::BLACK);
                    }
                }
                _ => {}
            }
        } else if matches!(node.tag.as_str(), "input" | "textarea") {
            // 输入框但还没有交互状态，显示 placeholder 或初始值
            let placeholder = node.attrs.get("placeholder").cloned().unwrap_or_default();
            let initial_value = Self::input_value_attr(node);
            
            if initial_value.is_empty() {
                node_to_draw.text = placeholder;
                node_to_draw.style.text_color = Some(Color::from_hex(0xBFBFBF));
            } else {
                node_to_draw.text = initial_value;
            }
        }
        
        // 注册交互元素
        self.register_interactive_element(node, &node_to_draw, &logical_bounds, interaction, taffy, false);

        // 绘制组件 - 特殊处理 input 和 button 组件
        match node.tag.as_str() {
                "input" | "textarea" => {
                    let focused = interaction.focused_input.as_ref()
                        .map(|f| f.id == component_id)
                        .unwrap_or(false);
                    let (cursor_pos, selection) = if focused {
                        let f = interaction.focused_input.as_ref().unwrap();
                        (f.cursor_pos, f.get_selection_range())
                    } else {
                        (0, None)
                    };
                    InputComponent::draw_with_selection(
                        &node_to_draw, canvas, self.text_renderer.as_deref(), 
                        x, y, w, h, sf, focused, cursor_pos, selection
                    );
                    
                    // 更新 text_offset（用于点击位置计算）
                    if focused {
                        if let Some(tr) = self.text_renderer.as_deref() {
                            let font_size = node_to_draw.style.font_size * sf;
                            let padding_left = 12.0 * sf;
                            let padding_right = 12.0 * sf;
                            let available_width = w - padding_left - padding_right;
                            
                            let text_width = tr.measure_text(&node_to_draw.text, font_size);
                            let mut text_offset = 0.0;
                        if text_width > available_width {
                            let cursor_text: String = node_to_draw.text.chars().take(cursor_pos).collect();
                            let cursor_x_in_text = tr.measure_text(&cursor_text, font_size);
                            
                            if cursor_x_in_text > available_width {
                                text_offset = available_width - cursor_x_in_text - font_size;
                            }
                        }
                        
                        if let Some(input) = &mut interaction.focused_input {
                            input.text_offset = text_offset;
                        }
                    }
                }
            }
            "button" => {
                let pressed = interaction.is_button_pressed(&component_id);
                ButtonComponent::draw_with_state(
                    &node_to_draw, canvas, self.text_renderer.as_deref(),
                    x, y, w, h, sf, pressed
                );
            }
            _ => {
                self.draw_component(canvas, &node_to_draw, x, y, w, h, sf);
            }
        }
        
        // 绘制子节点
        if !Self::is_leaf_component(&node.tag) {
            let is_scroll_view = node.tag == "scroll-view";
            let mut child_offset_y = 0.0;
            let scroll_position: f32;
            
            if is_scroll_view {
                // 计算 scroll-view 内容高度
                let mut content_height = 0.0f32;
                for child in &node.children {
                    let child_layout = taffy.layout(child.taffy_node).unwrap();
                    let child_bottom = child_layout.location.y + child_layout.size.height;
                    content_height = content_height.max(child_bottom);
                }
                
                // 获取滚动位置
                scroll_position = if let Some(controller) = interaction.get_scroll_controller(&component_id) {
                    controller.get_position()
                } else {
                    0.0
                };
                
                // 检查缓存是否需要更新
                let cache_needs_render = {
                    let cache = self.scroll_cache.get_or_create(
                        &component_id,
                        w as u32,
                        content_height.ceil() as u32,
                        (w / sf) as u32,
                        (h / sf) as u32,
                    );
                    cache.needs_render()
                };
                
                if cache_needs_render {
                    // 创建临时 Canvas 用于渲染
                    let mut temp_canvas = Canvas::new(w as u32, content_height.ceil() as u32);
                    temp_canvas.clear(node.style.background_color.unwrap_or(Color::TRANSPARENT));
                    
                    let text_color = node.style.text_color.unwrap_or(Color::BLACK);
                    
                    // 渲染所有子元素到临时 Canvas
                    for child in &node.children {
                        self.draw_child_to_cache(&mut temp_canvas, taffy, child, 0.0, 0.0, text_color, interaction);
                    }
                    
                    // 将临时 Canvas 的内容复制到缓存
                    if let Some(cache) = self.scroll_cache.get_mut(&component_id) {
                        // 直接替换缓存的 canvas
                        cache.canvas = temp_canvas;
                        cache.mark_clean();
                    }
                }
                
                // 从缓存复制可见区域到主 Canvas
                canvas.save();
                canvas.clip_rect(GeoRect::new(x, y, w, h));
                
                if let Some(cache) = self.scroll_cache.get(&component_id) {
                    cache.blit_to(canvas, scroll_position, x, y, sf);
                }
                
                canvas.restore();
                
                // 注册子元素的交互区域（需要考虑滚动偏移）
                child_offset_y = -scroll_position * sf;
                let text_color = node.style.text_color.unwrap_or(Color::BLACK);
                for child in &node.children {
                    self.register_child_interactions(taffy, child, x, y + child_offset_y, text_color, interaction, scroll_position, h / sf);
                }
            } else {
                let text_color = node.style.text_color.unwrap_or(Color::BLACK);
                for child in &node.children { 
                    self.draw_child_with_interaction(canvas, taffy, child, x, y + child_offset_y, text_color, interaction, scroll_offset, viewport_height); 
                }
            }
        }

        // 记录事件绑定
        for (et, h, d, is_catch) in &node.events {
            self.event_bindings.push(EventBinding { 
                event_type: et.clone(), 
                handler: h.clone(), 
                data: d.clone(), 
                bounds: logical_bounds,
                is_catch: *is_catch,
            });
        }
    }
    
    /// 渲染子节点到离屏缓存（不处理交互状态，纯渲染）
    fn draw_child_to_cache(
        &self,
        canvas: &mut Canvas,
        taffy: &Tree,
        node: &RenderNode,
        ox: f32,
        oy: f32,
        inherited_color: Color,
        interaction: &InteractionManager,
    ) {
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);
        let component_id = Self::get_component_id(node, &logical_bounds);
        
        let text_color = node.style.text_color.unwrap_or(inherited_color);
        let mut node_to_draw = Self::shallow_for_draw(node);
        if node_to_draw.style.text_color.is_none() {
            node_to_draw.style.text_color = Some(text_color);
        }
        
        // 应用交互状态
        if let Some(state) = interaction.get_state(&component_id) {
            match node.tag.as_str() {
                "checkbox" | "switch" => {
                    node_to_draw.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                    let checkbox_color = node.attrs.get("color")
                        .and_then(|c| super::components::parse_color_str(c))
                        .unwrap_or(Color::from_hex(0x09BB07));
                    if state.checked {
                        node_to_draw.style.background_color = Some(checkbox_color);
                        node_to_draw.style.border_color = Some(checkbox_color);
                    } else {
                        node_to_draw.style.background_color = Some(Color::WHITE);
                        node_to_draw.style.border_color = Some(Color::from_hex(0xD1D1D1));
                    }
                }
                "radio" => {
                    node_to_draw.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                    let radio_color = node.attrs.get("color")
                        .and_then(|c| super::components::parse_color_str(c))
                        .unwrap_or(Color::from_hex(0x09BB07));
                    if state.checked {
                        node_to_draw.style.background_color = Some(radio_color);
                        node_to_draw.style.border_color = Some(radio_color);
                    } else {
                        node_to_draw.style.background_color = Some(Color::WHITE);
                        node_to_draw.style.border_color = Some(Color::from_hex(0xD1D1D1));
                    }
                }
                "slider" => {
                    if let Ok(v) = state.value.parse::<f32>() {
                        node_to_draw.style.custom_data = v / 100.0;
                        if !node_to_draw.text.is_empty() {
                            node_to_draw.text = format!("{}", v as i32);
                        }
                    }
                }
                _ => {}
            }
        }
        
        // 绘制组件
        self.draw_component(canvas, &node_to_draw, x, y, w, h, sf);
        
        // 递归绘制子节点
        if !Self::is_leaf_component(&node.tag) {
            for child in &node.children {
                self.draw_child_to_cache(canvas, taffy, child, x, y, text_color, interaction);
            }
        }
    }
    
    /// 注册 scroll-view 子元素的交互区域
    fn register_child_interactions(
        &mut self,
        taffy: &Tree,
        node: &RenderNode,
        ox: f32,
        oy: f32,
        inherited_color: Color,
        interaction: &mut InteractionManager,
        scroll_position: f32,
        viewport_height: f32,
    ) {
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        
        // 检查是否在可见区域内
        let logical_y = y / sf;
        let logical_h = h / sf;
        let viewport_top = scroll_position;
        let viewport_bottom = scroll_position + viewport_height;
        
        // 只注册可见区域内的元素
        if logical_y + logical_h < viewport_top || logical_y > viewport_bottom {
            return;
        }
        
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);
        let text_color = node.style.text_color.unwrap_or(inherited_color);
        
        // 注册交互元素
        self.register_interactive_element(node, node, &logical_bounds, interaction, taffy, false);
        
        // 递归注册子元素
        if !Self::is_leaf_component(&node.tag) {
            for child in &node.children {
                self.register_child_interactions(taffy, child, x, y, text_color, interaction, scroll_position, viewport_height);
            }
        }
        
        // 记录事件绑定
        for (et, h, d, is_catch) in &node.events {
            self.event_bindings.push(EventBinding {
                event_type: et.clone(),
                handler: h.clone(),
                data: d.clone(),
                bounds: logical_bounds,
                is_catch: *is_catch,
            });
        }
    }
    
    fn draw_child_with_interaction(
        &mut self, 
        canvas: &mut Canvas, 
        taffy: &Tree, 
        node: &RenderNode, 
        ox: f32, 
        oy: f32, 
        inherited_color: Color,
        interaction: &mut InteractionManager,
        scroll_offset: f32,
        viewport_height: f32,
    ) {
        // 跳过 fixed 元素，它们会单独渲染
        if node.style.is_fixed {
            return;
        }
        // CSS 动画 / transform：由专门路径接管（含离屏仿射合成）
        if self.draw_with_transform(
            canvas,
            taffy,
            node,
            ox,
            oy,
            interaction,
            DrawKind::Child { inherited: inherited_color, scroll_offset, viewport_height },
        ) {
            return;
        }
        
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        
        if self.cull_outside_viewport(node, x, y, w, h, scroll_offset, viewport_height) {
            return;
        }

        if node.tag == "swiper" {
            self.draw_swiper_container(canvas, taffy, node, x, y, w, h, interaction,
                DrawKind::Child { inherited: inherited_color, scroll_offset, viewport_height });
            return;
        }

        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);

        let text_color = node.style.text_color.unwrap_or(inherited_color);
        let component_id = Self::get_component_id(node, &logical_bounds);
        
        // 检查是否需要修改节点（只有交互组件才需要 clone）
        let needs_modification = node.style.text_color.is_none() 
            || matches!(node.tag.as_str(), "checkbox" | "switch" | "radio" | "slider" | "input" | "textarea")
            && interaction.get_state(&component_id).is_some();
        
        let switch_progress = if node.tag == "switch" {
            interaction
                .get_state(&component_id)
                .map(|s| s.checked)
                .map(|checked| self.toggle_progress(interaction, &component_id, checked))
        } else {
            None
        };
        
        // 只在需要时才 clone
        let node_to_draw: std::borrow::Cow<RenderNode> = if needs_modification {
            let mut modified = Self::shallow_for_draw(node);
            if modified.style.text_color.is_none() {
                modified.style.text_color = Some(text_color);
            }
            
            // 应用交互状态
            if let Some(state) = interaction.get_state(&component_id) {
                match node.tag.as_str() {
                    // switch 的配色由组件自己按进度求值（轨道关态是微信的浅灰，不是纯白）
                    "switch" => {
                        modified.style.custom_data = switch_progress.unwrap_or(if state.checked { 1.0 } else { 0.0 });
                    }
                    "checkbox" => {
                        modified.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                        let checkbox_color = node.attrs.get("color")
                            .and_then(|c| super::components::parse_color_str(c))
                            .unwrap_or(Color::from_hex(0x09BB07));
                        if state.checked {
                            modified.style.background_color = Some(checkbox_color);
                            modified.style.border_color = Some(checkbox_color);
                        } else {
                            modified.style.background_color = Some(Color::WHITE);
                            modified.style.border_color = Some(Color::from_hex(0xD1D1D1));
                        }
                    }
                    "radio" => {
                        modified.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                        let radio_color = node.attrs.get("color")
                            .and_then(|c| super::components::parse_color_str(c))
                            .unwrap_or(Color::from_hex(0x09BB07));
                        if state.checked {
                            modified.style.background_color = Some(radio_color);
                            modified.style.border_color = Some(radio_color);
                        } else {
                            modified.style.background_color = Some(Color::WHITE);
                            modified.style.border_color = Some(Color::from_hex(0xD1D1D1));
                        }
                    }
                    "slider" => {
                        if let Ok(v) = state.value.parse::<f32>() {
                            modified.style.custom_data = v / 100.0;
                            if !modified.text.is_empty() {
                                modified.text = format!("{}", v as i32);
                            }
                        }
                    }
                    "input" | "textarea" => {
                        let placeholder = node.attrs.get("placeholder").cloned().unwrap_or_default();
                        let is_focused = interaction.focused_input.as_ref()
                            .map(|f| f.id == component_id)
                            .unwrap_or(false);
                        
                        if state.value.is_empty() && !is_focused {
                            modified.text = placeholder;
                            modified.style.text_color = Some(Color::from_hex(0xBFBFBF));
                        } else {
                            modified.text = state.value.clone();
                            modified.style.text_color = Some(Color::BLACK);
                        }
                    }
                    _ => {}
                }
            }
            std::borrow::Cow::Owned(modified)
        } else if matches!(node.tag.as_str(), "input" | "textarea") {
            // 输入框但还没有交互状态，显示 placeholder 或初始值
            let placeholder = node.attrs.get("placeholder").cloned().unwrap_or_default();
            let initial_value = Self::input_value_attr(node);
            
            let mut modified = Self::shallow_for_draw(node);
            if initial_value.is_empty() {
                modified.text = placeholder;
                modified.style.text_color = Some(Color::from_hex(0xBFBFBF));
            } else {
                modified.text = initial_value;
            }
            std::borrow::Cow::Owned(modified)
        } else {
            std::borrow::Cow::Borrowed(node)
        };

        // 注册交互元素（包括 scroll-view）
        self.register_interactive_element(node, &node_to_draw, &logical_bounds, interaction, taffy, false);

        // 绘制组件 - 特殊处理 input、button 和有点击事件的 view 组件
        match node.tag.as_str() {
                "input" | "textarea" => {
                    let focused = interaction.focused_input.as_ref()
                        .map(|f| f.id == component_id)
                        .unwrap_or(false);
                    let (cursor_pos, selection) = if focused {
                        let f = interaction.focused_input.as_ref().unwrap();
                        (f.cursor_pos, f.get_selection_range())
                    } else {
                        (0, None)
                    };
                    InputComponent::draw_with_selection(
                        &node_to_draw, canvas, self.text_renderer.as_deref(), 
                        x, y, w, h, sf, focused, cursor_pos, selection
                    );
                    
                    // 更新 text_offset（用于点击位置计算）
                    if focused {
                        if let Some(tr) = self.text_renderer.as_deref() {
                            let font_size = node_to_draw.style.font_size * sf;
                            let padding_left = 12.0 * sf;
                            let padding_right = 12.0 * sf;
                            let available_width = w - padding_left - padding_right;
                            
                            let text_width = tr.measure_text(&node_to_draw.text, font_size);
                            let mut text_offset = 0.0;
                            
                            if text_width > available_width {
                                let cursor_text: String = node_to_draw.text.chars().take(cursor_pos).collect();
                                let cursor_x_in_text = tr.measure_text(&cursor_text, font_size);
                                
                                if cursor_x_in_text > available_width {
                                    text_offset = available_width - cursor_x_in_text - font_size;
                                }
                            }
                            
                            if let Some(input) = &mut interaction.focused_input {
                                input.text_offset = text_offset;
                            }
                        }
                    }
                }
                "button" => {
                let pressed = interaction.is_button_pressed(&component_id);
                ButtonComponent::draw_with_state(
                    &node_to_draw, canvas, self.text_renderer.as_deref(),
                    x, y, w, h, sf, pressed
                );
            }
            _ => {
                self.draw_component(canvas, &node_to_draw, x, y, w, h, sf);
            }
        }
        
        if !Self::is_leaf_component(&node.tag) {
            let is_scroll_view = node.tag == "scroll-view";
            let has_overflow_hidden = node.style.overflow == super::components::Overflow::Hidden;
            let mut child_offset_x = 0.0;
            let mut child_offset_y = 0.0;
            let scroll_position: f32;
            
            // 检查是否是横向滚动
            let is_horizontal_scroll = is_scroll_view && node.attrs.get("scroll-x")
                .map(|s| s == "true" || s == "{{true}}")
                .unwrap_or(false);
            
            // 对于 overflow: hidden 的容器，应用裁剪
            if has_overflow_hidden || is_scroll_view {
                canvas.save();
                canvas.clip_rect(GeoRect::new(x, y, w, h));
            }
            
            if is_scroll_view {
                scroll_position = if let Some(controller) = interaction.get_scroll_controller(&component_id) {
                    let pos = controller.get_position();
                    if is_horizontal_scroll {
                        child_offset_x = -pos * sf; // 横向滚动
                    } else {
                        child_offset_y = -pos * sf; // 纵向滚动
                    }
                    pos
                } else {
                    0.0
                };
            } else {
                scroll_position = 0.0;
            }

            // 对于 scroll-view，只渲染可见区域内的子元素（视口裁剪优化）
            if is_scroll_view {
                if is_horizontal_scroll {
                    // 横向滚动：检查水平方向的可见性
                    let viewport_left = scroll_position * sf;
                    let viewport_right = viewport_left + w;
                    
                    for child in &node.children {
                        let child_layout = taffy.layout(child.taffy_node).unwrap();
                        let child_left = child_layout.location.x;
                        let child_right = child_left + child_layout.size.width;
                        
                        // 只渲染与视口相交的子元素
                        if child_right >= viewport_left && child_left <= viewport_right {
                            self.draw_child_with_interaction(canvas, taffy, child, x + child_offset_x, y, text_color, interaction, scroll_offset, viewport_height); 
                        }
                    }
                } else {
                    // 纵向滚动：检查垂直方向的可见性
                    let viewport_top = scroll_position * sf;
                    let viewport_bottom = viewport_top + h;
                    
                    for child in &node.children {
                        let child_layout = taffy.layout(child.taffy_node).unwrap();
                        let child_top = child_layout.location.y;
                        let child_bottom = child_top + child_layout.size.height;
                        
                        // 只渲染与视口相交的子元素
                        if child_bottom >= viewport_top && child_top <= viewport_bottom {
                            self.draw_child_with_interaction(canvas, taffy, child, x, y + child_offset_y, text_color, interaction, scroll_offset, viewport_height); 
                        }
                    }
                }
            } else {
                for child in &node.children { 
                    self.draw_child_with_interaction(canvas, taffy, child, x, y + child_offset_y, text_color, interaction, scroll_offset, viewport_height); 
                }
            }

            if has_overflow_hidden || is_scroll_view {
                canvas.restore();
            }
        }

        for (et, h, d, is_catch) in &node.events {
            self.event_bindings.push(EventBinding { 
                event_type: et.clone(), 
                handler: h.clone(), 
                data: d.clone(), 
                bounds: logical_bounds,
                is_catch: *is_catch,
            });
        }
    }
    
    fn draw_component(&self, canvas: &mut Canvas, node: &RenderNode, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        match node.tag.as_str() {
            "#text" | "text" => TextComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "button" => ButtonComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "icon" => IconComponent::draw(node, canvas, x, y, w, h, sf),
            "progress" => ProgressComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "switch" => SwitchComponent::draw(node, canvas, x, y, w, h, sf),
            "checkbox" => CheckboxComponent::draw(node, canvas, x, y, w, h, sf),
            "checkbox-group" => CheckboxGroupComponent::draw(node, canvas, x, y, w, h, sf),
            "radio" => RadioComponent::draw(node, canvas, x, y, w, h, sf),
            "radio-group" => RadioGroupComponent::draw(node, canvas, x, y, w, h, sf),
            "slider" => SliderComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "input" | "textarea" => InputComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "image" => ImageComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "video" => VideoComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "canvas" => CanvasComponent::draw(node, canvas, x, y, w, h, sf),
            // swiper 的背景由通用路径绘制；子项与指示点见 draw_swiper_container
            "swiper" => draw_background(canvas, &node.style, x, y, w, h),
            "rich-text" => RichTextComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "picker" => PickerComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            "picker-view" => PickerViewComponent::draw(node, canvas, self.text_renderer.as_deref(), x, y, w, h, sf),
            _ => ViewComponent::draw(node, canvas, x, y, w, h, sf),
        }
    }
    
    fn register_interactive_element(
        &self, 
        original_node: &RenderNode, 
        drawn_node: &RenderNode,
        bounds: &GeoRect, 
        interaction: &mut InteractionManager,
        taffy: &Tree,
        is_in_fixed_container: bool
    ) {
        let disabled = original_node.attrs.get("disabled")
            .map(|s| s == "true" || s == "{{true}}")
            .unwrap_or(false);
        
        let id = Self::get_component_id(original_node, bounds);
        let is_fixed = is_in_fixed_container || original_node.style.is_fixed;
        
        match original_node.tag.as_str() {
            "scroll-view" => {
                // 检查是否是横向滚动
                let scroll_x = original_node.attrs.get("scroll-x")
                    .map(|s| s == "true" || s == "{{true}}")
                    .unwrap_or(false);
                
                let mut content_height = 0.0;
                let mut content_width = 0.0;
                
                // 计算内容尺寸：所有子节点的边界最大值
                for child in original_node.children.iter() {
                    if let Ok(layout) = taffy.layout(child.taffy_node) {
                        let bottom = layout.location.y + layout.size.height;
                        let right = layout.location.x + layout.size.width;
                        if bottom > content_height {
                            content_height = bottom;
                        }
                        if right > content_width {
                            content_width = right;
                        }
                    }
                }
                
                // 转换为逻辑像素
                let logical_content_height = content_height / self.scale_factor;
                let logical_content_width = content_width / self.scale_factor;
                
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::ScrollArea,
                    id,
                    bounds: *bounds,
                    checked: false,
                    value: String::new(),
                    disabled,
                    min: 0.0,
                    max: 0.0,
                    content_height: logical_content_height,
                    viewport_height: bounds.height,
                    content_width: logical_content_width,
                    viewport_width: bounds.width,
                    is_horizontal: scroll_x,
                    is_fixed,
                });
            }
            "checkbox" => {
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Checkbox,
                    id,
                    bounds: *bounds,
                    checked: drawn_node.style.custom_data > 0.5,
                    value: Self::input_value_attr(original_node),
                    disabled,
                    min: 0.0,
                    max: 1.0,
                    content_height: 0.0,
                    viewport_height: 0.0,
                    content_width: 0.0,
                    viewport_width: 0.0,
                    is_horizontal: false,
                    is_fixed,
                });
            }
            "radio" => {
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Radio,
                    id,
                    bounds: *bounds,
                    checked: drawn_node.style.custom_data > 0.5,
                    value: Self::input_value_attr(original_node),
                    disabled,
                    min: 0.0,
                    max: 1.0,
                    content_height: 0.0,
                    viewport_height: 0.0,
                    content_width: 0.0,
                    viewport_width: 0.0,
                    is_horizontal: false,
                    is_fixed,
                });
            }
            "switch" => {
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Switch,
                    id,
                    bounds: *bounds,
                    checked: drawn_node.style.custom_data > 0.5,
                    value: original_node.text.clone(),
                    disabled,
                    min: 0.0,
                    max: 1.0,
                    content_height: 0.0,
                    viewport_height: 0.0,
                    content_width: 0.0,
                    viewport_width: 0.0,
                    is_horizontal: false,
                    is_fixed,
                });
            }
            "slider" => {
                let min = original_node.attrs.get("min").and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let max = original_node.attrs.get("max").and_then(|s| s.parse().ok()).unwrap_or(100.0);
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Slider,
                    id,
                    bounds: *bounds,
                    checked: false,
                    value: format!("{}", (drawn_node.style.custom_data * 100.0) as i32),
                    disabled,
                    min,
                    max,
                    content_height: 0.0,
                    viewport_height: 0.0,
                    content_width: 0.0,
                    viewport_width: 0.0,
                    is_horizontal: false,
                    is_fixed,
                });
            }
            "input" | "textarea" => {
                // 只使用原始 value 属性，不使用 placeholder
                let actual_value = Self::input_value_attr(original_node);
                // 如果已有状态，使用状态中的值
                let current_value = interaction.get_state(&id)
                    .map(|s| s.value.clone())
                    .unwrap_or(actual_value);
                
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Input,
                    id,
                    bounds: *bounds,
                    checked: false,
                    value: current_value,
                    disabled,
                    min: 0.0,
                    max: 0.0,
                    content_height: 0.0,
                    viewport_height: 0.0,
                    content_width: 0.0,
                    viewport_width: 0.0,
                    is_horizontal: false,
                    is_fixed,
                });
            }
            "button" => {
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Button,
                    id,
                    bounds: *bounds,
                    checked: false,
                    value: original_node.text.clone(),
                    disabled,
                    min: 0.0,
                    max: 0.0,
                    content_height: 0.0,
                    viewport_height: 0.0,
                    content_width: 0.0,
                    viewport_width: 0.0,
                    is_horizontal: false,
                    is_fixed,
                });
            }
            _ => {
                // view 等普通元素不需要注册为交互元素
                // 点击事件通过 event_bindings 处理
            }
        }
    }

    
    fn draw(&mut self, canvas: &mut Canvas, taffy: &Tree, node: &RenderNode, ox: f32, oy: f32) {
        if self.draw_transformed_plain(canvas, taffy, node, ox, oy, Color::BLACK) {
            return;
        }
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);

        self.draw_component(canvas, node, x, y, w, h, sf);
        
        if !Self::is_leaf_component(&node.tag) {
            let text_color = node.style.text_color.unwrap_or(Color::BLACK);
            for child in &node.children { 
                // fixed 子元素不在正常流内绘制，改由视口固定层单独绘制
                if child.style.is_fixed { continue; }
                self.draw_with_color(canvas, taffy, child, x, y, text_color); 
            }
        }

        for (et, h, d, is_catch) in &node.events {
            self.event_bindings.push(EventBinding { 
                event_type: et.clone(), 
                handler: h.clone(), 
                data: d.clone(), 
                bounds: logical_bounds,
                is_catch: *is_catch,
            });
        }
    }
    
    /// 无交互上下文的绘制路径（静态渲染 / 宿主外壳）上的动画与 transform 处理。
    fn draw_transformed_plain(
        &mut self,
        canvas: &mut Canvas,
        taffy: &Tree,
        node: &RenderNode,
        ox: f32,
        oy: f32,
        inherited: Color,
    ) -> bool {
        let Some((resolved, transform)) = self.resolve_animated_node(node) else { return false };
        if super::compose::is_identity(&transform) {
            self.draw_with_color(canvas, taffy, &resolved, ox, oy, inherited);
            return true;
        }
        if super::compose::is_translate_only(&transform) {
            self.draw_with_color(
                canvas,
                taffy,
                &resolved,
                ox + transform.translate_x,
                oy + transform.translate_y,
                inherited,
            );
            return true;
        }
        let layout = taffy.layout(node.taffy_node).unwrap();
        let (x, y) = (ox + layout.location.x, oy + layout.location.y);
        let (w, h) = (layout.size.width, layout.size.height);
        let pad = super::compose::transform_padding(w, h, &transform);
        let tw = (w + pad * 2.0).ceil() as u32;
        let th = (h + pad * 2.0).ceil() as u32;
        if w <= 0.0 || h <= 0.0 || tw == 0 || th == 0 || tw > 4096 || th > 4096 {
            self.draw_with_color(canvas, taffy, &resolved, ox, oy, inherited);
            return true;
        }
        let mut offscreen = Canvas::new(tw, th);
        offscreen.clear(Color::TRANSPARENT);
        let bindings_before = self.event_bindings.len();
        self.draw_with_color(
            &mut offscreen,
            taffy,
            &resolved,
            pad - layout.location.x,
            pad - layout.location.y,
            inherited,
        );
        self.event_bindings.truncate(bindings_before);
        super::compose::blit_transformed(
            canvas,
            &offscreen,
            (x - pad, y - pad),
            (x + w / 2.0, y + h / 2.0),
            &transform,
            1.0,
        );
        true
    }

    fn draw_with_color(&mut self, canvas: &mut Canvas, taffy: &Tree, node: &RenderNode, ox: f32, oy: f32, inherited_color: Color) {
        if self.draw_transformed_plain(canvas, taffy, node, ox, oy, inherited_color) {
            return;
        }
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);

        let text_color = node.style.text_color.unwrap_or(inherited_color);
        let mut node_with_color = Self::shallow_for_draw(node);
        if node_with_color.style.text_color.is_none() {
            node_with_color.style.text_color = Some(text_color);
        }

        self.draw_component(canvas, &node_with_color, x, y, w, h, sf);
        
        if !Self::is_leaf_component(&node.tag) {
            for child in &node.children { 
                // fixed 子元素不在正常流内绘制，改由视口固定层单独绘制
                if child.style.is_fixed { continue; }
                self.draw_with_color(canvas, taffy, child, x, y, text_color); 
            }
        }

        for (et, h, d, is_catch) in &node.events {
            self.event_bindings.push(EventBinding { 
                event_type: et.clone(), 
                handler: h.clone(), 
                data: d.clone(), 
                bounds: logical_bounds,
                is_catch: *is_catch,
            });
        }
    }

    /// 收集 fixed 子树（含嵌套）到列表。
    fn collect_fixed_nodes<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
        for child in &node.children {
            if child.style.is_fixed {
                out.push(child);
            }
            // fixed 子树内部不再单独收集（其内部随父一起按视口绘制）
            if !child.style.is_fixed {
                Self::collect_fixed_nodes(child, out);
            }
        }
    }

    /// 简单渲染路径下的 fixed 层：把 position:fixed 元素钉在视口而非内容流底部，
    /// 与浏览器 position:fixed 语义一致。
    ///
    /// 关键点：fixed 元素的 top/bottom/left/right 需相对「视口」解析，而 Taffy 是相对
    /// 根容器（内容全高）解析的。因此这里先按视口把该元素的宽/高约束出来，再以视口尺寸
    /// 为可用空间对其子树单独重排，最后按视口把整棵子树钉到目标位置——这样 top:0;bottom:0
    /// 的全屏遮罩才会是视口高、其居中的对话框才落在可见区内。
    fn draw_fixed_layer(&mut self, canvas: &mut Canvas, taffy: &mut Tree, roots: &[RenderNode], viewport_h: f32) {
        self.draw_fixed_layer_inner(canvas, taffy, roots, viewport_h, None);
    }

    /// fixed 覆盖层绘制。`interaction` 为 Some 时同时注册交互元素与命中区（宿主运行态），
    /// 为 None 时纯绘制（静态渲染/截图）。
    fn draw_fixed_layer_inner(
        &mut self,
        canvas: &mut Canvas,
        taffy: &mut Tree,
        roots: &[RenderNode],
        viewport_h: f32,
        mut interaction: Option<&mut InteractionManager>,
    ) {
        let sf = self.scale_factor;
        let vp_width = self.screen_width * sf;
        let mut fixed_ids: Vec<(NodeId, NodeStyle)> = Vec::new();
        {
            let mut fixed_nodes: Vec<&RenderNode> = Vec::new();
            for root in roots {
                if root.style.is_fixed {
                    fixed_nodes.push(root);
                }
                Self::collect_fixed_nodes(root, &mut fixed_nodes);
            }
            fixed_nodes.sort_by(|a, b| a.style.z_index.cmp(&b.style.z_index));
            for n in fixed_nodes {
                fixed_ids.push((n.taffy_node, n.style.clone()));
            }
        }

        // 建立 taffy_node -> RenderNode 引用查找（fixed 子树根）
        fn find_node<'a>(nodes: &'a [RenderNode], id: NodeId) -> Option<&'a RenderNode> {
            for n in nodes {
                if n.taffy_node == id { return Some(n); }
                if let Some(found) = find_node(&n.children, id) { return Some(found); }
            }
            None
        }

        for (id, st) in &fixed_ids {
            // 当前（相对根容器的）布局，用作缺省尺寸
            let cur = match taffy.layout(*id) { Ok(l) => *l, Err(_) => continue };
            let mut target_w = cur.size.width;
            let mut target_h = cur.size.height;
            // 视口约束：left+right → 宽；top+bottom → 高
            if let (Some(l), Some(r)) = (st.fixed_left, st.fixed_right) {
                target_w = (vp_width - l - r).max(0.0);
            }
            if let (Some(t), Some(b)) = (st.fixed_top, st.fixed_bottom) {
                target_h = (viewport_h - t - b).max(0.0);
            }

            // 以视口约束尺寸对该 fixed 子树单独重排（绝对定位，不影响主流）
            if let Ok(mut style) = taffy.style(*id).cloned() {
                style.position = Position::Relative;
                style.inset = Rect { top: auto(), right: auto(), bottom: auto(), left: auto() };
                style.size = Size { width: length(target_w), height: length(target_h) };
                if taffy.set_style(*id, style).is_ok() {
                    self.compute_with_text(
                        taffy,
                        *id,
                        Size {
                            width: AvailableSpace::Definite(target_w),
                            height: AvailableSpace::Definite(target_h),
                        },
                    );
                }
            }

            let new_layout = match taffy.layout(*id) { Ok(l) => *l, Err(_) => continue };
            let w = new_layout.size.width;
            let h = new_layout.size.height;

            let pinned_x = if let Some(left) = st.fixed_left {
                left
            } else if let Some(right) = st.fixed_right {
                vp_width - right - w
            } else {
                cur.location.x
            };
            let pinned_y = if let Some(bottom) = st.fixed_bottom {
                viewport_h - bottom - h
            } else if let Some(top) = st.fixed_top {
                top
            } else {
                cur.location.y
            };

            if let Some(node) = find_node(roots, *id) {
                // 重排后该节点作为子树根，location 约为 0，直接以钉住点为原点绘制
                let base_x = pinned_x - new_layout.location.x;
                let base_y = pinned_y - new_layout.location.y;
                match interaction.as_deref_mut() {
                    Some(im) => {
                        // 子树根自身的 is_fixed 需要清掉，否则会被「跳过 fixed」的分支拦下
                        let mut root = node.clone();
                        root.style.is_fixed = false;
                        self.draw_with_interaction(canvas, taffy, &root, base_x, base_y, im, 0.0, viewport_h);
                    }
                    None => self.draw(canvas, taffy, node, base_x, base_y),
                }
            }
        }
    }

    fn measure_text(&self, text: &str, size: f32) -> f32 {
        self.text_renderer.as_deref()
            .map(|tr| tr.measure_text(text, size))
            .unwrap_or(text.chars().count() as f32 * size * 0.6)
    }

    pub fn get_event_bindings(&self) -> &[EventBinding] { 
        &self.event_bindings 
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<&EventBinding> {
        self.event_bindings.iter().rev().find(|b| b.bounds.contains(&crate::Point::new(x, y)))
    }
    
    /// 命中测试并返回事件冒泡链。
    ///
    /// 对齐官方语义：`tap` 事件从最内层节点向外冒泡，`catchtap` 阻止继续冒泡。
    /// 由于渲染层是扁平的绑定列表，这里用包围盒面积升序近似节点由内到外的层级
    /// （子节点面积必然 <= 父节点）。遇到 `is_catch` 的绑定后停止（含该项）。
    ///
    /// 只返回与 `event_type` 匹配的绑定（点击对应 "tap"）。
    pub fn hit_test_bubble(&self, x: f32, y: f32, event_type: &str) -> Vec<EventBinding> {
        let p = crate::Point::new(x, y);
        let mut matched: Vec<EventBinding> = self.event_bindings.iter()
            .filter(|b| b.event_type == event_type && b.bounds.contains(&p))
            .cloned()
            .collect();
        
        // 面积升序：最内层（最小）在前
        matched.sort_by(|a, b| {
            let area_a = a.bounds.width * a.bounds.height;
            let area_b = b.bounds.width * b.bounds.height;
            area_a.partial_cmp(&area_b).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // 冒泡：从内到外，遇 catch 停止
        let mut chain = Vec::new();
        for b in matched {
            let stop = b.is_catch;
            chain.push(b);
            if stop {
                break;
            }
        }
        chain
    }
    
    /// 获取事件绑定数量
    pub fn event_count(&self) -> usize {
        self.event_bindings.len()
    }
    
    /// 打印所有事件绑定（调试用）
    pub fn debug_events(&self) {
        for (i, binding) in self.event_bindings.iter().enumerate() {
            println!("   [{}] {} -> {} bounds=({:.1},{:.1},{:.1},{:.1}) data={:?}", 
                i, binding.event_type, binding.handler,
                binding.bounds.x, binding.bounds.y, binding.bounds.width, binding.bounds.height,
                binding.data);
        }
    }
}

/// 按与 text.rs 绘制一致的贪心算法统计文本在给定可用宽度下的换行行数。
/// 用于第二遍布局修正文本盒子高度（含 `\n` 硬换行）。
fn count_wrapped_lines(tr: &TextRenderer, text: &str, max_width: f32, size: f32, letter_spacing: f32, bold: bool) -> usize {
    if max_width <= 0.0 {
        return text.split('\n').count().max(1);
    }
    let mut lines = 0usize;
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines += 1;
            continue;
        }
        let chars: Vec<char> = paragraph.chars().collect();
        let measure = |s: &[char]| -> f32 {
            s.iter().map(|c| tr.measure_char_weighted(*c, size, bold) + letter_spacing).sum()
        };
        lines += super::components::wrap_paragraph_lines(&chars, max_width, measure).len();
    }
    lines.max(1)
}
