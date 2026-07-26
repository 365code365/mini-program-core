//! Canvas 画布模块 - 核心渲染接口

use crate::{Color, Paint, PaintStyle, Path, Point, Rect};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// 画布状态
#[derive(Clone)]
struct CanvasState {
    clip_rect: Option<Rect>,
    translation: (f32, f32),
}

/// 画布 - 主要渲染接口
pub struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<Color>,
    clip_rect: Option<Rect>,
    translation: (f32, f32),
    state_stack: Vec<CanvasState>,
}

/// 图片缩放结果缓存（key 见 `Canvas::draw_image_cached`）
static SCALED_IMAGE_CACHE: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<Vec<Color>>>>,
> = std::sync::OnceLock::new();

fn scaled_image_cache(
) -> &'static std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<Vec<Color>>>> {
    SCALED_IMAGE_CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// 清空图片缩放缓存（切换小程序 / 内存压力时调用）
pub fn clear_scaled_image_cache() {
    if let Ok(mut g) = scaled_image_cache().lock() {
        g.clear();
    }
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![Color::TRANSPARENT; (width * height) as usize],
            clip_rect: None,
            translation: (0.0, 0.0),
            state_stack: Vec::new(),
        }
    }

    /// 保存当前状态（裁剪区域和变换）
    pub fn save(&mut self) {
        self.state_stack.push(CanvasState {
            clip_rect: self.clip_rect,
            translation: self.translation,
        });
    }

    /// 恢复上一次保存的状态
    pub fn restore(&mut self) {
        if let Some(state) = self.state_stack.pop() {
            self.clip_rect = state.clip_rect;
            self.translation = state.translation;
        }
    }

    /// 平移坐标系
    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.translation.0 += dx;
        self.translation.1 += dy;
    }

    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }
    
    /// 获取像素数据引用
    pub fn pixels(&self) -> &[Color] {
        &self.pixels
    }
    
    /// 获取像素数据可变引用
    pub fn pixels_mut(&mut self) -> &mut [Color] {
        &mut self.pixels
    }
    
    /// 从另一个 Canvas 复制像素数据
    pub fn copy_from(&mut self, src: &Canvas) {
        let copy_len = self.pixels.len().min(src.pixels.len());
        self.pixels[..copy_len].copy_from_slice(&src.pixels[..copy_len]);
    }

    /// 清空画布
    pub fn clear(&mut self, color: Color) {
        self.pixels.fill(color);
    }

    /// 设置裁剪区域
    pub fn clip_rect(&mut self, rect: Rect) {
        // Intersect with existing clip rect if any
        if let Some(current) = self.clip_rect {
            let x = current.x.max(rect.x);
            let y = current.y.max(rect.y);
            let right = current.right().min(rect.right());
            let bottom = current.bottom().min(rect.bottom());
            
            if right > x && bottom > y {
                self.clip_rect = Some(Rect::new(x, y, right - x, bottom - y));
            } else {
                // No intersection, empty rect
                self.clip_rect = Some(Rect::new(0.0, 0.0, 0.0, 0.0));
            }
        } else {
            self.clip_rect = Some(rect);
        }
    }

    /// 重置裁剪区域
    pub fn reset_clip(&mut self) {
        self.clip_rect = None;
    }

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

    /// 设置像素（带抗锯齿 coverage）
    fn set_pixel_aa(&mut self, x: i32, y: i32, color: Color, coverage: f32) {
        if coverage <= 0.0 { return; }
        let a = (color.a as f32 * coverage.min(1.0)) as u8;
        self.set_pixel(x, y, Color::new(color.r, color.g, color.b, a));
    }

    /// 绘制矩形
    pub fn draw_rect(&mut self, rect: &Rect, paint: &Paint) {
        match paint.style {
            PaintStyle::Fill => self.fill_rect(rect, &paint.color),
            PaintStyle::Stroke => self.stroke_rect(rect, paint),
            PaintStyle::FillAndStroke => {
                self.fill_rect(rect, &paint.color);
                self.stroke_rect(rect, paint);
            }
        }
    }

    /// 快速填充圆角矩形：实心内部用整块矩形填充，仅四个圆角做抗锯齿。
    /// 相比通用 4x 超采样扫描线填充（遍历整块面积）快一个数量级，
    /// 是圆角背景/卡片大量出现时的关键性能优化。
    pub fn fill_round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
        if w <= 0.0 || h <= 0.0 { return; }
        let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
        if r < 0.6 {
            self.fill_rect(&Rect::new(x, y, w, h), &color);
            return;
        }
        // 内部实心（中间整宽条 + 上下去角条），避开四角
        self.fill_rect(&Rect::new(x, y + r, w, h - 2.0 * r), &color);
        self.fill_rect(&Rect::new(x + r, y, w - 2.0 * r, r), &color);
        self.fill_rect(&Rect::new(x + r, y + h - r, w - 2.0 * r, r), &color);
        // 四角抗锯齿（仅各自 r×r 的角区，不与内部重叠）
        let (tx, ty) = self.translation;
        let corners = [
            (x + r, y + r, x, y),                 // 左上：角区 [x, x+r]
            (x + w - r, y + r, x + w - r, y),      // 右上
            (x + w - r, y + h - r, x + w - r, y + h - r), // 右下
            (x + r, y + h - r, x, y + h - r),      // 左下
        ];
        for &(cx, cy, bx, by) in &corners {
            let x0 = (bx + tx).floor() as i32;
            let y0 = (by + ty).floor() as i32;
            let x1 = (bx + tx + r).ceil() as i32;
            let y1 = (by + ty + r).ceil() as i32;
            let (ccx, ccy) = (cx + tx, cy + ty);
            for py in y0..y1 {
                for px in x0..x1 {
                    let dx = px as f32 + 0.5 - ccx;
                    let dy = py as f32 + 0.5 - ccy;
                    let d = (dx * dx + dy * dy).sqrt();
                    let cov = (r + 0.5 - d).clamp(0.0, 1.0);
                    if cov > 0.0 {
                        self.set_pixel_aa(px, py, color, cov);
                    }
                }
            }
        }
    }

    fn fill_rect(&mut self, rect: &Rect, color: &Color) {
        let tx = self.translation.0;
        let ty = self.translation.1;
        
        let mut x0 = (rect.x + tx).max(0.0) as i32;
        let mut y0 = (rect.y + ty).max(0.0) as i32;
        let mut x1 = (rect.right() + tx).min(self.width as f32) as i32;
        let mut y1 = (rect.bottom() + ty).min(self.height as f32) as i32;

        // 先把裁剪区并进矩形范围，这样逐像素时不必再判裁剪
        if let Some(clip) = &self.clip_rect {
            x0 = x0.max(clip.x as i32);
            y0 = y0.max(clip.y as i32);
            x1 = x1.min(clip.right() as i32);
            y1 = y1.min(clip.bottom() as i32);
        }
        if x1 <= x0 || y1 <= y0 {
            return;
        }

        // 不透明实色：整行 memset，跳过逐像素的越界/裁剪判断与混色。
        // 卡片背景、色块这类大面积填充占了绘制耗时的大头，行填充比逐像素快一个量级。
        if color.a == 255 {
            let width = self.width as usize;
            for y in y0..y1 {
                let start = y as usize * width + x0 as usize;
                let end = y as usize * width + x1 as usize;
                self.pixels[start..end].fill(*color);
            }
            return;
        }

        for y in y0..y1 {
            let row = y as usize * self.width as usize;
            for x in x0..x1 {
                let idx = row + x as usize;
                self.pixels[idx] = color.blend(&self.pixels[idx]);
            }
        }
    }

    fn stroke_rect(&mut self, rect: &Rect, paint: &Paint) {
        let w = paint.stroke_width;
        // 上边
        self.fill_rect(&Rect::new(rect.x, rect.y, rect.width, w), &paint.color);
        // 下边
        self.fill_rect(&Rect::new(rect.x, rect.bottom() - w, rect.width, w), &paint.color);
        // 左边
        self.fill_rect(&Rect::new(rect.x, rect.y, w, rect.height), &paint.color);
        // 右边
        self.fill_rect(&Rect::new(rect.right() - w, rect.y, w, rect.height), &paint.color);
    }

    /// 绘制圆形
    pub fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, paint: &Paint) {
        match paint.style {
            PaintStyle::Fill => self.fill_circle(cx, cy, radius, paint),
            PaintStyle::Stroke => self.stroke_circle(cx, cy, radius, paint),
            PaintStyle::FillAndStroke => {
                self.fill_circle(cx, cy, radius, paint);
                self.stroke_circle(cx, cy, radius, paint);
            }
        }
    }

    fn fill_circle(&mut self, cx: f32, cy: f32, radius: f32, paint: &Paint) {
        let cx = cx + self.translation.0;
        let cy = cy + self.translation.1;

        let r2 = radius * radius;
        let x0 = (cx - radius - 1.0).max(0.0) as i32;
        let y0 = (cy - radius - 1.0).max(0.0) as i32;
        let x1 = (cx + radius + 1.0).min(self.width as f32) as i32;
        let y1 = (cy + radius + 1.0).min(self.height as f32) as i32;

        for y in y0..y1 {
            for x in x0..x1 {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let d2 = dx * dx + dy * dy;

                if paint.anti_alias {
                    let d = d2.sqrt();
                    if d <= radius + 0.5 {
                        let coverage = (radius + 0.5 - d).min(1.0);
                        self.set_pixel_aa(x, y, paint.color, coverage);
                    }
                } else if d2 <= r2 {
                    self.set_pixel(x, y, paint.color);
                }
            }
        }
    }

    fn stroke_circle(&mut self, cx: f32, cy: f32, radius: f32, paint: &Paint) {
        let cx = cx + self.translation.0;
        let cy = cy + self.translation.1;

        let inner = radius - paint.stroke_width / 2.0;
        let outer = radius + paint.stroke_width / 2.0;

        let x0 = (cx - outer - 1.0).max(0.0) as i32;
        let y0 = (cy - outer - 1.0).max(0.0) as i32;
        let x1 = (cx + outer + 1.0).min(self.width as f32) as i32;
        let y1 = (cy + outer + 1.0).min(self.height as f32) as i32;

        for y in y0..y1 {
            for x in x0..x1 {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let d = (dx * dx + dy * dy).sqrt();

                if d >= inner && d <= outer {
                    if paint.anti_alias {
                        let coverage = (1.0 - (d - inner).abs().min(outer - d).min(1.0)).max(0.0);
                        let coverage = if d < inner + 0.5 { d - inner + 0.5 }
                                      else if d > outer - 0.5 { outer - d + 0.5 }
                                      else { 1.0 };
                        self.set_pixel_aa(x, y, paint.color, coverage.min(1.0));
                    } else {
                        self.set_pixel(x, y, paint.color);
                    }
                }
            }
        }
    }

    /// 绘制线段
    pub fn draw_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, paint: &Paint) {
        let x0 = x0 + self.translation.0;
        let y0 = y0 + self.translation.1;
        let x1 = x1 + self.translation.0;
        let y1 = y1 + self.translation.1;

        if paint.anti_alias {
            self.draw_line_aa(x0, y0, x1, y1, paint);
        } else {
            self.draw_line_bresenham(x0 as i32, y0 as i32, x1 as i32, y1 as i32, paint);
        }
    }

    /// Bresenham 直线算法
    fn draw_line_bresenham(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, paint: &Paint) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.set_pixel(x0, y0, paint.color);
            if x0 == x1 && y0 == y1 { break; }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// 抗锯齿直线 (Wu's algorithm)
    fn draw_line_aa(&mut self, mut x0: f32, mut y0: f32, mut x1: f32, mut y1: f32, paint: &Paint) {
        let steep = (y1 - y0).abs() > (x1 - x0).abs();
        if steep {
            std::mem::swap(&mut x0, &mut y0);
            std::mem::swap(&mut x1, &mut y1);
        }
        if x0 > x1 {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }

        let dx = x1 - x0;
        let dy = y1 - y0;
        let gradient = if dx == 0.0 { 1.0 } else { dy / dx };

        // 起点
        let xend = x0.round();
        let yend = y0 + gradient * (xend - x0);
        let xpxl1 = xend as i32;
        let mut intery = yend + gradient;

        // 终点
        let xend = x1.round();
        let xpxl2 = xend as i32;

        for x in xpxl1..=xpxl2 {
            let y = intery.floor() as i32;
            let frac = intery - intery.floor();

            if steep {
                self.set_pixel_aa(y, x, paint.color, 1.0 - frac);
                self.set_pixel_aa(y + 1, x, paint.color, frac);
            } else {
                self.set_pixel_aa(x, y, paint.color, 1.0 - frac);
                self.set_pixel_aa(x, y + 1, paint.color, frac);
            }
            intery += gradient;
        }
    }

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
    fn fill_path(&mut self, contours: &[Vec<Point>], paint: &Paint) {
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
            // 抗锯齿填充 - 使用边缘覆盖率计算
            for y in y0..=y1 {
                // 收集多个子扫描线的交点
                let sub_samples = 4;
                let mut all_intersections: Vec<Vec<f32>> = Vec::new();
                
                for sub in 0..sub_samples {
                    let scan_y = y as f32 + (sub as f32 + 0.5) / sub_samples as f32;
                    let mut intersections = Vec::new();
                    
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
                    all_intersections.push(intersections);
                }
                
                // 找出所有交点的 x 范围
                let mut x_min = f32::MAX;
                let mut x_max = f32::MIN;
                for intersections in &all_intersections {
                    for &x in intersections {
                        x_min = x_min.min(x);
                        x_max = x_max.max(x);
                    }
                }
                
                if x_min > x_max { continue; }
                
                let x0 = (x_min - 1.0).floor() as i32;
                let x1 = (x_max + 1.0).ceil() as i32;
                
                for x in x0..=x1 {
                    let px = x as f32;
                    let mut coverage = 0.0;
                    
                    // 计算每个子扫描线的覆盖
                    for intersections in &all_intersections {
                        let mut sub_coverage = 0.0;
                        
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
                                    sub_coverage += 1.0;
                                } else {
                                    // 部分覆盖
                                    let overlap_left = pixel_left.max(left);
                                    let overlap_right = pixel_right.min(right);
                                    sub_coverage += overlap_right - overlap_left;
                                }
                            }
                        }
                        
                        coverage += sub_coverage;
                    }
                    
                    coverage /= sub_samples as f32;
                    
                    if coverage > 0.0 {
                        self.set_pixel_aa(x, y, paint.color, coverage.min(1.0));
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
    fn stroke_path(&mut self, contours: &[Vec<Point>], paint: &Paint) {
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
    fn fill_stroke_joint(&mut self, cx: f32, cy: f32, r: f32, fill: &Paint) {
        const N: usize = 16;
        let mut poly = Vec::with_capacity(N);
        for i in 0..N {
            let a = i as f32 / N as f32 * std::f32::consts::TAU;
            poly.push(Point::new(cx + r * a.cos(), cy + r * a.sin()));
        }
        self.fill_path(&[poly], fill);
    }

    /// 导出为 RGBA 字节数组
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity((self.width * self.height * 4) as usize);
        for pixel in &self.pixels {
            data.push(pixel.r);
            data.push(pixel.g);
            data.push(pixel.b);
            data.push(pixel.a);
        }
        data
    }

    /// 保存为 PNG
    pub fn save_png(&self, path: &str) -> Result<(), String> {
        use image::{ImageBuffer, Rgba};

        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(
            self.width,
            self.height,
            self.to_rgba()
        ).ok_or("Failed to create image buffer")?;

        img.save(path).map_err(|e| e.to_string())
    }

    /// 直接设置像素（供文本渲染使用）
    pub fn set_pixel_direct(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32 {
            let idx = (y as u32 * self.width + x as u32) as usize;
            self.pixels[idx] = color;
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
                for row in 0..dst_h {
                    let src_row = (row * dst_w) as usize;
                    for col in 0..dst_w {
                        let color = pixels[src_row + col as usize];
                        if color.a > 0 {
                            self.set_pixel(base_x + col as i32, base_y + row as i32, color);
                        }
                    }
                }
            }
            None => self.draw_image(img_data, img_w, img_h, x, y, w, h, mode, radius),
        }
    }

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
        let (scale_x, scale_y, offset_x, offset_y) = match mode {
            "aspectFit" => {
                // 保持比例，完整显示，可能有留白
                let scale = (w / img_w as f32).min(h / img_h as f32);
                let scaled_w = img_w as f32 * scale;
                let scaled_h = img_h as f32 * scale;
                let ox = (w - scaled_w) / 2.0;
                let oy = (h - scaled_h) / 2.0;
                (scale, scale, ox, oy)
            }
            "aspectFill" => {
                // 保持比例，填满区域，可能裁剪
                let scale = (w / img_w as f32).max(h / img_h as f32);
                let scaled_w = img_w as f32 * scale;
                let scaled_h = img_h as f32 * scale;
                let ox = (w - scaled_w) / 2.0;
                let oy = (h - scaled_h) / 2.0;
                (scale, scale, ox, oy)
            }
            _ => {
                // scaleToFill: 拉伸填满
                (w / img_w as f32, h / img_h as f32, 0.0, 0.0)
            }
        };

        let dest_x0 = x as i32;
        let dest_y0 = y as i32;
        let dest_x1 = (x + w) as i32;
        let dest_y1 = (y + h) as i32;

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

                let c00 = sample(src_x, src_y);
                let c10 = sample(src_x + 1, src_y);
                let c01 = sample(src_x, src_y + 1);
                let c11 = sample(src_x + 1, src_y + 1);

                let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
                let r = lerp(lerp(c00.0, c10.0, fx), lerp(c01.0, c11.0, fx), fy) as u8;
                let g = lerp(lerp(c00.1, c10.1, fx), lerp(c01.1, c11.1, fx), fy) as u8;
                let b = lerp(lerp(c00.2, c10.2, fx), lerp(c01.2, c11.2, fx), fy) as u8;
                let a_f = lerp(lerp(c00.3, c10.3, fx), lerp(c01.3, c11.3, fx), fy);
                // 圆角边缘按覆盖率淡出 alpha，实现抗锯齿
                let a = (a_f * corner_cover) as u8;

                self.set_pixel(dest_x, dest_y, Color::new(r, g, b, a));
            }
        }
    }
}
