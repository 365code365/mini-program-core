//! scroll-view 的事件与页面级滚动请求，**SDK 引擎与桌面窗体共用**（各写一份的话迟早一边漏）。
//!
//! 每帧在推进完滚动控制器之后调一次 [`tick`]：
//! - `bindscroll`（位置变了）：`detail = {scrollLeft, scrollTop, scrollHeight, scrollWidth, deltaX, deltaY, isDrag}`；
//! - `bindscrolltoupper` / `bindscrolltolower`：**进入**阈值区的那一帧发一次（`detail.direction`
//!   为 top / bottom / left / right），离开后再进入才再发 —— 列表加载更多后内容变长，自然重新可触发；
//! - Skyline 的 `bindscrollstart` / `bindscrollend`（`{isDrag}`），`enhanced` 时的
//!   `binddragstart` / `binddragging` / `binddragend`；
//! - 自定义下拉刷新：`refresherpulling`（`{dy}`）→ 过阈值 `refresherwillrefresh` → 松手
//!   `refresherrefresh`（指示器停在阈值处）或 `refresherabort`（没过阈值）→ 页面把
//!   `refresher-triggered` 设成 false 后 `refresherrestore`。
//!
//! 第一次见到一个 scroll-view 时只记状态不发事件（页面刚打开停在顶部，不该冒一个 scrolltoupper）。
//! 依据：官方 component/scroll-view.html。`deltaX` / `deltaY` 的符号按微信：内容往上走
//! （scrollTop 变大）时 deltaY 为负，即 `上一次的值 − 这一次的值`。
//!
//! [`apply_page_scroll`]：`wx.pageScrollTo` 的请求（selector 换算位置、按 duration 滑过去）。
//! [`page_scroll`]：页面 `onPageScroll`（桌面窗体在 `update_scroll` 里另有一份，SDK 从前没有）。

use crate::renderer::WxmlRenderer;
use crate::runtime::MiniApp;
use crate::ui::interaction::InteractionManager;
use crate::ui::scroll_controller::ScrollController;
use crate::ui::scroll_view_state::RefresherPhase;
use serde_json::{json, Value};

/// 每帧推进 scroll-view 的事件。返回是否派发过事件（宿主据此看要不要取数据、重画）
pub fn tick(app: &mut MiniApp, renderer: Option<&WxmlRenderer>, interaction: &mut InteractionManager, time_ms: u64) -> bool {
    let Some(r) = renderer else { return false };
    let infos: Vec<_> = r.scroll_views().iter().filter(|s| s.scrollable).cloned().collect();
    let mut fired = false;
    for info in infos {
        let id = info.component_id.clone();
        let Some(c) = interaction.get_scroll_controller_mut(&id) else { continue };
        if let Some(rf) = &info.refresher {
            c.set_pull_threshold(rf.threshold);
        }
        let (pos, max, dragging, animating, gap) = (c.get_position(), c.get_max_scroll(), c.is_dragging, c.is_animating(), c.top_gap());
        let st = interaction.scroll_views.entry(id.clone()).or_default();
        let mut out: Vec<(&'static str, Value)> = Vec::new();
        let (sl, stp) = if info.horizontal { (pos, 0.0) } else { (0.0, pos) };
        let in_upper = pos <= info.upper_threshold;
        let in_lower = max > 0.0 && max - pos <= info.lower_threshold;
        if !st.tracked {
            st.tracked = true;
            st.reported_pos = pos;
            st.in_upper = in_upper;
            st.in_lower = in_lower;
            st.dragging = dragging;
            st.last_triggered = info.refresher.as_ref().map(|r| r.triggered);
            continue;
        }
        // 拖动开始 / 滚动开始
        if dragging && !st.dragging && info.enhanced {
            out.push(("dragstart", json!({ "scrollTop": stp, "scrollLeft": sl })));
        }
        let moved = (pos - st.reported_pos).abs() > 0.01;
        let moving = dragging || animating || moved;
        if moving && !st.moving {
            st.drag_started = dragging;
            out.push(("scrollstart", json!({ "isDrag": dragging })));
        }
        if moved {
            let d = st.reported_pos - pos;
            let (dx, dy) = if info.horizontal { (d, 0.0) } else { (0.0, d) };
            out.push((
                "scroll",
                json!({
                    "scrollLeft": sl, "scrollTop": stp,
                    "scrollHeight": info.content_height.max(info.bounds.height),
                    "scrollWidth": info.content_width.max(info.bounds.width),
                    "deltaX": dx, "deltaY": dy, "isDrag": dragging,
                }),
            ));
            if dragging && info.enhanced {
                out.push(("dragging", json!({ "scrollTop": stp, "scrollLeft": sl })));
            }
            st.reported_pos = pos;
        }
        if !dragging && st.dragging && info.enhanced {
            out.push(("dragend", json!({ "scrollTop": stp, "scrollLeft": sl, "velocity": { "x": 0, "y": 0 } })));
        }
        if !moving && st.moving {
            out.push(("scrollend", json!({ "isDrag": st.drag_started })));
        }
        // 阈值：进入那一帧发一次
        if in_upper && !st.in_upper && moved {
            out.push(("scrolltoupper", json!({ "direction": if info.horizontal { "left" } else { "top" } })));
        }
        if in_lower && !st.in_lower && moved {
            out.push(("scrolltolower", json!({ "direction": if info.horizontal { "right" } else { "bottom" } })));
        }
        st.in_upper = in_upper;
        st.in_lower = in_lower;
        let was_dragging = st.dragging;
        st.moving = moving;
        st.dragging = dragging;
        // 自定义下拉刷新
        if let Some(rf) = &info.refresher {
            let prev_triggered = st.last_triggered;
            st.last_triggered = Some(rf.triggered);
            let (phase, pulled, refresh_seq) = (st.refresher, st.pulled, st.refresh_seq);
            let mut next = phase;
            let mut new_pulled = pulled;
            let mut enter_refresh = false;
            let mut restore = false;
            match phase {
                RefresherPhase::Idle | RefresherPhase::Pulling if dragging && gap > 0.0 => {
                    next = RefresherPhase::Pulling;
                    if (gap - pulled).abs() > 0.5 {
                        if pulled < rf.threshold && gap >= rf.threshold {
                            out.push(("refresherwillrefresh", json!({ "dy": gap })));
                        }
                        out.push(("refresherpulling", json!({ "dy": gap })));
                        new_pulled = gap;
                    }
                }
                RefresherPhase::Pulling if !dragging && was_dragging => {
                    new_pulled = 0.0;
                    if interaction.get_scroll_controller_mut(&id).map(|c| c.take_pull_trigger()).unwrap_or(false) {
                        enter_refresh = true;
                        out.push(("refresherrefresh", json!({ "dy": rf.threshold })));
                    } else {
                        next = RefresherPhase::Idle;
                        out.push(("refresherabort", json!({ "dy": 0 })));
                    }
                }
                RefresherPhase::Refreshing => {
                    // 设回 false：属性从 true 变成 false，或者一直是 false 但页面又 setData 过它绑定的字段
                    restore = !rf.triggered
                        && (prev_triggered == Some(true)
                            || (!rf.triggered_keys.is_empty() && set_since(app, &info.owner, &rf.triggered_keys, refresh_seq)));
                }
                // 页面主动把 refresher-triggered 设成 true：进刷新态（指示器出现），不发 refresherrefresh
                RefresherPhase::Idle if rf.triggered && prev_triggered == Some(false) => enter_refresh = true,
                _ => {}
            }
            let seq = if enter_refresh { set_data_seq(app) } else { refresh_seq };
            if let Some(c) = interaction.get_scroll_controller_mut(&id) {
                if enter_refresh {
                    c.set_top_inset(rf.threshold);
                } else if restore {
                    c.set_top_inset(0.0);
                }
            }
            if enter_refresh {
                next = RefresherPhase::Refreshing;
            } else if restore {
                next = RefresherPhase::Idle;
                out.push(("refresherrestore", json!({})));
            }
            let st = interaction.scroll_views.entry(id.clone()).or_default();
            st.refresher = next;
            st.pulled = new_pulled;
            st.refresh_seq = seq;
        }
        for (name, detail) in out {
            fired |= super::own_events::dispatch_own(app, r, &id, name, detail, time_ms);
        }
    }
    if fired {
        app.pump_jobs();
    }
    fired
}

fn set_data_seq(app: &MiniApp) -> u64 {
    app.eval("__setDataSeq").ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

fn set_since(app: &MiniApp, owner: &str, keys: &[String], seq: u64) -> bool {
    let o = serde_json::to_string(owner).unwrap_or_else(|_| "''".into());
    let k = serde_json::to_string(keys).unwrap_or_else(|_| "[]".into());
    app.eval(&format!("__setSince({o}, {k}, {seq})")).map(|s| s.trim() == "1").unwrap_or(false)
}

/// 处理 `wx.pageScrollTo` 的请求：selector 优先（取第一个命中节点的顶边 + offsetTop），
/// 否则 scrollTop；按 duration 滑过去（0 就直接跳）。返回是否动了页面
pub fn apply_page_scroll(
    app: &mut MiniApp,
    renderer: Option<&WxmlRenderer>,
    interaction: &InteractionManager,
    page: &mut ScrollController,
) -> bool {
    let mut moved = false;
    for req in app.take_page_scroll_requests() {
        let sel = req.get("selector").and_then(|s| s.as_str()).unwrap_or("");
        let target = if !sel.is_empty() {
            let hit = renderer.and_then(|r| r.query_selector(sel, false, interaction).into_iter().next());
            match hit {
                Some(h) => h.rect.y + req.get("offsetTop").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                None => {
                    eprintln!("⚠️ wx.pageScrollTo：selector {sel:?} 没找到节点");
                    continue;
                }
            }
        } else {
            match req.get("scrollTop").and_then(|v| v.as_f64()) {
                Some(t) => t as f32,
                None => continue,
            }
        };
        let dur = req.get("duration").and_then(|v| v.as_f64()).unwrap_or(300.0) as f32 / 1000.0;
        page.animate_to(target, dur);
        moved = true;
    }
    moved
}

/// 页面 `onPageScroll`：位置变了超过半个像素就回调一次（与桌面 `update_scroll` 同一个阈值）
pub fn page_scroll(app: &mut MiniApp, pos: f32, last_sent: &mut f32) -> bool {
    if (pos - *last_sent).abs() <= 0.5 {
        return false;
    }
    *last_sent = pos;
    app.eval(&format!("__dispatchPage && __dispatchPage('onPageScroll', {{ scrollTop: {pos} }})")).ok();
    true
}

impl super::engine::MiniEngine {
    /// 当前页面定义了 `onPageScroll`：没定义就不派发（每页问一次逻辑层，结果按页记住）。
    ///
    /// 与微信一致（官方文档：「请只在需要的时候才在 page 中定义此方法，不要定义空方法」——
    /// 没定义时渲染层根本不往逻辑层发滚动事件）。从前每滚一帧都 `eval` 一段派发脚本：
    /// 一加 PLY110 上快甩时 QuickJS 光解析这段就占引擎线程 7%，页面根本没有这个回调
    pub(super) fn page_wants_scroll_events(&mut self) -> bool {
        let key = (self.page_stack.len(), self.page_opened_ms, self.current_route());
        if let Some((k, v)) = &self.page_scroll_handler {
            if *k == key {
                return *v;
            }
        }
        let v = self
            .app
            .eval("(typeof __currentPage === 'object' && __currentPage && typeof __currentPage.onPageScroll === 'function') ? 1 : 0")
            .map(|r| r.trim() == "1")
            .unwrap_or(true);
        self.page_scroll_handler = Some((key, v));
        v
    }
}

/// `InteractionManager::scroll_views` 里给页面 `onPageScroll` 记「上次报的位置」用的键。
/// 组件 id 不会是这个（NUL 开头），换页时随 `clear_page_state` 一起清
const PAGE_KEY: &str = "\u{0}page";

impl super::engine::MiniEngine {
    /// 某个 scroll-view 当前的滚动偏移（逻辑 px；`id` 是它的 `id` 属性或组件 id）。
    /// 诊断与测试用：宿主不需要它也能工作
    pub fn scroll_view_offset(&self, id: &str) -> Option<f32> {
        self.interaction.get_scroll_controller(id).map(|c| c.get_position())
    }

    /// SDK 每帧：scroll-view 事件 → `wx.pageScrollTo` → `onPageScroll`。返回是否要重画
    pub(super) fn tick_scroll_views(&mut self, now_ms: u64) -> bool {
        // swiper 的换页动画与自动播放按宿主给的时钟走（同一串 pump 时间 → 同一串画面）
        crate::renderer::components::swiper_set_clock(now_ms.max(1));
        let mut redraw = tick(&mut self.app, self.renderer.as_ref(), &mut self.interaction, now_ms);
        // swiper / picker-view 的事件（change / transition / animationfinish …）派完不用整页重画：
        // 回调里 setData 走下面的失效分析，swiper 自己的位移由 engine_swiper.rs 只重画它的盒子，
        // picker-view 的吸附动画照旧按「在动」整帧。从前这里 `redraw = true` → `mark_full`：
        // 轮播页每次自动翻页那一帧整页重画（一加上 60ms，bindchange 里改了一行字）
        super::swiper::tick(&mut self.app, self.renderer.as_ref(), now_ms);
        // picker 面板弹着时跟上页面改的 range（multiSelector 联动）
        if super::picker_sheet::sync_columns(&mut self.picker_sheet, self.renderer.as_ref()) {
            redraw = true;
        }
        // VideoContext 命令与 video 的组件事件（全屏画在合成那一步，要重新合成）
        if super::video::tick(&mut self.app, self.renderer.as_ref(), &self.interaction, now_ms) {
            redraw = true;
            self.dmg.compose_dirty = true;
        }
        // SelectorQuery / ScrollViewContext / IntersectionObserver（与桌面共用）
        let vp = self.viewport_height();
        let view = super::selector_query::PageView {
            scroll_top: self.scroll.get_position(),
            width: self.width as f32,
            height: vp,
            content_height: self.scroll.get_max_scroll() + vp,
        };
        if super::selector_query::tick(&mut self.app, self.renderer.as_ref(), &mut self.interaction, view) {
            redraw = true;
        }
        let route = self.current_route();
        super::observers::tick(&mut self.app, self.renderer.as_ref(), &self.interaction, &route, view, now_ms);
        // 操作菜单 / 图片预览的请求与动画（画在合成那一步，只要重新合成）
        if super::ui_overlay::take_sheet_requests(&mut self.app) {
            self.dmg.compose_dirty = true;
        }
        // wx.setBackgroundColor / wx.hideKeyboard
        let (wa_redraw, _) = super::window_api::take_requests(&mut self.app, self.renderer.as_ref(), &mut self.interaction);
        if wa_redraw {
            redraw = true;
        }
        // 原生 tabBar 的运行时 API：改了就重画 tabBar；显隐变了视口高度跟着变，重新排版
        let was_hidden = super::tabbar_api::hidden();
        if super::tabbar_api::take_requests(&mut self.app, self.config.tab_bar.as_ref()) {
            self.dmg.tabbar_route = None;
            redraw = true;
            if super::tabbar_api::hidden() != was_hidden {
                self.update_renderers();
            }
        }
        if apply_page_scroll(&mut self.app, self.renderer.as_ref(), &self.interaction, &mut self.scroll) {
            redraw = true;
        }
        let pos = self.scroll.get_position();
        let wants = self.page_wants_scroll_events();
        let st = self.interaction.scroll_views.entry(PAGE_KEY.to_string()).or_default();
        let mut last = if st.tracked { st.reported_pos } else { 0.0 };
        st.tracked = true;
        if wants && page_scroll(&mut self.app, pos, &mut last) {
            self.scroll_moved = true;
        }
        self.interaction.scroll_views.entry(PAGE_KEY.to_string()).or_default().reported_pos = last;
        // 事件处理函数里的 setData 这一帧就生效（不等下一帧开头）。
        //
        // 只记「数据脏了」，**不**要整页重画：与别处的 setData 一样交给失效分析（只重画变了的节点）。
        // 从前这里 `redraw = true` → 调用方 `mark_full`：onPageScroll / IntersectionObserver 回调里
        // 一 setData 就整页重画 —— 一加 PLY110 上滑「节点与滚动」页 42~71 帧 `整帧·结构`、
        // 每帧 24~31ms，屏上重复 9~11%（tests/sdk_damage_equivalence.rs 有这一条）
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
        }
        redraw
    }
}
