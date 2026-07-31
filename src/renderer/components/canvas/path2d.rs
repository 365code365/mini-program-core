//! 路径构建：moveTo/lineTo/arc/贝塞尔，以及转成引擎的 Path
//!
//! `Canvas2DContext` 的一片，由 `canvas2d/mod.rs` 组合。**纯搬迁**：从 1132 行的
//! components/canvas.rs 按职责切开，一行逻辑没改。
use super::*;

impl Canvas2DContext {
    /// 开始新路径
    pub fn begin_path(&mut self) {
        self.current_path.clear();
    }

    /// 关闭路径
    pub fn close_path(&mut self) {
        self.current_path.push(PathCommand::ClosePath);
    }

    /// 移动到指定点
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.current_path.push(PathCommand::MoveTo(x, y));
    }

    /// 绘制直线到指定点
    pub fn line_to(&mut self, x: f32, y: f32) {
        self.current_path.push(PathCommand::LineTo(x, y));
    }

    /// 绘制圆弧
    pub fn arc(&mut self, x: f32, y: f32, radius: f32, start_angle: f32, end_angle: f32, counter_clockwise: bool) {
        self.current_path.push(PathCommand::Arc(x, y, radius, start_angle, end_angle, counter_clockwise));
    }

    /// 绘制二次贝塞尔曲线
    pub fn quadratic_curve_to(&mut self, cpx: f32, cpy: f32, x: f32, y: f32) {
        self.current_path.push(PathCommand::QuadraticCurveTo(cpx, cpy, x, y));
    }

    /// 绘制三次贝塞尔曲线
    pub fn bezier_curve_to(&mut self, cp1x: f32, cp1y: f32, cp2x: f32, cp2y: f32, x: f32, y: f32) {
        self.current_path.push(PathCommand::BezierCurveTo(cp1x, cp1y, cp2x, cp2y, x, y));
    }

    /// 添加矩形路径
    pub fn rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.current_path.push(PathCommand::Rect(x, y, width, height));
    }

    /// 构建 Path 对象（所有坐标经当前仿射矩阵变换）
    pub(super) fn build_path(&self) -> Path {
        let mut path = Path::new();
        for cmd in &self.current_path {
            match cmd {
                PathCommand::MoveTo(x, y) => { let (px, py) = self.tp(*x, *y); path.move_to(px, py); }
                PathCommand::LineTo(x, y) => { let (px, py) = self.tp(*x, *y); path.line_to(px, py); }
                PathCommand::Arc(x, y, r, start, end, ccw) => {
                    // 以变换后圆心 + 平均缩放半径近似（不支持椭圆化的斜切）
                    let (cx, cy) = self.tp(*x, *y);
                    path.arc(cx, cy, *r * self.avg_scale(), *start, *end, *ccw);
                }
                PathCommand::QuadraticCurveTo(cpx, cpy, x, y) => {
                    let (c0, c1) = self.tp(*cpx, *cpy);
                    let (px, py) = self.tp(*x, *y);
                    path.quad_to(c0, c1, px, py);
                }
                PathCommand::BezierCurveTo(cp1x, cp1y, cp2x, cp2y, x, y) => {
                    let (a0, a1) = self.tp(*cp1x, *cp1y);
                    let (b0, b1) = self.tp(*cp2x, *cp2y);
                    let (px, py) = self.tp(*x, *y);
                    path.cubic_to(a0, a1, b0, b1, px, py);
                }
                PathCommand::Rect(x, y, w, h) => {
                    let p0 = self.tp(*x, *y);
                    let p1 = self.tp(*x + *w, *y);
                    let p2 = self.tp(*x + *w, *y + *h);
                    let p3 = self.tp(*x, *y + *h);
                    path.move_to(p0.0, p0.1);
                    path.line_to(p1.0, p1.1);
                    path.line_to(p2.0, p2.1);
                    path.line_to(p3.0, p3.1);
                    path.close();
                }
                PathCommand::ClosePath => { path.close(); }
            }
        }
        path
    }

    // ========== 圆形绘制 ==========
}
