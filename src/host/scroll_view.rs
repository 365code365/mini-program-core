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
    // 按引用遍历：每帧都走（每个 scroll-view 一次），从前先整份克隆 `ScrollViewInfo`（组件 id、
    // refresher 的键…）再 `entry(id.clone())`，滚着的时候每帧几次 jemalloc（真机上带锁）
    let mut fired = false;
    for info in r.scroll_views().iter().filter(|s| s.scrollable) {
        let id = info.component_id.as_str();
        let Some(c) = interaction.get_scroll_controller_mut(id) else { continue };
        if let Some(rf) = &info.refresher {
            c.set_pull_threshold(rf.threshold);
        }
        let (pos, max, dragging, animating, gap) = (c.get_position(), c.get_max_scroll(), c.is_dragging, c.is_animating(), c.top_gap());
        let release_v = c.release_velocity();
        // 横竖都能滚：横向那一半在 `"{id}#x"` 控制器上（ui/interaction/scroll_2d.rs）。单向的全是 None
        let xs = if info.both { x_state(interaction, id) } else { None };
        let (dragging, animating) = match xs {
            Some(x) => (dragging || x.dragging, animating || x.animating),
            None => (dragging, animating),
        };
        let st = state(&mut interaction.scroll_views, id);
        let mut out: Vec<(&'static str, Value)> = Vec::new();
        let (sl, stp) = match xs {
            Some(x) => (x.pos, pos),
            None if info.horizontal => (pos, 0.0),
            None => (0.0, pos),
        };
        let in_upper = pos <= info.upper_threshold;
        let in_lower = max > 0.0 && max - pos <= info.lower_threshold;
        let (in_left, in_right) = xs.map(|x| (x.pos <= info.upper_threshold, x.max > 0.0 && x.max - x.pos <= info.lower_threshold)).unwrap_or((false, false));
        if !st.tracked {
            st.tracked = true;
            st.reported_pos = pos;
            st.reported_x = sl;
            st.in_left = in_left;
            st.in_right = in_right;
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
        let x_moved = xs.is_some() && (sl - st.reported_x).abs() > 0.01;
        let moved = (pos - st.reported_pos).abs() > 0.01 || x_moved;
        let moving = dragging || animating || moved;
        if moving && !st.moving {
            st.drag_started = dragging;
            out.push(("scrollstart", json!({ "isDrag": dragging })));
        }
        if moved {
            let d = st.reported_pos - pos;
            let (dx, dy) = match xs {
                Some(_) => (st.reported_x - sl, d),
                None if info.horizontal => (d, 0.0),
                None => (0.0, d),
            };
            let mut detail = json!({
                "scrollLeft": sl, "scrollTop": stp,
                "scrollHeight": info.content_height.max(info.bounds.height),
                "scrollWidth": info.content_width.max(info.bounds.width),
                "deltaX": dx, "deltaY": dy, "isDrag": dragging,
            });
            // Skyline 3.6.6 `boundaryVelocity`：这一次滚动碰到边界（从界内进到界外）时是碰边那一刻的速度（逻辑 px/s，
            // 正 = scrollTop 变大的方向），否则官方是 NaN。JSON 带不了 NaN：没碰边时不给这个字段（读出来是 undefined，
            // `isNaN(e.detail.boundaryVelocity)` 两种都为真）
            let inside = |p: f32| p >= 0.0 && p <= max;
            if inside(st.reported_pos) && !inside(pos) {
                // 头一次报 scroll（静止起步）没有上一次的时刻：按一个 60Hz 帧算
                let dt = if st.last_scroll_ms > 0 && time_ms > st.last_scroll_ms { time_ms - st.last_scroll_ms } else { 16 };
                let v = (pos - st.reported_pos) * 1000.0 / dt as f32;
                detail["boundaryVelocity"] = json!(v);
            }
            st.last_scroll_ms = time_ms;
            out.push(("scroll", detail));
            if dragging && info.enhanced {
                out.push(("dragging", json!({ "scrollTop": stp, "scrollLeft": sl })));
            }
            st.reported_pos = pos;
            st.reported_x = sl;
        }
        if !dragging && st.dragging && info.enhanced {
            // 松手速度（逻辑 px/s，正 = scrollTop / scrollLeft 变大的方向；官方没写单位，待验）
            let (vx, vy) = match xs {
                Some(x) => (x.release_v, release_v),
                None if info.horizontal => (release_v, 0.0),
                None => (0.0, release_v),
            };
            out.push(("dragend", json!({ "scrollTop": stp, "scrollLeft": sl, "velocity": { "x": vx, "y": vy } })));
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
        // 双向的横向阈值：同一对事件，direction 是 left / right（官方 upper/lower-threshold 「距顶部/左边」）
        if xs.is_some() {
            if in_left && !st.in_left && x_moved {
                out.push(("scrolltoupper", json!({ "direction": "left" })));
            }
            if in_right && !st.in_right && x_moved {
                out.push(("scrolltolower", json!({ "direction": "right" })));
            }
            st.in_left = in_left;
            st.in_right = in_right;
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
            // ScrollViewContext.triggerRefresh / closeRefresh（host/selector_query.rs 记下，这里消费）
            let request = st.refresh_request.take();
            let status_before = st.refresh_status;
            let (phase, pulled, refresh_seq) = (st.refresher, st.pulled, st.refresh_seq);
            let mut next = phase;
            let mut new_pulled = pulled;
            let mut enter_refresh = false;
            let mut restore = false;
            // 下拉二级（two_level.rs）：这一帧打开 / 关闭
            let mut two_level_open = false;
            let mut two_level_close = false;
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
                    let (trigger, peak) =
                        interaction.get_scroll_controller_mut(id).map(|c| (c.take_pull_trigger(), c.take_pull_peak())).unwrap_or((false, 0.0));
                    if rf.two_level.is_some_and(|tl| peak >= tl.threshold) {
                        // 拉过二级阈值松手：打开二级（不发 refresherrefresh，官方：「继续下拉至 two-level-threshold 松手触发下拉二级」）
                        two_level_open = true;
                    } else if trigger {
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
            // 下拉二级：属性 / Context 请求 / 二级里滑动关闭（scroll_two_level.rs）
            if let Some(tl) = &rf.two_level {
                let st = state(&mut interaction.scroll_views, id);
                let (prev_two, two_req) = (st.last_two_triggered.replace(tl.triggered), st.two_level_request.take());
                let s2 = super::scroll_two_level::step(tl, phase, prev_two, two_req, dragging, was_dragging, gap, interaction.get_scroll_controller_mut(id));
                two_level_open |= s2.open && phase != RefresherPhase::Refreshing;
                two_level_close |= s2.close;
            }
            // 程序触发：与手拉到位松手一样进刷新态、发 refresherrefresh（官方「触发下拉刷新」；发不发事件官方没写，待验）
            match request {
                Some(true) if phase != RefresherPhase::Refreshing && !enter_refresh => {
                    enter_refresh = true;
                    out.push(("refresherrefresh", json!({ "dy": rf.threshold })));
                }
                Some(false) if phase == RefresherPhase::Refreshing => restore = true,
                _ => {}
            }
            // bindrefresherstatuschange（官方 RefreshStatus：0 Idle / 1 CanRefresh / 2 Refreshing / 3 Completed /
            // 5 CanTwoLevel / 6 TwoLevelOpening / 7 TwoLeveling / 8 TwoLevelClosing；4 Failed 引擎判断不了，不报）
            let animating_now = interaction.get_scroll_controller(id).map(|c| c.is_animating()).unwrap_or(false);
            let status = if two_level_open {
                6
            } else if two_level_close {
                8
            } else if enter_refresh {
                2
            } else if restore {
                3
            } else {
                match next {
                    RefresherPhase::Pulling if rf.two_level.is_some_and(|tl| gap >= tl.threshold) => 5,
                    RefresherPhase::Pulling if gap >= rf.threshold => 1,
                    RefresherPhase::Pulling => 0,
                    RefresherPhase::Refreshing => 2,
                    // 二级：打开的弹簧走完才算「打开了」
                    RefresherPhase::TwoLevel if status_before == 6 && animating_now => 6,
                    RefresherPhase::TwoLevel => 7,
                    // 复位 / 二级关闭完成、回到原处之后才回到 Idle
                    RefresherPhase::Idle if (status_before == 3 || status_before == 8) && gap > 0.5 => status_before,
                    RefresherPhase::Idle => 0,
                }
            };
            if status != status_before {
                out.push(("refresherstatuschange", json!({ "status": status, "dy": gap })));
            }
            let seq = if enter_refresh { set_data_seq(app) } else { refresh_seq };
            if let Some(c) = interaction.get_scroll_controller_mut(id) {
                if two_level_open {
                    c.set_top_inset(rf.two_level.map(|t| t.height).unwrap_or(0.0));
                } else if two_level_close {
                    c.set_top_inset(0.0);
                } else if enter_refresh {
                    c.set_top_inset(rf.threshold);
                } else if restore {
                    c.set_top_inset(0.0);
                }
            }
            if two_level_open {
                next = RefresherPhase::TwoLevel;
            } else if two_level_close {
                next = RefresherPhase::Idle;
            } else if enter_refresh {
                next = RefresherPhase::Refreshing;
            } else if restore {
                next = RefresherPhase::Idle;
                out.push(("refresherrestore", json!({})));
            }
            let st = state(&mut interaction.scroll_views, id);
            st.refresher = next;
            st.pulled = new_pulled;
            st.refresh_seq = seq;
            st.refresh_status = status;
        }
        for (name, detail) in out {
            fired |= super::own_events::dispatch_own(app, r, id, name, detail, time_ms);
        }
    }
    if fired {
        app.pump_jobs();
    }
    fired
}

/// sticky-header 的 `bindstickontopchange`（`detail = {isStickOnTop}`，吸顶状态变了才有）：渲染器画的时候按滚动算出、
/// 攒下（`wxml_renderer/sticky.rs`），宿主在每次重画页面之后调。SDK 与桌面共用。返回派发了几条
pub fn dispatch_sticky_events(app: &mut MiniApp, r: &mut WxmlRenderer, time_ms: u64) -> usize {
    let events = r.take_sticky_events();
    for (id, stuck) in &events {
        super::own_events::dispatch_own(app, r, id, "stickontopchange", json!({ "isStickOnTop": stuck }), time_ms);
    }
    events.len()
}

/// 双向 scroll-view 横向控制器这一帧的状态
#[derive(Clone, Copy)]
struct XState {
    pos: f32,
    max: f32,
    dragging: bool,
    animating: bool,
    release_v: f32,
}

fn x_state(interaction: &InteractionManager, id: &str) -> Option<XState> {
    let c = interaction.get_scroll_controller(&crate::ui::interaction::scroll_x_key(id))?;
    Some(XState { pos: c.get_position(), max: c.get_max_scroll(), dragging: c.is_dragging, animating: c.is_animating(), release_v: c.release_velocity() })
}

/// `scroll_views[id]`，没有就建。`entry(id.to_string())` 每次都分配一个键，这里只在第一次建时分配
pub(super) fn state<'a>(
    map: &'a mut std::collections::HashMap<String, crate::ui::scroll_view_state::ScrollViewState>,
    id: &str,
) -> &'a mut crate::ui::scroll_view_state::ScrollViewState {
    if !map.contains_key(id) {
        map.insert(id.to_string(), Default::default());
    }
    map.get_mut(id).expect("刚插入")
}

fn set_data_seq(app: &MiniApp) -> u64 {
    app.eval("__setDataSeq").ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

/// 页面 setData 过、重画之前调：`scroll-into-view` 绑定的字段这段时间里**值变过**（哪怕最后又设回原值），
/// 就忘掉「上次见到的值」，渲染时按「变了」再滚一次。
///
/// 聊天页的 `scrollToEnd`：`setData({intoView: ''}); setData({intoView: last})`。微信两次 setData 逐次生效，
/// 第二次是从 '' 变成 last、照样滚；这里两次落在同一帧，渲染时只见到 last（与上次相同）—— 不滚，
/// 键盘弹起列表变矮后最新消息被挡在下面（轻聊真机）。只在数据脏的帧、只问绑了字段的 scroll-view
///
/// 第一次见到的 scroll-view 先在一帧干净的数据上记下序号作起点（`data_dirty` 为假也要记），之后只在数据脏的帧核对
pub fn rearm_into_view(app: &MiniApp, renderer: Option<&WxmlRenderer>, interaction: &mut InteractionManager, data_dirty: bool) {
    let Some(r) = renderer else { return };
    let mut seq_now = None;
    for info in r.scroll_views().iter().filter(|s| s.scrollable && !s.into_keys.is_empty()) {
        let st = state(&mut interaction.scroll_views, &info.component_id);
        if st.into_seq.is_some() && !data_dirty {
            continue;
        }
        let seq = *seq_now.get_or_insert_with(|| set_data_seq(app));
        let Some(since) = st.into_seq.replace(seq) else { continue };
        if seq > since && changed_since(app, &info.owner, &info.into_keys, since) {
            st.last_into_view = None;
        }
    }
}

fn changed_since(app: &MiniApp, owner: &str, keys: &[String], seq: u64) -> bool {
    let o = serde_json::to_string(owner).unwrap_or_else(|_| "''".into());
    let k = serde_json::to_string(keys).unwrap_or_else(|_| "[]".into());
    app.eval(&format!("__changedSince({o}, {k}, {seq})")).map(|s| s.trim() == "1").unwrap_or(false)
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
