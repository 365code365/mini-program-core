//! `<picker>` 的原生选择面板（底部弹出的滚轮）。
//!
//! 微信里 picker 不是页面里的一段 DOM，而是宿主弹出的原生浮层：点一下 picker
//! 弹出底部面板，选好按「确定」才回调 `bindchange`，点「取消」或遮罩直接关闭。
//! 这里就是那一层：状态 + 绘制 + 命中，全部走视口坐标（不受页面滚动影响）。
//!
//! 面板画在 present 之后的 buffer 上（与 Toast/Modal 同一层机制），
//! 因此不参与页面的损伤区重绘，也不会被页面裁剪。

use crate::{Canvas, Color, Paint};
use crate::text::TextRenderer;
use std::time::Instant;

use super::{LOGICAL_WIDTH, LOGICAL_HEIGHT};
use crate::renderer::PickerBinding;

/// 面板各部分的逻辑尺寸（与微信目测一致）
pub const HEADER_H: f32 = 45.0;
pub const ITEM_H: f32 = 44.0;
/// 滚轮可见行数（奇数，选中项居中）
pub const VISIBLE_ROWS: usize = 5;
pub const WHEEL_H: f32 = ITEM_H * VISIBLE_ROWS as f32;
pub const SHEET_H: f32 = HEADER_H + WHEEL_H;
/// 入场/退场动画时长（秒）
const ANIM_SECS: f32 = 0.22;

// ── 按职责切开的几片 ──
/// 从 WXML 绑定构造面板状态
mod build;
/// 指针闭环（命中 / 按下 / 抬起 / 收尾）
mod pointer;
pub mod emit;
/// 面板绘制
mod render;
/// 省市区联动
mod region;
/// 滚轮列的拖动、惯性与回正
mod wheel;
/// 面板弹着时跟上页面数据（multiSelector 多列联动）
mod sync;
pub use sync::sync_columns;
pub use build::build;
pub use pointer::{hit_test, on_cancel, on_move, on_press, on_release, open_if_hit, reap, SheetMove};
pub use region::relink_region;
pub use render::{reach, render};
pub use wheel::WheelState;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SheetMode {
    Selector,
    MultiSelector,
    Time,
    Date,
    Region,
}

/// 面板上可按的东西
#[derive(Clone, PartialEq, Debug)]
pub enum SheetHit {
    Cancel,
    Confirm,
    /// 点在第 col 列、相对中心第 delta 行（delta 可负）
    Item { col: usize, delta: i32 },
    /// 点在遮罩上（关闭）
    Mask,
}

#[derive(Clone)]
pub struct PickerSheetState {
    /// 触发它的 picker 组件 id
    pub id: String,
    pub mode: SheetMode,
    /// 每列的选项文本
    pub columns: Vec<Vec<String>>,
    /// 每列当前选中的下标
    pub selected: Vec<usize>,
    /// `bindchange` 处理函数名
    pub handler: Option<String>,
    /// `bindcolumnchange` / `bindcancel` 处理函数名
    pub column_handler: Option<String>,
    pub cancel_handler: Option<String>,
    /// `header-text`：头部中间的标题（空串不画）
    pub header_text: String,
    /// 还没派发的列变化 (列, 新下标)：multiSelector 每列落定一次记一条（见 `pointer::on_release`）
    pub column_changes: Vec<(usize, usize)>,
    /// 点开它的那一点 `(x, 命中纵坐标, 在 fixed 层)`：change / cancel / columnchange 从这里经事件链派发，
    /// 事件对象与 tap 同一套（`currentTarget.dataset` 是 picker 自己的 `data-*`、`type` 对、会冒泡）
    pub origin: Option<(f32, f32, bool)>,
    /// 抬手里定下、还没派发的事件 `(事件类型, detail)`，由宿主拿着渲染器调 [`emit::flush`] 发出去
    pub outbox: Vec<(&'static str, serde_json::Value)>,
    pub visible: bool,
    pub pressed: Option<SheetHit>,
    /// 动画起点：入场从这时开始算，退场时重置
    pub anim_from: Instant,
    /// 正在退场（动画走完由宿主丢弃）
    pub closing: bool,
    /// `date` 模式的年月日范围（用于换月后重算天数）
    pub date_fields: DateFields,
    /// `date` 模式的 `start` / `end`（含两端，年月日）。月、日两列按它裁：起始年只从起始月开始，
    /// 起始年月只从起始日开始；结束那头同理。没写 start / end 时是年份列的两头（不裁月日）
    pub date_range: ((i32, u32, u32), (i32, u32, u32)),
    /// 弹出时的逻辑视口（宽, 高）。面板贴着视口底边、头部按宽度左右分「取消 / 确定」。
    /// 从前写死 375×667：别的尺寸上面板画在半空、按钮点不准（桌面窗体就是 375×667，不受影响）。
    pub viewport: (f32, f32),
    /// 滚轮列的拖动 / 回正（见 `wheel.rs`）。静止时每列就停在 `selected` 上
    pub wheel: WheelState,
}

/// `date` 模式显示到哪一级
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DateFields {
    Year,
    Month,
    Day,
}

impl PickerSheetState {
    /// 面板整体的纵向位移（0 = 完全展开，SHEET_H = 完全收起）
    pub fn slide_offset(&self) -> f32 {
        // 按帧时刻取样（frame_clock.rs）：画的那一刻相对 vsync 忽早忽晚，滑入会一顿一顿
        let t = (crate::frame_clock::now().saturating_duration_since(self.anim_from).as_secs_f32() / ANIM_SECS).clamp(0.0, 1.0);
        // ease-out cubic：起步快、收尾稳，和微信的弹出手感接近
        let eased = 1.0 - (1.0 - t).powi(3);
        if self.closing {
            SHEET_H * eased
        } else {
            SHEET_H * (1.0 - eased)
        }
    }

    /// 还有动画在走（面板入场/退场，或某列滚轮在回正），宿主据此续帧
    pub fn animating(&self) -> bool {
        self.slide_animating() || self.wheel_animating(crate::frame_clock::now())
    }

    /// 面板整体的入场/退场位移是否还在走
    fn slide_animating(&self) -> bool {
        crate::frame_clock::now().saturating_duration_since(self.anim_from).as_secs_f32() < ANIM_SECS
    }

    /// 退场动画是否已经走完，可以丢弃了。
    /// 只看面板位移：滚轮回正是面板里的事，面板都滑出去了没必要等它。
    pub fn finished_closing(&self) -> bool {
        self.closing && !self.slide_animating()
    }

    pub fn begin_close(&mut self) {
        if !self.closing {
            self.closing = true;
            self.anim_from = Instant::now();
            self.pressed = None;
        }
    }

    /// 把某一列的选中项移动 delta 行（点击非中心行 / 拖动松手后的落点）
    pub fn move_selection(&mut self, col: usize, delta: i32) {
        let Some(items) = self.columns.get(col) else { return };
        if items.is_empty() {
            return;
        }
        let cur = self.selected.get(col).copied().unwrap_or(0) as i32;
        let next = (cur + delta).clamp(0, items.len() as i32 - 1) as usize;
        let changed = self.selected.get(col).map(|s| *s != next).unwrap_or(false);
        if let Some(slot) = self.selected.get_mut(col) {
            *slot = next;
        }
        // multiSelector：某一列换了值就要发 bindcolumnchange（页面据此改后面几列的选项）
        if changed && self.mode == SheetMode::MultiSelector {
            self.column_changes.push((col, next));
        }
        // multiSelector 的联动（bindcolumnchange）与 date 的天数重算
        if self.mode == SheetMode::Date {
            self.rebuild_date_columns();
        }
    }

    /// 年/月变化后按 `date_range` 重建「月」「日」两列（闰年、大小月、start / end 裁边），
    /// 选中项按**数值**保持：在 3 月选着 31 日、换到 2 月就落在 28/29 日；
    /// 换到起始年而月份早于起始月就落在起始月。从前只重算天数、不看 start / end ——
    /// 年份列限住了，月日照样能滚到 start 之前（travel 出发日期能选到 start 前一天）
    pub(crate) fn rebuild_date_columns(&mut self) {
        if self.date_fields == DateFields::Year || self.columns.len() < 2 {
            return;
        }
        let ((sy, sm, sd), (ey, em, ed)) = self.date_range;
        let y = self.picked_number(0).unwrap_or(sy as i64) as i32;
        let m = self.picked_number(1).unwrap_or(1) as u32;
        let (m_lo, m_hi) = (if y == sy { sm } else { 1 }, if y == ey { em } else { 12 });
        let m_hi = m_hi.max(m_lo);
        let m = m.clamp(m_lo, m_hi);
        self.columns[1] = (m_lo..=m_hi).map(|v| format!("{}月", v)).collect();
        self.selected[1] = (m - m_lo) as usize;
        if self.date_fields != DateFields::Day || self.columns.len() < 3 {
            return;
        }
        let d = self.picked_number(2).unwrap_or(1) as u32;
        let days = days_in_month(y, m);
        let d_lo = if y == sy && m == sm { sd.min(days) } else { 1 };
        let d_hi = if y == ey && m == em { ed.min(days) } else { days }.max(d_lo);
        let d = d.clamp(d_lo, d_hi);
        self.columns[2] = (d_lo..=d_hi).map(|v| format!("{}日", v)).collect();
        self.selected[2] = (d - d_lo) as usize;
    }

    /// 按当前选择算出 `bindchange` 的 `detail.value`，类型与微信一致：
    /// `selector` 是数字下标、`multiSelector` 是下标数组、
    /// `time`/`date` 是字符串、`region` 是名称数组。
    ///
    /// 返回 `serde_json::Value` 而不是拼好的字符串 —— 拼字符串会踩两个坑：
    /// `time` 的 `09:05` 塞进 JS 字面量是语法错误，
    /// `date` 的 `2024-03-15` 会被当成算术表达式算成 2006。
    pub fn change_value(&self) -> serde_json::Value {
        use serde_json::json;
        match self.mode {
            SheetMode::Selector => json!(self.selected.first().copied().unwrap_or(0)),
            SheetMode::MultiSelector => json!(self.selected),
            SheetMode::Time => json!(format!(
                "{:02}:{:02}",
                self.picked_number(0).unwrap_or(0),
                self.picked_number(1).unwrap_or(0)
            )),
            SheetMode::Date => {
                let y = self.picked_number(0).unwrap_or(1970);
                json!(match self.date_fields {
                    DateFields::Year => format!("{:04}", y),
                    DateFields::Month => format!("{:04}-{:02}", y, self.picked_number(1).unwrap_or(1)),
                    DateFields::Day => format!(
                        "{:04}-{:02}-{:02}",
                        y,
                        self.picked_number(1).unwrap_or(1),
                        self.picked_number(2).unwrap_or(1)
                    ),
                })
            }
            SheetMode::Region => json!(self.picked_labels()),
        }
    }

    /// 取第 col 列当前选中项里的数字（去掉「年/月/日/时/分」后缀）
    fn picked_number(&self, col: usize) -> Option<i64> {
        let items = self.columns.get(col)?;
        let idx = self.selected.get(col).copied().unwrap_or(0);
        let s = items.get(idx)?;
        s.trim_end_matches(|c: char| !c.is_ascii_digit()).parse().ok()
    }

    /// 选中项的显示文本（供日志/测试）
    pub fn picked_labels(&self) -> Vec<String> {
        self.columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                col.get(self.selected.get(i).copied().unwrap_or(0))
                    .cloned()
                    .unwrap_or_default()
            })
            .collect()
    }
}

/// 某年某月的天数
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
            if leap { 29 } else { 28 }
        }
        _ => 30,
    }
}

// ────────────────────────────── 命中 ──────────────────────────────

/// 面板在视口里的顶边（含动画位移）
pub fn sheet_top(state: &PickerSheetState) -> f32 {
    state.viewport.1 - SHEET_H + state.slide_offset()
}

/// 遮罩不透明度低于这个值就不画（入场第一帧 / 退场最后一帧）
pub const MASK_VISIBLE_MIN: f32 = 0.002;

/// 遮罩的不透明度：随面板滑入从 0 淡到 0.5，退场反过来。
/// 绘制与「宿主原生组件被遮罩压暗多少」（`host::native_occlusion`）共用这一份。
pub fn mask_alpha(state: &PickerSheetState) -> f32 {
    let slide_ratio = 1.0 - state.slide_offset() / SHEET_H;
    0.5 * slide_ratio.clamp(0.0, 1.0)
}
