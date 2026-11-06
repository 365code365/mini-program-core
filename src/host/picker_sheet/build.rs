//! 从 WXML 的 picker 绑定构造面板状态，以及各 mode 的初值解析
//!
//! `picker_sheet` 的一片。**纯搬迁**：从 875 行按职责切开，一行逻辑没改。
use super::region::{region_columns_with, region_names};
use super::*;

/// 从页面里的一个 `<picker>` 造出面板状态。
///
/// 各模式的列来源：
/// - `selector` / `multiSelector`：直接用 `range`（后者是二维数组）
/// - `time`：时(0-23) + 分(0-59)，`value` 形如 `"12:30"`
/// - `date`：年/月/日，`start` / `end` 限定年份，`fields` 决定显示到哪一级
/// - `region`：省/市/区，数据来自内置表（见 `region_data`）
pub fn build(binding: &PickerBinding) -> PickerSheetState {
    let mode = match binding.mode.as_str() {
        "multiSelector" => SheetMode::MultiSelector,
        "time" => SheetMode::Time,
        "date" => SheetMode::Date,
        "region" => SheetMode::Region,
        _ => SheetMode::Selector,
    };
    let mut date_fields = DateFields::Day;
    let mut date_range = ((1970, 1, 1), (2100, 12, 31));
    let mut extra = SheetExtra::default();
    let (columns, selected) = match mode {
        SheetMode::Selector => {
            let col = binding.range.clone();
            let sel = binding
                .value
                .trim()
                .parse::<usize>()
                .unwrap_or(0)
                .min(col.len().saturating_sub(1));
            (vec![col], vec![sel])
        }
        SheetMode::MultiSelector => {
            let cols = binding.multi_range.clone();
            let picked = parse_index_list(&binding.value);
            let sel = cols
                .iter()
                .enumerate()
                .map(|(i, c)| picked.get(i).copied().unwrap_or(0).min(c.len().saturating_sub(1)))
                .collect();
            (cols, sel)
        }
        SheetMode::Time => {
            let hours: Vec<String> = (0..24).map(|h| format!("{:02}时", h)).collect();
            let mins: Vec<String> = (0..60).map(|m| format!("{:02}分", m)).collect();
            // 没写 value：当前时刻（本地时区）
            let (h, m) = parse_hh_mm(&binding.value).unwrap_or_else(current_hh_mm);
            // start / end（官方：「有效时间范围的开始 / 结束，hh:mm」）：列按范围裁、初值夹进范围（下面 rebuild）
            let lo = parse_hh_mm(&binding.start).unwrap_or((0, 0));
            let hi = parse_hh_mm(&binding.end).unwrap_or((23, 59)).max(lo);
            extra.time_range = (lo, hi);
            (vec![hours, mins], vec![h as usize, m as usize])
        }
        SheetMode::Date => {
            date_fields = match binding.fields.as_str() {
                "year" => DateFields::Year,
                "month" => DateFields::Month,
                _ => DateFields::Day,
            };
            let (cy, cm, cd) = current_ymd();
            let (y, m, d) = parse_ymd(&binding.value).unwrap_or((cy, cm, cd));
            let start = parse_ymd(&binding.start).unwrap_or(((y - 60).min(cy - 60), 1, 1));
            // `end="2026"` / `"2026-05"`：缺的月、日按那一年 / 那个月的最后一天算（含 end 当天）
            let mut end = parse_ymd_end(&binding.end).unwrap_or(((y + 60).max(cy + 60), 12, 31));
            if end < start {
                end = start;
            }
            date_range = (start, end);
            let (start_year, end_year) = (start.0, end.0);
            let years: Vec<String> = (start_year..=end_year).map(|v| format!("{}年", v)).collect();
            let yi = (y - start_year).clamp(0, years.len() as i32 - 1) as usize;
            let mut cols = vec![years];
            let mut sel = vec![yi];
            if date_fields != DateFields::Year {
                cols.push((1..=12).map(|v| format!("{}月", v)).collect());
                sel.push((m as usize).saturating_sub(1).min(11));
            }
            if date_fields == DateFields::Day {
                let days = days_in_month(y, m);
                cols.push((1..=days).map(|v| format!("{}日", v)).collect());
                sel.push((d as usize).saturating_sub(1).min(days as usize - 1));
            }
            (cols, sel)
        }
        SheetMode::Region => {
            let picked = region_names(&binding.value);
            extra.region_custom = Some(binding.custom_item.trim().to_string()).filter(|s| !s.is_empty());
            extra.region_level = match binding.level.as_str() {
                "province" => 1,
                "city" => 2,
                // region（默认）与 sub-district（街道没有数据，按三列）
                _ => 3,
            };
            region_columns_with(&picked, &extra)
        }
    };

    let mut st = PickerSheetState {
        id: binding.id.clone(),
        mode,
        columns,
        selected,
        handler: binding.change_handler.clone(),
        column_handler: binding.column_handler.clone(),
        cancel_handler: binding.cancel_handler.clone(),
        header_text: binding.header_text.clone(),
        column_changes: Vec::new(),
        origin: None,
        outbox: Vec::new(),
        visible: true,
        pressed: None,
        anim_from: crate::sys::frame_clock::now(),
        closing: false,
        date_fields,
        date_range,
        viewport: (LOGICAL_WIDTH as f32, LOGICAL_HEIGHT as f32),
        wheel: WheelState::default(),
        extra,
    };
    // 初值落在 start / end 之外时夹回范围里，月、日（分钟）两列按范围裁
    if mode == SheetMode::Date {
        st.rebuild_date_columns();
    }
    if mode == SheetMode::Time {
        st.rebuild_time_columns();
    }
    st
}

/// `[0,2]` / `[0, 2]` / `'[0,2]'` -> `[0, 2]`
fn parse_index_list(s: &str) -> Vec<usize> {
    s.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .filter_map(|p| p.trim().trim_matches('\'').trim_matches('"').parse().ok())
        .collect()
}

fn parse_hh_mm(s: &str) -> Option<(u32, u32)> {
    let mut it = s.trim().split(':');
    let h: u32 = it.next()?.trim().parse().ok()?;
    let m: u32 = it.next()?.trim().parse().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some((h, m))
}

fn parse_ymd(s: &str) -> Option<(i32, u32, u32)> {
    let mut it = s.trim().split('-');
    let y: i32 = it.next()?.trim().parse().ok()?;
    let m: u32 = it.next().and_then(|v| v.trim().parse().ok()).unwrap_or(1);
    let d: u32 = it.next().and_then(|v| v.trim().parse().ok()).unwrap_or(1);
    if !(1..=12).contains(&m) {
        return None;
    }
    Some((y, m, d.max(1)))
}

/// `end` 的写法可以只到年或月：缺的部分取那一段的最后一天
fn parse_ymd_end(s: &str) -> Option<(i32, u32, u32)> {
    let parts = s.trim().split('-').count();
    let (y, m, d) = parse_ymd(s)?;
    Some(match parts {
        1 => (y, 12, 31),
        2 => (y, m, super::days_in_month(y, m)),
        _ => (y, m, d),
    })
}

/// 本地时区相对 UTC 的秒数（东八区 +28800）。Unix（含 Android / iOS）用 `localtime_r` 的 `tm_gmtoff`，
/// 其它平台给 0（按 UTC）
fn local_offset_secs(utc_secs: i64) -> i64 {
    #[cfg(unix)]
    {
        let t = utc_secs as libc::time_t;
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        // SAFETY: localtime_r 只写我们给的 tm，t 是合法的 time_t
        if !unsafe { libc::localtime_r(&t, &mut tm) }.is_null() {
            return tm.tm_gmtoff as i64;
        }
    }
    let _ = utc_secs;
    0
}

/// 现在的本地时间戳（秒，已加上时区偏移）
fn local_now_secs() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    secs + local_offset_secs(secs)
}

/// 当前**本地**日期（date picker 的默认值，官方：「当天」）。没引入 chrono，用 Howard Hinnant 的
/// civil-from-days 算法从时间戳换算。从前按 UTC：东八区零点到八点之间默认值是昨天
pub fn current_ymd() -> (i32, u32, u32) {
    ymd_of(local_now_secs())
}

/// 时间戳（秒，本地）→ 年月日
fn ymd_of(secs: i64) -> (i32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((if m <= 2 { y + 1 } else { y }) as i32, m, d)
}

/// 当前本地时刻（time picker 没写 value 时的默认值）
fn current_hh_mm() -> (u32, u32) {
    let sod = local_now_secs().rem_euclid(86_400);
    ((sod / 3600) as u32, ((sod % 3600) / 60) as u32)
}

#[cfg(test)]
mod date_range_tests {
    use super::*;
    fn date(value: &str, start: &str, end: &str) -> PickerSheetState {
        build(&PickerBinding {
            id: "p".into(),
            bounds: crate::Rect::new(0.0, 0.0, 10.0, 10.0),
            mode: "date".into(),
            range: vec![],
            multi_range: vec![],
            value: value.into(),
            fields: "day".into(),
            start: start.into(),
            end: end.into(),
            change_handler: None,
            column_handler: None,
            cancel_handler: None,
            header_text: String::new(),
            custom_item: String::new(),
            level: String::new(),
            disabled: false,
            is_fixed: false,
        })
    }
    /// 起始年只从起始月起、起始月只从起始日起；往前滚滚不出 start
    #[test]
    fn start_裁掉起始年月之前的月日() {
        let mut s = date("2026-06-10", "2026-06-05", "2026-08-20");
        assert_eq!(s.columns[1], ["6月", "7月", "8月"]);
        assert_eq!(s.columns[2].first().map(String::as_str), Some("5日"));
        s.move_selection(2, -100);
        assert_eq!(s.change_value(), "2026-06-05");
        // 换到结束月：日只到 20
        s.move_selection(1, 2);
        assert_eq!(s.columns[2].last().map(String::as_str), Some("20日"));
        assert_eq!(s.change_value(), "2026-08-05");
        s.move_selection(2, 100);
        assert_eq!(s.change_value(), "2026-08-20");
    }
    /// 初值在范围外：夹到 start（微信同样落在边界上）
    #[test]
    fn 初值早于_start_落在_start() {
        let s = date("2026-01-01", "2026-06-05", "2027-01-31");
        assert_eq!(s.change_value(), "2026-06-05");
    }
    /// `end` 只写到年：含那一年 12 月 31 日
    #[test]
    fn end_只写年份() {
        let mut s = date("2026-12-01", "2026-01-01", "2026");
        s.move_selection(2, 100);
        assert_eq!(s.change_value(), "2026-12-31");
    }
}

#[cfg(test)]
mod wpb_tests {
    use super::*;
    fn binding(mode: &str, value: &str, start: &str, end: &str) -> PickerBinding {
        PickerBinding {
            id: "p".into(),
            bounds: crate::Rect::new(0.0, 0.0, 10.0, 10.0),
            mode: mode.into(),
            range: vec![],
            multi_range: vec![],
            value: value.into(),
            fields: String::new(),
            start: start.into(),
            end: end.into(),
            change_handler: None,
            column_handler: None,
            cancel_handler: None,
            header_text: String::new(),
            custom_item: String::new(),
            level: String::new(),
            disabled: false,
            is_fixed: false,
        }
    }

    /// B3：time 的 start / end 裁时列与起止小时的分钟列，初值夹进范围，值仍是 "hh:mm" 字符串
    #[test]
    fn b3_time_start_end_裁列与夹初值() {
        let mut s = build(&binding("time", "08:00", "09:01", "21:01"));
        assert_eq!(s.columns[0].first().map(String::as_str), Some("09时"));
        assert_eq!(s.columns[0].last().map(String::as_str), Some("21时"));
        assert_eq!(s.columns[1].first().map(String::as_str), Some("01分"), "起始小时从起始分钟起");
        assert_eq!(s.change_value(), serde_json::json!("09:01"), "初值早于 start 落在 start");
        s.move_selection(0, 100);
        assert_eq!(s.columns[1].last().map(String::as_str), Some("01分"), "结束小时只到结束分钟");
        assert_eq!(s.change_value(), serde_json::json!("21:01"));
        s.move_selection(0, -1);
        assert_eq!(s.columns[1].len(), 60, "中间的小时 00~59");
    }

    /// B3：没写 value 时 date / time 的默认值按本地时区（与系统 `date` 命令同一天）
    #[cfg(unix)]
    #[test]
    fn b3_date_默认值是本地的今天() {
        let out = std::process::Command::new("date").arg("+%Y-%m-%d").output().expect("date");
        let today = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let s = build(&binding("date", "", "", ""));
        assert_eq!(s.change_value(), serde_json::json!(today));
    }
}
