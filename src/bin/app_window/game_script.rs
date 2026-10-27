//! 小游戏无头输入脚本（game-doc/04 G3.6）：把命令行动作排成「第几帧之前喂哪个指针事件」。
//!
//! - 与小程序同一套动作开关（`headless_script`），按书写顺序从第 1 帧起排：
//!   `--touch x,y[,按住ms]` = 下 → 按住 → 抬；`--click x,y` = 下、下一帧抬；`--press x,y` = 只按下；
//!   `--swipe x0,y0,x1,y1[,步数]` = 下 → 每帧一步移动 → 抬（`--swipe-hold` 不抬）；`--wait 秒` = 空出这么多帧。
//!   其余动作（打字、滚轮、拖动计时）对游戏没有意义，忽略并提示一行。
//! - `--multitouch "7:100,200@0;7:110,200@16;9:300,400@32;u7@48;c9@64"`：按游戏时刻（毫秒）写的多指序列（07 §5），
//!   `id:x,y@ms`；没前缀的第一次出现是按下、之后是移动，`u` 抬起、`c` 取消（可以不写坐标，沿用最后一次）。
//! 坐标一律是游戏的逻辑像素（无头实例 375×667）。

use super::headless_script::Action;
use mini_render::game::touch::{PHASE_CANCEL, PHASE_DOWN, PHASE_MOVE, PHASE_UP};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ev {
    /// 在第几帧的 pump 之前喂进去（1 起）
    pub frame: u64,
    pub phase: i32,
    pub id: i32,
    pub x: f32,
    pub y: f32,
}

pub fn timeline(actions: &[Action], multi: Option<&str>, hz: u32) -> Vec<Ev> {
    let hz = hz.max(1) as f32;
    let mut out = Vec::new();
    let mut at = 1u64;
    let push = |frame, phase, x, y, out: &mut Vec<Ev>| out.push(Ev { frame, phase, id: 0, x, y });
    for a in actions {
        match a {
            Action::Touch { x, y, hold_ms } => {
                let hold = ((*hold_ms as f32 * hz / 1000.0).ceil() as u64).max(1);
                push(at, PHASE_DOWN, *x, *y, &mut out);
                push(at + hold, PHASE_UP, *x, *y, &mut out);
                at += hold + 1;
            }
            Action::Click(x, y) => {
                push(at, PHASE_DOWN, *x, *y, &mut out);
                push(at + 1, PHASE_UP, *x, *y, &mut out);
                at += 2;
            }
            Action::Press(x, y) => {
                push(at, PHASE_DOWN, *x, *y, &mut out);
                at += 1;
            }
            Action::Swipe { from, to, steps, hold } => {
                let n = (*steps).max(1);
                push(at, PHASE_DOWN, from.0, from.1, &mut out);
                for i in 1..=n {
                    let k = i as f32 / n as f32;
                    push(at + i as u64, PHASE_MOVE, from.0 + (to.0 - from.0) * k, from.1 + (to.1 - from.1) * k, &mut out);
                }
                at += n as u64 + 1;
                if !hold {
                    push(at, PHASE_UP, to.0, to.1, &mut out);
                    at += 1;
                }
            }
            Action::Wait(s) => at += (s * hz).round().max(0.0) as u64,
            other => eprintln!("⚠️ 小游戏无头不支持这个动作，忽略：{other:?}"),
        }
    }
    if let Some(m) = multi {
        match parse_multi(m, hz) {
            Ok(v) => out.extend(v),
            Err(e) => eprintln!("⚠️ --multitouch 解析失败（{e}），整段忽略"),
        }
    }
    // 同一帧内保持书写顺序（稳定排序）
    out.sort_by_key(|e| e.frame);
    out
}

/// `[u|c]<id>:<x>,<y>@<毫秒>`，`;` 分隔（game-doc/07 §5）。没有前缀的：这根手指没按着就是按下，按着就是移动。
/// 毫秒是相对第 1 帧的游戏时刻，落到 `round(ms × hz / 1000) + 1` 那一帧；抬起 / 取消可以不写坐标（沿用最后一次）
fn parse_multi(s: &str, hz: f32) -> Result<Vec<Ev>, String> {
    let mut out = Vec::new();
    let mut down: std::collections::HashMap<i32, (f32, f32)> = Default::default();
    for item in s.split(';').map(str::trim).filter(|t| !t.is_empty()) {
        let (body, ms) = item.split_once('@').ok_or_else(|| format!("{item:?} 缺 @毫秒"))?;
        let ms: f32 = ms.trim().parse().map_err(|_| format!("{item:?} 毫秒不是数字"))?;
        // 四舍五入到最近的帧：60Hz 下 @16 是第 2 帧，不是与 @0 挤在第 1 帧
        let frame = (ms.max(0.0) * hz / 1000.0).round() as u64 + 1;
        let (end, body) = match body.chars().next() {
            Some('u') => (Some(PHASE_UP), &body[1..]),
            Some('c') => (Some(PHASE_CANCEL), &body[1..]),
            _ => (None, body),
        };
        let (id, xy) = match body.split_once(':') {
            Some((id, xy)) => (id, Some(xy)),
            None => (body, None),
        };
        let id: i32 = id.trim().parse().map_err(|_| format!("{item:?} 指针 id 不是数字"))?;
        let xy = match xy {
            Some(xy) => {
                let v: Vec<f32> = xy.split(',').filter_map(|n| n.trim().parse().ok()).collect();
                if v.len() != 2 {
                    return Err(format!("{item:?} 坐标要写成 x,y"));
                }
                Some((v[0], v[1]))
            }
            None => None,
        };
        let phase = match end {
            Some(p) => {
                if !down.contains_key(&id) {
                    return Err(format!("{item:?} 这根手指没按着"));
                }
                p
            }
            None if down.contains_key(&id) => PHASE_MOVE,
            None => PHASE_DOWN,
        };
        let (x, y) = match xy.or_else(|| down.get(&id).copied()) {
            Some(p) => p,
            None => return Err(format!("{item:?} 按下 / 移动要给坐标")),
        };
        if end.is_some() {
            down.remove(&id);
        } else {
            down.insert(id, (x, y));
        }
        out.push(Ev { frame, phase, id, x, y });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 动作按顺序排帧() {
        let t = timeline(&[Action::Touch { x: 1.0, y: 2.0, hold_ms: 50 }, Action::Wait(0.05), Action::Click(5.0, 6.0)], None, 60);
        let f: Vec<(u64, i32)> = t.iter().map(|e| (e.frame, e.phase)).collect();
        // 50ms@60Hz = 3 帧
        assert_eq!(f, [(1, PHASE_DOWN), (4, PHASE_UP), (8, PHASE_DOWN), (9, PHASE_UP)]);
    }

    #[test]
    fn 多指语法() {
        let t = timeline(&[], Some("7:100,200@0;7:110,200@16;9:300,400@32;u7@48;c9:1,2@64"), 60);
        let f: Vec<(u64, i32, i32)> = t.iter().map(|e| (e.frame, e.phase, e.id)).collect();
        assert_eq!(f, [(1, PHASE_DOWN, 7), (2, PHASE_MOVE, 7), (3, PHASE_DOWN, 9), (4, PHASE_UP, 7), (5, PHASE_CANCEL, 9)]);
        assert_eq!((t[3].x, t[3].y), (110.0, 200.0), "抬起沿用最后坐标");
        assert!(parse_multi("u5@3", 60.0).is_err(), "没按着就抬");
        assert!(parse_multi("5@3", 60.0).is_err(), "按下没坐标");
        assert!(parse_multi("5:1,2", 60.0).is_err(), "缺时刻");
    }
}
