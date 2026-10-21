//! `<picker-view>` 的滚轮状态机：每列一个连续位置（以「项」为单位的浮点下标），跟手拖、松手按速度
//! 落到整项、吸附动画走完发事件。
//!
//! 依据官方 component/picker-view.html：
//! - `value`：数组，每列选中第几项（数字下标，超出范围按最后一项）；页面 setData 改它就跳过去；
//! - `bindchange`：滚动**停下**且选中项变了时，`detail.value` 是各列下标数组（number）；
//! - `bindpickstart` / `bindpickend`：开始滚动 / 滚动结束；
//! - `immediate-change`：手指一松就发 change（不等吸附动画走完）。
//!
//! 手势由宿主的手势仲裁（`host::input`）交来：每一列在绘制时登记成一个**纵向 swiper 候选**，
//! 键是 [`COLUMN_KEY_PREFIX`] + `<picker-view 组件 id>#<列号>`，swiper 的 drag 三个入口按前缀转到这里
//! —— 这样纵向手势的归属（与页面滚动、外层 scroll-view 的仲裁）和 swiper 完全一致，不用另起一套。
//! 时间取 swiper 的动画时钟（无头快照的 `--time` 固定它，结果可复现）。

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;

/// 列手势键的前缀（见模块注释）
pub const COLUMN_KEY_PREFIX: &str = "pv:";

/// 吸附动画时长（毫秒）。微信的滚轮约 200~300ms 落定
const SETTLE_MS: f64 = 260.0;

#[derive(Debug, Clone, Default)]
struct Column {
    /// 当前位置（项下标，浮点）
    pos: f32,
    /// 项高（逻辑像素）与项数：绘制时同步
    item_h: f32,
    count: usize,
    drag: Option<Drag>,
    /// 吸附动画：(起点, 终点, 开始时刻 ms)
    anim: Option<(f32, f32, u64)>,
}

#[derive(Debug, Clone, Copy)]
struct Drag {
    start_coord: f32,
    start_pos: f32,
    last_coord: f32,
    last_ms: u64,
    /// 速度（项 / 秒，正值往下标增大的方向）
    vel: f32,
}

#[derive(Debug, Clone, Default)]
struct PickerView {
    cols: Vec<Column>,
    /// 上次看到的 `value` 属性原文：变了才算页面改了受控值
    value_attr: String,
    /// 上次通知页面的各列下标
    reported: Vec<usize>,
    picking: bool,
    immediate: bool,
}

impl PickerView {
    fn indices(&self) -> Vec<usize> {
        self.cols.iter().map(|c| index_of(c.pos, c.count)).collect()
    }
}

fn index_of(pos: f32, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    (pos.round().max(0.0) as usize).min(count - 1)
}

type Events = Vec<(String, &'static str, Value)>;

static STATE: Mutex<Option<(HashMap<String, PickerView>, Events)>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut HashMap<String, PickerView>, &mut Events) -> R) -> R {
    let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let (map, ev) = g.get_or_insert_with(|| (HashMap::new(), Vec::new()));
    f(map, ev)
}

fn now() -> u64 {
    super::swiper_clock_ms()
}

fn parse_value(raw: &str) -> Vec<usize> {
    match super::parse_attr_json(raw) {
        Value::Array(a) => a.iter().map(|v| v.as_f64().unwrap_or(0.0).max(0.0) as usize).collect(),
        _ => Vec::new(),
    }
}

/// 绘制时同步：列数 / 每列项数与项高、受控 `value`。返回各列当前位置（项下标，浮点）
pub fn sync(id: &str, value_attr: &str, immediate: bool, cols: &[(usize, f32)]) -> Vec<f32> {
    with(|map, _| {
        let pv = map.entry(id.to_string()).or_default();
        pv.immediate = immediate;
        let fresh = pv.cols.len() != cols.len();
        if fresh {
            pv.cols = vec![Column::default(); cols.len()];
        }
        for (c, &(count, item_h)) in pv.cols.iter_mut().zip(cols) {
            c.count = count;
            c.item_h = item_h.max(1.0);
            if count > 0 && c.drag.is_none() && c.anim.is_none() {
                c.pos = c.pos.clamp(0.0, (count - 1) as f32);
            }
        }
        if fresh || pv.value_attr != value_attr {
            pv.value_attr = value_attr.to_string();
            let v = parse_value(value_attr);
            for (i, c) in pv.cols.iter_mut().enumerate() {
                if c.drag.is_none() {
                    c.anim = None;
                    c.pos = index_of(v.get(i).copied().unwrap_or(0) as f32, c.count) as f32;
                }
            }
            pv.reported = pv.indices();
        }
        pv.cols.iter().map(|c| c.pos).collect()
    })
}

/// 各列当前位置（没有状态时 None：按 `value` 属性画）
pub fn positions(id: &str) -> Option<Vec<f32>> {
    with(|map, _| map.get(id).map(|pv| pv.cols.iter().map(|c| c.pos).collect()))
}

fn split_key(key: &str) -> Option<(String, usize)> {
    let rest = key.strip_prefix(COLUMN_KEY_PREFIX)?;
    let (id, col) = rest.rsplit_once('#')?;
    Some((id.to_string(), col.parse().ok()?))
}

/// 列手势键（绘制时登记、手势仲裁交回来）
pub fn column_key(id: &str, col: usize) -> String {
    format!("{COLUMN_KEY_PREFIX}{id}#{col}")
}

pub fn drag_begin(key: &str, coord: f32) {
    drag_begin_at(key, coord, now());
}

fn drag_begin_at(key: &str, coord: f32, t: u64) {
    let Some((id, col)) = split_key(key) else { return };
    with(|map, ev| {
        let Some(pv) = map.get_mut(&id) else { return };
        if !pv.picking {
            pv.picking = true;
            ev.push((id.clone(), "pickstart", json!({})));
        }
        if let Some(c) = pv.cols.get_mut(col) {
            c.anim = None;
            c.drag = Some(Drag { start_coord: coord, start_pos: c.pos, last_coord: coord, last_ms: t, vel: 0.0 });
        }
    });
}

pub fn drag_update(key: &str, coord: f32) {
    drag_update_at(key, coord, now());
}

fn drag_update_at(key: &str, coord: f32, t: u64) {
    let Some((id, col)) = split_key(key) else { return };
    with(|map, _| {
        let Some(c) = map.get_mut(&id).and_then(|pv| pv.cols.get_mut(col)) else { return };
        let Some(d) = c.drag.as_mut() else { return };
        // 手指往上划（坐标变小）= 下标变大
        let raw = d.start_pos - (coord - d.start_coord) / c.item_h;
        let max = c.count.saturating_sub(1) as f32;
        // 越过两端：阻尼 0.3（橡皮筋）
        c.pos = if raw < 0.0 { raw * 0.3 } else if raw > max { max + (raw - max) * 0.3 } else { raw };
        let dt = t.saturating_sub(d.last_ms) as f32 / 1000.0;
        if dt > 0.0 {
            let v = -(coord - d.last_coord) / c.item_h / dt;
            d.vel = d.vel * 0.3 + v * 0.7;
            d.last_coord = coord;
            d.last_ms = t;
        }
    });
}

pub fn drag_end(key: &str) {
    drag_end_at(key, now());
}

fn drag_end_at(key: &str, t: u64) {
    let Some((id, col)) = split_key(key) else { return };
    with(|map, ev| {
        let Some(pv) = map.get_mut(&id) else { return };
        let immediate = pv.immediate;
        let Some(c) = pv.cols.get_mut(col) else { return };
        let Some(d) = c.drag.take() else { return };
        // 按松手速度再滑一段（惯性 ≈ 速度 × 0.25s），落到整项
        let fling = if t.saturating_sub(d.last_ms) > 100 { 0.0 } else { d.vel * 0.25 };
        let target = index_of(c.pos + fling, c.count) as f32;
        c.anim = Some((c.pos, target, t));
        if immediate {
            let mut idx = pv.indices();
            idx[col] = target as usize;
            if idx != pv.reported {
                pv.reported = idx.clone();
                ev.push((id.clone(), "change", json!({ "value": idx })));
            }
        }
    });
}

/// 每帧推进吸附动画；落定时发 change（选中项变了）与 pickend。返回是否还在动
pub fn tick() -> bool {
    tick_at(now())
}

fn tick_at(t: u64) -> bool {
    with(|map, ev| {
        let mut busy = false;
        for (id, pv) in map.iter_mut() {
            let mut any = false;
            for c in pv.cols.iter_mut() {
                if c.drag.is_some() {
                    any = true;
                    continue;
                }
                if let Some((from, to, t0)) = c.anim {
                    let p = ((t.saturating_sub(t0)) as f64 / SETTLE_MS).min(1.0) as f32;
                    let e = 1.0 - (1.0 - p).powi(3); // ease-out cubic
                    c.pos = from + (to - from) * e;
                    if p >= 1.0 {
                        c.pos = to;
                        c.anim = None;
                    } else {
                        any = true;
                    }
                }
            }
            busy |= any;
            if !any && pv.picking {
                pv.picking = false;
                let idx = pv.indices();
                if idx != pv.reported {
                    pv.reported = idx.clone();
                    ev.push((id.clone(), "change", json!({ "value": idx })));
                }
                ev.push((id.clone(), "pickend", json!({})));
            }
        }
        busy
    })
}

/// 有列在拖或在吸附：宿主续帧
pub fn needs_frame() -> bool {
    with(|map, _| map.values().any(|pv| pv.picking || pv.cols.iter().any(|c| c.drag.is_some() || c.anim.is_some())))
}

/// 有列正被手指拖着（触摸序列据此补 touchcancel，与 swiper 一致）
pub fn is_dragging() -> bool {
    with(|map, _| map.values().any(|pv| pv.cols.iter().any(|c| c.drag.is_some())))
}

/// 取走待派发的事件：(picker-view 组件 id, 事件名, detail)
pub fn take_events() -> Events {
    with(|_, ev| std::mem::take(ev))
}

/// 换页：状态表跟着页面走（组件 id 在不同页面可能撞）
pub fn clear() {
    with(|map, ev| {
        map.clear();
        ev.clear();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_attr_drives_position_and_drag_snaps_with_events() {
        let p = sync("pvT", "[2, 0]", false, &[(5, 40.0), (3, 40.0)]);
        assert_eq!(p, vec![2.0, 0.0]);
        // 往上拖 1.4 项 → 松手 → 吸附到第 3 项
        drag_begin_at(&column_key("pvT", 0), 300.0, 1000);
        drag_update_at(&column_key("pvT", 0), 244.0, 1300);
        drag_end_at(&column_key("pvT", 0), 1500);
        assert!(needs_frame());
        tick_at(2000);
        assert_eq!(positions("pvT").unwrap(), vec![3.0, 0.0]);
        let ev: Vec<_> = take_events().into_iter().filter(|e| e.0 == "pvT").collect();
        let names: Vec<_> = ev.iter().map(|e| e.1).collect();
        assert_eq!(names, vec!["pickstart", "change", "pickend"]);
        assert_eq!(ev[1].2["value"], json!([3, 0]));
        // 页面 setData 改 value：直接跳过去，不发 change
        sync("pvT", "[0, 2]", false, &[(5, 40.0), (3, 40.0)]);
        assert_eq!(positions("pvT").unwrap(), vec![0.0, 2.0]);
        assert!(take_events().iter().all(|e| e.0 != "pvT"));
        // 越界的值按最后一项
        sync("pvT", "[9, 9]", false, &[(5, 40.0), (3, 40.0)]);
        assert_eq!(positions("pvT").unwrap(), vec![4.0, 2.0]);
    }
}
