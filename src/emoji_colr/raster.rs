//! COLR 字形用的面积覆盖光栅化（与 font-rs / fontdue 同一种「有符号面积累加」算法）。
//!
//! 为什么不复用 `canvas/paths.rs`：那边是 even-odd + 子扫描线，按 Canvas 的语义写的；字形轮廓要 **nonzero**，
//! 而且 COLR 一个 emoji 要光栅化几十个图层 / 裁剪路径，累加法每条边只扫一遍、覆盖率是精确面积，便宜又平滑。
//! 覆盖率取 `min(1, |累加值|)`：重叠的同向轮廓不会叠出大于 1 的覆盖（等价于 nonzero）。
//!
//! 每行多留两列（stride = w + 2），x 先夹到 [0, w]：左边越界的边落在第 0 列，右边越界的落在多出的列里，
//! 按行做前缀和，行与行互不串扰。y 越界的行直接跳过。

/// 一组闭合轮廓（像素坐标，y 向下）
pub type Contours = Vec<Vec<(f32, f32)>>;

/// 把轮廓光栅化成 w×h 的覆盖率（0..=1）
pub fn coverage(contours: &Contours, w: usize, h: usize) -> Vec<f32> {
    let stride = w + 2;
    let mut acc = vec![0f32; stride * h];
    for c in contours {
        if c.len() < 2 {
            continue;
        }
        for i in 0..c.len() {
            let p0 = c[i];
            let p1 = c[(i + 1) % c.len()];
            line(&mut acc, stride, w, h, p0, p1);
        }
    }
    let mut out = vec![0f32; w * h];
    for y in 0..h {
        let mut sum = 0f32;
        let row = &acc[y * stride..(y + 1) * stride];
        for x in 0..w {
            sum += row[x];
            out[y * w + x] = sum.abs().min(1.0);
        }
    }
    out
}

fn line(acc: &mut [f32], stride: usize, w: usize, h: usize, p0: (f32, f32), p1: (f32, f32)) {
    let wf = w as f32;
    let (mut p0, mut p1) = ((p0.0.clamp(0.0, wf), p0.1), (p1.0.clamp(0.0, wf), p1.1));
    if p0.1 == p1.1 || !p0.1.is_finite() || !p1.1.is_finite() {
        return;
    }
    let dir = if p0.1 < p1.1 { 1.0 } else { std::mem::swap(&mut p0, &mut p1); -1.0 };
    let dxdy = (p1.0 - p0.0) / (p1.1 - p0.1);
    let y_start = p0.1.max(0.0);
    let y_end = p1.1.min(h as f32);
    if y_start >= y_end {
        return;
    }
    let mut x = p0.0 + (y_start - p0.1) * dxdy;
    let mut y = y_start.floor() as usize;
    while (y as f32) < y_end {
        let top = (y as f32).max(y_start);
        let bottom = ((y + 1) as f32).min(y_end);
        let dy = bottom - top;
        let xnext = x + dxdy * dy;
        let d = dy * dir;
        let (x0, x1) = if x < xnext { (x, xnext) } else { (xnext, x) };
        let base = y * stride;
        let x0f = x0.floor();
        let x0i = x0f as usize;
        let x1c = x1.ceil();
        let x1i = x1c as usize;
        if x1i <= x0i + 1 {
            // 这一段只跨一个像素：按中点分给这一格与右边一格
            let xm = 0.5 * (x + xnext) - x0f;
            acc[base + x0i] += d - d * xm;
            acc[base + x0i + 1] += d * xm;
        } else {
            let s = 1.0 / (x1 - x0);
            let fx0 = x0 - x0f;
            let a0 = 0.5 * s * (1.0 - fx0) * (1.0 - fx0);
            let fx1 = x1 - x1c + 1.0;
            let am = 0.5 * s * fx1 * fx1;
            acc[base + x0i] += d * a0;
            if x1i == x0i + 2 {
                acc[base + x0i + 1] += d * (1.0 - a0 - am);
            } else {
                let a1 = s * (1.5 - fx0);
                acc[base + x0i + 1] += d * (a1 - a0);
                for xi in x0i + 2..x1i - 1 {
                    acc[base + xi] += d * s;
                }
                let a2 = a1 + (x1i - x0i - 3) as f32 * s;
                acc[base + x1i - 1] += d * (1.0 - a2 - am);
            }
            acc[base + x1i] += d * am;
        }
        x = xnext;
        y += 1;
    }
}

/// 2D 仿射：(x, y) → (a·x + c·y + e, b·x + d·y + f)，与 ttf-parser 的 `Transform` 同一套分量
#[derive(Clone, Copy, Debug)]
pub struct Affine {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Affine {
    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }
    /// self ∘ t：先做 t，再做 self
    pub fn then_inner(&self, t: &Affine) -> Affine {
        Affine {
            a: self.a * t.a + self.c * t.b,
            b: self.b * t.a + self.d * t.b,
            c: self.a * t.c + self.c * t.d,
            d: self.b * t.c + self.d * t.d,
            e: self.a * t.e + self.c * t.f + self.e,
            f: self.b * t.e + self.d * t.f + self.f,
        }
    }
    pub fn invert(&self) -> Option<Affine> {
        let det = self.a * self.d - self.b * self.c;
        if det.abs() < 1e-12 {
            return None;
        }
        let i = 1.0 / det;
        Some(Affine {
            a: self.d * i,
            b: -self.b * i,
            c: -self.c * i,
            d: self.a * i,
            e: (self.c * self.f - self.d * self.e) * i,
            f: (self.b * self.e - self.a * self.f) * i,
        })
    }
}

/// 收集字形轮廓（ttf-parser 的 OutlineBuilder），按给定仿射变到像素坐标，曲线展平成折线
pub struct Flatten {
    pub m: Affine,
    pub out: Contours,
    cur: Vec<(f32, f32)>,
    last: (f32, f32),
}

impl Flatten {
    pub fn new(m: Affine) -> Self {
        Self { m, out: Vec::new(), cur: Vec::new(), last: (0.0, 0.0) }
    }
    fn push(&mut self, p: (f32, f32)) {
        self.cur.push(p);
        self.last = p;
    }
    /// 展平段数：控制点偏离弦的距离（像素）开方，0.2px 以内误差
    fn steps(p0: (f32, f32), c: (f32, f32), p1: (f32, f32)) -> usize {
        let mx = (p0.0 + p1.0) * 0.5 - c.0;
        let my = (p0.1 + p1.1) * 0.5 - c.1;
        let dev = (mx * mx + my * my).sqrt();
        ((dev / 0.2).sqrt().ceil() as usize).clamp(1, 32)
    }
    pub fn finish(mut self) -> Contours {
        if self.cur.len() > 1 {
            self.out.push(std::mem::take(&mut self.cur));
        }
        self.out
    }
}

impl ttf_parser::OutlineBuilder for Flatten {
    fn move_to(&mut self, x: f32, y: f32) {
        if self.cur.len() > 1 {
            self.out.push(std::mem::take(&mut self.cur));
        }
        self.cur.clear();
        let p = self.m.apply(x, y);
        self.push(p);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.m.apply(x, y);
        self.push(p);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (p0, c, p1) = (self.last, self.m.apply(x1, y1), self.m.apply(x, y));
        let n = Self::steps(p0, c, p1);
        for i in 1..=n {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            self.push((u * u * p0.0 + 2.0 * u * t * c.0 + t * t * p1.0, u * u * p0.1 + 2.0 * u * t * c.1 + t * t * p1.1));
        }
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (p0, c1, c2, p1) = (self.last, self.m.apply(x1, y1), self.m.apply(x2, y2), self.m.apply(x, y));
        let mid = ((c1.0 + c2.0) * 0.5, (c1.1 + c2.1) * 0.5);
        let n = (Self::steps(p0, mid, p1) * 2).min(48);
        for i in 1..=n {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            self.push((a * p0.0 + b * c1.0 + c * c2.0 + d * p1.0, a * p0.1 + b * c1.1 + c * c2.1 + d * p1.1));
        }
    }
    fn close(&mut self) {
        if self.cur.len() > 1 {
            self.out.push(std::mem::take(&mut self.cur));
        }
    }
}
