//! 覆盖层的光栅化小工具：直接往 0x00RRGGBB 输出缓冲里画圆角矩形、文字。
//!
//! 覆盖层画在合成之后的输出缓冲上，没有 Canvas 可用，所以这些函数自己逐像素写。
//! `draw_text_direct` 是从 ui_overlay.rs 原样搬出的旧画法（操作菜单的选项、图片预览还在用）；
//! 其余是提示框 / Modal / 操作菜单用的抗锯齿版（按覆盖率混色、浮点坐标）。
//! 旧版的圆角矩形、对勾、圆环没有抗锯齿，换成新画法后已删掉。
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint};


/// 直接绘制文字到 buffer
pub(super) fn draw_text_direct(
    buffer: &mut [u32],
    buf_width: u32, buf_height: u32,
    tr: &TextRenderer,
    text: &str,
    x: i32, y: i32,
    font_size: f32,
    color: Color
) {
    let text_width = (tr.measure_text(text, font_size) + 10.0) as u32;
    let text_height = (font_size * 1.5) as u32;
    
    if text_width == 0 || text_height == 0 { return; }
    
    let mut temp_canvas = Canvas::new(text_width.max(1), text_height.max(1));
    temp_canvas.clear(Color::TRANSPARENT);
    let paint = Paint::new().with_color(color);
    
    let baseline_y = font_size * 0.85;
    tr.draw_text(&mut temp_canvas, text, 0.0, baseline_y, font_size, &paint);
    
    let temp_pixels = temp_canvas.pixels();
    for py in 0..text_height as i32 {
        for px in 0..text_width as i32 {
            let src_idx = (py as u32 * text_width + px as u32) as usize;
            let dst_x = x + px;
            let dst_y = y + py;
            if dst_x >= 0 && dst_x < buf_width as i32 && dst_y >= 0 && dst_y < buf_height as i32 {
                let dst_idx = (dst_y as u32 * buf_width + dst_x as u32) as usize;
                if dst_idx < buffer.len() && src_idx < temp_pixels.len() {
                    let pixel = temp_pixels[src_idx];
                    if pixel.a > 0 {
                        let existing = buffer[dst_idx];
                        let bg_r = ((existing >> 16) & 0xFF) as u8;
                        let bg_g = ((existing >> 8) & 0xFF) as u8;
                        let bg_b = (existing & 0xFF) as u8;
                        
                        let alpha = pixel.a as f32 / 255.0;
                        let inv_alpha = 1.0 - alpha;
                        
                        let r = (pixel.r as f32 * alpha + bg_r as f32 * inv_alpha) as u8;
                        let g = (pixel.g as f32 * alpha + bg_g as f32 * inv_alpha) as u8;
                        let b = (pixel.b as f32 * alpha + bg_b as f32 * inv_alpha) as u8;
                        
                        buffer[dst_idx] = 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                    }
                }
            }
        }
    }
}


// ============ 抗锯齿版（提示框 / Modal 用，对齐 weui 2.x 的视觉）============
//
// 上面那几个是逐像素「在里面就写死颜色」，圆角是锯齿状的。微信的提示框与弹窗是系统控件，
// 圆角、对勾、转圈都是平滑的 —— 新画法一律按覆盖率混色，坐标用浮点（物理像素）。

/// 往 0x00RRGGBB 像素上按覆盖率 `a`（0~1）混一个颜色
#[inline]
pub(super) fn blend(dst: &mut u32, rgb: u32, a: f32) {
    if a <= 0.0 {
        return;
    }
    if a >= 1.0 {
        *dst = rgb & 0x00FF_FFFF;
        return;
    }
    let mix = |s: u32, d: u32| ((s as f32) * a + (d as f32) * (1.0 - a) + 0.5) as u32;
    let (d, s) = (*dst, rgb);
    *dst = (mix((s >> 16) & 0xFF, (d >> 16) & 0xFF) << 16)
        | (mix((s >> 8) & 0xFF, (d >> 8) & 0xFF) << 8)
        | mix(s & 0xFF, d & 0xFF);
}

/// 缓冲的 `rows` 那几行按 `keep` 压暗（遮罩 rgba(0,0,0,1-keep)；整屏遮罩传 (0, 高)）。
///
/// 查表（与逐像素 `(v as f32 * keep + 0.5) as u32` 逐字节相同）：Modal / 提示框的遮罩每帧盖满整屏，
/// 1080×2354 两百多万像素逐个三次浮点乘加，在一加 PLY110 上一帧要好几毫秒
/// 返回用的那张表（GPU 在覆盖层行区间外按它压暗，见 `compositor::footprint`）
pub(super) fn dim(buf: &mut [u32], w: u32, rows: (u32, u32), keep: f32) -> [u8; 256] {
    let lut = crate::compositor::footprint::lut_round(keep);
    crate::compositor::Footprint::apply_lut(buf, w, rows, &lut, 0);
    lut
}

/// 一维区间 [a, b) 落在像素 [p, p+1) 里的长度
#[inline]
fn span_cov(p: f32, a: f32, b: f32) -> f32 {
    ((p + 1.0).min(b) - p.max(a)).clamp(0.0, 1.0)
}

/// 抗锯齿圆角矩形（物理像素、浮点坐标），`alpha` 是整体不透明度。
/// `corners`：左上、右上、右下、左下哪几个角是圆的（Modal 按钮的按压底只圆下面两个角）。
pub(super) fn fill_rrect_aa(
    buf: &mut [u32],
    w: u32,
    h: u32,
    rect: (f32, f32, f32, f32),
    r: f32,
    rgb: u32,
    alpha: f32,
    corners: [bool; 4],
) {
    fill_rrect_aa_rows(buf, w, h, rect, r, rgb, alpha, corners, (0, h));
}

/// [`fill_rrect_aa`]，只写 `rows` 那几行 [a, b)（操作菜单的 GPU 图层：CPU 只画顶边圆角那几行，
/// 见 `action_sheet::layer`）。每一行的结果与整块画时逐字节相同 —— 只是循环范围收窄
#[allow(clippy::too_many_arguments)]
pub(super) fn fill_rrect_aa_rows(
    buf: &mut [u32],
    w: u32,
    h: u32,
    (x, y, rw, rh): (f32, f32, f32, f32),
    r: f32,
    rgb: u32,
    alpha: f32,
    corners: [bool; 4],
    rows: (u32, u32),
) {
    if rw <= 0.0 || rh <= 0.0 {
        return;
    }
    let r = r.min(rw / 2.0).min(rh / 2.0).max(0.0);
    let (x0, y0) = ((x.floor() as i32).max(0), (y.floor() as i32).max(0).max(rows.0 as i32));
    let (x1, y1) = (((x + rw).ceil() as i32).min(w as i32), ((y + rh).ceil() as i32).min(h as i32).min(rows.1 as i32));
    // 整行完全落在矩形里、又不在圆角那几行的像素，覆盖率恒为 1：这一段整段处理，
    // 只有左右两头那一两个像素按覆盖率算。ActionSheet 的白底是大半屏，从前每一帧
    // 逐像素算一遍覆盖率与圆角判断，一加 PLY110 上光这一步就几毫秒（结果逐字节不变）
    let (ix0, ix1) = ((x.ceil() as i32).max(x0), ((x + rw).floor() as i32).min(x1));
    for py in y0..y1 {
        let cy_cov = span_cov(py as f32, y, y + rh);
        let fy = py as f32 + 0.5;
        let plain = cy_cov >= 1.0 && fy >= y + r && fy <= y + rh - r && ix1 > ix0;
        if plain {
            let row = (py as u32 * w) as usize;
            for px in (x0..ix0).chain(ix1..x1) {
                let cov = span_cov(px as f32, x, x + rw);
                if cov > 0.0 {
                    blend(&mut buf[row + px as usize], rgb, cov * alpha);
                }
            }
            let span = &mut buf[row + ix0 as usize..row + ix1 as usize];
            if alpha >= 1.0 {
                span.fill(rgb & 0x00FF_FFFF);
            } else {
                for d in span.iter_mut() {
                    blend(d, rgb, alpha);
                }
            }
            continue;
        }
        for px in x0..x1 {
            let mut cov = cy_cov * span_cov(px as f32, x, x + rw);
            let fx = px as f32 + 0.5;
            // 落在某个圆角的方块里：按到圆心的距离再削一次
            let left = fx < x + r;
            let right = fx > x + rw - r;
            let top = fy < y + r;
            let bottom = fy > y + rh - r;
            let corner = match (left, right, top, bottom) {
                (true, _, true, _) if corners[0] => Some((x + r, y + r)),
                (_, true, true, _) if corners[1] => Some((x + rw - r, y + r)),
                (_, true, _, true) if corners[2] => Some((x + rw - r, y + rh - r)),
                (true, _, _, true) if corners[3] => Some((x + r, y + rh - r)),
                _ => None,
            };
            if let Some((ccx, ccy)) = corner {
                let d = ((fx - ccx).powi(2) + (fy - ccy).powi(2)).sqrt();
                cov = cov.min((r - d + 0.5).clamp(0.0, 1.0));
            }
            if cov > 0.0 {
                blend(&mut buf[(py as u32 * w + px as u32) as usize], rgb, cov * alpha);
            }
        }
    }
}

/// 四个角都圆
pub const ALL_CORNERS: [bool; 4] = [true; 4];

/// 文字宽度（没有字体时按「汉字 1em、ASCII 0.55em」估，只在单元测试里走到）
pub(super) fn text_width(tr: Option<&TextRenderer>, text: &str, size: f32, bold: bool) -> f32 {
    match tr {
        Some(tr) => tr.measure_text_weighted(text, size, 0.0, bold),
        None => text.chars().map(|c| if c.is_ascii() { size * 0.55 } else { size }).sum(),
    }
}

/// 英文单词里的字符（两个之间不能断行）
fn is_word_char(c: char) -> bool {
    c.is_ascii() && !c.is_ascii_whitespace()
}

/// 行尾那个还没写完的英文单词从哪里开始（字节下标）。整行就是一个词时返回 None（只能从词中间断）
fn trailing_word_start(line: &str) -> Option<usize> {
    let start = line
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_word_char(*c))
        .last()
        .map(|(i, _)| i)?;
    (start > 0).then_some(start)
}

/// 断行（`\n` 强制换行），最多 `max_lines` 行；装不下的在最后一行末尾补「…」。
///
/// 中文逐字断；英文单词整个挪到下一行，只有一个词整行都放不下时才从词中间断 ——
/// 即 `word-wrap: break-word` 的效果（从前一律逐字断，`showCancel: false` 断成 `fals` / `e`）。
/// 最后一行允许的那一行按字截（`-webkit-line-clamp` / `text-overflow: ellipsis` 都是按字截的）。
pub(super) fn wrap_lines(
    tr: Option<&TextRenderer>,
    text: &str,
    size: f32,
    max_w: f32,
    max_lines: usize,
    bold: bool,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut truncated = false;
    let paras: Vec<&str> = text.split('\n').collect();
    'outer: for (pi, para) in paras.iter().enumerate() {
        let mut cur = String::new();
        let mut cur_w = 0.0;
        for ch in para.chars() {
            let cw = text_width(tr, ch.encode_utf8(&mut [0u8; 4]), size, bold);
            if cur_w + cw > max_w && !cur.is_empty() {
                let last_allowed = lines.len() + 1 >= max_lines;
                let carry = match trailing_word_start(&cur) {
                    Some(i) if is_word_char(ch) && !last_allowed => cur.split_off(i),
                    _ => String::new(),
                };
                lines.push(cur.trim_end().to_string());
                cur = carry;
                cur_w = text_width(tr, &cur, size, bold);
                if lines.len() == max_lines {
                    truncated = true;
                    break 'outer;
                }
                // 折到下一行的空格不占位置（CSS 把行首的空白吃掉）
                if cur.is_empty() && ch == ' ' {
                    continue;
                }
            }
            cur.push(ch);
            cur_w += cw;
        }
        lines.push(cur);
        if lines.len() == max_lines {
            // 后面还有段落没放进来
            truncated = pi + 1 < paras.len();
            break;
        }
    }
    if truncated {
        if let Some(last) = lines.last_mut() {
            *last = ellipsize(tr, &format!("{last}…"), size, max_w, bold);
        }
    }
    lines
}

/// 单行装不下就从尾部砍字并补「…」
pub(super) fn ellipsize(tr: Option<&TextRenderer>, text: &str, size: f32, max_w: f32, bold: bool) -> String {
    if text_width(tr, text, size, bold) <= max_w {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.trim_end_matches('…').chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let s: String = chars.iter().collect::<String>() + "…";
        if text_width(tr, &s, size, bold) <= max_w {
            return s;
        }
    }
    "…".to_string()
}

/// 在基线上画一行字（物理像素：`x` 是左边，`baseline` 是基线）。
///
/// `bold`：字体有真粗体字面就用它；没有就错开 0.7px 再描一遍 —— 与页面文字
/// （`renderer::components::text`）同一个回退，弹窗标题与页面里的粗体看起来一样粗。
pub(super) fn draw_text_line(
    buf: &mut [u32],
    w: u32,
    h: u32,
    tr: &TextRenderer,
    text: &str,
    (x, baseline): (f32, f32),
    size: f32,
    rgb: u32,
    bold: bool,
    sf: f32,
) {
    if text.is_empty() {
        return;
    }
    let real_bold = bold && tr.has_bold_face();
    let faux = if bold && !real_bold { 0.7 * sf } else { 0.0 };
    let ascent = (size * 1.15).ceil();
    let tw = (tr.measure_text_weighted(text, size, 0.0, real_bold) + faux + 4.0).ceil() as u32;
    let th = (ascent + size * 0.45).ceil() as u32;
    let mut tmp = Canvas::new(tw.max(1), th.max(1));
    tmp.clear(Color::TRANSPARENT);
    let paint = Paint::new().with_color(Color::from_hex(rgb));
    // 小数部分留在临时画布里画，整数部分在贴的时候挪：字形落点与直接画一致
    let (ox, oy) = (x.floor(), (baseline - ascent).floor());
    let (fx, fy) = (x - ox, baseline - ascent - oy);
    tr.draw_text_weighted(&mut tmp, text, fx, ascent + fy, size, 0.0, real_bold, &paint);
    if faux > 0.0 {
        tr.draw_text_weighted(&mut tmp, text, fx + faux, ascent + fy, size, 0.0, real_bold, &paint);
    }
    let src = tmp.pixels();
    let (ox, oy) = (ox as i32, oy as i32);
    for ty in 0..th as i32 {
        let dy = oy + ty;
        if dy < 0 || dy >= h as i32 {
            continue;
        }
        for tx in 0..tw as i32 {
            let dx = ox + tx;
            if dx < 0 || dx >= w as i32 {
                continue;
            }
            let p = src[(ty as u32 * tw + tx as u32) as usize];
            if p.a > 0 {
                blend(&mut buf[(dy as u32 * w + dx as u32) as usize], rgb, p.a as f32 / 255.0);
            }
        }
    }
}
