//! 选择器面板绘制：标题栏、滚轮、选中行高亮、遮罩渐显
//!
//! `picker_sheet` 的一片。**纯搬迁**：从 875 行按职责切开，一行逻辑没改。
use super::*;

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
