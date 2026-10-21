//! 滚轮列的拖动与回正：手指上下拖一列，选项跟手走；松手按速度甩出去几行，再吸附到整行。
//!
//! 微信的 picker 面板是原生滚轮，上下滑是主要的选法。从前这里只认「点非中心行 → 跳一格」，
//! 按住上下拖完全不动 —— 无头测试只用 `--touch` 点，于是一直没暴露。
//!
//! 位置用「行」做单位的浮点数：整数 = 那一项正好落在中心。**松手那一刻就把 `selected`
//! 定成落点**，回正动画只是视觉 —— 这样不需要每帧 tick 去「提交」选择，动画途中按
//! 「确定」拿到的也是手指甩向的那一项。
//!
//! 时间一律由调用方传进来（`now`），单测可以用造出来的 `Instant` 精确驱动速度与动画。

use super::*;
use std::time::Duration;

/// 判成「拖动」而不是「点一下」的位移（逻辑像素），与触摸状态机同一个阈值
const DRAG_SLOP: f32 = crate::host::touch::TOUCH_SLOP;
/// 速度只看松手前这一小段：手指停住再松就没有惯性（与页面滚动同一个窗口）
const VELOCITY_WINDOW: Duration = Duration::from_millis(100);
/// 松手速度往前投射多久（秒），决定一甩能滑几行
const PROJECT_SECS: f32 = 0.3;
/// 速度上限（逻辑像素/秒）。宿主忙时输入事件会攒成一批送进来，
/// 时间戳挤在一起，算出来的速度能大到一甩直接飞到列尾
const MAX_VELOCITY: f32 = 5000.0;
/// 拖过首尾项时的阻尼（手指走 1 行，滚轮只走这么多），最多拉出 1 行
const OVERSCROLL_DAMP: f32 = 0.35;
const OVERSCROLL_MAX: f32 = 1.0;

/// 各列的动态：至多一列在被拖，任意列可以在回正
#[derive(Clone, Default)]
pub struct WheelState {
    press: Option<WheelPress>,
    /// 下标 = 列号；按需扩容
    settles: Vec<Option<Settle>>,
}

#[derive(Clone)]
struct WheelPress {
    col: usize,
    press_y: f32,
    /// 位移从阈值边界算起：不然一判成拖动，滚轮就先跳 10px
    anchor_y: f32,
    anchor_pos: f32,
    dragging: bool,
    /// 按下时这一列还在回正：这一下是「停住」，抬手不当成点选（iOS / 微信一致）
    stopped_settle: bool,
    pos: f32,
    samples: Vec<(Instant, f32)>,
}

/// 松手后从 `from` 缓动到整行 `to`
#[derive(Clone, Copy)]
struct Settle {
    from: f32,
    to: f32,
    start: Instant,
    secs: f32,
}

impl Settle {
    /// 此刻的位置；走完返回 None
    fn at(&self, now: Instant) -> Option<f32> {
        let t = now.saturating_duration_since(self.start).as_secs_f32() / self.secs;
        if t >= 1.0 {
            return None;
        }
        // ease-out cubic：与面板入场同一条曲线
        let eased = 1.0 - (1.0 - t).powi(3);
        Some(self.from + (self.to - self.from) * eased)
    }
}

/// 拖过首尾项：阻尼 + 封顶（松手后回正把它拉回来）
fn rubber_band(raw: f32, max: f32) -> f32 {
    if raw < 0.0 {
        -(-raw * OVERSCROLL_DAMP).min(OVERSCROLL_MAX)
    } else if raw > max {
        max + ((raw - max) * OVERSCROLL_DAMP).min(OVERSCROLL_MAX)
    } else {
        raw
    }
}

/// 松手速度（行/秒，正 = 往后面的项走，即手指上推）
fn release_velocity(samples: &[(Instant, f32)], now: Instant) -> f32 {
    let mut recent = samples
        .iter()
        .filter(|(t, _)| now.saturating_duration_since(*t) <= VELOCITY_WINDOW);
    let Some(&(t0, y0)) = recent.next() else { return 0.0 };
    let Some(&(t1, y1)) = recent.last() else { return 0.0 };
    let dt = t1.saturating_duration_since(t0).as_secs_f32();
    if dt < 0.001 {
        return 0.0;
    }
    let px_per_sec = ((y0 - y1) / dt).clamp(-MAX_VELOCITY, MAX_VELOCITY);
    px_per_sec / ITEM_H
}

impl PickerSheetState {
    /// 第 `col` 列此刻的位置（行，浮点）。静止时就是 `selected[col]`。
    pub fn column_pos(&self, col: usize, now: Instant) -> f32 {
        if let Some(p) = self.wheel.press.as_ref().filter(|p| p.col == col) {
            return p.pos;
        }
        if let Some(v) = self.wheel.settles.get(col).copied().flatten().and_then(|s| s.at(now)) {
            return v;
        }
        self.selected.get(col).copied().unwrap_or(0) as f32
    }

    /// 有列还在回正（宿主据此续帧）
    pub fn wheel_animating(&self, now: Instant) -> bool {
        self.wheel.settles.iter().flatten().any(|s| s.at(now).is_some())
    }

    /// 有手指正按在滚轮上拖（测试 / 诊断用）
    pub fn wheel_dragging(&self) -> bool {
        self.wheel.press.as_ref().map(|p| p.dragging).unwrap_or(false)
    }

    fn max_index(&self, col: usize) -> f32 {
        self.columns.get(col).map(|c| c.len().saturating_sub(1)).unwrap_or(0) as f32
    }

    /// 手指按在第 `col` 列上。还在回正的话就地停住。
    pub(super) fn wheel_press(&mut self, col: usize, y: f32, now: Instant) {
        self.wheel.press = None;
        if self.columns.get(col).map(|c| c.is_empty()).unwrap_or(true) {
            return;
        }
        let pos = self.column_pos(col, now);
        let stopped_settle = self.wheel.settles.get(col).copied().flatten().and_then(|s| s.at(now)).is_some();
        if let Some(slot) = self.wheel.settles.get_mut(col) {
            *slot = None;
        }
        self.wheel.press = Some(WheelPress {
            col,
            press_y: y,
            anchor_y: y,
            anchor_pos: pos,
            dragging: false,
            stopped_settle,
            pos,
            samples: vec![(now, y)],
        });
    }

    /// 手指移动。返回滚轮位置是否变了（宿主据此标脏）。
    pub(super) fn wheel_move(&mut self, y: f32, now: Instant) -> bool {
        let Some(col) = self.wheel.press.as_ref().map(|p| p.col) else { return false };
        let max = self.max_index(col);
        let Some(p) = self.wheel.press.as_mut() else { return false };
        if !p.dragging {
            let dy = y - p.press_y;
            if dy.abs() <= DRAG_SLOP {
                return false;
            }
            p.dragging = true;
            // 锚在阈值边界上，而不是这一次 move 的位置：move 稀疏时（快甩、宿主忙）
            // 第一个越过阈值的 move 可能已经离按下点几十像素，锚在那里会把这一段整个吞掉
            p.anchor_y = p.press_y + DRAG_SLOP.copysign(dy);
        }
        p.samples.push((now, y));
        p.samples.retain(|(t, _)| now.saturating_duration_since(*t) <= VELOCITY_WINDOW);
        let next = rubber_band(p.anchor_pos + (p.anchor_y - y) / ITEM_H, max);
        let changed = next != p.pos;
        p.pos = next;
        changed
    }

    /// 抬手。返回 `true` = 这次按下已被滚轮消费（拖过 / 停住了回正），
    /// 调用方**不要**再把它当成「点某一行」。
    pub(super) fn wheel_release(&mut self, y: f32, now: Instant) -> bool {
        // 抬手事件带的就是手指最后的位置，也要算一次位移：宿主送来的 move 有多密
        // 取决于它的事件节流，最后一个 move 与抬手之间可能还隔着一大段
        self.wheel_move(y, now);
        let Some(p) = self.wheel.press.take() else { return false };
        if !p.dragging && !p.stopped_settle {
            return false;
        }
        let max = self.max_index(p.col);
        let projected = if p.dragging {
            p.pos + release_velocity(&p.samples, now) * PROJECT_SECS
        } else {
            p.pos
        };
        let target = projected.round().clamp(0.0, max) as usize;
        self.settle_to(p.col, p.pos, target, now);
        true
    }

    /// 掐掉第 `col` 列的回正动画（点选直接跳行时用）
    pub(super) fn wheel_stop(&mut self, col: usize) {
        if let Some(slot) = self.wheel.settles.get_mut(col) {
            *slot = None;
        }
    }

    /// 手势被系统打断：不带惯性，就近落到整行
    pub(super) fn wheel_cancel(&mut self, now: Instant) {
        if let Some(p) = self.wheel.press.take() {
            let target = p.pos.round().clamp(0.0, self.max_index(p.col)) as usize;
            self.settle_to(p.col, p.pos, target, now);
        }
    }

    /// 定下第 `col` 列的落点，并从 `from` 起一段回正动画
    fn settle_to(&mut self, col: usize, from: f32, target: usize, now: Instant) {
        let old = self.selected.get(col).copied().unwrap_or(0);
        if target != old {
            // 与点选同一条路径：date 换月重算天数、region 换省换市重建后续列
            self.move_selection(col, target as i32 - old as i32);
            relink_region(self, col);
            // 后面的列内容可能整列换了，它们手上的动画已经没有意义
            if matches!(self.mode, SheetMode::Date | SheetMode::Region) {
                for s in self.wheel.settles.iter_mut().skip(col + 1) {
                    *s = None;
                }
            }
        }
        let to = self.selected.get(col).copied().unwrap_or(0) as f32;
        let dist = (to - from).abs();
        if self.wheel.settles.len() <= col {
            self.wheel.settles.resize(col + 1, None);
        }
        self.wheel.settles[col] = (dist > 0.001).then(|| Settle {
            from,
            to,
            start: now,
            // 近的快、远的慢一点，但别拖太久
            secs: (0.2 + 0.08 * dist).clamp(0.2, 0.8),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(mode: SheetMode, columns: Vec<Vec<String>>, selected: Vec<usize>) -> PickerSheetState {
        PickerSheetState {
            id: "p".into(),
            mode,
            columns,
            selected,
            handler: None,
            column_handler: None,
            cancel_handler: None,
            header_text: String::new(),
            column_changes: Vec::new(),
            visible: true,
            pressed: None,
            anim_from: Instant::now() - Duration::from_secs(1),
            closing: false,
            date_fields: DateFields::Day,
            viewport: (375.0, 667.0),
            wheel: WheelState::default(),
        }
    }

    fn items(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("项{i}")).collect()
    }

    /// 先走到阈值边界（还不算拖动），再从那里匀速推 `rows` 行，分 `steps` 步、每步 `ms` 毫秒。
    /// 手指上推 = y 变小。返回最后一次移动的 (y, 时刻)。
    fn drag(s: &mut PickerSheetState, col: usize, y0: f32, rows: f32, steps: u32, ms: u64, t0: Instant) -> (f32, Instant) {
        s.wheel_press(col, y0, t0);
        let anchor = y0 - DRAG_SLOP;
        let mut t = t0 + Duration::from_millis(ms);
        assert!(!s.wheel_move(anchor, t), "正好在阈值上还不算拖动");
        let mut y = anchor;
        for i in 1..=steps {
            t = t0 + Duration::from_millis(ms * (i as u64 + 1));
            y = anchor - rows * ITEM_H * i as f32 / steps as f32;
            s.wheel_move(y, t);
        }
        (y, t)
    }

    #[test]
    fn 慢慢拖两行再停住松手_落在第二行之后() {
        let mut s = sheet(SheetMode::Selector, vec![items(10)], vec![0]);
        let t0 = Instant::now();
        let (y, t) = drag(&mut s, 0, 400.0, 2.0, 20, 16, t0);
        assert!(s.wheel_dragging());
        assert!((s.column_pos(0, t) - 2.0).abs() < 0.01, "跟手：拖两行就走两行");
        // 停 200ms 再松：速度窗口里只剩松手那一个点，没有惯性
        let up = t + Duration::from_millis(200);
        assert!(s.wheel_release(y, up));
        assert_eq!(s.selected[0], 2);
        assert_eq!(s.change_value(), serde_json::json!(2));
    }

    #[test]
    fn 快速一甩会多滑几行() {
        let mut s = sheet(SheetMode::Selector, vec![items(30)], vec![0]);
        let t0 = Instant::now();
        // 80ms 推两行 ≈ 1100px/s
        let (y, t) = drag(&mut s, 0, 400.0, 2.0, 5, 16, t0);
        assert!(s.wheel_release(y, t + Duration::from_millis(8)));
        assert!(s.selected[0] > 2, "带速度松手应继续滑，实际停在 {}", s.selected[0]);
        // 回正动画从当前位置出发、走完落在 selected 上
        assert!(s.wheel_animating(t + Duration::from_millis(20)));
        let end = t + Duration::from_secs(2);
        assert!(!s.wheel_animating(end));
        assert_eq!(s.column_pos(0, end), s.selected[0] as f32);
    }

    #[test]
    fn 往下拖是往前面的项走_并且夹在首项() {
        let mut s = sheet(SheetMode::Selector, vec![items(3)], vec![2]);
        let t0 = Instant::now();
        s.wheel_press(0, 300.0, t0);
        // 手指往下拖 6 行，远超首项：位置被阻尼并封顶在 -1 行以内
        s.wheel_move(300.0 + DRAG_SLOP + 1.0, t0 + Duration::from_millis(16));
        s.wheel_move(300.0 + DRAG_SLOP + 1.0 + 6.0 * ITEM_H, t0 + Duration::from_millis(32));
        let pos = s.column_pos(0, t0 + Duration::from_millis(32));
        assert!((-OVERSCROLL_MAX..0.0).contains(&pos), "越过首项应有阻尼与上限，实际 {pos}");
        assert!(s.wheel_release(300.0 + 6.0 * ITEM_H, t0 + Duration::from_millis(40)));
        assert_eq!(s.selected[0], 0);
    }

    #[test]
    fn 没过阈值的按下抬手仍是点选() {
        let mut s = sheet(SheetMode::Selector, vec![items(5)], vec![0]);
        let t0 = Instant::now();
        s.wheel_press(0, 300.0, t0);
        assert!(!s.wheel_move(300.0 + DRAG_SLOP - 1.0, t0 + Duration::from_millis(16)));
        assert!(!s.wheel_release(300.0, t0 + Duration::from_millis(30)), "小抖动不该吞掉点选");
        assert_eq!(s.selected[0], 0);
    }

    #[test]
    fn 回正途中按一下只是停住_就近落行() {
        let mut s = sheet(SheetMode::Selector, vec![items(30)], vec![0]);
        let t0 = Instant::now();
        let (y, t) = drag(&mut s, 0, 400.0, 2.0, 5, 16, t0);
        s.wheel_release(y, t + Duration::from_millis(8));
        let flung_to = s.selected[0];
        // 回正跑到一半按住
        let mid = t + Duration::from_millis(60);
        let pos_mid = s.column_pos(0, mid);
        s.wheel_press(0, 300.0, mid);
        assert_eq!(s.column_pos(0, mid), pos_mid, "按住要就地停住，不能跳");
        assert!(s.wheel_release(300.0, mid + Duration::from_millis(50)), "停住的这一下不算点选");
        assert_eq!(s.selected[0], pos_mid.round() as usize);
        assert!(s.selected[0] <= flung_to);
    }

    #[test]
    fn 省市区拖省这一列会重建后面两列() {
        let (cols, sel) = super::super::region::region_columns(&[]);
        let mut s = sheet(SheetMode::Region, cols, sel);
        let before = s.columns[1].clone();
        let t0 = Instant::now();
        let (y, t) = drag(&mut s, 0, 400.0, 3.0, 20, 16, t0);
        s.wheel_release(y, t + Duration::from_millis(200));
        assert_eq!(s.selected[0], 3);
        assert_ne!(s.columns[1], before, "换了省，市这一列要跟着换");
        assert_eq!(s.selected[1], 0);
    }

    #[test]
    fn 被打断时不带惯性就近落行() {
        let mut s = sheet(SheetMode::Selector, vec![items(30)], vec![0]);
        let t0 = Instant::now();
        drag(&mut s, 0, 400.0, 1.4, 5, 16, t0);
        s.wheel_cancel(t0 + Duration::from_millis(90));
        assert_eq!(s.selected[0], 1);
        assert!(!s.wheel_dragging());
    }
}
