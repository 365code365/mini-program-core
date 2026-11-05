//! 变换合成：把离屏画布按 CSS `transform` 仿射贴回目标画布。
//!
//! 为什么要离屏：扫描线渲染器的绘制 API 都是「轴对齐矩形 + 绝对坐标」，
//! 旋转/缩放没法直接下发给每个子节点。做法是把带 transform 的子树先画到一张
//! 透明临时画布，再按矩阵逆变换逐像素双线性采样贴回 —— 这样 `transform` 对
//! 任意子树（含文字、图片、圆角、阴影）都成立，语义与浏览器一致。

use super::components::Transform;
use crate::{Canvas, Color};

/// transform 是否只是平移（可以直接用偏移绘制，不必离屏）
pub fn is_translate_only(t: &Transform) -> bool {
    (t.scale_x - 1.0).abs() < 1e-3
        && (t.scale_y - 1.0).abs() < 1e-3
        && t.rotate.abs() < 1e-3
        && t.skew_x.abs() < 1e-3
        && t.skew_y.abs() < 1e-3
}

/// transform 是否等价于「什么都不做」
pub fn is_identity(t: &Transform) -> bool {
    is_translate_only(t) && t.translate_x.abs() < 1e-3 && t.translate_y.abs() < 1e-3
}

/// 2x2 线性部分：rotate ∘ scale ∘ skew（与 CSS 常见书写顺序一致的近似）
fn linear_matrix(t: &Transform) -> (f32, f32, f32, f32) {
    let rad = t.rotate.to_radians();
    let (sin, cos) = (rad.sin(), rad.cos());
    let (sx, sy) = (t.scale_x, t.scale_y);
    let (kx, ky) = (t.skew_x.to_radians().tan(), t.skew_y.to_radians().tan());

    // skew 矩阵 [[1, kx], [ky, 1]]，再左乘 scale，再左乘 rotate
    let (a1, b1, c1, d1) = (sx, sx * kx, sy * ky, sy);
    (
        cos * a1 - sin * c1,
        cos * b1 - sin * d1,
        sin * a1 + cos * c1,
        sin * b1 + cos * d1,
    )
}

/// 计算带 transform 后子树在目标画布上的包围盒（用于确定采样范围；setData 失效分析也拿它算画到了哪）
pub(crate) fn transformed_bounds(
    src_w: f32,
    src_h: f32,
    src_pos: (f32, f32),
    center: (f32, f32),
    t: &Transform,
) -> (f32, f32, f32, f32) {
    let (a, b, c, d) = linear_matrix(t);
    let corners = [
        (src_pos.0, src_pos.1),
        (src_pos.0 + src_w, src_pos.1),
        (src_pos.0, src_pos.1 + src_h),
        (src_pos.0 + src_w, src_pos.1 + src_h),
    ];
    let (mut min_x, mut min_y) = (f32::MAX, f32::MAX);
    let (mut max_x, mut max_y) = (f32::MIN, f32::MIN);
    for (px, py) in corners {
        let (dx, dy) = (px - center.0, py - center.1);
        let tx = center.0 + a * dx + b * dy + t.translate_x;
        let ty = center.1 + c * dx + d * dy + t.translate_y;
        min_x = min_x.min(tx);
        min_y = min_y.min(ty);
        max_x = max_x.max(tx);
        max_y = max_y.max(ty);
    }
    (min_x, min_y, max_x, max_y)
}

/// 给 transform 子树开一张透明离屏，并裁到 [`source_clip`] 那一块。
/// 补条带只要一条时，旋转 / 缩放的元素不必整块光栅化；采不到的像素画了也白画，结果逐字节不变
pub fn offscreen_for(dst: &Canvas, size: (u32, u32), src_pos: (f32, f32), center: (f32, f32), t: &Transform) -> Canvas {
    let mut off = Canvas::new(size.0, size.1);
    off.clear(Color::TRANSPARENT);
    if let Some(r) = source_clip(dst, size.0 as f32, size.1 as f32, src_pos, center, t) {
        off.clip_rect(r);
    }
    off
}

/// [`blit_transformed`] 贴回目标时，目标裁剪区里的像素只会采到源画布的哪一块（源画布坐标，含双线性的 2px 余量）。
/// 离屏子树据此只画这一块：补条带只要一条时，带 transform 的元素不必整块光栅化。
/// 返回 None 表示要整块（裁剪区就是整个画布、或矩阵退化）
pub fn source_clip(dst: &Canvas, src_w: f32, src_h: f32, src_pos: (f32, f32), center: (f32, f32), t: &Transform) -> Option<crate::Rect> {
    let (bx0, bx1, by0, by1) = dst.draw_bounds();
    if bx0 <= 0 && by0 <= 0 && bx1 >= dst.width() as i32 && by1 >= dst.height() as i32 {
        return None;
    }
    let (a, b, c, d) = linear_matrix(t);
    let det = a * d - b * c;
    if det.abs() < 1e-6 {
        return None;
    }
    let (ia, ib, ic, id) = (d / det, -b / det, -c / det, a / det);
    let inv = |px: f32, py: f32| {
        let dx = px - center.0 - t.translate_x;
        let dy = py - center.1 - t.translate_y;
        (center.0 + ia * dx + ib * dy - src_pos.0, center.1 + ic * dx + id * dy - src_pos.1)
    };
    let pts = [inv(bx0 as f32, by0 as f32), inv(bx1 as f32, by0 as f32), inv(bx0 as f32, by1 as f32), inv(bx1 as f32, by1 as f32)];
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for (x, y) in pts {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let (x0, y0) = ((x0 - 2.0).floor().max(0.0), (y0 - 2.0).floor().max(0.0));
    let (x1, y1) = ((x1 + 2.0).ceil().min(src_w), (y1 + 2.0).ceil().min(src_h));
    Some(crate::Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0)))
}

/// 把 `src` 按 transform 合成到 `dst`。
///
/// - `src_pos`：`src` 左上角在目标画布中的（未变换）位置
/// - `center`：变换原点（CSS `transform-origin` 默认元素中心）
/// - `opacity`：整体透明度（0..1），与像素自身 alpha 相乘
pub fn blit_transformed(
    dst: &mut Canvas,
    src: &Canvas,
    src_pos: (f32, f32),
    center: (f32, f32),
    t: &Transform,
    opacity: f32,
) {
    blit_transformed_n(dst, src, src_pos, center, t, opacity, None, true);
}

/// [`blit_transformed`]，`threads` 给定时用这么多线程；`narrow = false` 不按源画布内容框收窄。
/// 单测拿这两个开关比「单线程、不收窄」（原来的逐点实现）与默认走法逐字节相同
fn blit_transformed_n(
    dst: &mut Canvas,
    src: &Canvas,
    src_pos: (f32, f32),
    center: (f32, f32),
    t: &Transform,
    opacity: f32,
    threads: Option<usize>,
    narrow: bool,
) {
    let (a, b, c, d) = linear_matrix(t);
    let det = a * d - b * c;
    if det.abs() < 1e-6 {
        return; // 退化矩阵（scale 0）：整体不可见
    }
    // 逆矩阵
    let (ia, ib, ic, id) = (d / det, -b / det, -c / det, a / det);

    let (src_w, src_h) = (src.width() as f32, src.height() as f32);
    let (min_x, min_y, max_x, max_y) = transformed_bounds(src_w, src_h, src_pos, center, t);

    // 只走目标裁剪区里的像素：区外的 `set_pixel` 本来就一个不写（结果逐字节不变）。
    // 局部重画（补条带）的裁剪区只有一条，带 transform 的元素（WXSS 页那几个旋转 / 缩放的方块）
    // 却要整块逐像素逆变换 + 双线性采样 —— 一加 PLY110 上补一条带 11~20ms，滑到那一屏每次都掉帧
    let (bx0, bx1, by0, by1) = dst.draw_bounds();
    let x0 = (min_x.floor().max(0.0) as i32).max(bx0);
    let y0 = (min_y.floor().max(0.0) as i32).max(by0);
    let x1 = (max_x.ceil() as i32).min(dst.width() as i32).min(bx1);
    let y1 = (max_y.ceil() as i32).min(dst.height() as i32).min(by1);
    if x1 <= x0 || y1 <= y0 {
        return;
    }

    // 源画布里有内容（alpha > 0）的那一块。旋转时外包框是转过的「离屏 + 边距」整块，
    // 而离屏四周的边距（给旋转溢出留的）与圆形封面的四角全透明：静心播放页 440rpx 的封面转到 45° 时
    // 外包框 ~2.2M 像素，有内容的只有 ~0.56M。每行按它解出 px 的区间，区间外的点采样必然全透明、一个不写。
    // 扫一遍边距也要时间：只在要贴的面积不比源画布小太多时扫（补条带那种细条不值得）
    let dst_area = (x1 - x0) as usize * (y1 - y0) as usize;
    let content = if narrow && dst_area * 2 >= src.pixels().len() {
        match content_bounds(src) {
            Some(b) => Some(b),
            None => return, // 整张透明：采样处处为 None
        }
    } else {
        None
    };

    // 不拷贝：源画布与目标不是同一块，借用就够（从前每次 to_vec 一整张离屏）
    let p = BlitRows {
        content,
        src: src.pixels(),
        sw: src.width() as i32,
        sh: src.height() as i32,
        src_w,
        src_h,
        src_pos,
        center,
        tx: t.translate_x,
        ty: t.translate_y,
        inv: (ia, ib, ic, id),
        opacity,
        x0,
        x1,
    };
    // x0..x1 / y0..y1 已经与画布边界、裁剪区（draw_bounds，与 set_pixel 同一套取整）求过交：
    // 直接写像素，省掉 set_pixel 每个点的 8 次比较与裁剪矩形的浮点截断（混合公式与 set_pixel 相同）
    let dw = dst.width() as usize;
    let rows = &mut dst.pixels_mut()[y0 as usize * dw..y1 as usize * dw];
    let threads = threads.unwrap_or_else(|| blit_threads(dst_area));
    if threads <= 1 {
        p.run(rows, dw, y0);
        return;
    }
    // 大块分给几个线程按行带做。每一行只读源画布、只写目标的这一行，与单线程逐字节相同。
    // 行带按「谁空了谁领下一条」分：手机大小核混着，静态均分的话落在小核上的那份拖住整帧；
    // 带子也不能太宽，圆形封面上下两头几乎全透明，按半屏切两个线程的活差好几倍。
    // 静心播放页（440rpx 的旋转封面 + 缩放光晕）一加 PLY110 上单线程贴回占引擎线程 67%
    let bands = std::sync::Mutex::new(rows.chunks_mut(BAND_ROWS * dw).enumerate());
    let work = || loop {
        let next = bands.lock().map(|mut it| it.next()).unwrap_or(None);
        let Some((i, band)) = next else { break };
        p.run(band, dw, y0 + (i * BAND_ROWS) as i32);
    };
    // 常驻帮手（blit_pool.rs）：每帧现起线程的开销与被排到小核上都省掉。帮手起不来、
    // 或别处正在用，就只有调用线程在领带子，结果一样
    super::blit_pool::run(threads, &work);
}

/// 分线程贴回的行带高度（行）。几十条带子分给 ≤4 个线程，锁只在领带子时拿一次
const BAND_ROWS: usize = 16;
/// 目标像素少于这么多就不分线程：起线程的几十微秒比省下的多（小方块、补条带、桌面的 gallery 场景）
const PARALLEL_MIN_PIXELS: usize = 160_000;

/// 大块贴回最多用几个线程（含调用线程）。`MINI_BLIT_THREADS=1` 强制单线程（对照 / 排查），
/// 默认取核数、最多 4 个（手机常见 4 个大核：引擎线程 + 3 个帮手）
fn blit_thread_count() -> usize {
    static MAX: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *MAX.get_or_init(|| {
        std::env::var("MINI_BLIT_THREADS")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).min(4))
            .max(1)
    })
}

/// 这一块贴回用几个线程：小块只用调用线程
fn blit_threads(pixels: usize) -> usize {
    if pixels < PARALLEL_MIN_PIXELS {
        1
    } else {
        blit_thread_count()
    }
}

/// 源画布里 alpha > 0 的像素的外包框（列 `[l, r)`、行 `[t, b)`），整张透明返回 None。
/// 只扫透明的边：上下从两头往里找第一行有内容的，左右每行只看还没覆盖到的那两截
fn content_bounds(src: &Canvas) -> Option<(usize, usize, usize, usize)> {
    let (w, h) = (src.width() as usize, src.height() as usize);
    let px = src.pixels();
    let row = |y: usize| &px[y * w..(y + 1) * w];
    let has = |c: &Color| c.a != 0;
    let t = (0..h).find(|&y| row(y).iter().any(has))?;
    let b = (t..h).rev().find(|&y| row(y).iter().any(has)).map_or(t + 1, |y| y + 1);
    let (mut l, mut r) = (w, 0usize);
    for y in t..b {
        let rw = row(y);
        if let Some(i) = rw[..l].iter().position(has) {
            l = i;
        }
        if let Some(i) = rw[r.max(l)..].iter().rposition(has) {
            r = r.max(l) + i + 1;
        }
    }
    Some((l, r, t, b))
}

/// 一次贴回里与行无关的量：逆矩阵、源画布、目标列范围
struct BlitRows<'a> {
    /// 源画布有内容的外包框（[`content_bounds`]）；None = 不按它收窄
    content: Option<(usize, usize, usize, usize)>,
    src: &'a [Color],
    sw: i32,
    sh: i32,
    src_w: f32,
    src_h: f32,
    src_pos: (f32, f32),
    center: (f32, f32),
    tx: f32,
    ty: f32,
    inv: (f32, f32, f32, f32),
    opacity: f32,
    x0: i32,
    x1: i32,
}

impl BlitRows<'_> {
    /// 这一行里采样可能不全透明的 px 区间（含两头的余量），落在 `[x0, x1)` 里；None = 整行都不用贴。
    ///
    /// 采样点 (ux, uy) 随 px 线性变：ux = ux0 + ia·px，uy = uy0 + ic·px。双线性取 floor(ux - 0.5) 与它右边一列，
    /// 有内容的列是 `[l, r)` 时，ux 落在 `[l - 0.5, r + 0.5)` 之外四个点就全透明（越界夹边也一样），行同理。
    /// 用 f64 解这两组不等式，范围在源画布上再各放宽 1px、px 上再放宽 1~2px：f32 算出来的 ux 与这里差不到 0.01，
    /// 区间外的点在 f32 下也必然在内容框外 —— 区间里照旧走原来的逐点判断，结果逐字节不变
    #[inline]
    fn row_span(&self, ib_dy: f32, id_dy: f32) -> Option<(i32, i32)> {
        let Some((l, r, t, b)) = self.content else { return Some((self.x0, self.x1)) };
        let (ia, _, ic, _) = self.inv;
        let (center, src_pos) = (self.center, self.src_pos);
        // px = 0 处的采样点（与逐点的式子同一个形状，只是换成 f64）
        let d0 = 0.5 - center.0 as f64 - self.tx as f64;
        let ux0 = center.0 as f64 + ia as f64 * d0 + ib_dy as f64 - src_pos.0 as f64;
        let uy0 = center.1 as f64 + ic as f64 * d0 + id_dy as f64 - src_pos.1 as f64;
        let (mut lo, mut hi) = (self.x0 as f64, self.x1 as f64);
        for (u0, k, a, z) in [(ux0, ia as f64, l as f64, r as f64), (uy0, ic as f64, t as f64, b as f64)] {
            let (min_u, max_u) = (a - 1.5, z + 1.5);
            if k.abs() < 1e-9 {
                if u0 < min_u || u0 > max_u {
                    return None;
                }
                continue;
            }
            let (p, q) = ((min_u - u0) / k, (max_u - u0) / k);
            lo = lo.max(p.min(q));
            hi = hi.min(p.max(q));
        }
        let xs = ((lo.floor() as i32) - 1).max(self.x0);
        let xe = ((hi.ceil() as i32) + 2).min(self.x1);
        (xs < xe).then_some((xs, xe))
    }

    /// 贴 `rows`（目标画布从第 `y_first` 行起的连续整行，每行 `dw` 个像素）
    fn run(&self, rows: &mut [Color], dw: usize, y_first: i32) {
        let (ia, ib, ic, id) = self.inv;
        let (center, src_pos) = (self.center, self.src_pos);
        for (i, row) in rows.chunks_mut(dw).enumerate() {
            let py = y_first + i as i32;
            // 只随行变的那几项提到行外：同一个表达式、同样的求值次序，结果逐位不变
            let dy = py as f32 + 0.5 - center.1 - self.ty;
            let (ib_dy, id_dy) = (ib * dy, id * dy);
            let Some((xs, xe)) = self.row_span(ib_dy, id_dy) else { continue };
            for px in xs..xe {
                // 目标像素中心 → 逆变换 → 源画布坐标
                let dx = px as f32 + 0.5 - center.0 - self.tx;
                let ux = center.0 + ia * dx + ib_dy - src_pos.0;
                let uy = center.1 + ic * dx + id_dy - src_pos.1;
                if ux < -0.5 || uy < -0.5 || ux > self.src_w + 0.5 || uy > self.src_h + 0.5 {
                    continue;
                }
                if let Some(color) = sample_bilinear(self.src, self.sw, self.sh, ux - 0.5, uy - 0.5) {
                    let alpha = (color.a as f32 * self.opacity).round().clamp(0.0, 255.0) as u8;
                    if alpha > 0 {
                        let c = Color::new(color.r, color.g, color.b, alpha);
                        let idx = px as usize;
                        row[idx] = if alpha == 255 { c } else { c.blend(&row[idx]) };
                    }
                }
            }
        }
    }
}

/// 双线性采样（按 alpha 加权，避免透明边缘把黑色带进来）
#[inline]
fn sample_bilinear(pixels: &[Color], w: i32, h: i32, fx: f32, fy: f32) -> Option<Color> {
    if w <= 0 || h <= 0 {
        return None;
    }
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;

    let (p00, p10, p01, p11) = if x0 >= 0 && y0 >= 0 && x0 + 1 < w && y0 + 1 < h {
        // 2x2 全在画布里（绝大多数点）：不必逐个夹边
        let i = (y0 * w + x0) as usize;
        let j = i + w as usize;
        (pixels[i], pixels[i + 1], pixels[j], pixels[j + 1])
    } else {
        let at = |x: i32, y: i32| -> Color {
            let x = x.clamp(0, w - 1);
            let y = y.clamp(0, h - 1);
            pixels[(y * w + x) as usize]
        };
        (at(x0, y0), at(x0 + 1, y0), at(x0, y0 + 1), at(x0 + 1, y0 + 1))
    };
    // 两条捷径，结果与下面的通用公式逐字节相同（改动时拿旧实现对 400 组随机缩放 / 旋转 / 倾斜 / 裁剪 / 透明度逐字节比过）：
    // - 四个都全透明：通用公式算出 a = 0 < 0.5，同样返回 None；
    // - 四个是同一个颜色：权重和与 1 只差几个 ulp，a 与各通道离整数不到 1e-3，四舍五入回到原值。
    // 离屏子树大片是透明底或纯色（光晕、圆形封面的描边与四角），这些点不必走四次乘除。
    // 静心播放页（缩放的光晕 + 旋转的封面）一加 PLY110 上每帧 43ms 画在这里
    if (p00.a | p10.a | p01.a | p11.a) == 0 {
        return None;
    }
    if p00 == p10 && p00 == p01 && p00 == p11 {
        return Some(p00);
    }

    let w00 = (1.0 - tx) * (1.0 - ty);
    let w10 = tx * (1.0 - ty);
    let w01 = (1.0 - tx) * ty;
    let w11 = tx * ty;

    let a = p00.a as f32 * w00 + p10.a as f32 * w10 + p01.a as f32 * w01 + p11.a as f32 * w11;
    if a < 0.5 {
        return None;
    }
    // 颜色按 alpha 加权平均：透明像素不该把它的 RGB 混进来
    let wa = |p: &Color, wt: f32| p.a as f32 * wt;
    let (s00, s10, s01, s11) = (wa(&p00, w00), wa(&p10, w10), wa(&p01, w01), wa(&p11, w11));
    let sum = s00 + s10 + s01 + s11;
    if sum < 1e-3 {
        return None;
    }
    let r = (p00.r as f32 * s00 + p10.r as f32 * s10 + p01.r as f32 * s01 + p11.r as f32 * s11) / sum;
    let g = (p00.g as f32 * s00 + p10.g as f32 * s10 + p01.g as f32 * s01 + p11.g as f32 * s11) / sum;
    let bl = (p00.b as f32 * s00 + p10.b as f32 * s10 + p01.b as f32 * s01 + p11.b as f32 * s11) / sum;
    Some(Color::new(
        r.round().clamp(0.0, 255.0) as u8,
        g.round().clamp(0.0, 255.0) as u8,
        bl.round().clamp(0.0, 255.0) as u8,
        a.round().clamp(0.0, 255.0) as u8,
    ))
}

/// 离屏画布需要的边距（px）：缩放放大与旋转都会让内容溢出原盒子。
pub fn transform_padding(w: f32, h: f32, t: &Transform) -> f32 {
    let long = w.max(h);
    let scale_pad = (t.scale_x.abs().max(t.scale_y.abs()) - 1.0).max(0.0) * long / 2.0;
    let rotate_pad = if t.rotate.abs() > 1e-3 || t.skew_x.abs() > 1e-3 || t.skew_y.abs() > 1e-3 {
        ((w * w + h * h).sqrt() - w.min(h)) / 2.0
    } else {
        0.0
    };
    (scale_pad.max(rotate_pad) + 2.0).ceil()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 分线程、按内容框收窄的贴回，与「单线程、不收窄」（原来的逐点实现）逐字节相同：
    /// 旋转 / 缩放 / 倾斜 / 平移 / 透明度 / 目标裁剪区都混着来，含 0° 与 90° 这类轴对齐（解区间时斜率为 0）；
    /// 源画布四周留宽透明边、中间圆形内容有透明洞、渐变与硬边（双线性四种分支都走到），目标底色不透明（半透明像素走混合）
    #[test]
    fn 分线程与内容框收窄的贴回与逐点实现逐字节相同() {
        let (sw, sh) = (760usize, 700usize);
        let mut src = Canvas::new(sw as u32, sh as u32);
        src.clear(Color::TRANSPARENT);
        for (i, p) in src.pixels_mut().iter_mut().enumerate() {
            let (x, y) = ((i % sw) as i32, (i / sw) as i32);
            let (dx, dy) = (x - 380, y - 330);
            if dx * dx + dy * dy < 240 * 240 && (x / 37 + y / 23) % 5 != 0 {
                *p = Color::new((x % 256) as u8, (y % 256) as u8, ((x ^ y) % 256) as u8, if y % 3 == 0 { 255 } else { (x % 200) as u8 + 55 });
            }
        }
        assert_eq!(content_bounds(&src), Some((141, 620, 91, 570)));
        let mut seed = 0x9e37_79b9u32;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed % 10_000) as f32 / 10_000.0
        };
        for case in 0..30 {
            let mut t = Transform::new();
            t.rotate = match case % 6 {
                0 => 0.0,
                1 => 90.0,
                _ => rnd() * 360.0,
            };
            t.scale_x = if case % 6 == 0 { 1.5 } else { 0.4 + rnd() * 1.4 };
            t.scale_y = if case % 3 == 0 { t.scale_x } else { 0.4 + rnd() * 1.4 };
            t.skew_x = if case % 4 == 3 { rnd() * 30.0 - 15.0 } else { 0.0 };
            t.translate_x = rnd() * 80.0 - 40.0;
            t.translate_y = rnd() * 80.0 - 40.0;
            let opacity = if case % 2 == 0 { 1.0 } else { 0.2 + rnd() * 0.8 };
            let clip = (case % 5 == 2).then(|| crate::Rect::new(rnd() * 300.0, rnd() * 300.0, 300.0 + rnd() * 500.0, 200.0 + rnd() * 600.0));
            let make = || {
                let mut c = Canvas::new(1100, 1200);
                c.clear(Color::new(30, 40, 90, 255));
                if let Some(r) = clip {
                    c.clip_rect(r);
                }
                c
            };
            let (pos, center) = ((150.0, 200.0), (530.0, 550.0));
            let mut reference = make();
            blit_transformed_n(&mut reference, &src, pos, center, &t, opacity, Some(1), false);
            for (threads, narrow) in [(1, true), (4, true), (4, false)] {
                let mut got = make();
                blit_transformed_n(&mut got, &src, pos, center, &t, opacity, Some(threads), narrow);
                assert!(got.pixels() == reference.pixels(), "第 {case} 组 {t:?} opacity {opacity} 线程 {threads} 收窄 {narrow} 不一致");
            }
        }
    }
}
