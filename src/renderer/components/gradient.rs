//! `background-image: linear-gradient(...)` 的绘制。
//!
//! 从 `base.rs` 拆出，并补了两条快路径。原实现对每个像素都要：算圆角覆盖率、
//! 把点投影到渐变轴、在色标里线性查找插值、再走一次带边界与裁剪判断的 `set_pixel`。
//! tea-app 的卡片上铺着整片渐变蒙层（`.tea-wash` / `.tea-fade` / `.hero-wash-fade`），
//! 一屏就有几十万像素走这条路，占掉滑动帧的一大块。
//!
//! 而实际用到的渐变几乎全是轴对齐的（`to bottom` / `to top` / `to right` / `to left`）：
//! - 垂直：同一行颜色相同 → 每行只算一次色，整行一次填充
//! - 水平：所有行颜色序列相同 → 只算一行，其余行整行复用
//!
//! 只有圆角波及的那几行退回逐像素（保证圆角抗锯齿不变），因此输出与逐像素实现一致。

use super::base::LinearGradientBg;
use crate::{Canvas, Color};

/// 绘制线性渐变背景（圆角边缘抗锯齿）。
pub fn draw_linear_gradient(
    canvas: &mut Canvas,
    grad: &LinearGradientBg,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radii: [f32; 4],
    opacity: f32,
) {
    if w <= 0.0 || h <= 0.0 || grad.stops.is_empty() {
        return;
    }
    let stops = &grad.stops;
    // 渐变方向单位向量：0deg 向上=(0,-1)，顺时针
    let a = grad.angle_deg.to_radians();
    let (dx, dy) = (a.sin(), -a.cos());
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let full = (w * dx).abs() + (h * dy).abs(); // 渐变线总长
    let [tl, tr, br, bl] = radii;

    let color_at = |t: f32| -> Color { sample_stops(stops, t) };

    // 圆角覆盖率（与图片圆角一致）
    let corner_cover = |px: f32, py: f32| -> f32 {
        let lx = px - x;
        let ly = py - y;
        let calc = |ccx: f32, ccy: f32, r: f32| -> f32 {
            if r <= 0.0 {
                return 1.0;
            }
            let d = ((lx - ccx) * (lx - ccx) + (ly - ccy) * (ly - ccy)).sqrt();
            (r + 0.5 - d).clamp(0.0, 1.0)
        };
        if lx < tl && ly < tl {
            calc(tl, tl, tl)
        } else if lx > w - tr && ly < tr {
            calc(w - tr, tr, tr)
        } else if lx > w - br && ly > h - br {
            calc(w - br, h - br, br)
        } else if lx < bl && ly > h - bl {
            calc(bl, h - bl, bl)
        } else {
            1.0
        }
    };

    // 只走画布裁剪区里的像素：区外的 `set_pixel` / `fill_span` / `blend_pixels` 本来就一个不写。
    // 局部重画（补条带、动画损伤区）的裁剪区常常只有一条，一个大半屏的斜向渐变（首页横幅）
    // 却要逐像素算完再被拒掉 —— 一加 PLY110 上一趟 10~17ms，滑到横幅附近每次补条带都掉帧。
    // 平移（`translate`）只作用在 `fill_rect` 那几条路上，这里的坐标本来就是画布坐标
    let (cx0, cx1, cy0, cy1) = canvas.draw_bounds();
    let x0 = (x.floor() as i32).max(cx0);
    let y0 = (y.floor() as i32).max(cy0);
    let x1 = ((x + w).ceil() as i32).min(cx1);
    let y1 = ((y + h).ceil() as i32).min(cy1);
    if x1 <= x0 || y1 <= y0 {
        return;
    }

    // 圆角只影响顶部/底部各一段行；这两段之外的行没有圆角衰减，可以整行处理
    let top_band = y + tl.max(tr);
    let bottom_band = y + h - bl.max(br);
    let plain_row = |py: i32| -> bool {
        let fy = py as f32 + 0.5;
        fy >= top_band && fy <= bottom_band
    };

    const AXIS_EPS: f32 = 1e-3;
    let vertical = dx.abs() < AXIS_EPS;
    let horizontal = dy.abs() < AXIS_EPS;

    // 水平渐变：一行的颜色序列对所有行都一样，只算一次
    let mut row_cache: Option<Vec<Color>> = None;
    if horizontal && x1 > x0 {
        let mut row = Vec::with_capacity((x1 - x0) as usize);
        for px in x0..x1 {
            let fx = px as f32 + 0.5;
            let proj = (fx - cx) * dx;
            let t = if full > 0.0 { proj / full + 0.5 } else { 0.5 };
            let mut c = color_at(t);
            c.a = (c.a as f32 * opacity) as u8;
            row.push(c);
        }
        row_cache = Some(row);
    }

    let mut row_buf: Vec<Color> = Vec::new();
    // 斜向、面积大的渐变按行缓存：补条带每次滑回页面顶部都要把头图那一截重算一遍
    // （一加 PLY110 上 1078×286 的一条 13~16ms，引擎线程上最热的就是这个闭包）。同一个元素、同一个画布位置
    // 每个像素的颜色是确定的，算过的行原样贴回去，结果与现算逐字节相同
    // 第一次见到的键不进缓存（`admit`）：键里有画布上的精确位置，定高 scroll-view 里滚一下位置就变，
    // 从前每帧都给几百行分配、哈希、塞进表，从来命不中，满 2M 像素再整表清掉 —— 兴趣圈帖子页三张 135° 渐变图
    // 真机上这一套占引擎线程 10%（诊断 20261025-100253-98de9a）。没进缓存的行走下面的整行现算，像素相同
    let cache_key = (!vertical && !horizontal && w * h >= row_cache::MIN_AREA)
        .then(|| row_cache::Key::new(grad, x, y, w, h, opacity))
        .filter(row_cache::admit);
    for py in y0..y1 {
        let fy = py as f32 + 0.5;
        if plain_row(py) {
            if vertical {
                let proj = (fy - cy) * dy;
                let t = if full > 0.0 { proj / full + 0.5 } else { 0.5 };
                let mut c = color_at(t);
                c.a = (c.a as f32 * opacity) as u8;
                canvas.fill_span(x0, x1, py, c);
                continue;
            }
            if let Some(row) = &row_cache {
                canvas.blend_pixels(x0, py, row, row.len());
                continue;
            }
        }
        if plain_row(py) && cache_key.is_some() {
            // 大块斜向渐变（首页头图）：这一行算过就直接贴（见 [`row_cache`]）
            let key = cache_key.as_ref().unwrap();
            let (bx0, bx1) = (x.floor() as i32, (x + w).ceil() as i32);
            row_cache::with_row(key, py, (bx1 - bx0).max(0) as usize, |row| {
                for (i, px) in (bx0..bx1).enumerate() {
                    let fx = px as f32 + 0.5;
                    let proj = (fx - cx) * dx + (fy - cy) * dy;
                    let t = if full > 0.0 { proj / full + 0.5 } else { 0.5 };
                    let mut c = color_at(t);
                    c.a = (c.a as f32 * opacity * 1.0) as u8;
                    row[i] = c;
                }
            }, |row| {
                let s = (x0 - bx0) as usize;
                let e = ((x1 - bx0) as usize).min(row.len());
                if e > s {
                    canvas.blend_pixels(x0, py, &row[s..e], e - s);
                }
            });
            continue;
        }
        if plain_row(py) {
            // 斜向渐变的无圆角行：颜色照旧逐像素算（同一个算式），但整行算好一次贴上去 ——
            // 从前每个像素一次 `set_pixel`（越界、裁剪矩形逐个判），一加 PLY110 上首页 135° 头图
            // 补一条带光这里就占引擎线程 15%。覆盖率在这些行恒为 1（`opacity * 1.0` 与 `opacity` 相同），
            // `blend_pixels` 按行裁剪、a=0 跳过、a=255 直写、其余混合，与 `set_pixel` 逐像素同一套规则
            let row = &mut row_buf;
            row.clear();
            for px in x0..x1 {
                let fx = px as f32 + 0.5;
                let proj = (fx - cx) * dx + (fy - cy) * dy;
                let t = if full > 0.0 { proj / full + 0.5 } else { 0.5 };
                let mut c = color_at(t);
                c.a = (c.a as f32 * opacity * 1.0) as u8;
                row.push(c);
            }
            canvas.blend_pixels(x0, py, row, row.len());
            continue;
        }
        for px in x0..x1 {
            let fx = px as f32 + 0.5;
            let cover = corner_cover(fx, fy);
            if cover <= 0.0 {
                continue;
            }
            let proj = (fx - cx) * dx + (fy - cy) * dy;
            let t = if full > 0.0 { proj / full + 0.5 } else { 0.5 };
            let mut c = color_at(t);
            c.a = (c.a as f32 * opacity * cover) as u8;
            canvas.set_pixel(px, py, c);
        }
    }
}

/// 在色标序列上按位置取色（含 alpha 插值）。
fn sample_stops(stops: &[(f32, Color)], t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    if t <= stops[0].0 {
        return stops[0].1;
    }
    let last = stops.len() - 1;
    if t >= stops[last].0 {
        return stops[last].1;
    }
    for i in 0..last {
        let (t0, c0) = stops[i];
        let (t1, c1) = stops[i + 1];
        if t >= t0 && t <= t1 {
            let r = if (t1 - t0).abs() < 1e-6 { 0.0 } else { (t - t0) / (t1 - t0) };
            return Color::new(
                (c0.r as f32 + (c1.r as f32 - c0.r as f32) * r) as u8,
                (c0.g as f32 + (c1.g as f32 - c0.g as f32) * r) as u8,
                (c0.b as f32 + (c1.b as f32 - c0.b as f32) * r) as u8,
                (c0.a as f32 + (c1.a as f32 - c0.a as f32) * r) as u8,
            );
        }
    }
    stops[last].1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grad(angle: f32) -> LinearGradientBg {
        LinearGradientBg {
            angle_deg: angle,
            stops: vec![
                (0.0, Color::new(249, 245, 238, 140)),
                (1.0, Color::new(249, 245, 238, 0)),
            ],
        }
    }

    /// 快路径必须与逐像素路径逐像素一致：用一个非轴对齐的角度（走慢路径）
    /// 和轴对齐角度分别画，再各自与「参考实现」比对。
    /// 这里的参考是「同一函数在 89.9deg（几乎水平但不触发快路径）」的连续性，
    /// 因此改为直接比对轴对齐快路径与手写逐像素结果。
    #[test]
    fn vertical_fast_path_matches_per_pixel() {
        let g = grad(180.0);
        let (w, h) = (40.0f32, 20.0f32);
        let mut fast = Canvas::new(40, 20);
        fast.clear(Color::new(0, 0, 0, 255));
        draw_linear_gradient(&mut fast, &g, 0.0, 0.0, w, h, [0.0; 4], 1.0);

        // 手写逐像素参考
        let mut slow = Canvas::new(40, 20);
        slow.clear(Color::new(0, 0, 0, 255));
        for py in 0..20i32 {
            let fy = py as f32 + 0.5;
            let t = (fy - h / 2.0) / h + 0.5;
            let c = sample_stops(&g.stops, t);
            for px in 0..40i32 {
                slow.set_pixel(px, py, c);
            }
        }
        assert_eq!(fast.pixels(), slow.pixels(), "垂直渐变快路径与逐像素结果应完全一致");
    }

    /// 水平渐变：行内颜色随 x 变化，各行相同
    #[test]
    fn horizontal_gradient_varies_along_x_only() {
        let g = grad(90.0);
        let mut c = Canvas::new(40, 20);
        c.clear(Color::new(0, 0, 0, 255));
        draw_linear_gradient(&mut c, &g, 0.0, 0.0, 40.0, 20.0, [0.0; 4], 1.0);
        let px = c.pixels();
        let at = |x: usize, y: usize| px[y * 40 + x];
        assert_eq!(at(3, 2), at(3, 17), "同一列在不同行应同色");
        assert_ne!(at(3, 2), at(30, 2), "同一行在不同列应不同色");
    }
}

/// 斜向渐变的行缓存（只给大块的：头图、整屏背景）。键里是元素在画布上的**精确**位置与尺寸（浮点的位），
/// 位置一变（画布窗口挪了、布局变了）就是新键 —— 像素与现算逐字节相同的前提就是同一组输入
pub(crate) mod row_cache {
    use super::{Color, LinearGradientBg};
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// 小于这个面积（物理像素）不缓存：按钮、卡片上的渐变现算很快
    pub const MIN_AREA: f32 = 120_000.0;
    /// 总共最多缓存这么多像素（约 8MB），满了整个清掉。
    ///
    /// 从前是 600 万像素（24MB）：键里有元素在画布上的精确位置，滑过一遍长页面，每个位置的大渐变都留一份。
    /// 真正贵的是首页头图那种一块（1078×286 ≈ 31 万像素），200 万像素够留六七块
    const MAX_PIXELS: usize = 2_000_000;

    #[derive(PartialEq, Eq, Hash, Clone)]
    pub struct Key(Vec<u32>);

    impl Key {
        pub fn new(g: &LinearGradientBg, x: f32, y: f32, w: f32, h: f32, opacity: f32) -> Self {
            let mut v = vec![x.to_bits(), y.to_bits(), w.to_bits(), h.to_bits(), opacity.to_bits(), g.angle_deg.to_bits()];
            for (t, c) in &g.stops {
                v.push(t.to_bits());
                v.push(u32::from_le_bytes([c.r, c.g, c.b, c.a]));
            }
            Key(v)
        }
    }

    #[derive(Default)]
    struct Rows {
        rows: HashMap<i32, Vec<Color>>,
    }

    thread_local! {
        static CACHE: RefCell<(HashMap<Key, Rows>, usize)> = RefCell::new((HashMap::new(), 0));
        /// 只见过一次的键（还没进 CACHE）
        static SEEN: RefCell<std::collections::HashSet<Key>> = RefCell::new(Default::default());
    }

    /// 只记这么多个「见过一次」的键，满了清掉（键只有几十字节，这里只防无限长）
    const MAX_SEEN: usize = 64;

    /// 这个键要不要用行缓存：已经在缓存里、或者之前见过一次（同一位置画第二遍，才值得存）。
    /// 第一次见到只记下来，返回 false
    pub fn admit(key: &Key) -> bool {
        if CACHE.with(|c| c.borrow().0.contains_key(key)) {
            return true;
        }
        SEEN.with(|s| {
            let mut s = s.borrow_mut();
            if s.remove(key) {
                return true;
            }
            if s.len() >= MAX_SEEN {
                s.clear();
            }
            s.insert(key.clone());
            false
        })
    }

    /// 整个清掉（进后台 / 内存告警，`trim_memory`）
    pub fn clear() {
        CACHE.with(|c| *c.borrow_mut() = (HashMap::new(), 0));
        SEEN.with(|s| s.borrow_mut().clear());
    }

    /// 占了多少字节
    pub fn bytes() -> usize {
        CACHE.with(|c| c.borrow().1 * 4)
    }

    /// 取第 `py` 行（没有就用 `fill` 算一行宽 `width` 的），交给 `use_row`
    pub fn with_row(key: &Key, py: i32, width: usize, fill: impl FnOnce(&mut [Color]), use_row: impl FnOnce(&[Color])) {
        CACHE.with(|c| {
            let mut c = c.borrow_mut();
            let (map, total) = &mut *c;
            if !map.get(key).map_or(false, |r| r.rows.contains_key(&py)) {
                if *total + width > MAX_PIXELS {
                    map.clear();
                    *total = 0;
                }
                let mut row = vec![Color::new(0, 0, 0, 0); width];
                fill(&mut row);
                *total += width;
                map.entry(key.clone()).or_default().rows.insert(py, row);
            }
            use_row(&map[key].rows[&py]);
        })
    }
}

#[cfg(test)]
mod row_cache_tests {
    use super::*;
    use crate::Rect as GeoRect;

    fn grad() -> LinearGradientBg {
        LinearGradientBg { angle_deg: 135.0, stops: vec![(0.0, Color::new(7, 193, 96, 255)), (1.0, Color::new(16, 174, 255, 255))] }
    }

    #[test]
    fn 行缓存贴回去与现算逐字节相同() {
        // 第一遍：整块现算（键第一次见，不进缓存）；第二遍：只画中间一条（键第二次见，算完存下）；
        // 第三遍：整块，中间那条命中缓存、其余行进缓存现算 —— 三遍逐字节对得上
        let (w, h) = (700u32, 400u32);
        let draw = |clip: Option<GeoRect>| {
            let mut c = Canvas::new(w, h);
            c.clear(Color::WHITE);
            if let Some(r) = clip {
                c.clip_rect(r);
            }
            draw_linear_gradient(&mut c, &grad(), 10.3, 7.7, 660.0, 380.0, [24.0; 4], 1.0);
            c.pixels().to_vec()
        };
        let full = draw(None);
        let band = draw(Some(GeoRect::new(0.0, 120.0, w as f32, 90.0)));
        let again = draw(None);
        assert!(full == again, "命中缓存的整块与现算不同");
        for y in 120..210usize {
            let r = y * w as usize..(y + 1) * w as usize;
            assert!(full[r.clone()] == band[r], "第 {y} 行：补条带命中缓存与整块现算不同");
        }
    }
}
