//! 路径光栅化：扫描线 + even-odd 填充、描边、接头补圆
//!
//! `Canvas` 的一片，由 `canvas/mod.rs` 组合。**纯搬迁**：从 1250 行的 canvas.rs 按职责
//! 切开，一行逻辑没改；判据是 65 张画廊图与 21 页整帧快照逐字节不变。
use super::*;

impl Canvas {
    /// 绘制路径
    pub fn draw_path(&mut self, path: &Path, paint: &Paint) {
        let mut contours = path.flatten(1.0);

        // Apply translation
        let tx = self.translation.0;
        let ty = self.translation.1;
        if tx != 0.0 || ty != 0.0 {
            for contour in &mut contours {
                for p in contour {
                    p.x += tx;
                    p.y += ty;
                }
            }
        }

        match paint.style {
            PaintStyle::Fill => self.fill_path(&contours, paint),
            PaintStyle::Stroke => self.stroke_path(&contours, paint),
            PaintStyle::FillAndStroke => {
                self.fill_path(&contours, paint);
                self.stroke_path(&contours, paint);
            }
        }
    }

    /// 填充路径（扫描线算法，支持抗锯齿）
    pub(super) fn fill_path(&mut self, contours: &[Vec<Point>], paint: &Paint) {
        if contours.is_empty() { return; }

        // 找边界
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for contour in contours {
            for p in contour {
                min_y = min_y.min(p.y);
                max_y = max_y.max(p.y);
            }
        }

        let y0 = (min_y - 1.0).floor() as i32;
        let y1 = (max_y + 1.0).ceil() as i32;

        if paint.anti_alias {
            // 抗锯齿填充：纵向多条子扫描线 + 横向按区间解析求覆盖。
            // 横向本来就是连续的，纵向档数决定「接近水平的边」有多少级灰度 ——
            // 4 档时圆弧顶部/箭头斜边看得出台阶，16 档基本看不出来。
            // 代价只落在路径包围盒上（圆角矩形另有快路径，不走这里）。
            //
            // 关键：**只遍历真正被区间覆盖的那几段 x**，不要横扫包围盒。
            // 环形路径（`border` 的圆角描边就是「外圈套内圈」）中间是空的，
            // 逐像素扫过去的话，662 像素宽的卡片每行有 650+ 个像素算出覆盖率 0，
            // 白付 8 次区间比较。实测这一项占 tea-app 分类页滑动帧的一半以上
            // （354ms/606ms）。改成按区间遍历后输出不变（覆盖率为 0 的像素本来就不写）。
            //
            // 边表按上端排序、逐行维护「活动边」：从前每条子扫描线都把所有轮廓的
            // 所有边扫一遍（圆角环展平后上百条边 × 4 条子扫描线 × 每行），
            // 而真正跨过这一行的通常只有四五条。行列范围也先与裁剪区求交 ——
            // 损伤区帧里一个高卡片的边框环大部分行都在裁剪区外，算完覆盖率再被
            // `set_pixel` 丢掉是白算。两项都不改变任何像素的结果。
            const SUB_SAMPLES: usize = 4;
            let mut edges: Vec<(Point, Point, f32, f32)> = Vec::new();
            for contour in contours {
                for i in 0..contour.len() {
                    let p0 = contour[i];
                    let p1 = contour[(i + 1) % contour.len()];
                    // 水平边与任何扫描线都不满足下面的相交条件
                    if p0.y != p1.y {
                        edges.push((p0, p1, p0.y.min(p1.y), p0.y.max(p1.y)));
                    }
                }
            }
            edges.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
            let (bx0, bx1, by0, by1) = self.draw_bounds();
            let mut next_edge = 0usize;
            let mut active: Vec<usize> = Vec::new();
            let mut all_intersections: Vec<Vec<f32>> = vec![Vec::new(); SUB_SAMPLES];
            let mut segments: Vec<(f32, f32)> = Vec::new();
            for y in y0.max(by0)..=y1.min(by1 - 1) {
                let (row_top, row_bottom) = (y as f32, y as f32 + 1.0);
                while next_edge < edges.len() && edges[next_edge].2 < row_bottom {
                    active.push(next_edge);
                    next_edge += 1;
                }
                active.retain(|&i| edges[i].3 > row_top);
                segments.clear();
                for (sub, intersections) in all_intersections.iter_mut().enumerate() {
                    intersections.clear();
                    let scan_y = y as f32 + (sub as f32 + 0.5) / SUB_SAMPLES as f32;

                    for &i in &active {
                        let (p0, p1, _, _) = &edges[i];
                        if (p0.y <= scan_y && p1.y > scan_y) || (p1.y <= scan_y && p0.y > scan_y) {
                            let t = (scan_y - p0.y) / (p1.y - p0.y);
                            let x = p0.x + t * (p1.x - p0.x);
                            intersections.push(x);
                        }
                    }

                    intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    for pair in intersections.chunks(2) {
                        if let [l, r] = pair {
                            if r > l {
                                segments.push((*l, *r));
                            }
                        }
                    }
                }
                if segments.is_empty() {
                    continue;
                }
                // 合并重叠（含各留 1px 余量后仍相接）的区间，保证同一像素只访问一次
                segments.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                let mut merged: Vec<(f32, f32)> = Vec::with_capacity(segments.len());
                for seg in segments.iter().copied() {
                    match merged.last_mut() {
                        Some(last) if seg.0 <= last.1 + 2.0 => last.1 = last.1.max(seg.1),
                        _ => merged.push(seg),
                    }
                }

                for (seg_l, seg_r) in merged {
                    let sx0 = ((seg_l - 1.0).floor() as i32).max(bx0);
                    let sx1 = ((seg_r + 1.0).ceil() as i32).min(bx1 - 1);
                    for x in sx0..=sx1 {
                        let px = x as f32;
                        let mut coverage = 0.0;

                        // 计算每个子扫描线的覆盖
                        for intersections in &all_intersections {
                            for pair in intersections.chunks(2) {
                                if pair.len() == 2 {
                                    let left = pair[0];
                                    let right = pair[1];

                                    // 计算这个像素在这个区间的覆盖
                                    let pixel_left = px;
                                    let pixel_right = px + 1.0;

                                    if pixel_right <= left || pixel_left >= right {
                                        // 完全在区间外
                                        continue;
                                    } else if pixel_left >= left && pixel_right <= right {
                                        // 完全在区间内
                                        coverage += 1.0;
                                    } else {
                                        // 部分覆盖
                                        let overlap_left = pixel_left.max(left);
                                        let overlap_right = pixel_right.min(right);
                                        coverage += overlap_right - overlap_left;
                                    }
                                }
                            }
                        }

                        coverage /= SUB_SAMPLES as f32;

                        if coverage > 0.0 {
                            self.set_pixel_aa(x, y, paint.color, coverage.min(1.0));
                        }
                    }
                }
            }
        } else {
            // 非抗锯齿填充
            for y in y0..=y1 {
                let mut intersections = Vec::new();
                let scan_y = y as f32 + 0.5;

                for contour in contours {
                    for i in 0..contour.len() {
                        let p0 = &contour[i];
                        let p1 = &contour[(i + 1) % contour.len()];

                        if (p0.y <= scan_y && p1.y > scan_y) || (p1.y <= scan_y && p0.y > scan_y) {
                            let t = (scan_y - p0.y) / (p1.y - p0.y);
                            let x = p0.x + t * (p1.x - p0.x);
                            intersections.push(x);
                        }
                    }
                }

                intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());

                for pair in intersections.chunks(2) {
                    if pair.len() == 2 {
                        let x0 = pair[0].floor() as i32;
                        let x1 = pair[1].ceil() as i32;
                        for x in x0..=x1 {
                            self.set_pixel(x, y, paint.color);
                        }
                    }
                }
            }
        }
    }

    /// 描边路径
    ///
    /// 细线（≤1.5px）用抗锯齿 Wu 直线；粗线按 `stroke_width` 展开为填充的四边形段
    /// + 顶点圆角关节，得到宽度精确、边缘平滑的描边（修复此前粗描边退化成 1px 锯齿线）。
    /// 注意：传入的 contours 已在 draw_path 中应用过平移，这里不再重复平移。
    pub(super) fn stroke_path(&mut self, contours: &[Vec<Point>], paint: &Paint) {
        if paint.stroke_width <= 1.5 {
            for contour in contours {
                for i in 0..contour.len().saturating_sub(1) {
                    // draw_line 会再次应用平移，这里先抵消（contours 已平移）
                    let (tx, ty) = self.translation;
                    self.translation = (0.0, 0.0);
                    self.draw_line(contour[i].x, contour[i].y, contour[i + 1].x, contour[i + 1].y, paint);
                    self.translation = (tx, ty);
                }
            }
            return;
        }

        let hw = (paint.stroke_width * 0.5).max(0.5);
        let fill = Paint::new()
            .with_color(paint.color)
            .with_style(PaintStyle::Fill)
            .with_anti_alias(paint.anti_alias);

        for contour in contours {
            if contour.len() < 2 { continue; }
            for i in 0..contour.len() - 1 {
                let p0 = contour[i];
                let p1 = contour[i + 1];
                let dx = p1.x - p0.x;
                let dy = p1.y - p0.y;
                let len = (dx * dx + dy * dy).sqrt();
                if len < 1e-4 { continue; }
                let (nx, ny) = (-dy / len * hw, dx / len * hw);
                // 段矩形（四个角，已在平移后的坐标系）
                let quad = vec![
                    Point::new(p0.x + nx, p0.y + ny),
                    Point::new(p1.x + nx, p1.y + ny),
                    Point::new(p1.x - nx, p1.y - ny),
                    Point::new(p0.x - nx, p0.y - ny),
                ];
                self.fill_path(&[quad], &fill);
                // 顶点圆角关节，填补相邻段之间的缝隙（覆盖接缝，避免 AA 双混色）
                self.fill_stroke_joint(p1.x, p1.y, hw, &fill);
            }
        }
    }

    /// 在描边顶点填充一个圆形关节（16 边形近似），不应用平移。
    pub(super) fn fill_stroke_joint(&mut self, cx: f32, cy: f32, r: f32, fill: &Paint) {
        const N: usize = 16;
        let mut poly = Vec::with_capacity(N);
        for i in 0..N {
            let a = i as f32 / N as f32 * std::f32::consts::TAU;
            poly.push(Point::new(cx + r * a.cos(), cy + r * a.sin()));
        }
        self.fill_path(&[poly], fill);
    }
}
