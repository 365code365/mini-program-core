//! 像素级操作：单点写入、行混合、掩膜混合、区间填充、按带清屏
//!
//! `Canvas` 的一片，由 `canvas/mod.rs` 组合。**纯搬迁**：从 1250 行的 canvas.rs 按职责
//! 切开，一行逻辑没改；判据是 65 张画廊图与 21 页整帧快照逐字节不变。
use super::*;

impl Canvas {
    /// 获取像素
    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> Color {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize]
        } else {
            Color::TRANSPARENT
        }
    }

    /// 设置像素（带 alpha 混合）
    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }

        // 检查裁剪区域
        if let Some(clip) = &self.clip_rect {
            if x < clip.x as i32 || x >= clip.right() as i32 ||
               y < clip.y as i32 || y >= clip.bottom() as i32 {
                return;
            }
        }

        let idx = (y as u32 * self.width + x as u32) as usize;
        if color.a == 255 {
            self.pixels[idx] = color;
        } else if color.a > 0 {
            self.pixels[idx] = color.blend(&self.pixels[idx]);
        }
    }

    /// 直接设置像素（供文本渲染使用）
    pub fn set_pixel_direct(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32 {
            let idx = (y as u32 * self.width + x as u32) as usize;
            self.pixels[idx] = color;
        }
    }

    /// 设置像素（带抗锯齿 coverage）
    pub(super) fn set_pixel_aa(&mut self, x: i32, y: i32, color: Color, coverage: f32) {
        if coverage <= 0.0 { return; }
        // 四舍五入而不是截断：截断会让每一档覆盖率都少一格 alpha，
        // 大面积渐变边缘上能看出整体偏浅。
        let a = (color.a as f32 * coverage.min(1.0)).round().clamp(0.0, 255.0) as u8;
        self.set_pixel(x, y, Color::new(color.r, color.g, color.b, a));
    }

    /// 当前有效绘制区间（画布边界 ∩ 裁剪矩形），返回 (x0, x1, y0, y1)。
    /// 逐像素循环时把它提到循环外算一次，能省掉每个像素 8 次比较和多次浮点截断。
    pub(super) fn draw_bounds(&self) -> (i32, i32, i32, i32) {
        let (mut x0, mut y0) = (0i32, 0i32);
        let (mut x1, mut y1) = (self.width as i32, self.height as i32);
        if let Some(clip) = &self.clip_rect {
            x0 = x0.max(clip.x as i32);
            y0 = y0.max(clip.y as i32);
            x1 = x1.min(clip.right() as i32);
            y1 = y1.min(clip.bottom() as i32);
        }
        (x0, x1, y0, y1)
    }

    /// 把一行像素混合到 (dst_x, dst_y)：越界与裁剪只在行级判一次。
    ///
    /// 图片 blit 是「大面积 + 逐像素」的典型场景，走 `set_pixel` 的话每个像素都要
    /// 重做一遍边界比较与裁剪矩形的浮点截断，占比相当可观。
    pub(super) fn blend_row(&mut self, dst_x: i32, dst_y: i32, src: &[Color], bounds: (i32, i32, i32, i32)) {
        let (bx0, bx1, by0, by1) = bounds;
        if dst_y < by0 || dst_y >= by1 {
            return;
        }
        // 与目标行的有效区间求交，同时算出对应的源起点
        let start = bx0.max(dst_x);
        let end = bx1.min(dst_x + src.len() as i32);
        if end <= start {
            return;
        }
        let row = dst_y as usize * self.width as usize;
        let src_off = (start - dst_x) as usize;
        for i in 0..(end - start) as usize {
            let color = src[src_off + i];
            if color.a == 0 {
                continue;
            }
            let idx = row + start as usize + i;
            self.pixels[idx] = if color.a == 255 {
                color
            } else {
                color.blend(&self.pixels[idx])
            };
        }
    }

    /// 把一整块像素按行合成到 (dst_x, dst_y)（尊重画布边界与当前裁剪矩形）。
    ///
    /// 大面积拷贝**必须**走这里而不是逐像素 `set_pixel`：后者每个像素都要重做
    /// 四次边界比较 + 裁剪矩形的浮点截断，一屏 750x1334 就是一百万次。
    /// scroll-view 从离屏缓存上屏正是这种形状的活儿，之前逐像素做，
    /// 拖动时每帧白付十来毫秒 —— 手上就是「滑动发抖」。
    ///
    /// - `src`：源像素，按 `src_width` 行主序排列
    /// - 整行不透明时直接 `copy_from_slice`，跳过逐像素的 alpha 判断
    pub fn blend_pixels(&mut self, dst_x: i32, dst_y: i32, src: &[Color], src_width: usize) {
        if src_width == 0 {
            return;
        }
        let bounds = self.draw_bounds();
        let rows = src.len() / src_width;
        for row in 0..rows {
            let line = &src[row * src_width..(row + 1) * src_width];
            self.blend_row_opaque_fast(dst_x, dst_y + row as i32, line, bounds);
        }
    }

    /// 用一张 8 位覆盖率掩膜按单色合成（`box-shadow` 的模糊边缘走这里）。
    ///
    /// `mask` 按 `mask_w` 行主序排列，值即覆盖率（0~255）。掩膜可以离线算好并缓存，
    /// 于是每帧只剩「一次乘加混合」，不必重复做模糊。
    pub fn blend_mask(&mut self, dst_x: i32, dst_y: i32, mask: &[u8], mask_w: usize, color: Color) {
        if mask_w == 0 || color.a == 0 {
            return;
        }
        let (bx0, bx1, by0, by1) = self.draw_bounds();
        let rows = mask.len() / mask_w;
        let width = self.width as usize;
        for row in 0..rows {
            let dy = dst_y + row as i32;
            if dy < by0 || dy >= by1 {
                continue;
            }
            let start = bx0.max(dst_x);
            let end = bx1.min(dst_x + mask_w as i32);
            if end <= start {
                continue;
            }
            let src_off = (start - dst_x) as usize;
            let len = (end - start) as usize;
            let line = &mask[row * mask_w + src_off..row * mask_w + src_off + len];
            let base = dy as usize * width + start as usize;
            for (i, &cov) in line.iter().enumerate() {
                if cov == 0 {
                    continue;
                }
                let a = (cov as u32 * color.a as u32 / 255) as u8;
                if a == 0 {
                    continue;
                }
                let idx = base + i;
                self.pixels[idx] = Color::new(color.r, color.g, color.b, a).blend(&self.pixels[idx]);
            }
        }
    }

    /// 用单色填一段横跨 `[x0, x1)` 的扫描线（语义与逐像素 `set_pixel` 完全一致，
    /// 但边界/裁剪只判一次，不透明时退化成 memset）。
    ///
    /// 渐变、圆角这类「按行算色」的绘制以前逐像素调 `set_pixel`，每个像素都要重做
    /// 边界比较与裁剪矩形的浮点截断。同一行颜色相同的场合（垂直渐变整行同色）
    /// 用这个接口可以少掉一个量级的开销，而输出一模一样。
    pub fn fill_span(&mut self, x0: i32, x1: i32, y: i32, color: Color) {
        if color.a == 0 {
            return;
        }
        let (bx0, bx1, by0, by1) = self.draw_bounds();
        if y < by0 || y >= by1 {
            return;
        }
        let start = bx0.max(x0);
        let end = bx1.min(x1);
        if end <= start {
            return;
        }
        let row = y as usize * self.width as usize;
        let span = &mut self.pixels[row + start as usize..row + end as usize];
        if color.a == 255 {
            span.fill(color);
            return;
        }
        for px in span.iter_mut() {
            *px = color.blend(px);
        }
    }

    /// 单行合成，带「整行不透明 → 直接内存拷贝」的快路径。
    pub(super) fn blend_row_opaque_fast(&mut self, dst_x: i32, dst_y: i32, src: &[Color], bounds: (i32, i32, i32, i32)) {
        let (bx0, bx1, by0, by1) = bounds;
        if dst_y < by0 || dst_y >= by1 {
            return;
        }
        let start = bx0.max(dst_x);
        let end = bx1.min(dst_x + src.len() as i32);
        if end <= start {
            return;
        }
        let src_off = (start - dst_x) as usize;
        let len = (end - start) as usize;
        let src_slice = &src[src_off..src_off + len];
        // 常见情况：整行都是不透明像素（离屏缓存里的页面背景就是这样），
        // 这时逐像素判 alpha 纯属浪费，直接整段拷贝。
        if src_slice.iter().all(|c| c.a == 255) {
            let row = dst_y as usize * self.width as usize + start as usize;
            self.pixels[row..row + len].copy_from_slice(src_slice);
            return;
        }
        self.blend_row(dst_x, dst_y, src, bounds);
    }

    /// 只清理一个矩形区域（其余像素保持原样）。用于损伤区局部重绘：
    /// 不能按整行清，否则会把矩形左右两侧的内容一起抹掉（裁剪会阻止它们被重画）。
    pub fn clear_area(&mut self, rect: &Rect, color: Color) {
        let x0 = rect.x.floor().clamp(0.0, self.width as f32) as usize;
        let x1 = rect.right().ceil().clamp(0.0, self.width as f32) as usize;
        let y0 = rect.y.floor().clamp(0.0, self.height as f32) as usize;
        let y1 = rect.bottom().ceil().clamp(0.0, self.height as f32) as usize;
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let width = self.width as usize;
        for y in y0..y1 {
            self.pixels[y * width + x0..y * width + x1].fill(color);
        }
    }

    /// 只清理 [y0, y1) 这一条横带（其余像素保持原样）。
    ///
    /// 整页画布可能上万像素高，而每帧真正会被绘制/上屏的只有视口附近一条带；
    /// 全画布 clear 属于纯浪费（首页 750x6870 的画布 ≈ 每帧 20MB memset）。
    pub fn clear_band(&mut self, y0: i32, y1: i32, color: Color) {
        let width = self.width as usize;
        let y0 = y0.clamp(0, self.height as i32) as usize;
        let y1 = y1.clamp(0, self.height as i32) as usize;
        if y1 <= y0 {
            return;
        }
        self.pixels[y0 * width..y1 * width].fill(color);
    }
}
