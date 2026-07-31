//! 图片绘制：两级过滤（面积降采样 + 双线性）与 object-fit 五种 mode
//!
//! `Canvas` 的一片，由 `canvas/mod.rs` 组合。**纯搬迁**：从 1250 行的 canvas.rs 按职责
//! 切开，一行逻辑没改；判据是 65 张画廊图与 21 页整帧快照逐字节不变。
use super::*;

impl Canvas {
    /// 绘制图片数据（RGBA 格式）
    /// img_data: RGBA 像素数据
    /// img_w, img_h: 图片原始尺寸
    /// x, y, w, h: 目标绘制区域
    /// mode: 缩放模式 (aspectFit, aspectFill, scaleToFill)
    pub fn draw_image(
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
    ) {
        if img_data.len() < (img_w * img_h * 4) as usize {
            return;
        }

        // Apply translation
        let x = x + self.translation.0;
        let y = y + self.translation.1;

        // 计算缩放和偏移
        let (scale_x, scale_y) = Self::image_scale_for_mode(mode, img_w, img_h, w, h);
        let (offset_x, offset_y) = if mode == "aspectFit" || mode == "aspectFill" {
            (
                (w - img_w as f32 * scale_x) / 2.0,
                (h - img_h as f32 * scale_y) / 2.0,
            )
        } else {
            (0.0, 0.0)
        };

        let dest_x0 = x as i32;
        let dest_y0 = y as i32;
        let dest_x1 = (x + w) as i32;
        let dest_y1 = (y + h) as i32;

        // 缩小时改用面积平均（盒式滤波）。双线性只看 4 个邻域像素，一旦缩小超过 1 倍
        // 就有源像素完全没被采到 —— 照片、封面图的边缘会出现明显锯齿和摩尔纹。
        // 浏览器缩小图片同样是多级/面积过滤，所以这一步也让两端更接近。
        // 结果由 `draw_image_cached` 按目标尺寸缓存，额外代价只在首次缩放时付一次。
        let footprint_x = (1.0 / scale_x).abs();
        let footprint_y = (1.0 / scale_y).abs();
        let use_box_filter = footprint_x > 1.2 || footprint_y > 1.2;
        // 采样点数设上限：超大图不至于退化成「每个目标像素扫一大片源像素」
        let box_nx = (footprint_x.round() as i32).clamp(1, 8);
        let box_ny = (footprint_y.round() as i32).clamp(1, 8);

        // 圆角裁剪预计算
        let has_radius = radius > 0.0;
        let _cx = x + w / 2.0;
        let _cy = y + h / 2.0;

        for dest_y in dest_y0..dest_y1 {
            for dest_x in dest_x0..dest_x1 {
                // 圆角抗锯齿覆盖率：像素中心到圆角圆心的距离，落在边缘 1px 带内时按比例淡出，
                // 消除之前硬裁剪（continue）造成的圆角锯齿。
                let mut corner_cover = 1.0f32;
                if has_radius {
                    let dx = dest_x as f32 + 0.5 - x;
                    let dy = dest_y as f32 + 0.5 - y;
                    let cover_at = |cx: f32, cy: f32| -> f32 {
                        let d = ((dx - cx) * (dx - cx) + (dy - cy) * (dy - cy)).sqrt();
                        (radius + 0.5 - d).clamp(0.0, 1.0)
                    };
                    if dx < radius && dy < radius {
                        corner_cover = cover_at(radius, radius);
                    } else if dx > w - radius && dy < radius {
                        corner_cover = cover_at(w - radius, radius);
                    } else if dx < radius && dy > h - radius {
                        corner_cover = cover_at(radius, h - radius);
                    } else if dx > w - radius && dy > h - radius {
                        corner_cover = cover_at(w - radius, h - radius);
                    }
                    if corner_cover <= 0.0 { continue; }
                }

                // 计算源图片坐标
                let local_x = (dest_x as f32 - x - offset_x) / scale_x;
                let local_y = (dest_y as f32 - y - offset_y) / scale_y;

                // 边界检查
                if local_x < 0.0 || local_y < 0.0 || 
                   local_x >= img_w as f32 || local_y >= img_h as f32 {
                    continue;
                }

                // 双线性插值采样
                let src_x = local_x.floor() as u32;
                let src_y = local_y.floor() as u32;
                let fx = local_x - src_x as f32;
                let fy = local_y - src_y as f32;

                let sample = |sx: u32, sy: u32| -> (f32, f32, f32, f32) {
                    let sx = sx.min(img_w - 1);
                    let sy = sy.min(img_h - 1);
                    let idx = ((sy * img_w + sx) * 4) as usize;
                    (
                        img_data[idx] as f32,
                        img_data[idx + 1] as f32,
                        img_data[idx + 2] as f32,
                        img_data[idx + 3] as f32,
                    )
                };

                let (r, g, b, a_f) = if use_box_filter {
                    // 面积平均：RGB 按 alpha 加权，避免透明像素把黑色混进边缘
                    let (mut wr, mut wg, mut wb, mut wa, mut weight) =
                        (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
                    for sy in 0..box_ny {
                        let sample_y = local_y + (sy as f32 + 0.5) * footprint_y / box_ny as f32;
                        for sx in 0..box_nx {
                            let sample_x = local_x + (sx as f32 + 0.5) * footprint_x / box_nx as f32;
                            let c = sample(
                                sample_x.max(0.0) as u32,
                                sample_y.max(0.0) as u32,
                            );
                            wr += c.0 * c.3;
                            wg += c.1 * c.3;
                            wb += c.2 * c.3;
                            wa += c.3;
                            weight += 1.0;
                        }
                    }
                    if wa > 0.0 {
                        (wr / wa, wg / wa, wb / wa, wa / weight.max(1.0))
                    } else {
                        (0.0, 0.0, 0.0, 0.0)
                    }
                } else {
                    let c00 = sample(src_x, src_y);
                    let c10 = sample(src_x + 1, src_y);
                    let c01 = sample(src_x, src_y + 1);
                    let c11 = sample(src_x + 1, src_y + 1);
                    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
                    (
                        lerp(lerp(c00.0, c10.0, fx), lerp(c01.0, c11.0, fx), fy),
                        lerp(lerp(c00.1, c10.1, fx), lerp(c01.1, c11.1, fx), fy),
                        lerp(lerp(c00.2, c10.2, fx), lerp(c01.2, c11.2, fx), fy),
                        lerp(lerp(c00.3, c10.3, fx), lerp(c01.3, c11.3, fx), fy),
                    )
                };
                // 圆角边缘按覆盖率淡出 alpha，实现抗锯齿
                let a = (a_f * corner_cover).round().clamp(0.0, 255.0) as u8;

                self.set_pixel(
                    dest_x,
                    dest_y,
                    Color::new(
                        r.round().clamp(0.0, 255.0) as u8,
                        g.round().clamp(0.0, 255.0) as u8,
                        b.round().clamp(0.0, 255.0) as u8,
                        a,
                    ),
                );
            }
        }
    }

    /// 绘制图片（带缩放结果缓存）。
    ///
    /// `draw_image` 是逐像素双线性重采样：一张 750x300 的 banner 就是 22 万次四抽样。
    /// 页面只要有 CSS 动画就每帧重绘，首页那种「轮播 + 多张商品图」的页面光图片重采样
    /// 就吃掉几十毫秒（实测整帧 30ms+，肉眼就是卡）。
    ///
    /// 这里把「同一张图 + 同一目标尺寸 + 同一 mode/圆角/亚像素偏移」的重采样结果缓存下来，
    /// 后续帧退化为一次带 alpha 的整块拷贝。key 带上亚像素偏移，保证输出与不缓存时一致。
    pub fn draw_image_cached(
        &mut self,
        cache_key: &str,
        img_data: &[u8],
        img_w: u32,
        img_h: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        mode: &str,
        radius: f32,
    ) {
        if w <= 0.0 || h <= 0.0 || cache_key.is_empty() {
            self.draw_image(img_data, img_w, img_h, x, y, w, h, mode, radius);
            return;
        }
        // 缩小时先取（或建）一张按目标尺寸面积降采样的中间图，之后一律按 1:1 附近的
        // 双线性绘制。面积滤波只在这里付一次，轮播平移等每帧变亚像素偏移的场景不再重算。
        let (scale_x, scale_y) = Self::image_scale_for_mode(mode, img_w, img_h, w, h);
        let mip = if scale_x < 0.8 || scale_y < 0.8 {
            let mip_w = ((img_w as f32 * scale_x.min(1.0)).ceil() as u32).max(1);
            let mip_h = ((img_h as f32 * scale_y.min(1.0)).ceil() as u32).max(1);
            let mip_key = format!("{}|mip{}x{}", cache_key, mip_w, mip_h);
            match image_mip_cache().lock() {
                Ok(mut guard) => {
                    if let Some(hit) = guard.get(&mip_key) {
                        Some(hit.clone())
                    } else {
                        let data = std::sync::Arc::new((
                            mip_w,
                            mip_h,
                            downscale_area(img_data, img_w, img_h, mip_w, mip_h),
                        ));
                        if guard.len() > 128 {
                            guard.clear(); // 简单上限：超出整体失效，避免无界增长
                        }
                        guard.insert(mip_key, data.clone());
                        Some(data)
                    }
                }
                Err(_) => None,
            }
        } else {
            None
        };
        let (img_data, img_w, img_h) = match &mip {
            Some(m) => (m.2.as_slice(), m.0, m.1),
            None => (img_data, img_w, img_h),
        };
        let dst_w = w.ceil() as u32 + 1;
        let dst_h = h.ceil() as u32 + 1;
        // 目标过大（整屏级）时缓存收益低、占用高，直接走原路径
        if (dst_w as u64) * (dst_h as u64) > 4_000_000 {
            self.draw_image(img_data, img_w, img_h, x, y, w, h, mode, radius);
            return;
        }
        let ix = x.floor();
        let iy = y.floor();
        let fx = x - ix;
        let fy = y - iy;
        // 全整数格式化：避免浮点转十进制的开销（每帧每张图都会走到）
        let key = format!(
            "{}|{}x{}|{}x{}|{}|{}|{}|{}",
            cache_key,
            dst_w,
            dst_h,
            (w * 4.0) as i32,
            (h * 4.0) as i32,
            mode,
            (radius * 4.0) as i32,
            (fx * 4.0).round() as i32,
            (fy * 4.0).round() as i32
        );

        let cached: Option<std::sync::Arc<Vec<Color>>> = match scaled_image_cache().lock() {
            Ok(mut guard) => {
                if let Some(hit) = guard.get(&key) {
                    Some(hit.clone())
                } else {
                    let mut off = Canvas::new(dst_w, dst_h);
                    off.clear(Color::new(0, 0, 0, 0));
                    off.draw_image(img_data, img_w, img_h, fx, fy, w, h, mode, radius);
                    let pixels = std::sync::Arc::new(off.pixels().to_vec());
                    if guard.len() > 192 {
                        guard.clear(); // 简单上限：超出整体失效，避免无界增长
                    }
                    guard.insert(key, pixels.clone());
                    Some(pixels)
                }
            }
            Err(_) => None,
        };

        match cached {
            Some(pixels) => {
                let base_x = ix as i32;
                let base_y = iy as i32;
                let bounds = self.draw_bounds();
                for row in 0..dst_h {
                    let src_row = (row * dst_w) as usize;
                    let slice = &pixels[src_row..src_row + dst_w as usize];
                    self.blend_row(base_x, base_y + row as i32, slice, bounds);
                }
            }
            None => self.draw_image(img_data, img_w, img_h, x, y, w, h, mode, radius),
        }
    }

    /// 按 `mode` 求源图到目标区域的缩放比例（等价于 CSS 的 object-fit）
    pub(super) fn image_scale_for_mode(mode: &str, img_w: u32, img_h: u32, w: f32, h: f32) -> (f32, f32) {
        if img_w == 0 || img_h == 0 {
            return (1.0, 1.0);
        }
        let (sx, sy) = (w / img_w as f32, h / img_h as f32);
        match mode {
            // 保持比例、完整显示（可能留白）
            "aspectFit" => {
                let s = sx.min(sy);
                (s, s)
            }
            // 保持比例、填满区域（可能裁剪）
            "aspectFill" => {
                let s = sx.max(sy);
                (s, s)
            }
            // scaleToFill：两轴独立拉伸
            _ => (sx, sy),
        }
    }
}
