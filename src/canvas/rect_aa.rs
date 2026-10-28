//! 轴对齐矩形的抗锯齿填充：`fill_path_aa_scan` 对这种形状的**逐字节等价**特化。
//!
//! 为什么要有：小游戏每帧几百个 `fillRect`，坐标几乎都是小数（位置 × dpr）。通用扫描线每一行都要
//! 4 条子扫描线 × 活动边求交、两次排序、一次 `Vec` 分配（合并区间）、每个像素再把 4 条子扫描线的区间各比一遍；
//! 一加 PLY110 上 500 个小矩形引擎工作中位 10.8ms，`fill_path` 是热点第一。
//!
//! 轴对齐矩形 [l, r) × [t, b) 在通用算法里的样子是确定的：
//! - 两条竖边的交点就是 `l`、`r` 本身（`x = p0.x + t * 0`），每条子扫描线要么两点都有、要么都没有；
//! - 一行里被盖住的子扫描线条数 k（0~4）只取决于 `t <= scan_y < b`；
//! - 每个像素在每条命中的子扫描线上的覆盖量相同，记 o；通用算法是 `0.0` 起逐条加 o，共 k 次，再 `/ 4`。
//!
//! 这里一行只算一次 k、一个像素只算一次 o，然后**按同样的顺序做同样的 f32 运算**（`k` 次 `+= o`，不写成 `o * k`：
//! 乘法与连加的舍入不一定相同），整格区间同样交给 `fill_span`。行列范围、裁剪、alpha 取整都与通用路径一字不差。
//! 判据：`rect_aa_tests` 随机 6000 例（小数坐标、负宽高、越界、半透明、裁剪矩形、平移）与通用路径逐字节相同。
use super::*;

/// 子扫描线条数：必须与 `paths.rs` 的 `fill_path_aa_scan` 相同（两边共用这一个常量）
pub(super) const AA_SUB_SAMPLES: usize = 4;

/// 轴对齐矩形 (l, r, t, b)，l ≤ r、t ≤ b
#[derive(Clone, Copy, Debug)]
pub(super) struct AxisRect {
    l: f32,
    r: f32,
    t: f32,
    b: f32,
}

/// 只有一个轮廓、四个顶点（或闭合时补回起点的五个）、边全是横平竖直的才认。任何非有限值都不认（交给通用路径）
pub(super) fn axis_rect(contours: &[Vec<Point>]) -> Option<AxisRect> {
    let [c] = contours else { return None };
    match c.len() {
        4 => axis_rect_pts(&c[..4]),
        5 if c[4] == c[0] => axis_rect_pts(&c[..4]),
        _ => None,
    }
}

/// 四个顶点（不含闭合点）是不是轴对齐矩形
fn axis_rect_pts(p: &[Point]) -> Option<AxisRect> {
    if !p.iter().all(|q| q.x.is_finite() && q.y.is_finite()) {
        return None;
    }
    // 两种走向：先横后竖（p0→p1 横）或先竖后横（p0→p1 竖）
    let h_first = p[0].y == p[1].y && p[1].x == p[2].x && p[2].y == p[3].y && p[3].x == p[0].x;
    let v_first = p[0].x == p[1].x && p[1].y == p[2].y && p[2].x == p[3].x && p[3].y == p[0].y;
    if !(h_first || v_first) {
        return None;
    }
    let (xa, xb) = if h_first { (p[0].x, p[1].x) } else { (p[0].x, p[2].x) };
    let (ya, yb) = if h_first { (p[0].y, p[2].y) } else { (p[0].y, p[1].y) };
    Some(AxisRect { l: xa.min(xb), r: xa.max(xb), t: ya.min(yb), b: ya.max(yb) })
}

impl Canvas {
    /// `y0` / `y1` 与 `fill_path` 算的相同（包围盒上下各扩 1 行）
    pub(super) fn fill_axis_rect_aa(&mut self, rect: AxisRect, y0: i32, y1: i32, paint: &Paint) {
        let AxisRect { l, r, t, b } = rect;
        // 高为 0：两条竖边都是零长，通用路径连边表都是空的；宽为 0：每条子扫描线的区间 r > l 不成立。都什么也不画
        if !(b > t) || !(r > l) {
            return;
        }
        let (bx0, bx1, by0, by1) = self.draw_bounds();
        // 与通用路径同一个式子：子扫描线放在每格 1/4 的中点
        let scan = |y: i32, sub: usize| y as f32 + (sub as f32 + 0.5) / AA_SUB_SAMPLES as f32;
        // 列范围与解析覆盖都不随行变：通用路径里合并后的区间就是 [l, r]，左右各留 1px 余量
        let sx0 = ((l - 1.0).floor() as i32).max(bx0);
        let sx1 = ((r + 1.0).ceil() as i32).min(bx1 - 1);
        // 整格列 [sl, sr]：像素整格落在 [l, r] 里（`x >= l && x + 1 <= r`），每条命中的子扫描线覆盖量恰为 1.0
        let (sl, sr) = (l.ceil() as i32, r.floor() as i32 - 1);
        let c = paint.color;
        // 覆盖量 o、命中 k 条 → 写入的 alpha：与通用路径同样「0.0 起连加 k 次 o、再 / 4」，再按 set_pixel_aa 取整
        let alpha = |o: f32, k: usize| -> u8 {
            let mut coverage = 0.0f32;
            for _ in 0..k {
                coverage += o;
            }
            coverage /= AA_SUB_SAMPLES as f32;
            if coverage > 0.0 { (c.a as f32 * coverage.min(1.0)).round().clamp(0.0, 255.0) as u8 } else { 0 }
        };
        for y in y0.max(by0)..=y1.min(by1 - 1) {
            let k = (0..AA_SUB_SAMPLES).filter(|&s| { let sy = scan(y, s); t <= sy && b > sy }).count();
            if k == 0 {
                continue;
            }
            let row = y as usize * self.width as usize;
            // 只逐像素算两侧的边缘列；中间整格那段覆盖率处处相同，整段 fill_span（它与逐个 set_pixel 同一套 alpha 规则，
            // 4 条全中时就是通用路径的「solid」整段）。从前每行把中间几十列逐个跳过，一加上这个函数占游戏线程 26%
            let (left_end, right_start) = if sr >= sl { (sl - 1, sr + 1) } else { (sx1, sx1 + 1) };
            for x in (sx0..=left_end.min(sx1)).chain(right_start.max(sx0)..=sx1) {
                let (pixel_left, pixel_right) = (x as f32, x as f32 + 1.0);
                let o = if pixel_right <= l || pixel_left >= r {
                    continue;
                } else if pixel_left >= l && pixel_right <= r {
                    1.0
                } else {
                    pixel_right.min(r) - pixel_left.max(l)
                };
                // = set_pixel_aa 展开：行列已夹在 draw_bounds（画布 ∩ 裁剪）里，set_pixel 的越界 / 裁剪判断恒为真
                let a = alpha(o, k);
                let idx = row + x as usize;
                if a == 255 {
                    self.pixels[idx] = Color::new(c.r, c.g, c.b, 255);
                } else if a > 0 {
                    self.pixels[idx] = Color::new(c.r, c.g, c.b, a).blend(&self.pixels[idx]);
                }
            }
            if sr >= sl {
                self.fill_span(sl, sr + 1, y, Color::new(c.r, c.g, c.b, alpha(1.0, k)));
            }
        }
    }
}

impl Canvas {
    /// 抗锯齿填充一个四边形 `q[0]→q[1]→q[2]→q[3]→闭合`，结果与「建 `Path`（move/line×3/close）再 `draw_path`（Fill + 抗锯齿）」
    /// 逐字节相同，只是不建 Path、不展平（那一趟每个 fillRect 五六次堆分配，真机 jemalloc 带锁）。
    /// 等价的根据：`Path::flatten` 对这条路径给出 `[q0, q1, q2, q3]`，q3 ≠ q0 时闭合再补一个 q0；
    /// `draw_path` 随后给每个点加画布平移、交给 `fill_path`。这里按同样的顺序做同样的事
    pub fn fill_quad_aa(&mut self, q: [Point; 4], paint: &Paint) {
        let (tx, ty) = self.translation;
        let mut p = q;
        if tx != 0.0 || ty != 0.0 {
            for v in &mut p {
                v.x += tx;
                v.y += ty;
            }
        }
        // 轴对齐 + 抗锯齿：直接走特化，不分配。y0 / y1 与 fill_path 同一个式子（闭合补的那个点就是 q0，不影响上下界）
        if paint.anti_alias {
            if let Some(r) = axis_rect_pts(&p) {
                let (lo, hi) = p.iter().fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(v.y), hi.max(v.y)));
                return self.fill_axis_rect_aa(r, (lo - 1.0).floor() as i32, (hi + 1.0).ceil() as i32, paint);
            }
        }
        // 其余（旋转 / 斜切 / 非有限值）照原样：与 flatten 相同地闭合，交给 fill_path
        let mut c = p.to_vec();
        // flatten 比的是平移之前的点（平移在 draw_path 里、flatten 之后才加）
        if q[3] != q[0] {
            c.push(p[0]);
        }
        self.fill_path(std::slice::from_ref(&c), paint);
    }
}

#[cfg(test)]
mod rect_aa_tests {
    use super::*;

    fn bounds_y(contours: &[Vec<Point>]) -> (i32, i32) {
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for p in contours.iter().flatten() {
            lo = lo.min(p.y);
            hi = hi.max(p.y);
        }
        ((lo - 1.0).floor() as i32, (hi + 1.0).ceil() as i32)
    }

    /// 快路与通用扫描线逐字节相同（随机小数坐标、负宽高、越界、零 / 亚像素尺寸、半透明、裁剪矩形、四种顶点走向）
    #[test]
    fn 轴对齐矩形快路与通用扫描线逐字节相同() {
        let mut seed = 0x9e3779b9u32;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as f32 / u32::MAX as f32
        };
        let (w, h) = (40u32, 32u32);
        for case in 0..6000 {
            // 底色随机（含半透明），让混合也被测到
            let bg = Color::new((rnd() * 255.0) as u8, (rnd() * 255.0) as u8, (rnd() * 255.0) as u8, [255, 128, 0][case % 3]);
            let clip = (case % 4 == 1).then(|| Rect::new(rnd() * 20.0, rnd() * 16.0, rnd() * 30.0, rnd() * 24.0));
            let make = || {
                let mut c = Canvas::new(w, h);
                c.clear(bg);
                if let Some(r) = clip {
                    c.clip_rect(r);
                }
                c
            };
            let (mut a, mut b) = (make(), make());
            let x = rnd() * 56.0 - 8.0;
            let y = rnd() * 48.0 - 8.0;
            // 尺寸：多数是普通小数，也有 0、亚像素、负数
            let sz = |v: f32, c: usize| match c % 7 { 0 => 0.0, 1 => v * 0.9, 2 => -v * 20.0, _ => v * 30.0 };
            let (rw, rh) = (sz(rnd(), case), sz(rnd(), case / 7));
            // 三分之一的例子落在 1/8 网格上：边正好压在子扫描线 / 像素边上（`<=` 与 `<` 的分界只有这样才测得到）
            let q = |v: f32| if case % 3 == 0 { (v * 8.0).round() / 8.0 } else { v };
            let (x, y, rw, rh) = (q(x), q(y), q(rw), q(rh));
            let pts = [Point::new(x, y), Point::new(x + rw, y), Point::new(x + rw, y + rh), Point::new(x, y + rh)];
            let mut c: Vec<Point> = match case % 4 {
                0 => pts.to_vec(),
                1 => pts.iter().rev().copied().collect(),
                2 => vec![pts[1], pts[2], pts[3], pts[0]],
                _ => vec![pts[3], pts[2], pts[1], pts[0]],
            };
            if case % 2 == 0 {
                c.push(c[0]);
            }
            let contours = vec![c];
            assert!(axis_rect(&contours).is_some(), "第 {case} 例没认出矩形");
            let alpha = [255u8, 200, 77, 1][case % 4];
            let paint = Paint::new()
                .with_color(Color::new((rnd() * 255.0) as u8, (rnd() * 255.0) as u8, (rnd() * 255.0) as u8, alpha))
                .with_style(PaintStyle::Fill)
                .with_anti_alias(true);
            let (y0, y1) = bounds_y(&contours);
            a.fill_path(&contours, &paint);
            b.fill_path_aa_scan(&contours, y0, y1, &paint);
            assert!(a.pixels() == b.pixels(), "第 {case} 例不一致：({x},{y}) {rw}×{rh} alpha {alpha}");
        }
    }

    /// fill_quad_aa 与「建 Path + draw_path」逐字节相同：轴对齐 / 旋转 / 退化（首尾重合）四边形、画布平移、裁剪、半透明
    #[test]
    fn 四边形直填与_draw_path_逐字节相同() {
        let mut seed = 0x1234_5678u32;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as f32 / u32::MAX as f32
        };
        for case in 0..3000 {
            let clip = (case % 3 == 1).then(|| Rect::new(rnd() * 20.0, rnd() * 16.0, rnd() * 30.0, rnd() * 24.0));
            let tr = (case % 4 == 2).then(|| (rnd() * 6.0 - 3.0, rnd() * 6.0 - 3.0));
            let make = || {
                let mut c = Canvas::new(40, 32);
                c.clear(Color::new(10, 200, 30, [255, 90][case % 2]));
                if let Some(r) = clip {
                    c.clip_rect(r);
                }
                if let Some((x, y)) = tr {
                    c.translate(x, y);
                }
                c
            };
            let (mut a, mut b) = (make(), make());
            let (x, y, w, h) = (rnd() * 50.0 - 5.0, rnd() * 40.0 - 5.0, rnd() * 30.0 - 8.0, rnd() * 24.0 - 6.0);
            let q: [Point; 4] = match case % 5 {
                // 斜的：随机扰动
                3 => [Point::new(x, y), Point::new(x + w, y + rnd() * 4.0), Point::new(x + w - rnd() * 3.0, y + h), Point::new(x, y + h)],
                // 退化：首尾重合
                4 => [Point::new(x, y), Point::new(x + w, y), Point::new(x + w, y + h), Point::new(x, y)],
                _ => [Point::new(x, y), Point::new(x + w, y), Point::new(x + w, y + h), Point::new(x, y + h)],
            };
            let paint = Paint::new()
                .with_color(Color::new(200, 40, 90, [255, 128, 3][case % 3]))
                .with_style(PaintStyle::Fill)
                .with_anti_alias(true);
            a.fill_quad_aa(q, &paint);
            let mut path = Path::new();
            path.move_to(q[0].x, q[0].y);
            path.line_to(q[1].x, q[1].y);
            path.line_to(q[2].x, q[2].y);
            path.line_to(q[3].x, q[3].y);
            path.close();
            b.draw_path(&path, &paint);
            assert!(a.pixels() == b.pixels(), "第 {case} 例不一致：{q:?}");
        }
    }

    /// fillRect 实际走的路径（move/line×3/close，展平后 5 个点）要能认出来，否则快路永远走不到
    #[test]
    fn 画布矩形路径能认出() {
        let mut p = Path::new();
        p.move_to(1.3, 2.7);
        p.line_to(9.1, 2.7);
        p.line_to(9.1, 7.2);
        p.line_to(1.3, 7.2);
        p.close();
        assert!(axis_rect(&p.flatten(1.0)).is_some());
        // 斜的不认
        let mut q = Path::new();
        q.move_to(1.0, 1.0);
        q.line_to(5.0, 2.0);
        q.line_to(4.0, 6.0);
        q.line_to(0.0, 5.0);
        q.close();
        assert!(axis_rect(&q.flatten(1.0)).is_none());
    }
}
