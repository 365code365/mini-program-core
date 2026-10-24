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

        // 逐像素的式子与从前逐字相同，只是把「只跟列有关 / 只跟行有关」的那几项提到循环外，
        // 并把循环范围收窄到有效绘制区间（区外 `set_pixel` 本来就一个不写）—— 结果逐字节不变。
        // 为什么要：轮播图翻页时每个亚像素偏移都要整张重采样一次（`draw_image_cached` 的缓存
        // 按 1/4 像素分档），一加 PLY110 上 banner 那张 966×444 一次 50ms，翻页那 0.5s 卡成十几帧
        let (bx0, bx1, by0, by1) = self.draw_bounds();
        let (lx0, lx1) = (dest_x0.max(bx0), dest_x1.min(bx1));
        let (ly0, ly1) = (dest_y0.max(by0), dest_y1.min(by1));
        if lx1 <= lx0 || ly1 <= ly0 {
            return;
        }
        // 列：源坐标（越界的列记 None）；面积滤波每列 box_nx 个采样列
        let cols: Vec<Option<(f32, u32, f32)>> = (lx0..lx1)
            .map(|dest_x| {
                let local_x = (dest_x as f32 - x - offset_x) / scale_x;
                if local_x < 0.0 || local_x >= img_w as f32 {
                    return None;
                }
                let src_x = local_x.floor() as u32;
                Some((local_x, src_x, local_x - src_x as f32))
            })
            .collect();
        let box_cols: Vec<u32> = if use_box_filter {
            cols.iter()
                .flat_map(|c| {
                    let local_x = c.map(|c| c.0).unwrap_or(0.0);
                    (0..box_nx).map(move |sx| {
                        let sample_x = local_x + (sx as f32 + 0.5) * footprint_x / box_nx as f32;
                        sample_x.max(0.0) as u32
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        let width = self.width as usize;
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
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let mut box_rows: Vec<u32> = Vec::with_capacity(box_ny as usize);

        for dest_y in ly0..ly1 {
            let local_y = (dest_y as f32 - y - offset_y) / scale_y;
            if local_y < 0.0 || local_y >= img_h as f32 {
                continue;
            }
            let src_y = local_y.floor() as u32;
            let fy = local_y - src_y as f32;
            if use_box_filter {
                box_rows.clear();
                for sy in 0..box_ny {
                    let sample_y = local_y + (sy as f32 + 0.5) * footprint_y / box_ny as f32;
                    box_rows.push(sample_y.max(0.0) as u32);
                }
            }
            let dy = dest_y as f32 + 0.5 - y;
            let row = dest_y as usize * width;
            for (ci, col) in cols.iter().enumerate() {
                let Some((_, src_x, fx)) = *col else { continue };
                let dest_x = lx0 + ci as i32;
                // 圆角抗锯齿覆盖率：像素中心到圆角圆心的距离，落在边缘 1px 带内时按比例淡出，
                // 消除之前硬裁剪（continue）造成的圆角锯齿。
                let mut corner_cover = 1.0f32;
                if has_radius {
                    let dx = dest_x as f32 + 0.5 - x;
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
                    if corner_cover <= 0.0 {
                        continue;
                    }
                }

                let (r, g, b, a_f) = if use_box_filter {
                    // 面积平均：RGB 按 alpha 加权，避免透明像素把黑色混进边缘
                    let (mut wr, mut wg, mut wb, mut wa, mut weight) =
                        (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
                    let bc = &box_cols[ci * box_nx as usize..(ci + 1) * box_nx as usize];
                    for &sy in &box_rows {
                        for &sx in bc {
                            let c = sample(sx, sy);
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
                    (
                        lerp(lerp(c00.0, c10.0, fx), lerp(c01.0, c11.0, fx), fy),
                        lerp(lerp(c00.1, c10.1, fx), lerp(c01.1, c11.1, fx), fy),
                        lerp(lerp(c00.2, c10.2, fx), lerp(c01.2, c11.2, fx), fy),
                        lerp(lerp(c00.3, c10.3, fx), lerp(c01.3, c11.3, fx), fy),
                    )
                };
                // 圆角边缘按覆盖率淡出 alpha，实现抗锯齿
                let a = (a_f * corner_cover).round().clamp(0.0, 255.0) as u8;
                let color = Color::new(
                    r.round().clamp(0.0, 255.0) as u8,
                    g.round().clamp(0.0, 255.0) as u8,
                    b.round().clamp(0.0, 255.0) as u8,
                    a,
                );
                // 与 `set_pixel` 相同的写法（越界 / 裁剪已经由循环范围保证）
                let idx = row + dest_x as usize;
                if a == 255 {
                    self.pixels[idx] = color;
                } else if a > 0 {
                    self.pixels[idx] = color.blend(&self.pixels[idx]);
                }
            }
        }
    }

    /// `draw_image_cached` 的缩放结果（`img_data` 已经是降采样过的那张）：同一张图 + 同一目标尺寸 +
    /// 同一 mode / 圆角 / 四分之一像素的亚像素偏移 `(fx, fy)` 共用一份。
    ///
    /// 与 [`Self::image_mip`] 一样在锁外算（预取线程也调，见 `components::image::prefetch`）
    #[allow(clippy::too_many_arguments)]
    pub fn image_scaled(
        cache_key: &str,
        img_data: &[u8],
        img_w: u32,
        img_h: u32,
        (fx, fy): (f32, f32),
        w: f32,
        h: f32,
        mode: &str,
        radius: f32,
    ) -> Option<std::sync::Arc<Vec<Color>>> {
        let dst_w = w.ceil() as u32 + 1;
        let dst_h = h.ceil() as u32 + 1;
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
        if let Some(hit) = scaled_image_cache().lock().ok()?.get(&key) {
            return Some(hit.clone());
        }
        let t = crate::renderer::frame_cost::start();
        let mut off = Canvas::new(dst_w, dst_h);
        off.clear(Color::new(0, 0, 0, 0));
        off.draw_image(img_data, img_w, img_h, fx, fy, w, h, mode, radius);
        let pixels = std::sync::Arc::new(off.pixels().to_vec());
        crate::renderer::frame_cost::record("缩放", t, || {
            format!("{} →{}×{}", crate::renderer::frame_cost::short(cache_key), dst_w, dst_h)
        });
        let bytes = pixels.len() * std::mem::size_of::<Color>();
        scaled_image_cache().lock().ok()?.insert(key, pixels.clone(), bytes);
        Some(pixels)
    }

    /// `draw_image_cached` 缩小时用的那张面积降采样中间图（不需要缩小时 None）。
    ///
    /// 降采样在锁**外面**做：图片预取（`components::image::prefetch`）在后台线程上也调它，
    /// 锁里算的话一张 1024² 的图要占着锁十几毫秒，引擎线程画任何一张图都得等它
    #[allow(clippy::type_complexity)]
    pub fn image_mip(
        cache_key: &str,
        img_data: &[u8],
        img_w: u32,
        img_h: u32,
        w: f32,
        h: f32,
        mode: &str,
    ) -> Option<std::sync::Arc<(u32, u32, Vec<u8>)>> {
        let (scale_x, scale_y) = Self::image_scale_for_mode(mode, img_w, img_h, w, h);
        if !(scale_x < 0.8 || scale_y < 0.8) {
            return None;
        }
        let mip_w = ((img_w as f32 * scale_x.min(1.0)).ceil() as u32).max(1);
        let mip_h = ((img_h as f32 * scale_y.min(1.0)).ceil() as u32).max(1);
        let mip_key = format!("{}|mip{}x{}", cache_key, mip_w, mip_h);
        if let Some(hit) = image_mip_cache().lock().ok()?.get(&mip_key) {
            return Some(hit.clone());
        }
        let t = crate::renderer::frame_cost::start();
        let data = std::sync::Arc::new((mip_w, mip_h, downscale_area(img_data, img_w, img_h, mip_w, mip_h)));
        crate::renderer::frame_cost::record("降采样", t, || {
            format!("{} {}×{}→{}×{}", crate::renderer::frame_cost::short(cache_key), img_w, img_h, mip_w, mip_h)
        });
        let bytes = data.2.len();
        image_mip_cache().lock().ok()?.insert(mip_key, data.clone(), bytes);
        Some(data)
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
        let mip = Self::image_mip(cache_key, img_data, img_w, img_h, w, h, mode);
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
        let cached = Self::image_scaled(cache_key, img_data, img_w, img_h, (x - ix, y - iy), w, h, mode, radius);

        match cached {
            Some(pixels) => {
                let base_x = ix as i32;
                let base_y = iy as i32;
                let bounds = self.draw_bounds();
                for row in 0..dst_h {
                    let src_row = (row * dst_w) as usize;
                    let slice = &pixels[src_row..src_row + dst_w as usize];
                    // 照片中间的行整行不透明，直接内存拷贝
                    self.blend_row_opaque_fast(base_x, base_y + row as i32, slice, bounds);
                }
            }
            None => self.draw_image(img_data, img_w, img_h, x, y, w, h, mode, radius),
        }
    }

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
