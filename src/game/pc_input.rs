//! 小游戏的 PC 键鼠（game-doc/02 §7.3）：`wx.onMouseDown / onMouseUp / onMouseMove / onWheel`、`wx.onKeyDown / onKeyUp`。
//!
//! 与触摸同一个时机派发（pump 第 1 步，rAF 之前）。字段按官方文档；真机字段以取证为准（06 §7）：
//! - 鼠标按键 `button`：0 左、1 中、2 右（与 DOM MouseEvent 相同）；
//! - `onMouseMove` 的 `movementX / movementY` 是相对上一次 move 的位移（第一次为 0）；
//! - 键盘 `{key, code, timeStamp}`：`key` 是字符或键名（`"a"`、`"Enter"`），`code` 是物理键（`"KeyA"`）。
//!
//! 鼠标 move 不做每帧上限：桌面鼠标是 125~1000Hz，同一帧只留最后一次（位移累加），游戏要的是「这一帧动了多少」。

use serde_json::{json, Value};

pub const MOUSE_DOWN: i32 = 0;
pub const MOUSE_MOVE: i32 = 1;
pub const MOUSE_UP: i32 = 2;

pub struct PcEvent {
    /// 监听表里的名字（`mousedown` …）；键盘是 `keyDown` / `keyUp`（走 __hostDispatch）
    pub kind: &'static str,
    pub t_us: u64,
    pub json: Value,
}

#[derive(Default)]
pub struct PcInput {
    queue: Vec<PcEvent>,
    last: Option<(f32, f32)>,
}

impl PcInput {
    pub fn mouse(&mut self, phase: i32, button: i32, x: f32, y: f32, t_us: u64) {
        if !x.is_finite() || !y.is_finite() {
            return;
        }
        let (mx, my) = self.last.map(|(lx, ly)| (x - lx, y - ly)).unwrap_or((0.0, 0.0));
        self.last = Some((x, y));
        match phase {
            MOUSE_MOVE => {
                // 同一帧里接着上一次 move：并成一次，位移累加
                if let Some(prev) = self.queue.last_mut().filter(|e| e.kind == "mousemove") {
                    let (px, py) = (prev.json["movementX"].as_f64().unwrap_or(0.0), prev.json["movementY"].as_f64().unwrap_or(0.0));
                    prev.json = json!({ "x": x, "y": y, "movementX": px + mx as f64, "movementY": py + my as f64 });
                    prev.t_us = t_us;
                    return;
                }
                self.queue.push(PcEvent { kind: "mousemove", t_us, json: json!({ "x": x, "y": y, "movementX": mx, "movementY": my }) });
            }
            MOUSE_DOWN | MOUSE_UP => {
                let kind = if phase == MOUSE_DOWN { "mousedown" } else { "mouseup" };
                self.queue.push(PcEvent { kind, t_us, json: json!({ "button": button.clamp(0, 4), "x": x, "y": y }) });
            }
            _ => {}
        }
    }

    pub fn wheel(&mut self, dx: f32, dy: f32, dz: f32, x: f32, y: f32, t_us: u64) {
        if ![dx, dy, dz, x, y].iter().all(|v| v.is_finite()) {
            return;
        }
        self.queue.push(PcEvent { kind: "wheel", t_us, json: json!({ "deltaX": dx, "deltaY": dy, "deltaZ": dz, "x": x, "y": y }) });
    }

    pub fn key(&mut self, down: bool, key: &str, code: &str, t_us: u64) {
        let kind = if down { "keyDown" } else { "keyUp" };
        self.queue.push(PcEvent { kind, t_us, json: json!({ "key": key, "code": code }) });
    }

    pub fn has_pending(&self) -> bool {
        !self.queue.is_empty()
    }

    pub fn drain(&mut self) -> Vec<PcEvent> {
        std::mem::take(&mut self.queue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 同帧鼠标移动并成一次_位移累加() {
        let mut p = PcInput::default();
        p.mouse(MOUSE_MOVE, 0, 10.0, 10.0, 1);
        p.mouse(MOUSE_MOVE, 0, 13.0, 14.0, 2);
        p.mouse(MOUSE_MOVE, 0, 15.0, 10.0, 3);
        let ev = p.drain();
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].json["movementX"], 5.0);
        assert_eq!(ev[0].json["movementY"], 0.0);
        assert_eq!(ev[0].json["x"], 15.0);
        // 下一帧从上一次的位置接着算
        p.mouse(MOUSE_MOVE, 0, 16.0, 10.0, 4);
        assert_eq!(p.drain()[0].json["movementX"], 1.0);
    }

    #[test]
    fn 按键与滚轮() {
        let mut p = PcInput::default();
        p.mouse(MOUSE_DOWN, 2, 1.0, 2.0, 1);
        p.wheel(0.0, -120.0, 0.0, 1.0, 2.0, 2);
        p.key(true, "a", "KeyA", 3);
        let ev = p.drain();
        assert_eq!(ev.iter().map(|e| e.kind).collect::<Vec<_>>(), ["mousedown", "wheel", "keyDown"]);
        assert_eq!(ev[0].json["button"], 2);
        assert_eq!(ev[1].json["deltaY"], -120.0);
        assert_eq!(ev[2].json["code"], "KeyA");
    }
}
