//! Swiper 轮播图组件

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::{Canvas, Color, Paint, PaintStyle};
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use taffy::prelude::*;

/// Swiper 状态管理器
pub struct SwiperStateManager {
    states: HashMap<String, SwiperState>,
}

/// 翻页滑动时长（毫秒）。微信的 swiper 换页是滑过去的，不是硬切。
pub const SLIDE_DURATION_MS: f32 = 300.0;

#[derive(Clone)]
pub struct SwiperState {
    pub current: usize,
    pub total: usize,
    pub last_update: std::time::Instant,
    pub autoplay_interval: u64,
    pub autoplay: bool,
    /// 上一页序号：换页后的 `SLIDE_DURATION_MS` 内在它与 `current` 之间插值，
    /// 于是换页是滑动而不是跳变。
    pub prev: usize,
}

impl SwiperStateManager {
    pub fn new() -> Self {
        Self { states: HashMap::new() }
    }
    
    pub fn get_or_create(&mut self, id: &str, total: usize, autoplay: bool, interval: u64) -> &mut SwiperState {
        self.states.entry(id.to_string()).or_insert_with(|| SwiperState {
            current: 0,
            total,
            last_update: std::time::Instant::now(),
            autoplay_interval: interval,
            autoplay,
            prev: 0,
        })
    }
    
    pub fn get(&self, id: &str) -> Option<&SwiperState> {
        self.states.get(id)
    }
    
    pub fn set_current(&mut self, id: &str, current: usize) {
        if let Some(state) = self.states.get_mut(id) {
            state.prev = state.current;
            state.current = current;
            state.last_update = std::time::Instant::now();
        }
    }
    
    pub fn next(&mut self, id: &str) {
        if let Some(state) = self.states.get_mut(id) {
            state.prev = state.current;
            state.current = (state.current + 1) % state.total;
            state.last_update = std::time::Instant::now();
        }
    }
    
    pub fn prev(&mut self, id: &str) {
        if let Some(state) = self.states.get_mut(id) {
            if state.current == 0 {
                state.current = state.total.saturating_sub(1);
            } else {
                state.current -= 1;
            }
            state.last_update = std::time::Instant::now();
        }
    }
    
    /// 当前登记了哪些 swiper（测试与诊断用；正常渲染路径不需要）
    pub fn debug_keys(&self) -> Vec<String> {
        let mut k: Vec<String> = self.states.keys().cloned().collect();
        k.sort();
        k
    }
    /// 是否存在开启了自动播放且多于一屏的 swiper
    pub fn any_autoplay(&self) -> bool {
        self.states.values().any(|s| s.autoplay && s.total > 1)
    }

    /// 检查并执行自动播放
    pub fn check_autoplay(&mut self, id: &str) -> bool {
        if let Some(state) = self.states.get_mut(id) {
            if state.autoplay && state.total > 1 {
                let elapsed = state.last_update.elapsed().as_millis() as u64;
                if elapsed >= state.autoplay_interval {
                    state.prev = state.current;
                    state.current = (state.current + 1) % state.total;
                    state.last_update = std::time::Instant::now();
                    return true;
                }
            }
        }
        false
    }

    /// 把所有「自动播放已到点」的 swiper 推进一页，返回是否有 swiper 真的翻了页。
    ///
    /// 这一步必须与**绘制解耦**。从前只有 `draw_swiper_container` 里会调
    /// `check_autoplay`，于是 swiper 一旦没被画到（滚出视口被裁掉、或者局部重绘帧
    /// 里整棵子树被剪掉），它的 `last_update` 就永远停在原地：
    /// `elapsed >= interval` 恒成立 → `any_needs_frame()` 每帧都说"要出帧" →
    /// 宿主每帧整屏重绘。带轮播的长页面因此被永久钉在 ~100FPS 且帧间隔忽大忽小。
    ///
    /// 现在由宿主每帧调一次：翻页只在到点那一帧发生，其余帧 swiper 不再索要重绘。
    pub fn advance_due_autoplay(&mut self) -> bool {
        let mut flipped = false;
        for state in self.states.values_mut() {
            if !state.autoplay || state.total <= 1 {
                continue;
            }
            if state.last_update.elapsed().as_millis() as u64 >= state.autoplay_interval {
                state.prev = state.current;
                state.current = (state.current + 1) % state.total;
                state.last_update = std::time::Instant::now();
                flipped = true;
            }
        }
        flipped
    }

    /// 是否有 swiper 现在就需要出帧：正在滑动换页，或自动播放已到点该翻页。
    ///
    /// 从前只要 swiper 有多于一项就无条件要求「每帧重绘」，而 swiper 本身是硬切页 ——
    /// 等于让带轮播的页面永远按刷新率整屏重绘（首页每帧 8~14ms 就是这么来的）。
    pub fn any_needs_frame(&self) -> bool {
        self.states.values().any(|s| {
            if s.total <= 1 {
                return false;
            }
            let elapsed = s.last_update.elapsed().as_millis() as f32;
            if elapsed < SLIDE_DURATION_MS {
                return true; // 换页滑动进行中
            }
            s.autoplay && elapsed >= s.autoplay_interval as f32
        })
    }

    /// 当前换页滑动的位置（在 `prev` 与 `current` 之间的浮点页号）与是否仍在滑动
    pub fn slide_position(&self, id: &str) -> Option<(f32, bool)> {
        let s = self.states.get(id)?;
        let elapsed = s.last_update.elapsed().as_millis() as f32;
        if elapsed >= SLIDE_DURATION_MS || s.prev == s.current {
            return Some((s.current as f32, false));
        }
        // ease-out cubic：起步快、收尾稳，和微信的换页手感接近
        let t = (elapsed / SLIDE_DURATION_MS).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - t).powi(3);
        Some((s.prev as f32 + (s.current as f32 - s.prev as f32) * eased, true))
    }
}

/// 全局 Swiper 状态管理器
pub static SWIPER_MANAGER: Lazy<Mutex<SwiperStateManager>> = Lazy::new(|| {
    Mutex::new(SwiperStateManager::new())
});

/// 推进所有到点的自动播放 swiper，返回是否有 swiper 翻页（宿主每帧调一次）
pub fn advance_due_swipers() -> bool {
    SWIPER_MANAGER
        .lock()
        .map(|mut m| m.advance_due_autoplay())
        .unwrap_or(false)
}

/// 是否有 swiper 现在需要出帧（换页滑动中，或自动播放到点）
pub fn swiper_needs_frame() -> bool {
    SWIPER_MANAGER
        .lock()
        .map(|m| m.any_needs_frame())
        .unwrap_or(false)
}

/// 是否存在正在自动播放的 swiper（宿主据此决定是否继续按刷新率出帧）
pub fn has_autoplay_swiper() -> bool {
    SWIPER_MANAGER
        .lock()
        .map(|m| m.any_autoplay())
        .unwrap_or(false)
}

/// Swiper 组件
///
/// 设计说明：swiper **是一个真正的布局容器**，不是自绘控件。
/// 它按 HTML 的 `.wx-swiper{display:flex;flex-direction:row}` +
/// `.wx-swiper-item{flex:0 0 100%}` 建立布局，当前页通过整行横向偏移 + 裁剪呈现。
///
/// 早先的实现把 swiper 建成叶子节点，item 里的内容由组件自己"猜着画"
/// （见到 image 就铺满并 return、见到带背景的 view 就当成 28px 标签……），
/// 于是 banner 上的浮层文字、flex 排版、绝对定位、动画全都失效 —— 那等于在引擎里
/// 塞了第二套渲染器。现在整棵 item 子树走与普通节点完全相同的布局与绘制路径。
pub struct SwiperComponent;

impl SwiperComponent {
    /// swiper 默认高度（与 HTML 端 `.wx-swiper{height:150px}` 一致）
    pub const DEFAULT_HEIGHT: f32 = 150.0;

    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;

        // 排布方向随 `vertical` 属性；不换行、超出隐藏；未指定高度时用默认高度
        let vertical = node
            .get_attr("vertical")
            .map(|s| s == "true" || s == "{{true}}")
            .unwrap_or(false);
        ts.flex_direction = if vertical { FlexDirection::Column } else { FlexDirection::Row };
        ts.flex_wrap = FlexWrap::NoWrap;
        if dim_is_auto(ts.size.width) {
            ts.size.width = percent(1.0_f32);
        }
        if dim_is_auto(ts.size.height) {
            ts.size.height = length(Self::DEFAULT_HEIGHT * sf);
        }

        // 子树由渲染器统一构建（swiper 不再是叶子）
        let tn = ctx.taffy.new_leaf(ts).unwrap();

        Some(RenderNode {
            tag: "swiper".into(),
            text: String::new(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }

    /// 稳定的 swiper 标识：优先用 id，否则用位置兜底
    pub fn state_id(node: &RenderNode, x: f32, y: f32) -> String {
        node.attrs
            .get("id")
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("swiper_{:.0}_{:.0}", x, y))
    }

    /// 取当前页下标，并推进自动播放。
    pub fn current_index(node: &RenderNode, x: f32, y: f32) -> usize {
        let total = node.children.len();
        if total == 0 {
            return 0;
        }
        let id = Self::state_id(node, x, y);
        let autoplay = Self::bool_attr(node, "autoplay");
        let interval = node
            .attrs
            .get("interval")
            .and_then(|s| s.parse().ok())
            .unwrap_or(5000u64);
        // WXML 显式给了 current 时以它为准（受控用法）
        let explicit = node.attrs.get("current").and_then(|s| s.parse::<usize>().ok());

        if let Ok(mut manager) = SWIPER_MANAGER.lock() {
            {
                let state = manager.get_or_create(&id, total, autoplay, interval);
                state.total = total;
                state.autoplay = autoplay;
                state.autoplay_interval = interval;
            }
            manager.check_autoplay(&id);
            let mut current = manager.get(&id).map(|s| s.current).unwrap_or(0);
            if let Some(idx) = explicit {
                // 受控值变化时同步状态（避免自动播放与页面 current 打架）
                if !autoplay && idx < total && idx != current {
                    manager.set_current(&id, idx);
                    current = idx;
                }
            }
            current.min(total - 1)
        } else {
            0
        }
    }

    /// 当前换页滑动位置（浮点页号）与是否仍在滑动。
    /// 必须在 [`Self::current_index`] 之后调用（后者负责推进自动播放状态）。
    pub fn slide_position(node: &RenderNode, x: f32, y: f32) -> Option<(f32, bool)> {
        let id = Self::state_id(node, x, y);
        SWIPER_MANAGER.lock().ok()?.slide_position(&id)
    }

    /// 是否纵向轮播（偏移轴由此决定）
    pub fn is_vertical(node: &RenderNode) -> bool {
        Self::bool_attr(node, "vertical")
    }

    fn bool_attr(node: &RenderNode, name: &str) -> bool {
        node.attrs
            .get(name)
            .map(|s| s == "true" || s == "{{true}}")
            .unwrap_or(false)
    }

    /// 绘制指示点（在 item 内容之上）
    pub fn draw_indicators(
        node: &RenderNode,
        canvas: &mut Canvas,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        sf: f32,
        current: usize,
    ) {
        let total = node.children.len();
        let show = node
            .attrs
            .get("indicator-dots")
            .map(|s| s == "true" || s == "{{true}}")
            .unwrap_or(false);
        if !show || total < 2 {
            return;
        }
        let indicator_color = node
            .attrs
            .get("indicator-color")
            .and_then(|s| parse_color_str(s))
            .unwrap_or(Color::new(0, 0, 0, 76));
        let active_color = node
            .attrs
            .get("indicator-active-color")
            .and_then(|s| parse_color_str(s))
            .unwrap_or(Color::WHITE);

        let dot = 7.0 * sf;
        let gap = 6.0 * sf;
        let total_w = total as f32 * dot + (total - 1) as f32 * gap;
        let start_x = x + (w - total_w) / 2.0;
        let cy = y + h - 8.0 * sf - dot / 2.0;
        for i in 0..total {
            let cx = start_x + i as f32 * (dot + gap) + dot / 2.0;
            let color = if i == current { active_color } else { indicator_color };
            let paint = Paint::new()
                .with_color(color)
                .with_style(PaintStyle::Fill)
                .with_anti_alias(true);
            canvas.draw_circle(cx, cy, dot / 2.0, &paint);
        }
    }

    /// 获取当前显示的 swiper-item 索引
    pub fn get_current(swiper_id: &str) -> usize {
        if let Ok(manager) = SWIPER_MANAGER.lock() {
            manager.get(swiper_id).map(|s| s.current).unwrap_or(0)
        } else {
            0
        }
    }
    
    /// 切换到下一页
    pub fn next(swiper_id: &str) {
        if let Ok(mut manager) = SWIPER_MANAGER.lock() {
            manager.next(swiper_id);
        }
    }
    
    /// 切换到上一页
    pub fn prev(swiper_id: &str) {
        if let Ok(mut manager) = SWIPER_MANAGER.lock() {
            manager.prev(swiper_id);
        }
    }
    
    /// 切换到指定页
    pub fn set_current(swiper_id: &str, index: usize) {
        if let Ok(mut manager) = SWIPER_MANAGER.lock() {
            manager.set_current(swiper_id, index);
        }
    }
}

/// SwiperItem 组件
pub struct SwiperItemComponent;

impl SwiperItemComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        
        // 对齐 HTML `.wx-swiper-item{flex:0 0 100%;height:100%}`：
        // 每个 item 占满一屏且不参与收缩，整行由 swiper 横向偏移呈现
        ts.size.width = percent(1.0_f32);
        ts.min_size.width = percent(1.0_f32);
        ts.size.height = percent(1.0_f32);
        ts.flex_shrink = 0.0;
        ts.flex_grow = 0.0;
        ts.flex_direction = FlexDirection::Column;
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "swiper-item".into(),
            text: String::new(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
}
