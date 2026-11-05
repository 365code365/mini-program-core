//! `<progress active>` 的进度动画：从左往右涨到 `percent`，走完发 `bindactiveend`。
//!
//! 依据官方 component/progress.html：
//! - `active`：进度条从左往右的动画；
//! - `active-mode`：`backwards`（默认，动画从头播）/ `forwards`（从上次结束点接着播）；
//! - `duration`：进度增加 1% 所需毫秒数，默认 30；
//! - `bindactiveend`：动画完成事件。官方没写 detail，WebView 版发 `{curPercent}`，这里照它（待真机核对）。
//!
//! 状态按组件 id 存在进程级表里（与 `picker_view` 同一个路子），事件由宿主每帧取走
//! （`host::swiper::tick` 一起派）。时间用渲染器的动画时钟：无头快照 `--time` 定住时钟时
//! 不做动画、直接画终态（否则定住的钟让进度条永远停在 0，像素基线就没法比）。
//!
//! 每帧代价：只有带 `active` 的 progress 走到这里（调用方先判标签与属性），一次表查找；
//! 动画进行中只重画进度条自己的盒子（调用方登记动画包围盒），不整帧。

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug)]
struct Anim {
    /// 这一段的终点（属性 percent）与起点
    target: f32,
    from: f32,
    /// 起点时刻（秒，动画时钟）与时长（秒）
    start: f32,
    dur: f32,
    /// 上一帧画出来的进度（forwards 从这里接着播）
    shown: f32,
    /// 这一段的 activeend 已经发过
    ended: bool,
}

type Events = Vec<(String, &'static str, Value)>;

static STATE: Mutex<Option<(HashMap<String, Anim>, Events)>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut HashMap<String, Anim>, &mut Events) -> R) -> R {
    let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let (map, ev) = g.get_or_insert_with(|| (HashMap::new(), Vec::new()));
    f(map, ev)
}

/// 这一帧该画的进度（0~100）与动画是否还在走。
///
/// `attrs` 是节点属性（读 `active` / `active-mode` / `duration`），`target` 是夹过的 percent，
/// `now` 是动画时钟（秒），`live`：时钟是不是在走（无头快照定住时为假，直接给终态）
pub fn frame(id: &str, attrs: &HashMap<String, String>, target: f32, now: f32, live: bool) -> (f32, bool) {
    let active = attrs.get("active").is_some_and(|s| super::attr_is_true(s));
    if !active || !live {
        if active {
            // 定住的钟：不留状态，下一次钟走起来时按「第一次看见」从头播
            with(|m, _| m.remove(id));
        }
        return (target, false);
    }
    let forwards = attrs.get("active-mode").map(|s| s.trim()) == Some("forwards");
    // 每 1% 多少毫秒；负数 / 非数按默认 30
    let per = attrs
        .get("duration")
        .and_then(|s| s.trim().parse::<f32>().ok())
        .filter(|v| v.is_finite() && *v >= 0.0)
        .unwrap_or(30.0);
    with(|map, ev| {
        let a = map.entry(id.to_string()).or_insert(Anim { target: f32::NAN, from: 0.0, start: now, dur: 0.0, shown: 0.0, ended: false });
        if a.target != target {
            // 第一次见到、或页面改了 percent：backwards 从 0 重播，forwards 从上次画到的地方接着播
            a.from = if forwards && a.target.is_finite() { a.shown } else { 0.0 };
            a.target = target;
            a.start = now;
            a.dur = (target - a.from).abs() * per / 1000.0;
            a.ended = false;
        }
        let t = if a.dur <= 0.0 { 1.0 } else { ((now - a.start) / a.dur).clamp(0.0, 1.0) };
        a.shown = a.from + (a.target - a.from) * t;
        if t >= 1.0 && !a.ended {
            a.ended = true;
            ev.push((id.to_string(), "activeend", json!({ "curPercent": a.target })));
        }
        (a.shown, t < 1.0)
    })
}

/// 取走待派发的 `activeend`：(progress 组件 id, 事件名, detail)
pub fn take_events() -> Events {
    with(|_, ev| std::mem::take(ev))
}

/// 换页：状态跟着页面走（组件 id 在不同页面可能撞）
pub fn clear() {
    with(|m, ev| {
        m.clear();
        ev.clear();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs(kv: &[(&str, &str)]) -> HashMap<String, String> {
        kv.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn backwards_从零涨到_percent_走完发一次_activeend() {
        let a = attrs(&[("active", "true"), ("duration", "10")]);
        // 50% × 10ms = 0.5s
        assert_eq!(frame("pgA", &a, 50.0, 1.0, true), (0.0, true));
        let (p, run) = frame("pgA", &a, 50.0, 1.25, true);
        assert!((p - 25.0).abs() < 0.01 && run, "{p}");
        assert_eq!(frame("pgA", &a, 50.0, 1.6, true), (50.0, false));
        frame("pgA", &a, 50.0, 1.7, true);
        let ev: Vec<_> = take_events().into_iter().filter(|e| e.0 == "pgA").collect();
        assert_eq!(ev.len(), 1, "activeend 只发一次");
        assert_eq!(ev[0].2["curPercent"], json!(50.0));
        // percent 改成 80：backwards 从 0 重播
        assert_eq!(frame("pgA", &a, 80.0, 2.0, true).0, 0.0);
    }

    #[test]
    fn forwards_从上次结束点接着播() {
        let a = attrs(&[("active", "true"), ("active-mode", "forwards"), ("duration", "10")]);
        frame("pgB", &a, 40.0, 0.0, true);
        frame("pgB", &a, 40.0, 1.0, true);
        // 40 → 60：起点是 40，20% × 10ms = 0.2s
        assert_eq!(frame("pgB", &a, 60.0, 2.0, true).0, 40.0);
        let (p, _) = frame("pgB", &a, 60.0, 2.1, true);
        assert!((p - 50.0).abs() < 0.01, "{p}");
    }

    #[test]
    fn 没有_active_或钟定住时直接画终态() {
        assert_eq!(frame("pgC", &attrs(&[]), 70.0, 0.0, true), (70.0, false));
        assert_eq!(frame("pgC", &attrs(&[("active", "true")]), 70.0, 0.0, false), (70.0, false));
    }
}
