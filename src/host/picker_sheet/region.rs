//! 省市区三列联动：换省要重算市、换市要重算区
//!
//! `picker_sheet` 的一片。**纯搬迁**：从 875 行按职责切开，一行逻辑没改。
use super::*;

/// `['广东省','广州市','天河区']` -> Vec<String>
pub(super) fn region_names(s: &str) -> Vec<String> {
    s.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|p| p.trim().trim_matches('\'').trim_matches('"').to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// 按已选省/市造出三列，并定位到当前选择
pub(super) fn region_columns(picked: &[String]) -> (Vec<Vec<String>>, Vec<usize>) {
    let provinces: Vec<String> = crate::host::region_data::PROVINCES.iter().map(|(p, _)| p.to_string()).collect();
    let pi = picked
        .first()
        .and_then(|name| provinces.iter().position(|p| p == name))
        .unwrap_or(0);
    let cities: Vec<String> = crate::host::region_data::cities_of(pi);
    let ci = picked
        .get(1)
        .and_then(|name| cities.iter().position(|c| c == name))
        .unwrap_or(0);
    let districts = crate::host::region_data::districts_of(pi, ci);
    let di = picked
        .get(2)
        .and_then(|name| districts.iter().position(|d| d == name))
        .unwrap_or(0);
    (vec![provinces, cities, districts], vec![pi, ci, di])
}

/// 省/市列变化后重建后续列（region 的联动）
pub fn relink_region(state: &mut PickerSheetState, changed_col: usize) {
    if state.mode != SheetMode::Region || state.columns.len() < 3 {
        return;
    }
    let pi = state.selected[0];
    if changed_col == 0 {
        state.columns[1] = crate::host::region_data::cities_of(pi);
        state.selected[1] = 0;
    }
    if changed_col <= 1 {
        let ci = state.selected[1].min(state.columns[1].len().saturating_sub(1));
        state.selected[1] = ci;
        state.columns[2] = crate::host::region_data::districts_of(pi, ci);
        state.selected[2] = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_sheet(mode: SheetMode, columns: Vec<Vec<String>>, selected: Vec<usize>) -> PickerSheetState {
        PickerSheetState {
            id: "p".into(),
            mode,
            columns,
            selected,
            handler: Some("onChange".into()),
            visible: true,
            pressed: None,
            // 起点放到过去 → slide_offset 归零，面板完全展开，命中坐标稳定
            anim_from: Instant::now() - std::time::Duration::from_secs(1),
            closing: false,
            date_fields: DateFields::Day,
        }
    }

    fn cols(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn selector_change_value_is_index() {
        let mut s = open_sheet(SheetMode::Selector, vec![cols(&["A", "B", "C"])], vec![0]);
        s.move_selection(0, 2);
        assert_eq!(s.selected[0], 2);
        assert_eq!(s.change_value(), serde_json::json!(2));
    }

    #[test]
    fn selector_move_clamps_at_edges() {
        let mut s = open_sheet(SheetMode::Selector, vec![cols(&["A", "B"])], vec![0]);
        s.move_selection(0, 9);
        assert_eq!(s.selected[0], 1); // 不越界
        s.move_selection(0, -9);
        assert_eq!(s.selected[0], 0);
    }

    #[test]
    fn multi_selector_change_value_is_index_array() {
        let mut s = open_sheet(
            SheetMode::MultiSelector,
            vec![cols(&["a", "b"]), cols(&["x", "y", "z"])],
            vec![0, 0],
        );
        s.move_selection(1, 2);
        assert_eq!(s.change_value(), serde_json::json!([0, 2]));
    }

    #[test]
    fn time_change_value_is_hh_mm() {
        let hours: Vec<String> = (0..24).map(|h| format!("{:02}时", h)).collect();
        let mins: Vec<String> = (0..60).map(|m| format!("{:02}分", m)).collect();
        let mut s = open_sheet(SheetMode::Time, vec![hours, mins], vec![9, 5]);
        assert_eq!(s.change_value(), serde_json::json!("09:05"));
        s.move_selection(0, 3); // 9 -> 12 时
        assert_eq!(s.change_value(), serde_json::json!("12:05"));
    }

    #[test]
    fn date_change_value_is_iso() {
        let years = vec!["2023年".to_string(), "2024年".to_string()];
        let months: Vec<String> = (1..=12).map(|m| format!("{}月", m)).collect();
        let days: Vec<String> = (1..=31).map(|d| format!("{}日", d)).collect();
        let s = open_sheet(SheetMode::Date, vec![years, months, days], vec![1, 2, 14]);
        assert_eq!(s.change_value(), serde_json::json!("2024-03-15"));
    }

    #[test]
    fn date_day_column_shrinks_for_february() {
        let years = vec!["2023年".to_string()]; // 平年
        let months: Vec<String> = (1..=12).map(|m| format!("{}月", m)).collect();
        let days: Vec<String> = (1..=31).map(|d| format!("{}日", d)).collect();
        let mut s = open_sheet(SheetMode::Date, vec![years, months, days], vec![0, 0, 30]);
        s.move_selection(1, 1); // 1月 -> 2月，触发天数重算
        assert_eq!(s.columns[2].len(), 28);
        assert_eq!(s.selected[2], 27); // 原来选的 31 号被夹到 28 号
    }

    #[test]
    fn hit_test_maps_header_and_wheel() {
        let s = open_sheet(SheetMode::Selector, vec![cols(&["A", "B", "C"])], vec![1]);
        let top = sheet_top(&s);
        // 头部左半 = 取消，右半 = 确定
        assert_eq!(hit_test(&s, 10.0, top + 10.0), SheetHit::Cancel);
        assert_eq!(hit_test(&s, LOGICAL_WIDTH as f32 - 10.0, top + 10.0), SheetHit::Confirm);
        // 遮罩
        assert_eq!(hit_test(&s, 10.0, top - 20.0), SheetHit::Mask);
        // 滚轮中心行 = delta 0
        let wheel_center = top + HEADER_H + WHEEL_H / 2.0;
        assert_eq!(hit_test(&s, 100.0, wheel_center), SheetHit::Item { col: 0, delta: 0 });
    }

    /// 回归：交给 `__callPageMethod` 的载荷必须是**扁平的** `{"value": ...}`。
    ///
    /// 那个函数把第二个参数整体当 dataset，事件对象里 `detail` 直接指向它。
    /// 从前多包了一层 `{detail:{value:..}}`，页面里 `e.detail.value` 于是是 undefined ——
    /// 选了「选项二」按确定，页面纹丝不动。
    #[test]
    fn confirm_payload_is_flat_value_object() {
        let s = open_sheet(SheetMode::Selector, vec![cols(&["A", "B", "C"])], vec![1]);
        let payload = serde_json::json!({ "value": s.change_value() });
        assert_eq!(payload, serde_json::json!({ "value": 1 }));
        // 序列化出来要是合法 JSON（同时也是合法 JS 对象字面量）
        assert_eq!(payload.to_string(), r#"{"value":1}"#);
    }

    /// `time` / `date` 的值必须是带引号的字符串。
    /// 拼字符串字面量时 `09:05` 是 JS 语法错误，`2024-03-15` 会被算成 2006。
    #[test]
    fn time_and_date_payloads_stay_quoted_strings() {
        let hours: Vec<String> = (0..24).map(|h| format!("{:02}时", h)).collect();
        let mins: Vec<String> = (0..60).map(|m| format!("{:02}分", m)).collect();
        let t = open_sheet(SheetMode::Time, vec![hours, mins], vec![9, 5]);
        assert_eq!(
            serde_json::json!({ "value": t.change_value() }).to_string(),
            r#"{"value":"09:05"}"#
        );

        let years = vec!["2024年".to_string()];
        let months: Vec<String> = (1..=12).map(|m| format!("{}月", m)).collect();
        let days: Vec<String> = (1..=31).map(|d| format!("{}日", d)).collect();
        let d = open_sheet(SheetMode::Date, vec![years, months, days], vec![0, 2, 14]);
        assert_eq!(
            serde_json::json!({ "value": d.change_value() }).to_string(),
            r#"{"value":"2024-03-15"}"#
        );
    }

    #[test]
    fn confirm_value_after_wheel_tap() {
        // 模拟：点中心下一行 → 选择 +1 → 确定值随之更新
        let mut s = open_sheet(SheetMode::Selector, vec![cols(&["A", "B", "C"])], vec![0]);
        let top = sheet_top(&s);
        let one_below = top + HEADER_H + WHEEL_H / 2.0 + ITEM_H;
        if let SheetHit::Item { col, delta } = hit_test(&s, 100.0, one_below) {
            s.move_selection(col, delta);
        }
        assert_eq!(s.change_value(), serde_json::json!(1));
    }
}

// ───────────────────── 面板的输入处理（两端共用） ─────────────────────
//
// 从桌面窗体搬进来：弹面板、按下反馈、确定/取消/滚选/点遮罩关闭，全是纯状态迁移，
// 与窗口系统无关。移动端 SDK 从前**完全没接** picker —— 点一下 `<picker>` 什么都不
// 发生（`engine.rs` 的模块注释里那句「picker 面板还没接进来」就是指这个）。
