//! 字体度量：Canvas `measureText` 的 `TextMetrics`（小游戏，game-doc/03 §2.2）与按真实度量对基线。
//! 从 text.rs 拆出来（那边已到 500 行）；只读字形度量，不光栅化。

use super::TextRenderer;

impl TextRenderer {
    /// 主字体的 (ascent, descent)，像素，都为正（descent 是基线以下的深度）。字体没有水平行度量时按 0.8 / 0.2 em
    pub fn line_metrics(&self, size: f32) -> (f32, f32) {
        match self.main_font.horizontal_line_metrics(size) {
            Some(m) if m.ascent > 0.0 => (m.ascent, (-m.descent).max(0.0)),
            _ => (size * 0.8, size * 0.2),
        }
    }

    /// 字形墨迹的包围盒（相对起点、基线）：(left, right, ascent, descent)，left 向左为正、descent 向下为正，
    /// 与 HTML `actualBoundingBox*` 同号。空串全 0
    pub fn ink_bounds(&self, text: &str, size: f32, bold: bool) -> (f32, f32, f32, f32) {
        let bold_face = bold && self.has_bold_face();
        let (mut x, mut left, mut right, mut asc, mut desc) = (0.0f32, f32::MAX, f32::MIN, f32::MIN, f32::MIN);
        for ch in text.chars() {
            if crate::emoji::is_zero_width(ch) {
                continue;
            }
            let m = self.font_for_weight(ch, bold_face).metrics(ch, size);
            if m.width > 0 && m.height > 0 {
                left = left.min(x + m.xmin as f32);
                right = right.max(x + m.xmin as f32 + m.width as f32);
                asc = asc.max(m.ymin as f32 + m.height as f32);
                desc = desc.max(-(m.ymin as f32));
            }
            x += self.measure_char_weighted(ch, size, bold);
        }
        if left == f32::MAX {
            return (0.0, 0.0, 0.0, 0.0);
        }
        (-left, right, asc, desc)
    }
}
