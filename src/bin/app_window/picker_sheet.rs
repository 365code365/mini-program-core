//! `<picker>` 的原生选择面板（底部弹出的滚轮）。
//!
//! 微信里 picker 不是页面里的一段 DOM，而是宿主弹出的原生浮层：点一下 picker
//! 弹出底部面板，选好按「确定」才回调 `bindchange`，点「取消」或遮罩直接关闭。
//! 这里就是那一层：状态 + 绘制 + 命中，全部走视口坐标（不受页面滚动影响）。
//!
//! 面板画在 present 之后的 buffer 上（与 Toast/Modal 同一层机制），
//! 因此不参与页面的损伤区重绘，也不会被页面裁剪。

use mini_render::{Canvas, Color, Paint};
use mini_render::text::TextRenderer;
use std::time::Instant;

use super::{LOGICAL_WIDTH, LOGICAL_HEIGHT};

/// 面板各部分的逻辑尺寸（与微信目测一致）
pub const HEADER_H: f32 = 45.0;
pub const ITEM_H: f32 = 44.0;
/// 滚轮可见行数（奇数，选中项居中）
pub const VISIBLE_ROWS: usize = 5;
pub const WHEEL_H: f32 = ITEM_H * VISIBLE_ROWS as f32;
pub const SHEET_H: f32 = HEADER_H + WHEEL_H;
/// 入场/退场动画时长（秒）
const ANIM_SECS: f32 = 0.22;

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
    pub visible: bool,
    pub pressed: Option<SheetHit>,
    /// 动画起点：入场从这时开始算，退场时重置
    pub anim_from: Instant,
    /// 正在退场（动画走完由宿主丢弃）
    pub closing: bool,
    /// `date` 模式的年月日范围（用于换月后重算天数）
    pub date_fields: DateFields,
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
        let t = (self.anim_from.elapsed().as_secs_f32() / ANIM_SECS).clamp(0.0, 1.0);
        // ease-out cubic：起步快、收尾稳，和微信的弹出手感接近
        let eased = 1.0 - (1.0 - t).powi(3);
        if self.closing {
            SHEET_H * eased
        } else {
            SHEET_H * (1.0 - eased)
        }
    }

    /// 动画是否还在进行（宿主据此续帧）
    pub fn animating(&self) -> bool {
        self.anim_from.elapsed().as_secs_f32() < ANIM_SECS
    }

    /// 退场动画是否已经走完，可以丢弃了
    pub fn finished_closing(&self) -> bool {
        self.closing && !self.animating()
    }

    pub fn begin_close(&mut self) {
        if !self.closing {
            self.closing = true;
            self.anim_from = Instant::now();
            self.pressed = None;
        }
    }

    /// 把某一列的选中项移动 delta 行（点击非中心行 / 滚轮）
    pub fn move_selection(&mut self, col: usize, delta: i32) {
        let Some(items) = self.columns.get(col) else { return };
        if items.is_empty() {
            return;
        }
        let cur = self.selected.get(col).copied().unwrap_or(0) as i32;
        let next = (cur + delta).clamp(0, items.len() as i32 - 1) as usize;
        if let Some(slot) = self.selected.get_mut(col) {
            *slot = next;
        }
        // multiSelector 的联动（bindcolumnchange）与 date 的天数重算
        if self.mode == SheetMode::Date {
            self.rebuild_date_days();
        }
    }

    /// 年/月变化后重算「日」这一列（闰年、大小月）
    fn rebuild_date_days(&mut self) {
        if self.date_fields != DateFields::Day || self.columns.len() < 3 {
            return;
        }
        let year: i32 = self
            .columns
            .first()
            .and_then(|c| c.get(self.selected[0]))
            .and_then(|s| s.trim_end_matches('年').parse().ok())
            .unwrap_or(2000);
        let month: u32 = self
            .columns
            .get(1)
            .and_then(|c| c.get(self.selected[1]))
            .and_then(|s| s.trim_end_matches('月').parse().ok())
            .unwrap_or(1);
        let days = days_in_month(year, month);
        let new_col: Vec<String> = (1..=days).map(|d| format!("{}日", d)).collect();
        if self.selected[2] >= new_col.len() {
            self.selected[2] = new_col.len() - 1;
        }
        self.columns[2] = new_col;
    }

    /// 按当前选择算出 `bindchange` 的 `detail.value`（已是 JS 侧能解析的字符串形式）
    pub fn change_value(&self) -> String {
        match self.mode {
            SheetMode::Selector => self.selected.first().copied().unwrap_or(0).to_string(),
            SheetMode::MultiSelector => {
                let parts: Vec<String> = self.selected.iter().map(|v| v.to_string()).collect();
                format!("[{}]", parts.join(","))
            }
            SheetMode::Time => {
                let h = self.picked_number(0).unwrap_or(0);
                let m = self.picked_number(1).unwrap_or(0);
                format!("{:02}:{:02}", h, m)
            }
            SheetMode::Date => {
                let y = self.picked_number(0).unwrap_or(1970);
                match self.date_fields {
                    DateFields::Year => format!("{:04}", y),
                    DateFields::Month => format!("{:04}-{:02}", y, self.picked_number(1).unwrap_or(1)),
                    DateFields::Day => format!(
                        "{:04}-{:02}-{:02}",
                        y,
                        self.picked_number(1).unwrap_or(1),
                        self.picked_number(2).unwrap_or(1)
                    ),
                }
            }
            SheetMode::Region => {
                let names: Vec<String> = self
                    .columns
                    .iter()
                    .enumerate()
                    .map(|(i, col)| {
                        col.get(self.selected.get(i).copied().unwrap_or(0))
                            .cloned()
                            .unwrap_or_default()
                    })
                    .collect();
                let quoted: Vec<String> = names.iter().map(|n| format!("'{}'", n)).collect();
                format!("[{}]", quoted.join(","))
            }
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
    LOGICAL_HEIGHT as f32 - SHEET_H + state.slide_offset()
}

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

/// 把面板画到窗口 buffer 上（0xAARRGGBB，A 恒为 FF）
pub fn render(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    sf: f32,
    state: &PickerSheetState,
    text_renderer: Option<&TextRenderer>,
) {
    let slide_ratio = 1.0 - state.slide_offset() / SHEET_H;
    // 遮罩随面板一起淡入，避免「先黑一下再滑上来」
    let mask_alpha = 0.5 * slide_ratio.clamp(0.0, 1.0);
    if mask_alpha > 0.002 {
        let keep = 1.0 - mask_alpha;
        for px in buffer.iter_mut() {
            let v = *px;
            let r = (((v >> 16) & 0xFF) as f32 * keep) as u32;
            let g = (((v >> 8) & 0xFF) as f32 * keep) as u32;
            let b = ((v & 0xFF) as f32 * keep) as u32;
            *px = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        }
    }

    let top = (sheet_top(state) * sf) as i32;
    let sheet_h = (SHEET_H * sf).ceil() as i32;
    let sheet_w = width as i32;
    fill_rect(buffer, width, height, 0, top, sheet_w, sheet_h, 0xFFFF_FFFF);

    let header_h = (HEADER_H * sf) as i32;
    let font = 17.0 * sf;
    let pad = (16.0 * sf) as i32;

    // 头部按压反馈
    if let Some(hit) = &state.pressed {
        let half = sheet_w / 2;
        match hit {
            SheetHit::Cancel => fill_rect(buffer, width, height, 0, top, half, header_h, 0xFFF2_F2F2),
            SheetHit::Confirm => fill_rect(buffer, width, height, half, top, sheet_w - half, header_h, 0xFFF2_F2F2),
            _ => {}
        }
    }

    if let Some(tr) = text_renderer {
        let text_y = top + (header_h - font as i32) / 2;
        draw_text(buffer, width, height, tr, "取消", pad, text_y, font, Color::from_hex(0x888888));
        let confirm_w = tr.measure_text("确定", font) as i32;
        draw_text(
            buffer, width, height, tr, "确定",
            sheet_w - pad - confirm_w, text_y, font,
            Color::from_hex(0x576B95),
        );
    }

    // 头部分割线
    fill_rect(buffer, width, height, 0, top + header_h - 1, sheet_w, 1, 0xFFE5_E5E5);

    // 滚轮
    let wheel_top = top + header_h;
    let item_h = (ITEM_H * sf) as i32;
    let center_row = (VISIBLE_ROWS / 2) as i32;
    let cols = state.columns.len().max(1);
    let col_w = sheet_w / cols as i32;

    // 选中行的上下指示线
    let ind_top = wheel_top + center_row * item_h;
    fill_rect(buffer, width, height, 0, ind_top, sheet_w, 1, 0xFFD9_D9D9);
    fill_rect(buffer, width, height, 0, ind_top + item_h - 1, sheet_w, 1, 0xFFD9_D9D9);

    if let Some(tr) = text_renderer {
        for (ci, items) in state.columns.iter().enumerate() {
            let sel = state.selected.get(ci).copied().unwrap_or(0) as i32;
            let cx = ci as i32 * col_w;
            for row in 0..VISIBLE_ROWS as i32 {
                let idx = sel + (row - center_row);
                if idx < 0 || idx as usize >= items.len() {
                    continue;
                }
                let label = &items[idx as usize];
                let is_center = row == center_row;
                let dist = (row - center_row).abs();
                // 离中心越远越淡（模拟滚轮的透视衰减）
                let (color, size) = if is_center {
                    (Color::from_hex(0x000000), 17.0 * sf)
                } else if dist == 1 {
                    (Color::from_hex(0x707070), 16.0 * sf)
                } else {
                    (Color::from_hex(0xB0B0B0), 15.0 * sf)
                };
                let tw = tr.measure_text(label, size) as i32;
                let tx = cx + (col_w - tw) / 2;
                let ty = wheel_top + row * item_h + (item_h - size as i32) / 2;
                draw_text(buffer, width, height, tr, label, tx, ty, size, color);
            }
            // 列分隔（多列时）
            if ci + 1 < cols {
                let lx = (ci as i32 + 1) * col_w;
                fill_rect(buffer, width, height, lx, wheel_top, 1, item_h * VISIBLE_ROWS as i32, 0xFFF0_F0F0);
            }
        }
    }
}

fn fill_rect(buffer: &mut [u32], width: u32, height: u32, x: i32, y: i32, w: i32, h: i32, color: u32) {
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + w).min(width as i32);
    let y1 = (y + h).min(height as i32);
    for py in y0..y1 {
        let row = py as usize * width as usize;
        for px in x0..x1 {
            let idx = row + px as usize;
            if idx < buffer.len() {
                buffer[idx] = color;
            }
        }
    }
}

/// 文字直接混合到 buffer（与 ui_overlay 里同名函数同样的做法）
fn draw_text(
    buffer: &mut [u32],
    buf_w: u32,
    buf_h: u32,
    tr: &TextRenderer,
    text: &str,
    x: i32,
    y: i32,
    font_size: f32,
    color: Color,
) {
    if text.is_empty() {
        return;
    }
    let tw = (tr.measure_text(text, font_size) + 10.0).ceil() as u32;
    let th = (font_size * 1.5).ceil() as u32;
    if tw == 0 || th == 0 {
        return;
    }
    let mut tmp = Canvas::new(tw.max(1), th.max(1));
    tmp.clear(Color::TRANSPARENT);
    let paint = Paint::new().with_color(color);
    tr.draw_text(&mut tmp, text, 0.0, font_size * 0.85, font_size, &paint);
    let pixels = tmp.pixels();
    for py in 0..th as i32 {
        for px in 0..tw as i32 {
            let src = py as usize * tw as usize + px as usize;
            let dx = x + px;
            let dy = y + py;
            if dx < 0 || dy < 0 || dx >= buf_w as i32 || dy >= buf_h as i32 {
                continue;
            }
            let dst = dy as usize * buf_w as usize + dx as usize;
            if dst >= buffer.len() || src >= pixels.len() {
                continue;
            }
            let p = pixels[src];
            if p.a == 0 {
                continue;
            }
            let a = p.a as f32 / 255.0;
            let inv = 1.0 - a;
            let bg = buffer[dst];
            let r = (p.r as f32 * a + ((bg >> 16) & 0xFF) as f32 * inv) as u32;
            let g = (p.g as f32 * a + ((bg >> 8) & 0xFF) as f32 * inv) as u32;
            let b = (p.b as f32 * a + (bg & 0xFF) as f32 * inv) as u32;
            buffer[dst] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        }
    }
}

// ────────────────────────────── 由 <picker> 构造面板 ──────────────────────────────

use mini_render::renderer::PickerBinding;

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

/// `['广东省','广州市','天河区']` -> Vec<String>
fn region_names(s: &str) -> Vec<String> {
    s.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|p| p.trim().trim_matches('\'').trim_matches('"').to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// 按已选省/市造出三列，并定位到当前选择
fn region_columns(picked: &[String]) -> (Vec<Vec<String>>, Vec<usize>) {
    let provinces: Vec<String> = super::region_data::PROVINCES.iter().map(|(p, _)| p.to_string()).collect();
    let pi = picked
        .first()
        .and_then(|name| provinces.iter().position(|p| p == name))
        .unwrap_or(0);
    let cities: Vec<String> = super::region_data::cities_of(pi);
    let ci = picked
        .get(1)
        .and_then(|name| cities.iter().position(|c| c == name))
        .unwrap_or(0);
    let districts = super::region_data::districts_of(pi, ci);
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
        state.columns[1] = super::region_data::cities_of(pi);
        state.selected[1] = 0;
    }
    if changed_col <= 1 {
        let ci = state.selected[1].min(state.columns[1].len().saturating_sub(1));
        state.selected[1] = ci;
        state.columns[2] = super::region_data::districts_of(pi, ci);
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
        assert_eq!(s.change_value(), "2");
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
        assert_eq!(s.change_value(), "[0,2]");
    }

    #[test]
    fn time_change_value_is_hh_mm() {
        let hours: Vec<String> = (0..24).map(|h| format!("{:02}时", h)).collect();
        let mins: Vec<String> = (0..60).map(|m| format!("{:02}分", m)).collect();
        let mut s = open_sheet(SheetMode::Time, vec![hours, mins], vec![9, 5]);
        assert_eq!(s.change_value(), "09:05");
        s.move_selection(0, 3); // 9 -> 12 时
        assert_eq!(s.change_value(), "12:05");
    }

    #[test]
    fn date_change_value_is_iso() {
        let years = vec!["2023年".to_string(), "2024年".to_string()];
        let months: Vec<String> = (1..=12).map(|m| format!("{}月", m)).collect();
        let days: Vec<String> = (1..=31).map(|d| format!("{}日", d)).collect();
        let s = open_sheet(SheetMode::Date, vec![years, months, days], vec![1, 2, 14]);
        assert_eq!(s.change_value(), "2024-03-15");
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

    #[test]
    fn confirm_value_after_wheel_tap() {
        // 模拟：点中心下一行 → 选择 +1 → 确定值随之更新
        let mut s = open_sheet(SheetMode::Selector, vec![cols(&["A", "B", "C"])], vec![0]);
        let top = sheet_top(&s);
        let one_below = top + HEADER_H + WHEEL_H / 2.0 + ITEM_H;
        if let SheetHit::Item { col, delta } = hit_test(&s, 100.0, one_below) {
            s.move_selection(col, delta);
        }
        assert_eq!(s.change_value(), "1");
    }
}
