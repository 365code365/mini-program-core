//! 触摸序列状态机与微信事件对象。
//!
//! 之前宿主只有「按下 / 移动 / 抬起」三段临时逻辑，抬起时用
//! `位移 < 10px && 时长 < 300ms` 判定成一次 tap，然后只派发 `tap`。
//! 与微信的差距是结构性的：
//!
//! - **`touchstart` / `touchmove` / `touchend` / `touchcancel` 一个都没有**，
//!   凡是自己实现手势的页面（滑动删除、拖拽排序、手写签名、自定义下拉）全是死的；
//! - **没有 `longpress` / `longtap`**（微信是按住 350ms 触发）；
//! - tap 有 300ms 上限 —— 微信没有：按住两秒再松手，`longpress` 与 `tap` 都会触发。
//!   按住不动稍久一点就点不动，手上就是「操作没反应」；
//! - 事件对象里 `touches` / `changedTouches` 是空数组，也没有 `target` 与
//!   `currentTarget` 的区分（`e.target.dataset` 与 `e.currentTarget.dataset`
//!   在小程序里是两回事，列表项取 id 全靠它）。
//!
//! 这里把一次触摸抽成状态机（纯逻辑、时钟由调用方注入，可无头回归），
//! 由宿主把窗口的指针事件喂进来，产出**微信语义的事件序列**。

use std::collections::HashMap;

/// 判定「移动」的阈值（逻辑像素）。超过它就取消 tap 与按压态，
/// 与微信/浏览器的 touch slop 同量级。
pub const TOUCH_SLOP: f32 = 10.0;
/// 长按触发时长（毫秒），与微信一致
pub const LONG_PRESS_MS: u64 = 350;

/// 状态机产出的一个事件
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchOut {
    Start,
    Move,
    End,
    Cancel,
    /// 长按：微信会同时派发 `longpress` 与（旧名）`longtap`
    LongPress,
    /// 抬起且未移动、未被其它手势接管 —— 应当派发 `tap`
    Tap,
}

impl TouchOut {
    /// 对应的微信事件名（`Tap` 走宿主既有的点击链路，这里给出名字便于日志）
    pub fn event_name(&self) -> &'static str {
        match self {
            TouchOut::Start => "touchstart",
            TouchOut::Move => "touchmove",
            TouchOut::End => "touchend",
            TouchOut::Cancel => "touchcancel",
            TouchOut::LongPress => "longpress",
            TouchOut::Tap => "tap",
        }
    }
}

/// 一次触摸（单指）的全过程
#[derive(Debug, Clone, Copy)]
struct Active {
    id: u32,
    start: (f32, f32),
    pos: (f32, f32),
    started_ms: u64,
    /// 超过 slop：tap 作废
    moved: bool,
    long_fired: bool,
    /// 被滚动等手势接管：后续不再派发 touch 事件，tap 作废
    canceled: bool,
}

/// 触摸状态机。时钟由调用方以毫秒注入，便于测试与无头回归。
#[derive(Debug, Default)]
pub struct TouchTracker {
    active: Option<Active>,
    next_id: u32,
}

impl TouchTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前是否有手指按着（用于决定 move 要不要派发）
    pub fn is_active(&self) -> bool {
        self.active.map(|a| !a.canceled).unwrap_or(false)
    }

    /// 这次触摸是否已经移动超过阈值（超过就不该再有按压态与 tap）
    pub fn moved(&self) -> bool {
        self.active.map(|a| a.moved).unwrap_or(false)
    }

    /// 触摸起点（侧滑返回等边缘手势要用）
    pub fn start_pos(&self) -> Option<(f32, f32)> {
        self.active.map(|a| a.start)
    }

    /// 当前触点（用于构造事件对象）
    pub fn pos(&self) -> Option<(f32, f32)> {
        self.active.map(|a| a.pos)
    }

    /// 触点标识（微信事件对象里的 `identifier`）
    pub fn identifier(&self) -> u32 {
        self.active.map(|a| a.id).unwrap_or(0)
    }

    /// 手指按下
    pub fn press(&mut self, x: f32, y: f32, now_ms: u64) -> Vec<TouchOut> {
        self.next_id = self.next_id.wrapping_add(1);
        self.active = Some(Active {
            id: self.next_id,
            start: (x, y),
            pos: (x, y),
            started_ms: now_ms,
            moved: false,
            long_fired: false,
            canceled: false,
        });
        vec![TouchOut::Start]
    }

    /// 手指移动。返回 `[Move]`；没有按下或已被接管时返回空。
    pub fn move_to(&mut self, x: f32, y: f32, _now_ms: u64) -> Vec<TouchOut> {
        let Some(a) = &mut self.active else { return vec![] };
        if a.canceled {
            return vec![];
        }
        a.pos = (x, y);
        if !a.moved {
            let (dx, dy) = ((x - a.start.0).abs(), (y - a.start.1).abs());
            if dx > TOUCH_SLOP || dy > TOUCH_SLOP {
                a.moved = true;
            }
        }
        vec![TouchOut::Move]
    }

    /// 每帧调用：到时间就产出长按。
    ///
    /// 长按必须由**帧驱动**：只在 move/up 里判时间的话，手指一直不动就永远等不到
    /// 那一下（页面里长按菜单、长按选中就此失效）。
    pub fn tick(&mut self, now_ms: u64) -> Vec<TouchOut> {
        let Some(a) = &mut self.active else { return vec![] };
        if a.canceled || a.moved || a.long_fired {
            return vec![];
        }
        if now_ms.saturating_sub(a.started_ms) >= LONG_PRESS_MS {
            a.long_fired = true;
            return vec![TouchOut::LongPress];
        }
        vec![]
    }

    /// 手指抬起。未移动且未被接管时追加一个 `Tap`（**不看按了多久**，与微信一致）。
    pub fn release(&mut self, x: f32, y: f32, _now_ms: u64) -> Vec<TouchOut> {
        let Some(mut a) = self.active.take() else { return vec![] };
        if a.canceled {
            return vec![];
        }
        a.pos = (x, y);
        let mut out = vec![TouchOut::End];
        if !a.moved {
            out.push(TouchOut::Tap);
        }
        out
    }

    /// 被别的手势接管（滚动开始）或指针离开窗口：产出一次 `touchcancel`。
    ///
    /// 浏览器与 Skyline 都是这个语义 —— 手势竞争的失败方收到 cancel，
    /// 页面自己实现的拖拽才知道该收尾。
    pub fn cancel(&mut self) -> Vec<TouchOut> {
        let Some(a) = &mut self.active else { return vec![] };
        if a.canceled {
            return vec![];
        }
        a.canceled = true;
        vec![TouchOut::Cancel]
    }

    /// 抬起时清干净（cancel 之后仍会来一次 release）
    pub fn finish(&mut self) {
        self.active = None;
    }
}

/// 事件对象里的一个节点信息（`target` / `currentTarget`）
pub struct NodeInfo<'a> {
    pub id: &'a str,
    pub dataset: &'a HashMap<String, String>,
    pub offset_left: f32,
    pub offset_top: f32,
}

/// 按微信语义拼一个事件对象（JSON 文本，交给 `__dispatchEvent`）。
///
/// - `target`：**真正被点到的最内层节点**；`currentTarget`：挂着这个处理函数的节点。
///   两者的 `dataset` 常常不同，小程序里取列表项 id 全靠这点区分。
/// - `touches`：当前屏幕上的所有触点；`changedTouches`：本次变化的触点。
///   `touchend` 时 `touches` 为空（手指已离开），`changedTouches` 仍是那个点。
/// - `timeStamp`：页面打开到事件发生的毫秒数。
pub fn event_json(
    event_type: &str,
    target: &NodeInfo,
    current_target: &NodeInfo,
    point: (f32, f32),
    identifier: u32,
    time_stamp_ms: u64,
    detail: serde_json::Value,
) -> String {
    let touch = serde_json::json!({
        "identifier": identifier,
        "pageX": point.0,
        "pageY": point.1,
        "clientX": point.0,
        "clientY": point.1,
    });
    // 手指已离开屏幕的事件里 touches 应为空
    let touches: Vec<serde_json::Value> = if matches!(event_type, "touchend" | "touchcancel") {
        vec![]
    } else {
        vec![touch.clone()]
    };
    let node = |n: &NodeInfo| {
        serde_json::json!({
            "id": n.id,
            "dataset": n.dataset,
            "offsetLeft": n.offset_left,
            "offsetTop": n.offset_top,
        })
    };
    serde_json::json!({
        "type": event_type,
        "timeStamp": time_stamp_ms,
        "target": node(target),
        "currentTarget": node(current_target),
        "detail": detail,
        "touches": touches,
        "changedTouches": [touch],
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(out: Vec<TouchOut>) -> Vec<&'static str> {
        out.iter().map(|o| o.event_name()).collect()
    }

    /// 点一下：start → end + tap
    #[test]
    fn simple_tap_sequence() {
        let mut t = TouchTracker::new();
        assert_eq!(names(t.press(10.0, 10.0, 0)), ["touchstart"]);
        assert_eq!(names(t.release(11.0, 11.0, 80)), ["touchend", "tap"]);
    }

    /// 按住两秒再松手：微信照样触发 tap（长按也触发一次）。
    /// 旧实现有 300ms 上限，按久一点就点不动。
    #[test]
    fn long_hold_still_taps_and_fires_longpress_once() {
        let mut t = TouchTracker::new();
        t.press(10.0, 10.0, 0);
        assert!(t.tick(100).is_empty(), "还没到 350ms 不该触发长按");
        assert_eq!(names(t.tick(400)), ["longpress"]);
        assert!(t.tick(900).is_empty(), "长按只触发一次");
        assert_eq!(names(t.release(10.0, 10.0, 2000)), ["touchend", "tap"]);
    }

    /// 移动超过阈值：tap 作废，长按也不再触发
    #[test]
    fn moving_beyond_slop_cancels_tap_and_longpress() {
        let mut t = TouchTracker::new();
        t.press(10.0, 10.0, 0);
        assert_eq!(names(t.move_to(14.0, 10.0, 10)), ["touchmove"]);
        assert!(!t.moved(), "4px 还在阈值内");
        t.move_to(40.0, 10.0, 20);
        assert!(t.moved(), "30px 超过阈值");
        assert!(t.tick(500).is_empty(), "移动过就不该长按");
        assert_eq!(names(t.release(40.0, 10.0, 600)), ["touchend"], "不该有 tap");
    }

    /// 被滚动接管：产出一次 touchcancel，之后不再有 touch 事件，也没有 tap
    #[test]
    fn takeover_emits_cancel_once_then_silence() {
        let mut t = TouchTracker::new();
        t.press(10.0, 10.0, 0);
        assert_eq!(names(t.cancel()), ["touchcancel"]);
        assert!(t.cancel().is_empty(), "cancel 只产出一次");
        assert!(t.move_to(10.0, 60.0, 30).is_empty());
        assert!(t.tick(500).is_empty());
        assert!(t.release(10.0, 60.0, 600).is_empty(), "接管后不该再有 touchend/tap");
    }

    /// 没有按下时的移动/抬起不该凭空产生事件
    #[test]
    fn events_without_press_are_ignored() {
        let mut t = TouchTracker::new();
        assert!(t.move_to(1.0, 1.0, 0).is_empty());
        assert!(t.release(1.0, 1.0, 0).is_empty());
        assert!(t.tick(999).is_empty());
    }

    /// 事件对象字段与微信对齐：touchend 的 touches 为空、changedTouches 仍有点，
    /// target 与 currentTarget 各自带自己的 dataset
    #[test]
    fn event_object_shape() {
        let mut inner = HashMap::new();
        inner.insert("id".to_string(), "7".to_string());
        let mut outer = HashMap::new();
        outer.insert("kind".to_string(), "card".to_string());
        let target = NodeInfo { id: "cell7", dataset: &inner, offset_left: 12.0, offset_top: 30.0 };
        let current = NodeInfo { id: "list", dataset: &outer, offset_left: 0.0, offset_top: 0.0 };

        let json = event_json("touchend", &target, &current, (5.0, 6.0), 3, 1234, serde_json::json!({}));
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "touchend");
        assert_eq!(v["timeStamp"], 1234);
        assert_eq!(v["target"]["id"], "cell7");
        assert_eq!(v["target"]["dataset"]["id"], "7");
        assert_eq!(v["currentTarget"]["dataset"]["kind"], "card");
        assert_eq!(v["target"]["offsetTop"], 30.0);
        assert!(v["touches"].as_array().unwrap().is_empty(), "touchend 的 touches 应为空");
        assert_eq!(v["changedTouches"][0]["identifier"], 3);
        assert_eq!(v["changedTouches"][0]["pageX"], 5.0);

        let start = event_json("touchstart", &target, &current, (5.0, 6.0), 3, 1, serde_json::json!({}));
        let v2: serde_json::Value = serde_json::from_str(&start).unwrap();
        assert_eq!(v2["touches"].as_array().unwrap().len(), 1);
    }
}
