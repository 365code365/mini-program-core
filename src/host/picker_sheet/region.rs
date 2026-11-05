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

/// 按已选省/市造出三列，并定位到当前选择（没有 custom-item、level=region；单测用）
#[cfg(test)]
pub(super) fn region_columns(picked: &[String]) -> (Vec<Vec<String>>, Vec<usize>) {
    region_columns_with(picked, &SheetExtra::default())
}

/// 某一列的「真实下标」：有 custom-item 时第 0 项是它（None），其余往后挪一位
fn real(i: usize, custom: bool) -> Option<usize> {
    if custom { i.checked_sub(1) } else { Some(i) }
}

/// 第 `col` 列的选项：父级选的是 custom-item（`None`）时这一列只剩 custom-item（官方：「可为每一列的顶部添加一个自定义的项」，
/// 微信选了它之后后面的列也只剩它）
fn column(col: usize, pi: Option<usize>, ci: Option<usize>, custom: &Option<String>) -> Vec<String> {
    use crate::host::region_data::{cities_of, districts_of, provinces};
    let mut out: Vec<String> = custom.iter().cloned().collect();
    let items = match col {
        0 => provinces().iter().map(|p| p.name.clone()).collect(),
        1 => pi.map(cities_of).unwrap_or_default(),
        _ => match (pi, ci) {
            (Some(p), Some(c)) => districts_of(p, c),
            _ => Vec::new(),
        },
    };
    if col > 0 && custom.is_some() && (pi.is_none() || (col == 2 && ci.is_none())) {
        return out;
    }
    out.extend(items);
    out
}

/// 同上，带 custom-item / level（列数 = level）
pub(super) fn region_columns_with(picked: &[String], extra: &SheetExtra) -> (Vec<Vec<String>>, Vec<usize>) {
    let custom = &extra.region_custom;
    let level = extra.region_level.clamp(1, 3);
    let find = |col: &Vec<String>, k: usize| picked.get(k).and_then(|name| col.iter().position(|p| p == name)).unwrap_or(0);
    let c0 = column(0, None, None, custom);
    let s0 = find(&c0, 0);
    let mut cols = vec![c0];
    let mut sel = vec![s0];
    let pi = real(s0, custom.is_some());
    if level >= 2 {
        let c1 = column(1, pi, None, custom);
        let s1 = find(&c1, 1);
        let ci = real(s1, custom.is_some());
        cols.push(c1);
        sel.push(s1);
        if level >= 3 {
            let c2 = column(2, pi, ci, custom);
            sel.push(find(&c2, 2));
            cols.push(c2);
        }
    }
    (cols, sel)
}

/// 省/市列变化后重建后续列（region 的联动）
pub fn relink_region(state: &mut PickerSheetState, changed_col: usize) {
    if state.mode != SheetMode::Region || state.columns.len() < 2 {
        return;
    }
    let custom = state.extra.region_custom.clone();
    let pi = real(state.selected[0], custom.is_some());
    if changed_col == 0 {
        state.columns[1] = column(1, pi, None, &custom);
        state.selected[1] = 0;
    }
    if changed_col <= 1 && state.columns.len() >= 3 {
        let s1 = state.selected[1].min(state.columns[1].len().saturating_sub(1));
        state.selected[1] = s1;
        state.columns[2] = column(2, pi, real(s1, custom.is_some()), &custom);
        state.selected[2] = 0;
    }
}

/// region 的 `detail.code`：每一列选中项的 6 位区划码（custom-item 给空串，待验）
pub(super) fn region_codes(state: &PickerSheetState) -> Vec<String> {
    let provinces = crate::host::region_data::provinces();
    let custom = state.extra.region_custom.is_some();
    let pick = |k: usize| state.selected.get(k).and_then(|&i| real(i, custom));
    let p = pick(0).and_then(|i| provinces.get(i));
    let c = p.and_then(|p| pick(1).and_then(|i| p.cities.get(i)));
    let d = c.and_then(|c| pick(2).and_then(|i| c.districts.get(i)));
    let codes = [p.map(|p| p.code.clone()), c.map(|c| c.code.clone()), d.map(|d| d.0.clone())];
    codes.into_iter().take(state.columns.len()).map(|c| c.unwrap_or_default()).collect()
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
            column_handler: None,
            cancel_handler: None,
            header_text: String::new(),
            column_changes: Vec::new(),
            origin: None,
            outbox: Vec::new(),
            visible: true,
            pressed: None,
            // 起点放到过去 → slide_offset 归零，面板完全展开，命中坐标稳定
            anim_from: Instant::now() - std::time::Duration::from_secs(1),
            closing: false,
            date_fields: DateFields::Day,
            date_range: ((1970, 1, 1), (2100, 12, 31)),
            viewport: (LOGICAL_WIDTH as f32, LOGICAL_HEIGHT as f32),
            wheel: WheelState::default(),
            extra: SheetExtra::default(),
        }
    }

    fn cols(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn region(picked: &[&str], extra: SheetExtra) -> PickerSheetState {
        let (c, s) = region_columns_with(&cols(picked), &extra);
        let mut st = open_sheet(SheetMode::Region, c, s);
        st.extra = extra;
        st
    }

    /// B3：完整省市区 + 6 位区划码；custom-item 在每列顶部、选了它后面的列只剩它；level 决定列数
    #[test]
    fn b3_region_完整数据_code_custom_item_level() {
        let s = region(&["广东省", "广州市", "海珠区"], SheetExtra::default());
        assert_eq!(s.change_value(), serde_json::json!(["广东省", "广州市", "海珠区"]));
        assert_eq!(region_codes(&s), ["440000", "440100", "440105"]);
        let mut c = region(&["广东省", "广州市", "海珠区"], SheetExtra { region_custom: Some("全部".into()), ..Default::default() });
        assert_eq!(c.columns.iter().map(|col| col[0].as_str()).collect::<Vec<_>>(), ["全部", "全部", "全部"]);
        assert_eq!(c.change_value(), serde_json::json!(["广东省", "广州市", "海珠区"]), "custom-item 不打乱按名字的定位");
        c.move_selection(1, -100);
        relink_region(&mut c, 1);
        assert_eq!(c.columns[2], ["全部"], "市选了 custom-item，区这一列只剩它");
        assert_eq!(region_codes(&c), ["440000", "", ""]);
        let p = region(&["北京市"], SheetExtra { region_level: 1, ..Default::default() });
        assert_eq!(p.columns.len(), 1);
        assert_eq!(p.change_value(), serde_json::json!(["北京市"]));
        let ct = region(&["北京市", "北京市"], SheetExtra { region_level: 2, ..Default::default() });
        assert_eq!(region_codes(&ct), ["110000", "110100"]);
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
