//! 路径模块 - 支持贝塞尔曲线等复杂路径

use crate::geometry::Point;

/// 路径命令
#[derive(Debug, Clone)]
pub enum PathCommand {
    MoveTo(Point),
    LineTo(Point),
    QuadTo(Point, Point),      // 控制点, 终点
    CubicTo(Point, Point, Point), // 控制点1, 控制点2, 终点
    Close,
}

/// 路径
#[derive(Debug, Clone, Default)]
pub struct Path {
    commands: Vec<PathCommand>,
    current: Point,
}

impl Path {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn move_to(&mut self, x: f32, y: f32) -> &mut Self {
        let p = Point::new(x, y);
        self.commands.push(PathCommand::MoveTo(p));
        self.current = p;
        self
    }

    pub fn line_to(&mut self, x: f32, y: f32) -> &mut Self {
        let p = Point::new(x, y);
        self.commands.push(PathCommand::LineTo(p));
        self.current = p;
        self
    }

    pub fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) -> &mut Self {
        let ctrl = Point::new(cx, cy);
        let end = Point::new(x, y);
        self.commands.push(PathCommand::QuadTo(ctrl, end));
        self.current = end;
        self
    }

    pub fn cubic_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) -> &mut Self {
        let c1 = Point::new(c1x, c1y);
        let c2 = Point::new(c2x, c2y);
        let end = Point::new(x, y);
        self.commands.push(PathCommand::CubicTo(c1, c2, end));
        self.current = end;
        self
    }

    pub fn close(&mut self) -> &mut Self {
        self.commands.push(PathCommand::Close);
        self
    }

    /// 添加矩形
    pub fn add_rect(&mut self, x: f32, y: f32, w: f32, h: f32) -> &mut Self {
        self.move_to(x, y)
            .line_to(x + w, y)
            .line_to(x + w, y + h)
            .line_to(x, y + h)
            .close()
    }

    /// 添加圆角矩形
    pub fn add_round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32) -> &mut Self {
        let r = r.min(w / 2.0).min(h / 2.0);
        let k = 0.5522847498; // 贝塞尔曲线近似圆弧的系数
        let kr = k * r;

        self.move_to(x + r, y);
        self.line_to(x + w - r, y);
        self.cubic_to(x + w - r + kr, y, x + w, y + r - kr, x + w, y + r);
        self.line_to(x + w, y + h - r);
        self.cubic_to(x + w, y + h - r + kr, x + w - r + kr, y + h, x + w - r, y + h);
        self.line_to(x + r, y + h);
        self.cubic_to(x + r - kr, y + h, x, y + h - r + kr, x, y + h - r);
        self.line_to(x, y + r);
        self.cubic_to(x, y + r - kr, x + r - kr, y, x + r, y);
        self.close()
    }
    
    /// 添加四角独立圆角的矩形
    pub fn add_round_rect_varying(&mut self, x: f32, y: f32, w: f32, h: f32, 
                                   tl: f32, tr: f32, br: f32, bl: f32) -> &mut Self {
        let k = 0.5522847498; // 贝塞尔曲线近似圆弧的系数
        
        // 限制圆角不超过边长的一半
        let tl = tl.min(w / 2.0).min(h / 2.0);
        let tr = tr.min(w / 2.0).min(h / 2.0);
        let br = br.min(w / 2.0).min(h / 2.0);
        let bl = bl.min(w / 2.0).min(h / 2.0);
        
        // 从左上角开始，顺时针绘制
        self.move_to(x + tl, y);
        
        // 上边 + 右上角
        self.line_to(x + w - tr, y);
        if tr > 0.0 {
            let kr = k * tr;
            self.cubic_to(x + w - tr + kr, y, x + w, y + tr - kr, x + w, y + tr);
        }
        
        // 右边 + 右下角
        self.line_to(x + w, y + h - br);
        if br > 0.0 {
            let kr = k * br;
            self.cubic_to(x + w, y + h - br + kr, x + w - br + kr, y + h, x + w - br, y + h);
        }
        
        // 下边 + 左下角
        self.line_to(x + bl, y + h);
        if bl > 0.0 {
            let kr = k * bl;
            self.cubic_to(x + bl - kr, y + h, x, y + h - bl + kr, x, y + h - bl);
        }
        
        // 左边 + 左上角
        self.line_to(x, y + tl);
        if tl > 0.0 {
            let kr = k * tl;
            self.cubic_to(x, y + tl - kr, x + tl - kr, y, x + tl, y);
        }
        
        self.close()
    }

    /// 添加椭圆
    pub fn add_oval(&mut self, cx: f32, cy: f32, rx: f32, ry: f32) -> &mut Self {
        let k = 0.5522847498;
        let kx = k * rx;
        let ky = k * ry;

        self.move_to(cx + rx, cy);
        self.cubic_to(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry);
        self.cubic_to(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy);
        self.cubic_to(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry);
        self.cubic_to(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy);
        self.close()
    }
    
    /// 添加圆形
    pub fn add_circle(&mut self, cx: f32, cy: f32, r: f32) -> &mut Self {
        self.add_oval(cx, cy, r, r)
    }
    
    /// 添加圆弧（Canvas 2D API 兼容）
    /// cx, cy: 圆心坐标
    /// radius: 半径
    /// start_angle, end_angle: 起始和结束角度（弧度）
    /// counter_clockwise: 是否逆时针
    pub fn arc(&mut self, cx: f32, cy: f32, radius: f32, start_angle: f32, end_angle: f32, counter_clockwise: bool) -> &mut Self {
        let mut start = start_angle;
        let mut end = end_angle;
        
        // 处理逆时针
        if counter_clockwise {
            std::mem::swap(&mut start, &mut end);
        }
        
        // 规范化角度
        while end < start {
            end += std::f32::consts::TAU;
        }
        
        let angle_diff = end - start;
        
        // 如果是完整的圆，使用 add_circle
        if angle_diff >= std::f32::consts::TAU - 0.001 {
            return self.add_circle(cx, cy, radius);
        }
        
        // 使用贝塞尔曲线近似圆弧
        // 每 90 度一段
        let segments = ((angle_diff / (std::f32::consts::PI / 2.0)).ceil() as usize).max(1);
        let segment_angle = angle_diff / segments as f32;
        
        // 起点
        let start_x = cx + radius * start.cos();
        let start_y = cy + radius * start.sin();
        
        if self.commands.is_empty() {
            self.move_to(start_x, start_y);
        } else {
            self.line_to(start_x, start_y);
        }
        
        // 贝塞尔曲线近似圆弧的系数
        let k = 4.0 / 3.0 * (segment_angle / 4.0).tan();
        
        let mut current_angle = start;
        for _ in 0..segments {
            let next_angle = current_angle + segment_angle;
            
            let cos_curr = current_angle.cos();
            let sin_curr = current_angle.sin();
            let cos_next = next_angle.cos();
            let sin_next = next_angle.sin();
            
            // 控制点
            let cp1x = cx + radius * (cos_curr - k * sin_curr);
            let cp1y = cy + radius * (sin_curr + k * cos_curr);
            let cp2x = cx + radius * (cos_next + k * sin_next);
            let cp2y = cy + radius * (sin_next - k * cos_next);
            
            // 终点
            let end_x = cx + radius * cos_next;
            let end_y = cy + radius * sin_next;
            
            self.cubic_to(cp1x, cp1y, cp2x, cp2y, end_x, end_y);
            
            current_angle = next_angle;
        }
        
        self
    }
    

    pub fn commands(&self) -> &[PathCommand] {
        &self.commands
    }

    /// 将路径转换为点序列（用于光栅化）
    pub fn flatten(&self, tolerance: f32) -> Vec<Vec<Point>> {
        let mut contours = Vec::new();
        let mut current_contour = Vec::new();
        let mut current = Point::default();
        let mut start = Point::default();

        for cmd in &self.commands {
            match cmd {
                PathCommand::MoveTo(p) => {
                    if !current_contour.is_empty() {
                        contours.push(std::mem::take(&mut current_contour));
                    }
                    current = *p;
                    start = *p;
                    current_contour.push(*p);
                }
                PathCommand::LineTo(p) => {
                    current_contour.push(*p);
                    current = *p;
                }
                PathCommand::QuadTo(ctrl, end) => {
                    flatten_quad(&current, ctrl, end, tolerance, &mut current_contour);
                    current = *end;
                }
                PathCommand::CubicTo(c1, c2, end) => {
                    flatten_cubic(&current, c1, c2, end, tolerance, &mut current_contour);
                    current = *end;
                }
                PathCommand::Close => {
                    if current != start {
                        current_contour.push(start);
                    }
                    current = start;
                }
            }
        }

        if !current_contour.is_empty() {
            contours.push(current_contour);
        }

        contours
    }
}

/// 二次贝塞尔曲线展平
fn flatten_quad(p0: &Point, p1: &Point, p2: &Point, tolerance: f32, out: &mut Vec<Point>) {
    let steps = ((p0.distance(p1) + p1.distance(p2)) / tolerance).ceil() as usize;
    let steps = steps.max(2).min(100);

    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let t2 = t * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;

        out.push(Point::new(
            mt2 * p0.x + 2.0 * mt * t * p1.x + t2 * p2.x,
            mt2 * p0.y + 2.0 * mt * t * p1.y + t2 * p2.y,
        ));
    }
}

/// 三次贝塞尔曲线展平
fn flatten_cubic(p0: &Point, p1: &Point, p2: &Point, p3: &Point, tolerance: f32, out: &mut Vec<Point>) {
    let steps = ((p0.distance(p1) + p1.distance(p2) + p2.distance(p3)) / tolerance).ceil() as usize;
    let steps = steps.max(2).min(100);

    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;

        out.push(Point::new(
            mt3 * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t3 * p3.x,
            mt3 * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t3 * p3.y,
        ));
    }
}
