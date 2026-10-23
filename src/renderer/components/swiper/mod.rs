//! Swiper 轮播组件：布局容器 + 状态机（`state.rs`）+ 缓动与时钟（`motion.rs`）。
//!
//! swiper **是一个真正的布局容器**，不是自绘控件：按 HTML 的 `.wx-swiper{display:flex}` +
//! `.wx-swiper-item{flex:0 0 100%}` 建立布局，当前页通过整行沿主轴偏移 + 裁剪呈现，item 子树
//! 走与普通节点完全相同的布局与绘制路径。`previous-margin` / `next-margin` 落成容器主轴上的
//! 内边距、`display-multiple-items` 落成 item 的百分比宽度（见 `wxml_renderer::layout`），
//! 于是相邻项露出一截、一屏放几项都由布局算出来，绘制只管偏移。
//!
//! 全局状态表按 swiper 的组件 id（`id` 属性 / `wx:key` 身份 / 坐标兜底，同事件绑定）存，
//! 换页动画、自动播放、手指拖动、事件队列都在这里；宿主每帧调 [`advance_due_swipers`]
//! 推进，取走 [`take_swiper_events`] 派发给逻辑层（`host::swiper`）。

/// Skyline 的 layout-type / transformer-type（项的变换与不透明度）
pub mod effects;
mod indicator;
mod motion;
mod state;

pub use motion::{now_ms as swiper_clock_ms, set_clock_ms as swiper_set_clock, Easing};
pub use state::{SwiperConfig, SwiperEvent, SwiperStateManager};

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::Canvas;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use taffy::prelude::*;

/// 全局 Swiper 状态管理器
pub static SWIPER_MANAGER: Lazy<Mutex<SwiperStateManager>> = Lazy::new(|| Mutex::new(SwiperStateManager::new()));

fn with<R>(default: R, f: impl FnOnce(&mut SwiperStateManager) -> R) -> R {
    SWIPER_MANAGER.lock().map(|mut m| f(&mut m)).unwrap_or(default)
}

/// 推进所有 swiper（换页动画、到点自动翻页），返回是否有 swiper 动了（宿主每帧调一次）。
///
/// 这一步必须与**绘制解耦**：从前只在绘制里推进自动播放，swiper 一旦没被画到（滚出视口、
/// 局部重绘帧里整棵子树被剪掉），计时就停在原地，`needs_frame` 每帧都说「要出帧」。
pub fn advance_due_swipers() -> bool {
    let now = motion::now_ms();
    with(false, |m| m.advance(now))
}

/// 取走 [`advance_due_swipers`] 推进时动了的 swiper（组件 id）。宿主据此只重画这几个盒子
pub fn take_moved_swipers() -> Vec<String> {
    with(Vec::new(), |m| m.take_moved())
}

/// 是否有 swiper 现在需要出帧（动画中、拖动中、自动播放到点）
pub fn swiper_needs_frame() -> bool {
    let now = motion::now_ms();
    with(false, |m| m.needs_frame(now)) || super::picker_view::needs_frame()
}

/// 是否存在正在自动播放的 swiper（宿主据此决定是否继续按刷新率出帧）
pub fn has_autoplay_swiper() -> bool {
    with(false, |m| m.any_autoplay())
}

/// 有没有 swiper 正被手指拖着（触摸序列据此补 touchcancel，与滚动接管一致）
pub fn swiper_is_dragging() -> bool {
    with(false, |m| m.any_dragging()) || super::picker_view::is_dragging()
}

/// 取走待派发的事件（change / transition / animationfinish）
pub fn take_swiper_events() -> Vec<SwiperEvent> {
    with(Vec::new(), |m| m.take_events())
}

/// 手势锁定到 swiper：从按下点开始拖（`coord` 是主轴上的逻辑坐标）
pub fn swiper_drag_begin(id: &str, coord: f32) {
    // picker-view 的列也登记成纵向 swiper 候选（手势仲裁一致），键带前缀，转给它自己的状态机
    if id.starts_with(super::picker_view::COLUMN_KEY_PREFIX) {
        return super::picker_view::drag_begin(id, coord);
    }
    let now = motion::now_ms();
    with((), |m| m.begin_drag(id, coord, now));
}

pub fn swiper_drag_update(id: &str, coord: f32) {
    if id.starts_with(super::picker_view::COLUMN_KEY_PREFIX) {
        return super::picker_view::drag_update(id, coord);
    }
    let now = motion::now_ms();
    with((), |m| m.update_drag(id, coord, now));
}

pub fn swiper_drag_end(id: &str) {
    if id.starts_with(super::picker_view::COLUMN_KEY_PREFIX) {
        return super::picker_view::drag_end(id);
    }
    let now = motion::now_ms();
    with((), |m| m.end_drag(id, now));
}

pub struct SwiperComponent;

impl SwiperComponent {
    /// swiper 默认高度（与 HTML 端 `.wx-swiper{height:150px}` 一致）
    pub const DEFAULT_HEIGHT: f32 = 150.0;

    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        let vertical = node.get_attr("vertical").map(truthy).unwrap_or(false);
        ts.flex_direction = if vertical { FlexDirection::Column } else { FlexDirection::Row };
        ts.flex_wrap = FlexWrap::NoWrap;
        if dim_is_auto(ts.size.width) {
            ts.size.width = percent(1.0_f32);
        }
        if dim_is_auto(ts.size.height) {
            ts.size.height = length(Self::DEFAULT_HEIGHT * sf);
        }
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        Some(RenderNode { tag: "swiper".into(), text: String::new(), attrs, taffy_node: tn, style: ns, children: vec![], events })
    }

    /// 这一帧的配置（属性已是模板求值后的值）
    pub fn config(node: &RenderNode) -> SwiperConfig {
        let a = &node.attrs;
        let num = |k: &str, d: u64| a.get(k).and_then(|s| s.trim().parse::<f64>().ok()).filter(|v| *v >= 0.0).map(|v| v as u64).unwrap_or(d);
        SwiperConfig {
            total: node.children.len(),
            per_view: num("display-multiple-items", 1).max(1) as usize,
            circular: Self::bool_attr(node, "circular"),
            vertical: Self::is_vertical(node),
            autoplay: Self::bool_attr(node, "autoplay"),
            interval: num("interval", 5000),
            duration: num("duration", 500),
            easing: a.get("easing-function").map(|s| Easing::parse(s)).unwrap_or_default(),
            current_attr: a.get("current").and_then(|s| s.trim().parse::<f64>().ok()).filter(|v| *v >= 0.0).map(|v| v as usize),
            direction: match a.get("direction").map(|s| s.trim()) {
                Some("positive") => 1,
                Some("negative") => -1,
                _ => 0,
            },
            scroll_with_animation: a.get("scroll-with-animation").map(|s| truthy(s)).unwrap_or(true),
        }
    }

    /// `previous-margin` / `next-margin`（逻辑 px，接受 px 与 rpx）
    pub fn margins(node: &RenderNode, screen_width: f32) -> (f32, f32) {
        let (_, _, m0, m1) = swiper_layout_params(&node.attrs, screen_width);
        (m0, m1)
    }

    pub fn is_vertical(node: &RenderNode) -> bool {
        Self::bool_attr(node, "vertical")
    }

    fn bool_attr(node: &RenderNode, name: &str) -> bool {
        node.attrs.get(name).map(|s| truthy(s)).unwrap_or(false)
    }

    /// 绘制指示点（在 item 内容之上）。颜色默认值按官方文档：`indicator-color` 为
    /// rgba(0,0,0,.3)、`indicator-active-color` 为 #000000
    #[allow(clippy::too_many_arguments)]
    /// 指示点（尺寸 / 位置属性与 Skyline `indicator-type` 动画见 `indicator.rs`）。`pos` 是连续位置
    #[allow(clippy::too_many_arguments)]
    pub fn draw_indicators(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32, current: usize, pages: usize, pos: f32) {
        let show = Self::bool_attr(node, "indicator-dots");
        if !show || pages < 2 {
            return;
        }
        let circular = Self::bool_attr(node, "circular");
        indicator::draw(node, canvas, (x, y, w, h), sf, pos, current, pages, Self::is_vertical(node), circular);
    }
}

/// 布尔属性：写了且不是 `false` / `0` 就是真（`<swiper autoplay>` 不带值也算开）
fn truthy(s: &str) -> bool {
    let s = s.trim();
    s != "false" && s != "0"
}

/// 布局要用的四个属性：(纵向, 一屏几项, 前边距, 后边距)，边距为逻辑 px
pub fn swiper_layout_params(attrs: &std::collections::HashMap<String, String>, screen_width: f32) -> (bool, usize, f32, f32) {
    let vertical = attrs.get("vertical").map(|s| truthy(s)).unwrap_or(false);
    let per_view = attrs
        .get("display-multiple-items")
        .and_then(|s| s.trim().parse::<f64>().ok())
        .map(|v| v.max(1.0) as usize)
        .unwrap_or(1);
    let len = |k: &str| {
        attrs
            .get(k)
            .and_then(|v| {
                let v = v.trim();
                match v.strip_suffix("rpx") {
                    Some(n) => n.trim().parse::<f32>().ok().map(|n| n * screen_width / 750.0),
                    None => v.strip_suffix("px").unwrap_or(v).trim().parse::<f32>().ok(),
                }
            })
            .unwrap_or(0.0)
            .max(0.0)
    };
    (vertical, per_view, len("previous-margin"), len("next-margin"))
}

/// 页数：非衔接 `total − display-multiple-items + 1`，衔接 `total`
pub fn swiper_pages(cfg: &SwiperConfig) -> usize {
    if cfg.circular {
        cfg.total.max(1)
    } else {
        (cfg.total.saturating_sub(cfg.per_view.max(1)) + 1).max(1)
    }
}

/// SwiperItem 组件
pub struct SwiperItemComponent;

impl SwiperItemComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        // 对齐 HTML `.wx-swiper-item{flex:0 0 100%;height:100%}`；一屏几项、两侧边距由
        // 布局阶段按父 swiper 的属性改写（见 wxml_renderer::layout）
        ts.size.width = percent(1.0_f32);
        ts.min_size.width = percent(1.0_f32);
        ts.size.height = percent(1.0_f32);
        ts.flex_shrink = 0.0;
        ts.flex_grow = 0.0;
        ts.flex_direction = FlexDirection::Column;
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        Some(RenderNode { tag: "swiper-item".into(), text: String::new(), attrs, taffy_node: tn, style: ns, children: vec![], events })
    }
}
