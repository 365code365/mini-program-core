//! 已经缩好的图 1:1 整行拷贝（同层渲染的视频帧）。
//!
//! 从 images.rs 原样搬出（**纯搬迁**：那个文件超了 500 行），函数体与测试一行没改。
use super::*;

impl Canvas {
    /// 图已经是按 `mode` 缩放好的尺寸（与目标差不到一个像素）时 1:1 整行拷进去，返回 true；否则返回 false
    /// （调用方改走 [`Self::draw_image`]）。
    ///
    /// 同层渲染的视频用它：宿主读回画面时就按视频框的设备像素尺寸让 GPU 缩好（`videos` 事件的 `box`），
    /// 这里只剩拷贝。从前每一帧都走逐像素双线性，960×400 的帧在一加 PLY110 上要 17~20ms，
    /// 30 帧的视频一秒就有 30 帧超时。偏移取整到整像素；圆角处按与 `draw_image` 同一个覆盖率淡出。
    #[allow(clippy::too_many_arguments)]
    pub fn blit_image_exact(
        &mut self,
        img_data: &[u8],
        img_w: u32,
        img_h: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        mode: &str,
        radius: f32,
    ) -> bool {
        if img_w == 0 || img_h == 0 || img_data.len() < (img_w * img_h * 4) as usize {
            return false;
        }
        let (sx, sy) = Self::image_scale_for_mode(mode, img_w, img_h, w, h);
        if ((sx - 1.0) * img_w as f32).abs() >= 1.0 || ((sy - 1.0) * img_h as f32).abs() >= 1.0 {
            return false;
        }
        let (x, y) = (x + self.translation.0, y + self.translation.1);
        let ox = (x + (w - img_w as f32) / 2.0).round() as i32;
        let oy = (y + (h - img_h as f32) / 2.0).round() as i32;
        // 只画在框里（aspectFill 的图比框大）、画布裁剪区里
        let (bx0, bx1, by0, by1) = self.draw_bounds();
        let (cx0, cx1) = ((x as i32).max(bx0).max(ox), ((x + w) as i32).min(bx1).min(ox + img_w as i32));
        let (cy0, cy1) = ((y as i32).max(by0).max(oy), ((y + h) as i32).min(by1).min(oy + img_h as i32));
        if cx1 <= cx0 || cy1 <= cy0 {
            return true;
        }
        let width = self.width as usize;
        let cover = |px: i32, py: i32| -> f32 {
            if radius <= 0.0 {
                return 1.0;
            }
            let (dx, dy) = (px as f32 + 0.5 - x, py as f32 + 0.5 - y);
            let at = |cx: f32, cy: f32| (radius + 0.5 - ((dx - cx) * (dx - cx) + (dy - cy) * (dy - cy)).sqrt()).clamp(0.0, 1.0);
            if dx < radius && dy < radius {
                at(radius, radius)
            } else if dx > w - radius && dy < radius {
                at(w - radius, radius)
            } else if dx < radius && dy > h - radius {
                at(radius, h - radius)
            } else if dx > w - radius && dy > h - radius {
                at(w - radius, h - radius)
            } else {
                1.0
            }
        };
        let n = (cx1 - cx0) as usize;
        for py in cy0..cy1 {
            let src = ((py - oy) as usize * img_w as usize + (cx0 - ox) as usize) * 4;
            let row = py as usize * width;
            let in_corner_rows = radius > 0.0 && ((py as f32 + 0.5 - y) < radius || (py as f32 + 0.5 - y) > h - radius);
            let src_row = &img_data[src..src + n * 4];
            // 不在圆角行、整行不透明（GL 读回的视频帧 alpha 恒为 255）：覆盖率全是 1，
            // 下面逐像素那条路径也是原样写入，这里整行字节拷贝（`Color` 就是 RGBA 四个字节），结果逐字节相同
            if !in_corner_rows && src_row.chunks_exact(4).all(|s| s[3] == 255) {
                let dst = &mut self.pixels[row + cx0 as usize..row + cx1 as usize];
                super::color_bytes_mut(dst).copy_from_slice(src_row);
                continue;
            }
            for (i, px) in (cx0..cx1).enumerate() {
                let s = &img_data[src + i * 4..src + i * 4 + 4];
                let c = if in_corner_rows { cover(px, py) } else { 1.0 };
                let a = (s[3] as f32 * c).round().clamp(0.0, 255.0) as u8;
                let color = Color::new(s[0], s[1], s[2], a);
                let idx = row + px as usize;
                if a == 255 {
                    self.pixels[idx] = color;
                } else if a > 0 {
                    self.pixels[idx] = color.blend(&self.pixels[idx]);
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 已经缩好的图（同层渲染的视频帧）1:1 整行拷贝，与逐像素双线性画出来逐字节相同（整像素位置、带圆角）
    #[test]
    fn 尺寸正好时整行拷贝与逐像素缩放逐字节相同() {
        let (iw, ih) = (40u32, 30u32);
        let img: Vec<u8> = (0..iw * ih).flat_map(|i| [(i * 7 % 256) as u8, (i * 13 % 256) as u8, (i % 256) as u8, 255]).collect();
        for (mode, w, h) in [("aspectFit", 40.0, 50.0), ("aspectFill", 30.0, 30.0), ("scaleToFill", 40.0, 30.0)] {
            let mut a = Canvas::new(64, 64);
            let mut b = Canvas::new(64, 64);
            a.clear(Color::WHITE);
            b.clear(Color::WHITE);
            assert!(a.blit_image_exact(&img, iw, ih, 7.0, 5.0, w, h, mode, 6.0), "{mode}");
            b.draw_image(&img, iw, ih, 7.0, 5.0, w, h, mode, 6.0);
            assert!(a.pixels() == b.pixels(), "{mode}：整行拷贝与 draw_image 不一致");
        }
        // 尺寸对不上（框刚变、帧还是旧尺寸）：不接，交给 draw_image
        let mut c = Canvas::new(64, 64);
        assert!(!c.blit_image_exact(&img, iw, ih, 0.0, 0.0, 60.0, 45.0, "aspectFit", 0.0));
    }
}
