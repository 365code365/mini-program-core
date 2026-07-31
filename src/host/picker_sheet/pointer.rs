//! 面板的指针闭环：命中即打开、按下选中滚轮、抬起提交、动画收尾
//!
//! `picker_sheet` 的一片。**纯搬迁**：从 875 行按职责切开，一行逻辑没改。
use super::*;

/// 逻辑视口坐标 -> 面板上的可按对象
pub fn hit_test(state: &PickerSheetState, x: f32, y: f32) -> SheetHit {
    let top = sheet_top(state);
    if y < top {
        return SheetHit::Mask;
    }
    if y < top + HEADER_H {
        // 头部：左半「取消」右半「确定」，中间空白不响应（按微信是两个独立按钮）
        return if x < LOGICAL_WIDTH as f32 / 2.0 {
            SheetHit::Cancel
        } else {
            SheetHit::Confirm
        };
    }
    let cols = state.columns.len().max(1);
    let col_w = LOGICAL_WIDTH as f32 / cols as f32;
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
    let binding = renderer.and_then(|r| {
        r.picker_hit(x, y, true)
            .or_else(|| {
                if on_fixed_layer {
                    None
                } else {
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
    Some(build(&binding))
}

/// 面板可见时的按下：记录按压的头部按钮以给出反馈。返回是否消费了事件。
pub fn on_press(sheet: &mut Option<PickerSheetState>, x: f32, y: f32) -> bool {
    let Some(sheet) = sheet.as_mut().filter(|s| s.visible) else {
        return false;
    };
    let hit = hit_test(sheet, x, y);
    sheet.pressed = match hit {
        SheetHit::Cancel | SheetHit::Confirm => Some(hit),
        _ => None,
    };
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
        return true;
    }
    match hit_test(sheet, x, y) {
        SheetHit::Confirm => {
            let value = sheet.change_value();
            let handler = sheet.handler.clone();
            let labels = sheet.picked_labels().join(" ");
            sheet.begin_close();
            if let Some(handler) = handler {
                // `__callPageMethod` 的第二个参数就是 dataset，事件对象里 `detail` 直接指向它。
                // 多包一层 `{detail:{...}}` 会让页面拿到 `e.detail.detail.value`，
                // 于是 `e.detail.value` 是 undefined —— 选了「选项二」按确定却没反应就是这个。
                let payload = serde_json::json!({ "value": value });
                println!("👆 picker 确定 -> {handler} value={value} ({labels})");
                app.eval(&format!("__callPageMethod('{handler}', {payload})")).ok();
            }
        }
        SheetHit::Cancel | SheetHit::Mask => sheet.begin_close(),
        SheetHit::Item { col, delta } => {
            sheet.move_selection(col, delta);
            if delta != 0 {
                relink_region(sheet, col);
            }
        }
    }
    true
}

/// 退场动画走完后丢弃面板。返回是否真的丢了（宿主据此标脏重绘）。
pub fn reap(sheet: &mut Option<PickerSheetState>) -> bool {
    if sheet.as_ref().map(|s| s.finished_closing()).unwrap_or(false) {
        *sheet = None;
        return true;
    }
    false
}
