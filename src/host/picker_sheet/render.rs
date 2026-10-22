//! 选择器面板绘制：标题栏、滚轮、选中行高亮、遮罩渐显
//!
//! `picker_sheet` 的一片（从 875 行按职责切开）。滚轮按每列的**浮点位置**画
//! （拖动 / 回正时停在两行之间）；位置是整数时与从前逐字节一致 —— 静止面板的
//! 像素基线（interaction-check 的 picker_open）不因拖动支持而变。
use super::*;
use crate::compositor::footprint::{self, Footprint};

/// 把面板画到窗口 buffer 上（0xAARRGGBB，A 恒为 FF）
pub fn render(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    sf: f32,
    state: &PickerSheetState,
    text_renderer: Option<&TextRenderer>,
    rows: (u32, u32),
) -> Footprint {
    let mut fp = Footprint::NONE;
    // 遮罩随面板一起淡入，避免「先黑一下再滑上来」。
    // 查表（与逐像素 `(c as f32 * keep) as u32` 逐字节相同），表也交给 GPU 在面板外压暗；
    // `rows` 是只合成的那几行（GPU 直通帧只要面板那一截，面板总在 [`reach`] 之内）
    let mask_alpha = super::mask_alpha(state);
    if mask_alpha > MASK_VISIBLE_MIN {
        let lut = footprint::lut_trunc(1.0 - mask_alpha);
        Footprint::apply_lut(buffer, width, rows, &lut, 0xFF00_0000);
        fp.dim(&lut);
    }
    let top = (sheet_top(state) * sf) as i32;
    // 下面每一笔都落在面板顶边以下
    fp.touch(top as f32, height as f32, height);
    draw_sheet(buffer, width, height, sf, state, text_renderer, top);
    fp
}

/// 不看时间的最大足迹：面板完全展开时顶边以下（入场 / 退场途中面板只会更低）
pub fn reach(height: u32, sf: f32, state: &PickerSheetState) -> Footprint {
    let mut fp = Footprint::NONE;
    let top = ((state.viewport.1 - SHEET_H) * sf) as i32;
    fp.touch(top as f32, height as f32, height);
    fp
}

fn draw_sheet(buffer: &mut [u32], width: u32, height: u32, sf: f32, state: &PickerSheetState, text_renderer: Option<&TextRenderer>, top: i32) {
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

    let full = (0, height as i32);
    if let Some(tr) = text_renderer {
        let text_y = top + (header_h - font as i32) / 2;
        draw_text(buffer, width, height, tr, "取消", pad, text_y, font, Color::from_hex(0x888888), full);
        let confirm_w = tr.measure_text("确定", font) as i32;
        draw_text(
            buffer, width, height, tr, "确定",
            sheet_w - pad - confirm_w, text_y, font,
            Color::from_hex(0x576B95),
            full,
        );
        // header-text：头部正中（WeUI weui-picker__title：17px、rgba(0,0,0,.9)），太长时左右各让出按钮位
        if !state.header_text.is_empty() {
            let tw = tr.measure_text(&state.header_text, font) as i32;
            let room = sheet_w - 2 * (pad + confirm_w + pad);
            if tw <= room.max(0) {
                draw_text(buffer, width, height, tr, &state.header_text, (sheet_w - tw) / 2, text_y, font, Color::from_hex(0x191919), full);
            }
        }
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
        let now = Instant::now();
        // 拖动时上下各有一行半露在外面：裁到滚轮区（上不压头部分割线，下到面板底边）
        let wheel_clip = (wheel_top, top + sheet_h);
        for (ci, items) in state.columns.iter().enumerate() {
            let pos = state.column_pos(ci, now);
            let cx = ci as i32 * col_w;
            let first = (pos - center_row as f32 - 1.0).floor() as i32;
            let last = (pos + center_row as f32 + 1.0).ceil() as i32;
            for idx in first..=last {
                if idx < 0 || idx as usize >= items.len() {
                    continue;
                }
                // 这一项落在第几行（浮点）；静止时就是整数行号 0..VISIBLE_ROWS
                let off = idx as f32 - pos;
                let slot = center_row as f32 + off;
                if slot <= -1.0 || slot >= VISIBLE_ROWS as f32 {
                    continue;
                }
                let label = &items[idx as usize];
                let (color, pt) = row_style(off.abs());
                let size = pt * sf;
                let tw = tr.measure_text(label, size) as i32;
                let tx = cx + (col_w - tw) / 2;
                // 乘的是取整后的物理行高：与指示线、静止时的旧算式同一套网格
                let ty = wheel_top + (slot * item_h as f32).floor() as i32 + (item_h - size as i32) / 2;
                draw_text(buffer, width, height, tr, label, tx, ty, size, color, wheel_clip);
            }
            // 列分隔（多列时）
            if ci + 1 < cols {
                let lx = (ci as i32 + 1) * col_w;
                fill_rect(buffer, width, height, lx, wheel_top, 1, item_h * VISIBLE_ROWS as i32, 0xFFF0_F0F0);
            }
        }
    }
}

/// 离中心 `d` 行的字色与字号（磅）：越远越淡越小，模拟滚轮的透视衰减。
///
/// 整数距离上与旧版的三档完全一致（0 → 黑 17、1 → 0x707070 16、≥2 → 0xB0B0B0 15），
/// 两档之间按距离线性过渡，拖动时不会一过半行就「啪」地换色。字号按半磅取档：
/// 字形缓存以字号为键，连续字号会在一次拖动里攒下几十份同一个字的位图。
fn row_style(d: f32) -> (Color, f32) {
    fn lerp(a: u32, b: u32, t: f32) -> u32 {
        (a as f32 + (b as f32 - a as f32) * t).round() as u32
    }
    let d = d.min(2.0);
    let gray = if d <= 1.0 { lerp(0x00, 0x70, d) } else { lerp(0x70, 0xB0, d - 1.0) };
    let pt = ((17.0 - d) * 2.0).round() / 2.0;
    (Color::from_hex(gray * 0x01_01_01), pt)
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

/// 文字直接混合到 buffer（与 ui_overlay 里同名函数同样的做法），
/// 只写 `clip_y`（物理行，左闭右开）范围内的行
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
    clip_y: (i32, i32),
) {
    let (clip_top, clip_bottom) = (clip_y.0.max(0), clip_y.1.min(buf_h as i32));
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
            if dx < 0 || dy < clip_top || dx >= buf_w as i32 || dy >= clip_bottom {
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
