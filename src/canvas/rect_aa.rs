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
    let p = match c.len() {
        4 => &c[..4],
        5 if c[4] == c[0] => &c[..4],
        _ => return None,
    };
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
        let (sl, sr) = (l.ceil() as i32, r.floor() as i32 - 1);
        let solid_a = (paint.color.a as f32).round().clamp(0.0, 255.0) as u8;
        let solid_color = Color::new(paint.color.r, paint.color.g, paint.color.b, solid_a);
        for y in y0.max(by0)..=y1.min(by1 - 1) {
            let k = (0..AA_SUB_SAMPLES).filter(|&s| { let sy = scan(y, s); t <= sy && b > sy }).count();
            if k == 0 {
                continue;
            }
            // 4 条子扫描线都盖住时，[ceil(l), floor(r)) 这段覆盖率恰为 1，通用路径整段 fill_span
            let solid = k == AA_SUB_SAMPLES && sr >= sl;
            for x in sx0..=sx1 {
                if solid && x >= sl && x <= sr {
                    continue;
                }
                let px = x as f32;
                let (pixel_left, pixel_right) = (px, px + 1.0);
                let o = if pixel_right <= l || pixel_left >= r {
                    continue;
                } else if pixel_left >= l && pixel_right <= r {
                    1.0
                } else {
                    pixel_right.min(r) - pixel_left.max(l)
                };
                let mut coverage = 0.0f32;
                for _ in 0..k {
                    coverage += o;
                }
                coverage /= AA_SUB_SAMPLES as f32;
                if coverage > 0.0 {
                    self.set_pixel_aa(x, y, paint.color, coverage.min(1.0));
                }
            }
            if solid {
                self.fill_span(sl, sr + 1, y, solid_color);
            }
        }
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
