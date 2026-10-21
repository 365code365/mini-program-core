//! `wx.createIntersectionObserver()`：宿主侧（SDK 与桌面共用）。
//!
//! 依据官方 api/wxml/IntersectionObserver.html：
//! - `relativeTo(selector, margins)` / `relativeToViewport(margins)` 定参照区域，margins 往外扩（负数往里缩）；
//! - `observe(target, cb)`：相交比例**跨过** `thresholds` 里的某个值时回调；`initialRatio` 是「上一次」的初值
//!   （默认 0，所以一开始就在视口里的目标会立刻回调一次）；`observeAll` 时选择器命中的每个节点各算各的；
//! - 回调参数 `{id, dataset, intersectionRatio, intersectionRect, boundingClientRect, relativeRect, time}`，
//!   矩形都是视口坐标的 `{left, top, right, bottom, width, height}`。
//!
//! 「跨过阈值」按区间算：比例落在第几个阈值区间（阈值 0 要求严格大于 0 才算进来，其余是大于等于），
//! 区间变了就回调 —— 与浏览器 IntersectionObserver 的语义一致。
//! 登记表放在线程局部（一个引擎实例一条线程，同 swiper / video 的状态表）；每个观察者记下创建它的页面路由，
//! 只在那一页在前台时计算（`navigateTo` 出去再回来接着算）。页面销毁后的观察者由逻辑层在下一次回调时自己断开。

use super::selector_query::{client_rect, PageView};
use crate::renderer::WxmlRenderer;
use crate::runtime::MiniApp;
use crate::ui::interaction::InteractionManager;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashMap;

struct Observer {
    id: u64,
    target: String,
    /// 参照节点的选择器；None = 视口
    relative: Option<String>,
    /// (left, right, top, bottom)
    margins: (f32, f32, f32, f32),
    thresholds: Vec<f32>,
    initial_bucket: usize,
    all: bool,
    route: String,
    /// 每个目标节点（组件 id）上一次所在的区间
    last: HashMap<String, usize>,
}

thread_local! {
    static OBSERVERS: RefCell<Vec<Observer>> = const { RefCell::new(Vec::new()) };
}

/// 有没有活着的观察者（宿主据此决定要不要每帧算）
pub fn active() -> bool {
    OBSERVERS.with(|o| !o.borrow().is_empty())
}

/// 引擎销毁 / 重新启动小程序时清空
pub fn clear() {
    OBSERVERS.with(|o| o.borrow_mut().clear());
}

/// 比例落在第几个阈值区间
fn bucket(ratio: f32, thresholds: &[f32]) -> usize {
    thresholds.iter().filter(|&&t| if t <= 0.0 { ratio > 0.0 } else { ratio >= t - 1e-6 }).count()
}

fn rect_json(l: f32, t: f32, r: f32, b: f32) -> Value {
    json!({ "left": l, "top": t, "right": r, "bottom": b, "width": (r - l).max(0.0), "height": (b - t).max(0.0) })
}

fn take_requests(app: &mut MiniApp) {
    for req in app.take_host_requests("intersection") {
        let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
        match req.get("op").and_then(Value::as_str) {
            Some("observe") => {
                let rel = req.get("rel").cloned().unwrap_or(Value::Null);
                let m = rel.get("margins").cloned().unwrap_or(Value::Null);
                let mf = |k: &str| m.get(k).and_then(Value::as_f64).unwrap_or(0.0) as f32;
                let mut thresholds: Vec<f32> = req
                    .get("thresholds")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(Value::as_f64).map(|v| (v as f32).clamp(0.0, 1.0)).collect())
                    .unwrap_or_default();
                if thresholds.is_empty() {
                    thresholds.push(0.0);
                }
                thresholds.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let initial = req.get("initialRatio").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                let ob = Observer {
                    id,
                    target: req.get("target").and_then(Value::as_str).unwrap_or("").to_string(),
                    relative: rel.get("selector").and_then(Value::as_str).map(str::to_string),
                    margins: (mf("left"), mf("right"), mf("top"), mf("bottom")),
                    initial_bucket: bucket(initial, &thresholds),
                    thresholds,
                    all: req.get("observeAll").and_then(Value::as_bool).unwrap_or(false),
                    route: req.get("route").and_then(Value::as_str).unwrap_or("").to_string(),
                    last: HashMap::new(),
                };
                OBSERVERS.with(|o| o.borrow_mut().push(ob));
            }
            Some("disconnect") => OBSERVERS.with(|o| o.borrow_mut().retain(|ob| ob.id != id)),
            _ => {}
        }
    }
}

/// 每帧一次：收登记 / 断开，再给前台页面的观察者算一遍。返回是否回调过（回调里可能 setData）
pub fn tick(
    app: &mut MiniApp,
    renderer: Option<&WxmlRenderer>,
    interaction: &InteractionManager,
    route: &str,
    view: PageView,
    t_ms: u64,
) -> bool {
    take_requests(app);
    let Some(r) = renderer.filter(|r| r.has_layout()) else { return false };
    if !active() {
        return false;
    }
    let mut fires: Vec<(u64, Value)> = Vec::new();
    OBSERVERS.with(|o| {
        for ob in o.borrow_mut().iter_mut() {
            if !ob.route.is_empty() && ob.route.trim_start_matches('/') != route.trim_start_matches('/') {
                continue;
            }
            let rel = match &ob.relative {
                None => Some((0.0, 0.0, view.width, view.height)),
                Some(sel) => r.query_selector(sel, false, interaction).first().map(|h| client_rect(h, view.scroll_top)),
            };
            let Some((rl, rt, rr, rb)) = rel else { continue };
            let (ml, mr, mt, mb) = ob.margins;
            let (rl, rt, rr, rb) = (rl - ml, rt - mt, rr + mr, rb + mb);
            for h in r.query_selector(&ob.target, ob.all, interaction) {
                let (l, t, rgt, b) = client_rect(&h, view.scroll_top);
                let (il, it, ir, ib) = (l.max(rl), t.max(rt), rgt.min(rr), b.min(rb));
                let inter = (ir - il).max(0.0) * (ib - it).max(0.0);
                let area = (rgt - l) * (b - t);
                let ratio = if area > 0.0 { (inter / area).clamp(0.0, 1.0) } else { 0.0 };
                let now = bucket(ratio, &ob.thresholds);
                let prev = *ob.last.get(&h.component_id).unwrap_or(&ob.initial_bucket);
                ob.last.insert(h.component_id.clone(), now);
                if now == prev {
                    continue;
                }
                let inter_rect = if ir > il && ib > it { rect_json(il, it, ir, ib) } else { rect_json(0.0, 0.0, 0.0, 0.0) };
                fires.push((
                    ob.id,
                    json!({
                        "id": h.id,
                        "dataset": h.dataset,
                        "time": t_ms,
                        "intersectionRatio": ratio,
                        "intersectionRect": inter_rect,
                        "boundingClientRect": rect_json(l, t, rgt, b),
                        "relativeRect": rect_json(rl, rt, rr, rb),
                    }),
                ));
            }
        }
    });
    for (id, entry) in &fires {
        if let Err(e) = app.eval(&format!("__intersectionFire({id}, {entry})")) {
            eprintln!("⚠️ IntersectionObserver 回调失败：{e}");
        }
    }
    !fires.is_empty()
}
