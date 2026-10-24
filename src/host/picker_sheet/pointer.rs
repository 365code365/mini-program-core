//! 面板的指针闭环：命中即打开、按下 / 拖动滚轮、抬起提交、动画收尾
//!
//! `picker_sheet` 的一片（从 875 行按职责切开）。拖动本身的算式在 `wheel.rs`，
//! 这里只做分派：按在滚轮上 → 交给滚轮；滚轮没消费的抬手 → 照旧按「点哪一行」处理。
use super::*;

/// 逻辑视口坐标 -> 面板上的可按对象
pub fn hit_test(state: &PickerSheetState, x: f32, y: f32) -> SheetHit {
    let top = sheet_top(state);
    if y < top {
        return SheetHit::Mask;
    }
    if y < top + HEADER_H {
        // 头部：左半「取消」右半「确定」，中间空白不响应（按微信是两个独立按钮）
        return if x < state.viewport.0 / 2.0 {
            SheetHit::Cancel
        } else {
            SheetHit::Confirm
        };
    }
    let cols = state.columns.len().max(1);
    let col_w = state.viewport.0 / cols as f32;
    let col = ((x / col_w) as usize).min(cols - 1);
    let wheel_top = top + HEADER_H;
    let center_row = VISIBLE_ROWS / 2;
    let row = ((y - wheel_top) / ITEM_H).floor() as i32;
    SheetHit::Item { col, delta: row - center_row as i32 }
}

// ────────────────────────────── 绘制 ──────────────────────────────

/// 点击落在某个 `<picker>` 上时构造面板状态。返回 `None` 表示这一点不是 picker。
///
/// 覆盖层上的 picker 用**视口**坐标，正常流的用内容坐标（y + 滚动量）——
/// 两套坐标混用会出现「弹窗里的 picker 点不开 / 页面滚过之后点空气能弹出面板」。
pub fn open_if_hit(
    renderer: Option<&crate::renderer::WxmlRenderer>,
    scroll_pos: f32,
    x: f32,
    y: f32,
) -> Option<PickerSheetState> {
    let on_fixed_layer = renderer.map(|r| r.fixed_layer_hit(x, y)).unwrap_or(false);
    let mut origin = None;
    let binding = renderer.and_then(|r| {
        r.picker_hit(x, y, true)
            .map(|b| { origin = Some((x, y, true)); b })
            .or_else(|| {
                if on_fixed_layer {
                    None
                } else {
                    origin = Some((x, y + scroll_pos, false));
                    r.picker_hit(x, y + scroll_pos, false)
                }
            })
            .cloned()
    });
    let Some(binding) = binding else {
        if std::env::var("MINI_PICKER_LOG").is_ok() {
            if let Some(r) = renderer {
                eprintln!(
                    "PICKER miss @({x},{y}) scroll={scroll_pos} regions={:?}",
                    r.picker_regions()
                        .iter()
                        .map(|p| (p.id.clone(), p.bounds))
                        .collect::<Vec<_>>()
                );
            }
        }
        return None;
    };
    if std::env::var("MINI_PICKER_LOG").is_ok() {
        eprintln!("👆 picker -> 打开选择面板 mode={}", binding.mode);
    }
    let mut sheet = build(&binding);
    sheet.origin = origin;
    Some(sheet)
}

/// 面板可见时的按下：记录按压的头部按钮以给出反馈；按在滚轮上就让那一列准备跟手。
/// 返回是否消费了事件。
pub fn on_press(sheet: &mut Option<PickerSheetState>, x: f32, y: f32) -> bool {
    let Some(sheet) = sheet.as_mut().filter(|s| s.visible) else {
        return false;
    };
    let hit = hit_test(sheet, x, y);
    sheet.pressed = match hit {
        SheetHit::Cancel | SheetHit::Confirm => Some(hit.clone()),
        _ => None,
    };
    match hit {
        SheetHit::Item { col, .. } if !sheet.closing => sheet.wheel_press(col, y, Instant::now()),
        // 按在头部 / 遮罩上：上一次没收尾的拖动（理论上不会有）作废
        _ => sheet.wheel_cancel(Instant::now()),
    }
    true
}

/// [`on_move`] 的结果
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SheetMove {
    /// 没有可见的面板：事件归页面
    NotOpen,
    /// 面板吃掉了事件，但画面没变（没按在滚轮上、还没过拖动阈值、或只是鼠标悬停）
    Idle,
    /// 滚轮跟手动了：要重画
    Moved,
}

/// 面板可见时的移动：按在滚轮上的那一列跟着手指上下走。
///
/// 面板可见时**一律吞掉**移动（返回 `Idle` / `Moved`）：面板与遮罩盖住整屏，
/// 拖动不能漏到底下去滚页面。列在按下时就定了，横向漂移不换列（与微信一致）。
pub fn on_move(sheet: &mut Option<PickerSheetState>, _x: f32, y: f32) -> SheetMove {
    let Some(sheet) = sheet.as_mut().filter(|s| s.visible) else {
        return SheetMove::NotOpen;
    };
    if sheet.closing {
        return SheetMove::Idle;
    }
    if sheet.wheel_move(y, Instant::now()) {
        SheetMove::Moved
    } else {
        SheetMove::Idle
    }
}

/// 手势被系统打断（来电、系统返回手势接管）：滚轮就近落行。返回是否消费了事件。
pub fn on_cancel(sheet: &mut Option<PickerSheetState>) -> bool {
    let Some(sheet) = sheet.as_mut().filter(|s| s.visible) else {
        return false;
    };
    sheet.pressed = None;
    sheet.wheel_cancel(Instant::now());
    true
}

/// 面板可见时的抬手：确定 / 取消 / 选项滚动 / 点遮罩关闭。返回是否消费了事件。
pub fn on_release(
    sheet: &mut Option<PickerSheetState>,
    app: &mut crate::runtime::MiniApp,
    x: f32,
    y: f32,
) -> bool {
    let Some(sheet) = sheet.as_mut().filter(|s| s.visible) else {
        return false;
    };
    sheet.pressed = None;
    // 退场动画进行中时，只吞事件不再响应
    if sheet.closing {
        sheet.wheel_cancel(Instant::now());
        return true;
    }
    // 拖过（松手按速度甩出去再吸附）或按住停下了回正：这一下不再当成「点某一行」
    if sheet.wheel_release(y, Instant::now()) {
        flush_column_changes(sheet, app);
        return true;
    }
    match hit_test(sheet, x, y) {
        SheetHit::Confirm => {
            let value = sheet.change_value();
            let labels = sheet.picked_labels().join(" ");
            sheet.begin_close();
            println!("👆 picker 确定 value={value} ({labels})");
            let detail = super::emit::change_detail(sheet, value);
            sheet.outbox.push(("change", detail));
        }
        SheetHit::Cancel | SheetHit::Mask => {
            sheet.begin_close();
            // bindcancel：取消、点遮罩收起都算（微信语义）
            sheet.outbox.push(("cancel", serde_json::json!({})));
        }
        SheetHit::Item { col, delta } => {
            // 点选是直接跳到那一行（不做滚动动画）；这一列要是还在回正，先掐掉，
            // 否则动画走完那一下会从旧落点跳回来
            sheet.wheel_stop(col);
            sheet.move_selection(col, delta);
            if delta != 0 {
                relink_region(sheet, col);
            }
        }
    }
    flush_column_changes(sheet, app);
    true
}

/// 把这次抬手里落定的列变化排给页面：`bindcolumnchange` 的 `detail = {column, value}`，都是数字
fn flush_column_changes(sheet: &mut PickerSheetState, _app: &mut crate::runtime::MiniApp) {
    for (column, value) in std::mem::take(&mut sheet.column_changes) {
        sheet.outbox.push(("columnchange", serde_json::json!({ "column": column, "value": value })));
    }
}

/// 退场动画走完后丢弃面板。返回是否真的丢了（宿主据此标脏重绘）。
pub fn reap(sheet: &mut Option<PickerSheetState>) -> bool {
    if sheet.as_ref().map(|s| s.finished_closing()).unwrap_or(false) {
        *sheet = None;
        return true;
    }
    false
}
