//! 从 WXML 的 picker 绑定构造面板状态，以及各 mode 的初值解析
//!
//! `picker_sheet` 的一片。**纯搬迁**：从 875 行按职责切开，一行逻辑没改。
use super::region::{region_columns, region_names};
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
            let (h, m) = parse_hh_mm(&binding.value).unwrap_or_else(current_hh_mm);
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
            let start_year = parse_ymd(&binding.start).map(|(v, ..)| v).unwrap_or((y - 60).min(cy - 60));
            let end_year = parse_ymd(&binding.end).map(|(v, ..)| v).unwrap_or((y + 60).max(cy + 60));
            let end_year = end_year.max(start_year);
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
            region_columns(&picked)
        }
    };

    PickerSheetState {
        id: binding.id.clone(),
        mode,
        columns,
        selected,
        handler: binding.change_handler.clone(),
        visible: true,
        pressed: None,
        anim_from: Instant::now(),
        closing: false,
        date_fields,
    }
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

/// 当前 UTC 日期。没引入 chrono，用 Howard Hinnant 的 civil-from-days 算法从
/// Unix 时间戳直接换算（只用于 date/time picker 的默认值，时区差一小时无妨）。
pub fn current_ymd() -> (i32, u32, u32) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
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

fn current_hh_mm() -> (u32, u32) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let sod = secs.rem_euclid(86_400);
    ((sod / 3600) as u32, ((sod % 3600) / 60) as u32)
}
