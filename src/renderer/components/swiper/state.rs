//! swiper 的状态机：当前页、浮点位置、换页动画、手指拖动、自动播放、事件。
//!
//! 位置 `pos` 是「第几页」的浮点数（0 = 第一页贴着容器起点）。非衔接模式在 `[0, 页数−1]`
//! 里，衔接（`circular`）模式可以越过两端（-0.3 表示正从最后一页往第一页走），静止时归一化。
//!
//! 语义按官方 component/swiper.html：
//! - `bindchange`：current 改变时触发，`detail = {current, source}`，source 为 `autoplay` /
//!   `touch` / 其他原因空串（页面改 `current` 属性也算，官方 Tip 专门提醒过别在回调里无条件
//!   setData current）；松手 / 自动翻页**开始**时就发，不等动画走完；
//! - `bindanimationfinish`：动画结束，detail 同 change；
//! - `bindtransition`：位置每变一次发一次，`detail = {dx, dy}`（相对当前页的像素偏移）；
//! - `duration` 默认 500ms、`interval` 默认 5000ms；非衔接模式自动播放到最后一页后回到第一页
//!   （往回滑过去），衔接模式一直往前。
//!
//! 页数：非衔接 `total − display-multiple-items + 1`（最后几项一起显示时就到头了），衔接为 `total`。

use super::motion::Easing;
use serde_json::{json, Value};
use std::collections::HashMap;

/// 一次 sync 用到的配置（每帧从节点属性解析）
#[derive(Debug, Clone)]
pub struct SwiperConfig {
    pub total: usize,
    pub per_view: usize,
    pub circular: bool,
    pub vertical: bool,
    pub autoplay: bool,
    pub interval: u64,
    pub duration: u64,
    pub easing: Easing,
    /// WXML 写了 `current` 时的值（受控）
    pub current_attr: Option<usize>,
    /// Skyline `direction`：0 = all；1 = positive（只许往前一页方向划：横向右滑 / 纵向下滑）；
    /// -1 = negative（只许往后一页方向划：横向左滑 / 纵向上滑）
    pub direction: i8,
    /// Skyline `scroll-with-animation`（默认 true）：false 时页面改 `current` 直接跳过去，不走动画
    pub scroll_with_animation: bool,
}

impl Default for SwiperConfig {
    fn default() -> Self {
        Self {
            total: 0,
            per_view: 1,
            circular: false,
            vertical: false,
            autoplay: false,
            interval: 5000,
            duration: 500,
            easing: Easing::default(),
            current_attr: None,
            direction: 0,
            scroll_with_animation: true,
        }
    }
}

/// 松手时手指速度超过它（逻辑 px/s）就按方向翻一页，不看拖了多远
const FLICK_VELOCITY: f32 = 300.0;
/// 非衔接模式拖过两端时的阻尼（手指走 1，内容走 0.3）
const EDGE_RESISTANCE: f32 = 0.3;

#[derive(Debug, Clone, Copy)]
struct Anim {
    from: f32,
    to: f32,
    start: u64,
    dur: u64,
    easing: Easing,
    source: &'static str,
}

#[derive(Debug, Clone)]
struct Drag {
    start_pos: f32,
    start_coord: f32,
    /// (坐标, 时刻 ms)，只留最近 100ms
    samples: Vec<(f32, u64)>,
}

#[derive(Debug, Clone)]
pub struct SwiperState {
    pub current: usize,
    pub total: usize,
    pub autoplay: bool,
    pub autoplay_interval: u64,
    pub circular: bool,
    pub per_view: usize,
    pub duration: u64,
    pub easing: Easing,
    pub vertical: bool,
    pub pos: f32,
    direction: i8,
    scroll_with_animation: bool,
    anim: Option<Anim>,
    drag: Option<Drag>,
    /// 上一次翻页（或拖动结束）的时刻：自动播放从这里开始计间隔
    last_flip: u64,
    /// 一项沿主轴的逻辑像素（拖动换算用）
    item_px: f32,
    last_current_attr: Option<usize>,
    /// 上一次报 transition 时的位置
    reported_pos: f32,
}

impl SwiperState {
    fn pages(&self) -> usize {
        if self.circular {
            self.total.max(1)
        } else {
            (self.total.saturating_sub(self.per_view) + 1).max(1)
        }
    }

    fn max_pos(&self) -> f32 {
        (self.pages() - 1) as f32
    }

    /// 位置 → 第几页（衔接模式取模）
    fn page_of(&self, pos: f32) -> usize {
        let r = pos.round() as i64;
        if self.circular {
            r.rem_euclid(self.total.max(1) as i64) as usize
        } else {
            r.clamp(0, self.max_pos() as i64) as usize
        }
    }

    fn animate(&mut self, to: f32, now: u64, source: &'static str, scale: f32) {
        let from = self.pos;
        if (to - from).abs() < 1e-4 {
            self.pos = to;
            self.anim = None;
            return;
        }
        let dur = ((self.duration as f32) * scale.clamp(0.0, 1.0)).round() as u64;
        if dur == 0 {
            self.pos = to;
            self.anim = None;
            return;
        }
        self.anim = Some(Anim { from, to, start: now, dur, easing: self.easing, source });
    }

    fn detail(&self, source: &str) -> Value {
        json!({ "current": self.current, "source": source })
    }

    /// 相对当前页的偏移（像素，符号同微信：往下一页走为正）
    fn transition_detail(&self) -> Value {
        let mut off = self.pos - self.current as f32;
        if self.circular && self.total > 0 {
            let t = self.total as f32;
            off = (off + t / 2.0).rem_euclid(t) - t / 2.0;
        }
        let px = off * self.item_px;
        if self.vertical {
            json!({ "dx": 0, "dy": px })
        } else {
            json!({ "dx": px, "dy": 0 })
        }
    }
}

/// 一条待派发的 swiper 事件：(swiper 的组件 id, 事件名, detail)
pub type SwiperEvent = (String, &'static str, Value);

pub struct SwiperStateManager {
    states: HashMap<String, SwiperState>,
    events: Vec<SwiperEvent>,
}

impl Default for SwiperStateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SwiperStateManager {
    pub fn new() -> Self {
        Self { states: HashMap::new(), events: Vec::new() }
    }

    pub fn get(&self, id: &str) -> Option<&SwiperState> {
        self.states.get(id)
    }

    /// 当前登记了哪些 swiper（测试与诊断用）
    pub fn debug_keys(&self) -> Vec<String> {
        let mut k: Vec<String> = self.states.keys().cloned().collect();
        k.sort();
        k
    }

    /// 绘制期：按这一帧的属性更新状态，返回 (位置, 是否在动, 当前页)。
    /// 受控 `current` 只在**值变了**时生效（与微信一致；否则自动播放 / 拖动一换页就被拽回去）
    pub fn sync(&mut self, id: &str, cfg: &SwiperConfig, item_px: f32, now: u64) -> (f32, bool, usize) {
        let st = self.states.entry(id.to_string()).or_insert_with(|| {
            let mut s = SwiperState {
                current: 0,
                total: cfg.total,
                autoplay: cfg.autoplay,
                autoplay_interval: cfg.interval,
                circular: cfg.circular,
                per_view: cfg.per_view.max(1),
                duration: cfg.duration,
                easing: cfg.easing,
                vertical: cfg.vertical,
                pos: 0.0,
                direction: cfg.direction,
                scroll_with_animation: cfg.scroll_with_animation,
                anim: None,
                drag: None,
                last_flip: now,
                item_px,
                last_current_attr: cfg.current_attr,
                reported_pos: 0.0,
            };
            let c = cfg.current_attr.unwrap_or(0).min(s.pages() - 1);
            s.current = c;
            s.pos = c as f32;
            s.reported_pos = s.pos;
            s
        });
        st.total = cfg.total;
        // 运行时打开 autoplay：从这一刻起算 interval（微信的 autoplay 观察者是重排一次定时器）。
        // 不重置的话 last_flip 还停在上次翻页 / 建状态那会儿，一打开就立刻翻一页
        if cfg.autoplay && !st.autoplay {
            st.last_flip = now;
        }
        st.autoplay = cfg.autoplay;
        st.autoplay_interval = cfg.interval.max(1);
        st.circular = cfg.circular;
        st.per_view = cfg.per_view.max(1);
        st.duration = cfg.duration;
        st.easing = cfg.easing;
        st.vertical = cfg.vertical;
        st.direction = cfg.direction;
        st.scroll_with_animation = cfg.scroll_with_animation;
        if item_px > 0.0 {
            st.item_px = item_px;
        }
        // 项数变少：当前页 / 位置收回到范围里
        if st.current >= st.pages() {
            st.current = st.pages() - 1;
        }
        if !st.circular && st.anim.is_none() && st.drag.is_none() {
            st.pos = st.pos.clamp(0.0, st.max_pos());
        }
        if cfg.current_attr != st.last_current_attr {
            st.last_current_attr = cfg.current_attr;
            if let Some(c) = cfg.current_attr {
                let c = c.min(st.pages() - 1);
                if c != st.current && st.drag.is_none() {
                    // 衔接模式走最近的那一圈
                    let mut to = c as f32;
                    if st.circular {
                        let t = st.total as f32;
                        let base = st.pos - st.pos.rem_euclid(t);
                        to = [base + c as f32 - t, base + c as f32, base + c as f32 + t]
                            .into_iter()
                            .min_by(|a, b| (a - st.pos).abs().total_cmp(&(b - st.pos).abs()))
                            .unwrap_or(to);
                    }
                    st.current = c;
                    let scale = if st.scroll_with_animation { 1.0 } else { 0.0 };
                    st.animate(to, now, "", scale);
                    st.last_flip = now;
                    self.events.push((id.to_string(), "change", st.detail("")));
                    let st = &self.states[id];
                    if st.anim.is_none() {
                        self.events.push((id.to_string(), "animationfinish", st.detail("")));
                    }
                }
            }
        }
        let st = &self.states[id];
        (st.pos, st.anim.is_some() || st.drag.is_some(), st.current)
    }

    /// 宿主每帧：推进动画、到点自动翻页。返回是否有 swiper 位置变了（要重画）
    pub fn advance(&mut self, now: u64) -> bool {
        let mut changed = false;
        let mut ids: Vec<String> = self.states.keys().cloned().collect();
        ids.sort();
        for id in ids {
            let Some(st) = self.states.get_mut(&id) else { continue };
            if st.drag.is_some() {
                continue;
            }
            if let Some(a) = st.anim {
                let p = now.saturating_sub(a.start) as f32 / a.dur.max(1) as f32;
                st.pos = a.from + (a.to - a.from) * a.easing.apply(p);
                changed = true;
                if p >= 1.0 {
                    st.pos = a.to;
                    st.anim = None;
                    if st.circular && st.total > 0 {
                        st.pos = st.pos.rem_euclid(st.total as f32);
                    }
                    st.last_flip = now;
                    let d = st.detail(a.source);
                    self.events.push((id.clone(), "animationfinish", d));
                }
                if (st.pos - st.reported_pos).abs() > 1e-4 {
                    st.reported_pos = st.pos;
                    let d = st.transition_detail();
                    self.events.push((id.clone(), "transition", d));
                }
                continue;
            }
            if st.autoplay && st.pages() > 1 && now.saturating_sub(st.last_flip) >= st.autoplay_interval {
                let next = st.current + 1;
                let to = if st.circular {
                    st.pos.round() + 1.0
                } else if next >= st.pages() {
                    0.0
                } else {
                    next as f32
                };
                st.current = st.page_of(to);
                st.animate(to, now, "autoplay", 1.0);
                st.last_flip = now;
                let d = st.detail("autoplay");
                self.events.push((id.clone(), "change", d));
                changed = true;
            }
        }
        changed
    }

    /// 有没有 swiper 现在要出帧：动画中、拖动中、或自动播放到点
    pub fn needs_frame(&self, now: u64) -> bool {
        self.states.values().any(|s| {
            s.anim.is_some()
                || s.drag.is_some()
                || (s.autoplay && s.pages() > 1 && now.saturating_sub(s.last_flip) >= s.autoplay_interval)
        })
    }

    pub fn any_autoplay(&self) -> bool {
        self.states.values().any(|s| s.autoplay && s.pages() > 1)
    }

    pub fn any_dragging(&self) -> bool {
        self.states.values().any(|s| s.drag.is_some())
    }

    // ---------- 手指拖动（`host::input` 在手势锁定到 swiper 时调） ----------

    /// 开始拖：动画停在当前画面上，自动播放计时暂停
    pub fn begin_drag(&mut self, id: &str, coord: f32, now: u64) {
        let Some(st) = self.states.get_mut(id) else { return };
        st.anim = None;
        st.drag = Some(Drag { start_pos: st.pos, start_coord: coord, samples: vec![(coord, now)] });
    }

    pub fn update_drag(&mut self, id: &str, coord: f32, now: u64) {
        let Some(st) = self.states.get_mut(id) else { return };
        let Some(d) = st.drag.as_mut() else { return };
        let item = st.item_px.max(1.0);
        let mut raw = d.start_pos - (coord - d.start_coord) / item;
        // direction：positive 只许往前一页（位置变小），negative 只许往后一页（位置变大）
        match st.direction {
            1 => raw = raw.min(d.start_pos),
            -1 => raw = raw.max(d.start_pos),
            _ => {}
        }
        d.samples.push((coord, now));
        d.samples.retain(|(_, t)| now.saturating_sub(*t) <= 100);
        let max = st.max_pos();
        st.pos = if st.circular {
            raw
        } else if raw < 0.0 {
            raw * EDGE_RESISTANCE
        } else if raw > max {
            max + (raw - max) * EDGE_RESISTANCE
        } else {
            raw
        };
        if (st.pos - st.reported_pos).abs() > 1e-4 {
            st.reported_pos = st.pos;
            let d = st.transition_detail();
            self.events.push((id.to_string(), "transition", d));
        }
    }

    /// 松手：快速一甩按方向翻一页，否则吸附到最近的一页；当前页变了立刻发 change（source = touch）
    pub fn end_drag(&mut self, id: &str, now: u64) {
        let Some(st) = self.states.get_mut(id) else { return };
        let Some(d) = st.drag.take() else { return };
        let v = match (d.samples.first(), d.samples.last()) {
            (Some(a), Some(b)) if b.1 > a.1 => (b.0 - a.0) / ((b.1 - a.1) as f32 / 1000.0),
            _ => 0.0,
        };
        let start_page = d.start_pos.round();
        let mut to = if v.abs() > FLICK_VELOCITY {
            // 手指往左（坐标变小）= 去下一页
            if v < 0.0 { start_page + 1.0 } else { start_page - 1.0 }
        } else {
            st.pos.round()
        };
        to = to.clamp(start_page - 1.0, start_page + 1.0);
        match st.direction {
            1 => to = to.min(start_page),
            -1 => to = to.max(start_page),
            _ => {}
        }
        if !st.circular {
            to = to.clamp(0.0, st.max_pos());
        }
        let page = st.page_of(to);
        // 剩下的距离越短动画越短（满一页用满 duration），免得只差几像素也慢慢挪半秒
        let scale = (to - st.pos).abs().max(0.25);
        st.animate(to, now, "touch", scale);
        st.last_flip = now;
        let changed = page != st.current;
        st.current = page;
        let still = st.anim.is_none();
        if changed {
            let dt = st.detail("touch");
            self.events.push((id.to_string(), "change", dt));
        }
        if still {
            let st = &self.states[id];
            self.events.push((id.to_string(), "animationfinish", st.detail("touch")));
        }
    }

    pub fn take_events(&mut self) -> Vec<SwiperEvent> {
        std::mem::take(&mut self.events)
    }
}
