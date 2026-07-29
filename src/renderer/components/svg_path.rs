//! 极简 SVG `path` 的 `d` 解析器：把矢量图标数据直接变成引擎的 `Path`。
//!
//! 为什么需要它：内置图标（`<icon>`）要对齐微信，就得用微信自己那套字形，而
//! 那套字形唯一可拿到的权威形式就是 SVG 路径数据（见 `icon_data.rs`）。之前用
//! 圆 + 圆角矩形 + 手搓多边形去「近似」，对勾的起笔/收笔比例、叉号的粗细、
//! 时钟指针的长度全是拍出来的，放大一看就不像。
//!
//! 支持的命令：`M m L l H h V v C c S s Q q T t A a Z z`，含
//! - 隐式重复参数组（`M` 之后的重复组按 `L` 处理，`m` 按 `l`）；
//! - 省掉分隔符的写法（`10-4.477`、`.849.849`、`1e-3`）；
//! - 圆弧标志位的紧凑写法（`a8.8 8.8 0 100-17.6` 里 `100-17.6` = 大弧 1、
//!   顺时针 0、然后 `0`、`-17.6`）—— 这是 SVG 里最容易写错的一处，标志位必须
//!   按「单个字符」读，不能按数字读。
//!
//! 填充规则：引擎的 `Canvas::fill_path` 是扫描线取交点后**按对**填充，也就是
//! `even-odd`。微信那套图标正是靠子路径反向绕出「洞」（对勾是圆里的洞，不是
//! 白色描边），even-odd 能原样还原，所以这里不需要额外的 nonzero 支持。

use crate::Path;

/// 2×3 仿射变换：`x' = a·x + c·y + e`，`y' = b·x + d·y + f`。
///
/// 仿射变换把贝塞尔曲线映射成贝塞尔曲线，所以只变换控制点就够了，不需要先展平。
#[derive(Debug, Clone, Copy)]
pub struct Xf {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Xf {
    pub const IDENTITY: Xf = Xf { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };

    pub fn scale_translate(sx: f32, sy: f32, ox: f32, oy: f32) -> Self {
        Xf { a: sx, b: 0.0, c: 0.0, d: sy, e: ox, f: oy }
    }

    #[inline]
    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }

    /// `self` 之后再做 `outer`（即 `outer ∘ self`）。
    pub fn then(&self, outer: &Xf) -> Xf {
        Xf {
            a: outer.a * self.a + outer.c * self.b,
            b: outer.b * self.a + outer.d * self.b,
            c: outer.a * self.c + outer.c * self.d,
            d: outer.b * self.c + outer.d * self.d,
            e: outer.a * self.e + outer.c * self.f + outer.e,
            f: outer.b * self.e + outer.d * self.f + outer.f,
        }
    }

    /// 绕原点旋转 `deg` 度（屏幕坐标系，y 轴向下，正角为顺时针）。
    pub fn rotate(deg: f32) -> Self {
        let r = deg.to_radians();
        let (s, c) = (r.sin(), r.cos());
        Xf { a: c, b: s, c: -s, d: c, e: 0.0, f: 0.0 }
    }

    pub fn translate(tx: f32, ty: f32) -> Self {
        Xf { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: tx, f: ty }
    }
}

/// 把 SVG 路径数据按 `xf` 变换后追加到 `out`。
pub fn append_svg_path(d: &str, xf: &Xf, out: &mut Path) {
    let mut lx = Lexer { b: d.as_bytes(), i: 0 };
    // 当前点、子路径起点、上一段的控制点（S/T 需要做反射）
    let (mut cx, mut cy) = (0.0f32, 0.0f32);
    let (mut sxp, mut syp) = (0.0f32, 0.0f32);
    let (mut pcx, mut pcy) = (0.0f32, 0.0f32); // 上一段三次曲线的第二控制点
    let (mut qcx, mut qcy) = (0.0f32, 0.0f32); // 上一段二次曲线的控制点
    let mut prev_cubic = false;
    let mut prev_quad = false;
    let mut cmd = 0u8;
    let tp = |x: f32, y: f32| xf.apply(x, y);

    loop {
        lx.skip_sep();
        match lx.b.get(lx.i) {
            None => break,
            Some(c) if c.is_ascii_alphabetic() => {
                cmd = *c;
                lx.i += 1;
            }
            // 隐式重复参数组：沿用上一个命令。首个命令必须是字母，否则数据非法。
            Some(_) if cmd == 0 => break,
            Some(_) => {}
        }
        let rel = cmd.is_ascii_lowercase();
        let (mut is_cubic, mut is_quad) = (false, false);

        match cmd.to_ascii_uppercase() {
            b'M' => {
                let (Some(x), Some(y)) = (lx.num(), lx.num()) else { break };
                let (nx, ny) = if rel { (cx + x, cy + y) } else { (x, y) };
                let (px, py) = tp(nx, ny);
                out.move_to(px, py);
                (cx, cy) = (nx, ny);
                (sxp, syp) = (nx, ny);
                // 后续重复组当直线处理
                cmd = if rel { b'l' } else { b'L' };
            }
            b'L' => {
                let (Some(x), Some(y)) = (lx.num(), lx.num()) else { break };
                let (nx, ny) = if rel { (cx + x, cy + y) } else { (x, y) };
                let (px, py) = tp(nx, ny);
                out.line_to(px, py);
                (cx, cy) = (nx, ny);
            }
            b'H' => {
                let Some(x) = lx.num() else { break };
                let nx = if rel { cx + x } else { x };
                let (px, py) = tp(nx, cy);
                out.line_to(px, py);
                cx = nx;
            }
            b'V' => {
                let Some(y) = lx.num() else { break };
                let ny = if rel { cy + y } else { y };
                let (px, py) = tp(cx, ny);
                out.line_to(px, py);
                cy = ny;
            }
            b'C' => {
                let (Some(x1), Some(y1), Some(x2), Some(y2), Some(x), Some(y)) =
                    (lx.num(), lx.num(), lx.num(), lx.num(), lx.num(), lx.num()) else { break };
                let (bx, by) = if rel { (cx, cy) } else { (0.0, 0.0) };
                let (c1, c2, e) = ((bx + x1, by + y1), (bx + x2, by + y2), (bx + x, by + y));
                emit_cubic(out, &tp, c1, c2, e);
                (pcx, pcy) = c2;
                (cx, cy) = e;
                is_cubic = true;
            }
            b'S' => {
                let (Some(x2), Some(y2), Some(x), Some(y)) =
                    (lx.num(), lx.num(), lx.num(), lx.num()) else { break };
                let (bx, by) = if rel { (cx, cy) } else { (0.0, 0.0) };
                // 第一控制点 = 上一段第二控制点关于当前点的反射（上一段不是三次曲线则取当前点）
                let c1 = if prev_cubic { (2.0 * cx - pcx, 2.0 * cy - pcy) } else { (cx, cy) };
                let (c2, e) = ((bx + x2, by + y2), (bx + x, by + y));
                emit_cubic(out, &tp, c1, c2, e);
                (pcx, pcy) = c2;
                (cx, cy) = e;
                is_cubic = true;
            }
            b'Q' => {
                let (Some(x1), Some(y1), Some(x), Some(y)) =
                    (lx.num(), lx.num(), lx.num(), lx.num()) else { break };
                let (bx, by) = if rel { (cx, cy) } else { (0.0, 0.0) };
                let (c, e) = ((bx + x1, by + y1), (bx + x, by + y));
                let (qx, qy) = tp(c.0, c.1);
                let (ex, ey) = tp(e.0, e.1);
                out.quad_to(qx, qy, ex, ey);
                (qcx, qcy) = c;
                (cx, cy) = e;
                is_quad = true;
            }
            b'T' => {
                let (Some(x), Some(y)) = (lx.num(), lx.num()) else { break };
                let (bx, by) = if rel { (cx, cy) } else { (0.0, 0.0) };
                let c = if prev_quad { (2.0 * cx - qcx, 2.0 * cy - qcy) } else { (cx, cy) };
                let e = (bx + x, by + y);
                let (qx, qy) = tp(c.0, c.1);
                let (ex, ey) = tp(e.0, e.1);
                out.quad_to(qx, qy, ex, ey);
                (qcx, qcy) = c;
                (cx, cy) = e;
                is_quad = true;
            }
            b'A' => {
                let (Some(rx), Some(ry), Some(rot), Some(large), Some(sweep), Some(x), Some(y)) =
                    (lx.num(), lx.num(), lx.num(), lx.flag(), lx.flag(), lx.num(), lx.num())
                    else { break };
                let (bx, by) = if rel { (cx, cy) } else { (0.0, 0.0) };
                let e = (bx + x, by + y);
                for [c1x, c1y, c2x, c2y, ex, ey] in
                    arc_to_cubics((cx, cy), rx, ry, rot, large, sweep, e)
                {
                    emit_cubic(out, &tp, (c1x, c1y), (c2x, c2y), (ex, ey));
                }
                (cx, cy) = e;
            }
            b'Z' => {
                out.close();
                (cx, cy) = (sxp, syp);
                // `Z` 不带参数：后面若不是新命令字母就说明数据到头了，
                // 不 break 会在同一个 `Z` 上死循环。
                lx.skip_sep();
                match lx.b.get(lx.i) {
                    Some(c) if c.is_ascii_alphabetic() => {}
                    _ => break,
                }
            }
            _ => break, // 未知命令，安全退出（宁可少画也不要死循环）
        }
        prev_cubic = is_cubic;
        prev_quad = is_quad;
    }
}

fn emit_cubic(
    out: &mut Path,
    tp: &impl Fn(f32, f32) -> (f32, f32),
    c1: (f32, f32),
    c2: (f32, f32),
    e: (f32, f32),
) {
    let (a, b) = (tp(c1.0, c1.1), tp(c2.0, c2.1));
    let f = tp(e.0, e.1);
    out.cubic_to(a.0, a.1, b.0, b.1, f.0, f.1);
}

/// SVG 椭圆弧（端点参数化）→ 一串三次贝塞尔。按 W3C SVG 实现附录 F.6 换算圆心，
/// 再按不超过 90° 一段切开，每段用 `alpha = sin dθ (sqrt(4 + 3 tan²(dθ/2)) - 1) / 3` 逼近。
fn arc_to_cubics(
    from: (f32, f32),
    rx_in: f32,
    ry_in: f32,
    rot_deg: f32,
    large: f32,
    sweep: f32,
    to: (f32, f32),
) -> Vec<[f32; 6]> {
    let (x0, y0) = from;
    let (x1, y1) = to;
    // 起点终点重合 → 按规范整段忽略
    if (x0 - x1).abs() < 1e-6 && (y0 - y1).abs() < 1e-6 {
        return Vec::new();
    }
    let (mut rx, mut ry) = (rx_in.abs(), ry_in.abs());
    if rx < 1e-6 || ry < 1e-6 {
        // 退化为直线
        return vec![[x0, y0, x1, y1, x1, y1]];
    }
    let phi = rot_deg.to_radians();
    let (sp, cp) = (phi.sin(), phi.cos());
    let (dx2, dy2) = ((x0 - x1) / 2.0, (y0 - y1) / 2.0);
    let x1p = cp * dx2 + sp * dy2;
    let y1p = -sp * dx2 + cp * dy2;
    // 半径不够大时按规范整体放大
    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    let (rx2, ry2) = (rx * rx, ry * ry);
    let num = (rx2 * ry2 - rx2 * y1p * y1p - ry2 * x1p * x1p).max(0.0);
    let den = rx2 * y1p * y1p + ry2 * x1p * x1p;
    let sign = if (large > 0.5) != (sweep > 0.5) { 1.0 } else { -1.0 };
    let coef = sign * (num / den.max(1e-12)).sqrt();
    let cxp = coef * rx * y1p / ry;
    let cyp = -coef * ry * x1p / rx;
    let cx = cp * cxp - sp * cyp + (x0 + x1) / 2.0;
    let cy = sp * cxp + cp * cyp + (y0 + y1) / 2.0;

    let ux = (x1p - cxp) / rx;
    let uy = (y1p - cyp) / ry;
    let vx = (-x1p - cxp) / rx;
    let vy = (-y1p - cyp) / ry;
    let theta1 = uy.atan2(ux);
    let mut dtheta = (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    if sweep < 0.5 && dtheta > 0.0 {
        dtheta -= std::f32::consts::TAU;
    } else if sweep > 0.5 && dtheta < 0.0 {
        dtheta += std::f32::consts::TAU;
    }

    let segs = (dtheta.abs() / std::f32::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step = dtheta / segs as f32;
    let alpha = {
        let t = (step / 2.0).tan();
        step.sin() * ((4.0 + 3.0 * t * t).sqrt() - 1.0) / 3.0
    };
    let point = |t: f32| {
        let (st, ct) = (t.sin(), t.cos());
        (cx + rx * ct * cp - ry * st * sp, cy + rx * ct * sp + ry * st * cp)
    };
    let deriv = |t: f32| {
        let (st, ct) = (t.sin(), t.cos());
        (-rx * st * cp - ry * ct * sp, -rx * st * sp + ry * ct * cp)
    };

    let mut out = Vec::with_capacity(segs);
    for i in 0..segs {
        let t1 = theta1 + step * i as f32;
        let t2 = t1 + step;
        let p1 = point(t1);
        let p2 = point(t2);
        let d1 = deriv(t1);
        let d2 = deriv(t2);
        out.push([
            p1.0 + alpha * d1.0,
            p1.1 + alpha * d1.1,
            p2.0 - alpha * d2.0,
            p2.1 - alpha * d2.1,
            p2.0,
            p2.1,
        ]);
    }
    out
}

struct Lexer<'a> {
    b: &'a [u8],
    i: usize,
}

impl Lexer<'_> {
    fn skip_sep(&mut self) {
        while matches!(self.b.get(self.i), Some(b' ' | b',' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    /// 读一个数字。允许 `+`/`-` 前导、`.5` 这种省略整数位、以及科学计数法。
    fn num(&mut self) -> Option<f32> {
        self.skip_sep();
        let start = self.i;
        let mut i = self.i;
        if matches!(self.b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let mut seen_dot = false;
        let mut seen_digit = false;
        while i < self.b.len() {
            match self.b[i] {
                b'0'..=b'9' => {
                    seen_digit = true;
                    i += 1;
                }
                // 第二个小数点属于**下一个**数字（`.849.849` 是两个数）
                b'.' if !seen_dot => {
                    seen_dot = true;
                    i += 1;
                }
                b'e' | b'E' if seen_digit => {
                    let mut j = i + 1;
                    if matches!(self.b.get(j), Some(b'+' | b'-')) {
                        j += 1;
                    }
                    if matches!(self.b.get(j), Some(b'0'..=b'9')) {
                        i = j;
                        while matches!(self.b.get(i), Some(b'0'..=b'9')) {
                            i += 1;
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        if !seen_digit {
            return None;
        }
        let s = std::str::from_utf8(&self.b[start..i]).ok()?;
        let v = s.parse().ok()?;
        self.i = i;
        Some(v)
    }

    /// 读圆弧标志位：只吃一个 `0`/`1` 字符。
    fn flag(&mut self) -> Option<f32> {
        self.skip_sep();
        match self.b.get(self.i) {
            Some(b'0') => {
                self.i += 1;
                Some(0.0)
            }
            Some(b'1') => {
                self.i += 1;
                Some(1.0)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bbox(d: &str) -> (f32, f32, f32, f32) {
        let mut p = Path::new();
        append_svg_path(d, &Xf::IDENTITY, &mut p);
        let contours = p.flatten(0.2);
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for c in &contours {
            for pt in c {
                x0 = x0.min(pt.x);
                y0 = y0.min(pt.y);
                x1 = x1.max(pt.x);
                y1 = y1.max(pt.y);
            }
        }
        (x0, y0, x1, y1)
    }

    fn contours(d: &str) -> usize {
        let mut p = Path::new();
        append_svg_path(d, &Xf::IDENTITY, &mut p);
        p.flatten(0.2).len()
    }

    #[test]
    fn parses_absolute_and_relative_lines() {
        let (x0, y0, x1, y1) = bbox("M2 3 L10 3 l0 5 Z");
        assert!((x0 - 2.0).abs() < 0.01 && (y0 - 3.0).abs() < 0.01);
        assert!((x1 - 10.0).abs() < 0.01 && (y1 - 8.0).abs() < 0.01);
    }

    #[test]
    fn implicit_repeat_after_moveto_is_lineto() {
        // 第二组 `20 20` 必须当 L 而不是再来一次 M（否则只剩一个孤立点）
        let (x0, y0, x1, y1) = bbox("M0 0 20 20");
        assert!((x1 - 20.0).abs() < 0.01 && (y1 - 20.0).abs() < 0.01);
        assert!(x0.abs() < 0.01 && y0.abs() < 0.01);
    }

    #[test]
    fn parses_h_v_and_tight_numbers() {
        // `.849.849` / `10-4.477` 这类省分隔符写法
        let (x0, y0, x1, y1) = bbox("M.849.849H10-4.477V5Z");
        assert!(x0 <= -4.47 && x1 >= 9.99, "x range {x0}..{x1}");
        assert!(y0 <= 0.85 && y1 >= 4.99, "y range {y0}..{y1}");
    }

    #[test]
    fn arc_flags_are_single_chars() {
        // weui 的圆环写法：`a8.8 8.8 0 100-17.6 8.8 8.8 0 000 17.6`
        // 标志位若按数字读，`100` 会被吃成 100 → 整段崩掉
        let d = "M12 20.8a8.8 8.8 0 100-17.6 8.8 8.8 0 000 17.6z";
        let (x0, y0, x1, y1) = bbox(d);
        assert!((x0 - 3.2).abs() < 0.2, "left {x0}");
        assert!((x1 - 20.8).abs() < 0.2, "right {x1}");
        assert!((y0 - 3.2).abs() < 0.2, "top {y0}");
        assert!((y1 - 20.8).abs() < 0.2, "bottom {y1}");
    }

    #[test]
    fn smooth_cubic_reflects_previous_control_point() {
        // weui `success` 的外圆就是 C+S 写的，半径必须还原成 10（2..22）
        let d = "M12 22C6.477 22 2 17.523 2 12S6.477 2 12 2s10 4.477 10 10-4.477 10-10 10z";
        let (x0, y0, x1, y1) = bbox(d);
        assert!((x0 - 2.0).abs() < 0.15, "left {x0}");
        assert!((x1 - 22.0).abs() < 0.15, "right {x1}");
        assert!((y0 - 2.0).abs() < 0.15, "top {y0}");
        assert!((y1 - 22.0).abs() < 0.15, "bottom {y1}");
    }

    #[test]
    fn subpaths_are_kept_separate_for_even_odd_holes() {
        // 圆 + 对勾两条子路径 → 两个轮廓，even-odd 才能把对勾挖成洞
        let d = "M12 22C6.477 22 2 17.523 2 12S6.477 2 12 2s10 4.477 10 10-4.477 10-10 10z\
                 m-1.177-7.86l-2.765-2.767L7 12.431l3.119 3.121a1 1 0 001.414 0l5.952-5.95-1.062-1.062-5.6 5.6z";
        assert_eq!(contours(d), 2);
    }

    #[test]
    fn transform_scales_and_offsets() {
        let mut p = Path::new();
        let xf = Xf::scale_translate(2.0, 0.5, 10.0, 100.0);
        append_svg_path("M0 0H24V24Z", &xf, &mut p);
        let c = p.flatten(0.2);
        let max_x = c[0].iter().map(|p| p.x).fold(f32::MIN, f32::max);
        let max_y = c[0].iter().map(|p| p.y).fold(f32::MIN, f32::max);
        assert!((max_x - 58.0).abs() < 0.01, "max_x {max_x}"); // 10 + 24*2
        assert!((max_y - 112.0).abs() < 0.01, "max_y {max_y}"); // 100 + 24*0.5
    }

    #[test]
    fn rotate_90_maps_box_as_expected() {
        // 12x24 的箭头字形转 90° 后应落在 24x12 的盒子里：(x,y) -> (vh - y, x)
        let xf = Xf::rotate(90.0).then(&Xf::translate(24.0, 0.0));
        let mut p = Path::new();
        append_svg_path("M0 0H12V24Z", &xf, &mut p);
        let c = p.flatten(0.2);
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for pt in &c[0] {
            x0 = x0.min(pt.x);
            y0 = y0.min(pt.y);
            x1 = x1.max(pt.x);
            y1 = y1.max(pt.y);
        }
        assert!(x0.abs() < 0.01 && (x1 - 24.0).abs() < 0.01, "x {x0}..{x1}");
        assert!(y0.abs() < 0.01 && (y1 - 12.0).abs() < 0.01, "y {y0}..{y1}");
    }

    #[test]
    fn malformed_data_does_not_hang() {
        // 缺参数 / 非法命令 / 只有 Z，都必须能停下来
        for d in ["M", "M1", "M1 1L", "ZZZ", "Q1 2 3", "A1 1 0 1", "M1 1zzz", "1 2 3"] {
            let mut p = Path::new();
            append_svg_path(d, &Xf::IDENTITY, &mut p);
        }
    }
}
