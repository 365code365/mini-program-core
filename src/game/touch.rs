//! 小游戏的触摸（game-doc/02 §7.1~7.2，测试 05 GT-T）。
//!
//! 宿主**每个指针**调一次 `mr_game_touch(phase, id, x, y, force, t_us)`（Android 的 POINTER_DOWN/UP 拆成那根指针的
//! down/up，MOVE 对每根指针、每个历史采样各一次）。这里只排队；下一次 pump 的第 1 步（rAF 之前）按到达顺序
//! 逐个变成 `touchstart / touchmove / touchend / touchcancel`，于是这一帧到的手指在这一帧的 rAF 里就看得到。
//!
//! 语义（与 W3C TouchEvent 一致，微信小游戏同形；取证项见 game-doc/06 §7）：
//! - `touches`：事件之后仍在屏上的全部触点（按按下先后）；`changedTouches`：这次变化的那几根；
//! - `identifier`：一次按下到抬起不变。**取证前的默认：分配当前最小的空闲值（从 0 起）**，不透传宿主 id ——
//!   有游戏把 identifier 当数组下标；Android 的 pointerId 本来就是这个规律，两种做法在 Android 上结果相同，
//!   iOS（UITouch 指针）只能这样；
//! - 最多 10 根：第 11 根起整段序列忽略（它的 move / up 也不派发），宿主日志一行；
//! - 单帧单指最多 8 次 move（1000Hz 采样的保护，06）：超出的并进这根手指最近一次排队的 move（保留最后坐标）；
//! - 进后台 / surface 没了：对所有按着的手指派发一次 `touchcancel`（`changedTouches` = 全部，`touches` = 空）。

use serde_json::{json, Value};

pub const PHASE_DOWN: i32 = 0;
pub const PHASE_MOVE: i32 = 1;
pub const PHASE_UP: i32 = 2;
pub const PHASE_CANCEL: i32 = 3;

/// 同时在屏上的触点上限
pub const MAX_TOUCHES: usize = 10;
/// 单帧单指最多派发的 move 数
pub const MAX_MOVES_PER_FRAME: usize = 8;

#[derive(Clone, Copy, Debug)]
struct Raw {
    phase: i32,
    host: i32,
    x: f32,
    y: f32,
    force: f32,
    t_us: u64,
}

#[derive(Clone, Copy, Debug)]
struct Slot {
    host: i32,
    ident: u32,
    x: f32,
    y: f32,
    force: f32,
}

/// 排好队、等派发的一个事件
#[derive(Debug)]
pub struct TouchEvent {
    pub kind: &'static str,
    /// 宿主时刻（µs，与 pump 同一时钟），换成游戏时间轴由调用方做
    pub t_us: u64,
    pub json: Value,
}

#[derive(Default)]
pub struct TouchInput {
    active: Vec<Slot>,
    queue: Vec<Raw>,
    /// 超过上限而被忽略的宿主指针：它们后续的 move / up 也不派发
    ignored: Vec<i32>,
    /// 宿主要求「全部取消」（进后台）
    cancel_all: Option<u64>,
    logs: Vec<String>,
}

impl TouchInput {
    /// 宿主来了一个指针事件（只排队）。phase 非法、坐标非有限值的丢掉
    pub fn push(&mut self, phase: i32, host: i32, x: f32, y: f32, force: f32, t_us: u64) {
        if !(PHASE_DOWN..=PHASE_CANCEL).contains(&phase) || !x.is_finite() || !y.is_finite() {
            return;
        }
        let force = if force.is_finite() { force.clamp(0.0, 1.0) } else { 0.0 };
        if phase == PHASE_MOVE {
            let moves = self.queue.iter().filter(|r| r.host == host && r.phase == PHASE_MOVE).count();
            if moves >= MAX_MOVES_PER_FRAME {
                if let Some(last) = self.queue.iter_mut().rev().find(|r| r.host == host && r.phase == PHASE_MOVE) {
                    (last.x, last.y, last.force, last.t_us) = (x, y, force, t_us);
                }
                return;
            }
        }
        self.queue.push(Raw { phase, host, x, y, force, t_us });
    }

    /// 进后台 / surface 丢失：取消全部按着的手指（排在已到的事件之后）
    pub fn cancel_all(&mut self, t_us: u64) {
        self.cancel_all = Some(t_us);
    }

    pub fn has_pending(&self) -> bool {
        !self.queue.is_empty() || self.cancel_all.is_some()
    }

    /// 按着的手指数（诊断）
    pub fn active(&self) -> usize {
        self.active.len()
    }

    /// 取走宿主日志（超限忽略等）
    pub fn take_logs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.logs)
    }

    /// 把队列按到达顺序变成事件
    pub fn drain(&mut self) -> Vec<TouchEvent> {
        let mut out = Vec::new();
        for r in std::mem::take(&mut self.queue) {
            let pos = self.active.iter().position(|s| s.host == r.host);
            match r.phase {
                PHASE_DOWN => {
                    if pos.is_some() || self.ignored.contains(&r.host) {
                        // 同一根指针没抬就又按下（宿主丢了 up）：当成一次移动，不重复分配 identifier
                        if let Some(i) = pos {
                            self.update(i, &r);
                            out.push(self.event("touchmove", r.t_us, &[i]));
                        }
                        continue;
                    }
                    if self.active.len() >= MAX_TOUCHES {
                        self.ignored.push(r.host);
                        self.logs.push(format!("⚠️ 触点超过 {MAX_TOUCHES} 个：指针 {} 被忽略（直到抬起）", r.host));
                        continue;
                    }
                    let ident = (0u32..).find(|n| !self.active.iter().any(|s| s.ident == *n)).unwrap_or(0);
                    self.active.push(Slot { host: r.host, ident, x: r.x, y: r.y, force: r.force });
                    let i = self.active.len() - 1;
                    out.push(self.event("touchstart", r.t_us, &[i]));
                }
                PHASE_MOVE => {
                    if let Some(i) = pos {
                        self.update(i, &r);
                        out.push(self.event("touchmove", r.t_us, &[i]));
                    }
                }
                _ => {
                    if let Some(i) = pos {
                        self.update(i, &r);
                        let gone = self.active.remove(i);
                        let kind = if r.phase == PHASE_UP { "touchend" } else { "touchcancel" };
                        out.push(TouchEvent { kind, t_us: r.t_us, json: self.payload(&[gone]) });
                    } else {
                        self.ignored.retain(|h| *h != r.host);
                    }
                }
            }
        }
        if let Some(t) = self.cancel_all.take() {
            self.ignored.clear();
            if !self.active.is_empty() {
                let gone = std::mem::take(&mut self.active);
                out.push(TouchEvent { kind: "touchcancel", t_us: t, json: self.payload(&gone) });
            }
        }
        out
    }

    fn update(&mut self, i: usize, r: &Raw) {
        let s = &mut self.active[i];
        (s.x, s.y, s.force) = (r.x, r.y, r.force);
    }

    fn event(&self, kind: &'static str, t_us: u64, changed: &[usize]) -> TouchEvent {
        let c: Vec<Slot> = changed.iter().map(|&i| self.active[i]).collect();
        TouchEvent { kind, t_us, json: self.payload(&c) }
    }

    /// `{touches, changedTouches}`（timeStamp 由调用方按游戏时间轴补上）
    fn payload(&self, changed: &[Slot]) -> Value {
        json!({
            "touches": self.active.iter().map(touch_json).collect::<Vec<_>>(),
            "changedTouches": changed.iter().map(touch_json).collect::<Vec<_>>(),
        })
    }
}

/// Touch 对象：小游戏里 screen / client / page 三组坐标都是视图内逻辑坐标（取证项，02 §7.2）
fn touch_json(s: &Slot) -> Value {
    let (x, y) = (s.x as f64, s.y as f64);
    json!({ "identifier": s.ident, "screenX": x, "screenY": y, "clientX": x, "clientY": y, "pageX": x, "pageY": y, "force": s.force as f64 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lens(e: &TouchEvent) -> (usize, usize) {
        (e.json["touches"].as_array().unwrap().len(), e.json["changedTouches"].as_array().unwrap().len())
    }

    #[test]
    fn 双指交错_identifier_稳定_touches_长度对() {
        let mut t = TouchInput::default();
        t.push(PHASE_DOWN, 7, 10.0, 10.0, 0.0, 1);
        t.push(PHASE_DOWN, 9, 50.0, 50.0, 0.0, 2);
        t.push(PHASE_UP, 7, 11.0, 11.0, 0.0, 3);
        t.push(PHASE_UP, 9, 51.0, 51.0, 0.0, 4);
        let ev = t.drain();
        let kinds: Vec<_> = ev.iter().map(|e| e.kind).collect();
        assert_eq!(kinds, ["touchstart", "touchstart", "touchend", "touchend"]);
        assert_eq!(ev.iter().map(lens).collect::<Vec<_>>(), [(1, 1), (2, 1), (1, 1), (0, 1)]);
        assert_eq!(ev[0].json["changedTouches"][0]["identifier"], 0);
        assert_eq!(ev[1].json["changedTouches"][0]["identifier"], 1);
        assert_eq!(ev[2].json["changedTouches"][0]["identifier"], 0, "抬起的是先按下的那根");
        assert_eq!(ev[2].json["touches"][0]["identifier"], 1);
        assert_eq!(ev[2].json["changedTouches"][0]["clientX"], 11.0, "抬起点的坐标");
    }

    #[test]
    fn 空出来的_identifier_被下一根复用() {
        let mut t = TouchInput::default();
        t.push(PHASE_DOWN, 1, 0.0, 0.0, 0.0, 1);
        t.push(PHASE_DOWN, 2, 0.0, 0.0, 0.0, 1);
        t.push(PHASE_UP, 1, 0.0, 0.0, 0.0, 2);
        t.push(PHASE_DOWN, 3, 0.0, 0.0, 0.0, 3);
        let ev = t.drain();
        assert_eq!(ev[3].json["changedTouches"][0]["identifier"], 0);
    }

    #[test]
    fn 第十一根被忽略_它的后续也不派发() {
        let mut t = TouchInput::default();
        for i in 0..11 {
            t.push(PHASE_DOWN, i, i as f32, 0.0, 0.0, 1);
        }
        t.push(PHASE_MOVE, 10, 5.0, 5.0, 0.0, 2);
        t.push(PHASE_UP, 10, 5.0, 5.0, 0.0, 3);
        let ev = t.drain();
        assert_eq!(ev.len(), 10);
        assert_eq!(t.take_logs().len(), 1);
        assert_eq!(t.active(), 10);
    }

    #[test]
    fn 单帧单指_move_最多_8_次_最后坐标保留() {
        let mut t = TouchInput::default();
        t.push(PHASE_DOWN, 0, 0.0, 0.0, 0.0, 0);
        for i in 1..=100 {
            t.push(PHASE_MOVE, 0, i as f32, 0.0, 0.0, i);
        }
        let ev = t.drain();
        let moves: Vec<_> = ev.iter().filter(|e| e.kind == "touchmove").collect();
        assert_eq!(moves.len(), 8);
        assert_eq!(moves[7].json["changedTouches"][0]["clientX"], 100.0);
        assert_eq!(moves[7].t_us, 100);
    }

    #[test]
    fn 全部取消() {
        let mut t = TouchInput::default();
        t.push(PHASE_DOWN, 0, 0.0, 0.0, 0.0, 0);
        t.push(PHASE_DOWN, 1, 0.0, 0.0, 0.0, 0);
        t.drain();
        t.cancel_all(5);
        let ev = t.drain();
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].kind, "touchcancel");
        assert_eq!(lens(&ev[0]), (0, 2));
        assert!(t.drain().is_empty(), "没按着的就不再派发");
    }

    #[test]
    fn 没按下的_move_up_与非法值丢掉() {
        let mut t = TouchInput::default();
        t.push(PHASE_MOVE, 3, 0.0, 0.0, 0.0, 0);
        t.push(PHASE_UP, 3, 0.0, 0.0, 0.0, 0);
        t.push(9, 3, 0.0, 0.0, 0.0, 0);
        t.push(PHASE_DOWN, 3, f32::NAN, 0.0, 0.0, 0);
        assert!(t.drain().is_empty());
    }
}
