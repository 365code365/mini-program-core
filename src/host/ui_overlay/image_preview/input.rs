//! 预览层的指针输入：单指（按下 / 移动 / 抬起：翻页、下拉关闭、轻点关闭 / 双击缩放、放大后平移）
//! 与第二根手指（[`on_pointer2`]：捏合缩放）。从 mod.rs 搬出来（那个文件过了 500 行）。
use super::*;

pub fn on_press(vp: (f32, f32), x: f32, y: f32) -> bool {
    let now = crate::frame_clock::now();
    let top = TOP.with(Cell::get);
    with(|s| {
        let Some(p) = s.as_mut() else { return false };
        if matches!(p.phase, Phase::Closing(..)) {
            return true;
        }
        p.vp = vp;
        let pos = pos_at(p, now);
        let caught = p.slide.is_some();
        // 滑动中途按住：停在手指下面（位置接着按下这一刻的往下算）
        p.slide = None;
        p.back = None;
        let mut track = Tracker::default();
        track.push(Instant::now(), x, y);
        let on_close = chrome::hits_close(vp.0, top, x, y);
        // 缩放动画中途按住：停在这一刻
        let v = p.zoom.at(now);
        p.zoom.set(v);
        p.pinch = None;
        p.p2 = None;
        p.rebase = false;
        p.drag = Some(Drag { x0: x, y0: y, x, y, axis: None, pos0: pos, caught, on_close, track, pan0: (v.px, v.py) });
        p.dirty = true;
        true
    })
}

pub fn on_move(x: f32, y: f32) -> bool {
    let now = crate::frame_clock::now();
    with(|s| {
        let Some(p) = s.as_mut() else { return false };
        let Some(d) = p.drag.as_mut() else { return true };
        if std::mem::take(&mut p.rebase) {
            // 捏合刚结束：从这里重新起算平移（剩下的手指可能不是原来那根）
            let v = p.zoom.at(now);
            (d.x0, d.y0, d.pan0) = (x, y, (v.px, v.py));
            d.axis = Some(if v.zoomed() { Axis::Pan } else { Axis::Pinch });
        }
        d.x = x;
        d.y = y;
        d.track.push(Instant::now(), x, y);
        match d.axis {
            Some(Axis::Pinch) => {
                if let (Some(pinch), Some(b)) = (p.pinch, p.p2) {
                    p.zoom.set(pinch.update((x, y), b, p.vp));
                }
                p.dirty = true;
                return true;
            }
            Some(Axis::Pan) => {
                let z = p.zoom.at(now).z;
                let (px, py) = zoom::rubber_pan(z, d.pan0.0 + x - d.x0, d.pan0.1 + y - d.y0, p.fit, p.vp);
                p.zoom.set(zoom::View { z, px, py });
                p.dirty = true;
                return true;
            }
            _ => {}
        }
        if d.axis.is_none() && !d.on_close && p.zoom.at(now).zoomed() {
            // 放大着：单指拖动是平移，不翻页、不下拉
            if (x - d.x0).abs().max((y - d.y0).abs()) >= TAP_SLOP {
                d.axis = Some(Axis::Pan);
            }
            return true;
        }
        if d.axis.is_none() && !d.on_close {
            let (dx, dy) = (x - d.x0, y - d.y0);
            if dx.abs().max(dy.abs()) >= TAP_SLOP {
                // 竖着往下拉、而且不是滑动中途按住的：下拉关闭；其余都算左右翻
                let down = dy > 0.0 && dy.abs() > dx.abs() * 1.2 && !d.caught && d.pos0.fract().abs() < 1e-3;
                d.axis = Some(if down { Axis::Down } else { Axis::X });
                // 手指已经走出去的那段不吞掉：从按下的地方起算（跟手不跳）
            }
        }
        true
    })
}

pub fn on_release(x: f32, y: f32) -> bool {
    let now = crate::frame_clock::now();
    let top = TOP.with(Cell::get);
    let consumed = visible();
    let toggle = with(|s| {
        let Some(p) = s.as_mut() else { return None };
        let Some(mut d) = p.drag.clone() else { return None };
        d.x = x;
        d.y = y;
        p.drag = Some(d.clone());
        let look = look_at(p, now);
        let (vx, vy) = d.track.velocity(Instant::now());
        p.drag = None;
        p.dirty = true;
        match d.axis {
            None => {
                if d.on_close && chrome::hits_close(p.vp.0, top, x, y) {
                    p.phase = Phase::Closing(now, look);
                } else if d.caught {
                    // 滑动中途按住又松开：滑到最近的那张
                    settle(p, look.pos, 0.0, now);
                } else if !p.items[p.index].video.is_empty() {
                    return Some(p.items[p.index].video.clone());
                } else if p.last_tap.is_some_and(|(t, tx, ty)| {
                    now.saturating_duration_since(t).as_secs_f32() < zoom::DOUBLE_TAP_SECS && (x - tx).hypot(y - ty) < zoom::DOUBLE_TAP_DIST
                }) {
                    // 双击：放大 / 还原（第一下的关闭撤掉）
                    p.last_tap = None;
                    p.pending_close = None;
                    let (fit, vp) = (p.fit, p.vp);
                    p.zoom.double_tap(now, x, y, fit, vp);
                } else {
                    // 单击：等一下看是不是双击，过了时限才关（take_requests）
                    p.last_tap = Some((now, x, y));
                    p.pending_close = Some(now);
                }
            }
            Some(Axis::Pinch) => {
                let (fit, vp) = (p.fit, p.vp);
                p.zoom.settle(now, fit, vp);
            }
            Some(Axis::Pan) => {
                // 放大着拖到边、再拖出去一截（超过五分之一屏）：换张，与微信一样
                let v = p.zoom.at(now);
                let (mx, _) = zoom::pan_limit(v.z, p.fit, p.vp);
                let over = v.px.abs() - mx;
                let raw = d.pan0.0 + d.x - d.x0;
                let pulled = raw.abs() - mx;
                let next = if raw > 0.0 { p.index.checked_sub(1) } else { Some(p.index + 1).filter(|i| *i < p.items.len()) };
                match next.filter(|_| over > 0.0 && pulled > p.vp.0 * 0.2) {
                    Some(i) => {
                        p.zoom = zoom::Zoom::default();
                        p.slide = Some((now, p.index as f32 - i as f32, 0.0));
                        p.index = i;
                    }
                    None => {
                        let (fit, vp) = (p.fit, p.vp);
                        p.zoom.settle(now, fit, vp);
                    }
                }
            }
            Some(Axis::Down) => {
                if d.y - d.y0 > DISMISS_DY || vy > DISMISS_VY {
                    p.phase = Phase::Closing(now, look);
                } else {
                    p.back = Some((now, d.x - d.x0, d.y - d.y0));
                }
            }
            Some(Axis::X) => {
                let dx = d.x - d.x0;
                let target = if dx <= -p.vp.0 * 0.25 || vx < -FLING_VX {
                    (d.pos0.floor() + 1.0).min(p.items.len() as f32 - 1.0)
                } else if dx >= p.vp.0 * 0.25 || vx > FLING_VX {
                    (d.pos0.ceil() - 1.0).max(0.0)
                } else {
                    look.pos.round().clamp(0.0, p.items.len() as f32 - 1.0)
                };
                if target as usize != p.index {
                    p.zoom = zoom::Zoom::default();
                }
                p.index = target as usize;
                p.slide = Some((now, look.pos - target, -vx / p.vp.0.max(1.0)));
            }
        }
        None
    });
    if let Some(v) = toggle {
        video::toggle(&v);
    }
    sync_video();
    consumed
}

/// 滑到离 `pos` 最近的那张
fn settle(p: &mut Preview, pos: f32, v: f32, now: Instant) {
    let target = pos.round().clamp(0.0, p.items.len() as f32 - 1.0);
    p.index = target as usize;
    p.slide = Some((now, pos - target, v));
}

/// 第二根手指（宿主只在两指同时按着时送）：`phase` 0 按下 / 1 移动 / 2 抬起 / 其它 取消，逻辑坐标。
/// 第一根手指抬起而第二根还按着时，宿主先送第二根的抬起、之后把它当第一根接着送（位置会跳一下，
/// 这里用 `rebase` 接住）
pub fn on_pointer2(phase: i32, vp: (f32, f32), x: f32, y: f32) -> bool {
    let now = crate::frame_clock::now();
    with(|s| {
        let Some(p) = s.as_mut() else { return false };
        if matches!(p.phase, Phase::Closing(..)) {
            return true;
        }
        match phase {
            0 => {
                // 只有当前那张停稳了才捏（滑动中途 / 下拉中不接）
                let Some(d) = p.drag.as_mut() else { return true };
                if p.slide.is_some() || d.axis == Some(Axis::Down) || d.axis == Some(Axis::X) || !p.items[p.index].video.is_empty() {
                    return true;
                }
                p.vp = vp;
                d.axis = Some(Axis::Pinch);
                p.back = None;
                p.pending_close = None;
                p.last_tap = None;
                let v = p.zoom.at(now);
                p.pinch = Some(zoom::Pinch::begin((d.x, d.y), (x, y), v, vp));
                p.p2 = Some((x, y));
            }
            1 => {
                if let (Some(pinch), Some(d)) = (p.pinch, p.drag.as_ref()) {
                    p.p2 = Some((x, y));
                    p.zoom.set(pinch.update((d.x, d.y), (x, y), p.vp));
                }
            }
            _ => {
                if p.pinch.take().is_some() {
                    p.p2 = None;
                    p.rebase = true;
                    let (fit, vp) = (p.fit, p.vp);
                    p.zoom.settle(now, fit, vp);
                }
            }
        }
        p.dirty = true;
        true
    })
}
