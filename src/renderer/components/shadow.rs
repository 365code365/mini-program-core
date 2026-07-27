//! `box-shadow`：一次算好模糊掩膜并缓存，每帧只做一次混合。
//!
//! 以前是「用 blur/2 层逐渐扩大的半透明圆角矩形叠出模糊感」：
//! `box-shadow: 0 8rpx 24rpx rgba(...)` 在 2 倍屏上 blur=24px → 12 层，
//! 每层都要把整张卡片面积（600x200 ≈ 12 万像素）做一遍带抗锯齿的路径填充 + 半透明混合，
//! 一张卡片一帧就是 140 万次像素混合。tea-app 里带阴影的卡片一屏有三四张，
//! 于是 **76% 的绘制时间花在 view 上**（实测 60 帧拖动里 448ms/586ms），
//! 手上就是「滑动发抖」。
//!
//! 现在按 CSS 的定义做：把「阴影形状」画成一张 8 位覆盖率掩膜，用可分离盒式模糊
//! 三遍逼近高斯（σ = blur/2，与浏览器一致），掩膜按几何尺寸缓存；
//! 每帧只剩 `Canvas::blend_mask` 一趟乘加。同一尺寸的卡片共用一张掩膜。

use crate::{Canvas, Color};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// 掩膜：(宽, 高, 覆盖率数据)
type Mask = Arc<(usize, usize, Vec<u8>)>;

fn mask_cache() -> &'static Mutex<HashMap<String, Mask>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Mask>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 画一层外阴影。`(x, y, w, h)` 是**盒子**的位置尺寸（物理像素）。
pub fn draw_outer_shadow(
    canvas: &mut Canvas,
    color: Color,
    offset_x: f32,
    offset_y: f32,
    blur: f32,
    spread: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
) {
    // 阴影形状 = 盒子按 spread 外扩
    let sw = (w + spread * 2.0).max(0.0);
    let sh = (h + spread * 2.0).max(0.0);
    if sw < 1.0 || sh < 1.0 || color.a == 0 {
        return;
    }
    let radius = (radius + spread).max(0.0);
    let blur = blur.max(0.0);
    let mask = get_mask(sw, sh, radius, blur);
    let (mw, _mh, data) = &*mask;
    // 掩膜比形状四周各多 pad，绘制时要把这段外扩补回去
    let pad = pad_for(blur) as f32;
    let dst_x = (x - spread - pad + offset_x).round() as i32;
    let dst_y = (y - spread - pad + offset_y).round() as i32;
    canvas.blend_mask(dst_x, dst_y, data, *mw, color);
}

/// 单遍盒式模糊的半径。CSS 的 `blur` 对应 σ = blur/2；三遍盒式模糊逼近该高斯时
/// 盒**宽** d ≈ 1.88σ，故半径 r = (d-1)/2。
fn blur_radius(blur: f32) -> usize {
    if blur <= 0.0 {
        return 0;
    }
    let sigma = blur / 2.0;
    (((sigma * 1.88) - 1.0) / 2.0).round().max(1.0) as usize
}

/// 掩膜四周要留的余量：三遍模糊最多扩散 3r 像素，留够才不会把边缘切掉。
fn pad_for(blur: f32) -> usize {
    blur_radius(blur) * 3 + 2
}

fn get_mask(w: f32, h: f32, radius: f32, blur: f32) -> Mask {
    // 量化到整像素，让「同一批卡片」共用一张掩膜（尺寸差 1px 也算不同键，
    // 但布局产出的卡片尺寸本来就是一致的）
    let wi = w.round().max(1.0) as usize;
    let hi = h.round().max(1.0) as usize;
    let ri = radius.round().max(0.0) as usize;
    let bi = blur.round().max(0.0) as usize;
    let key = format!("{wi}x{hi}r{ri}b{bi}");
    if let Ok(guard) = mask_cache().lock() {
        if let Some(hit) = guard.get(&key) {
            return hit.clone();
        }
    }
    let mask = Arc::new(build_mask(wi, hi, ri as f32, bi as f32));
    if let Ok(mut guard) = mask_cache().lock() {
        // 简单上限：超了整体失效，避免长跑内存无界增长
        if guard.len() > 64 {
            guard.clear();
        }
        guard.insert(key, mask.clone());
    }
    mask
}

/// 生成「圆角矩形 + 高斯模糊」的覆盖率掩膜。
fn build_mask(w: usize, h: usize, radius: f32, blur: f32) -> (usize, usize, Vec<u8>) {
    let pad = pad_for(blur);
    let mw = w + pad * 2;
    let mh = h + pad * 2;
    let mut data = vec![0u8; mw * mh];

    // 1) 圆角矩形的覆盖率（角上用有符号距离做一像素抗锯齿）
    let r = radius.min(w as f32 / 2.0).min(h as f32 / 2.0).max(0.0);
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let (ex, ey) = (cx - r, cy - r); // 直边半长
    for py in 0..h {
        let fy = py as f32 + 0.5 - cy;
        for px in 0..w {
            let fx = px as f32 + 0.5 - cx;
            let dx = fx.abs() - ex;
            let dy = fy.abs() - ey;
            let cov = if dx <= 0.0 || dy <= 0.0 {
                // 在十字区域内：只需判到最近直边的距离
                let d = dx.max(dy) - r;
                (0.5 - d).clamp(0.0, 1.0)
            } else {
                let d = (dx * dx + dy * dy).sqrt() - r;
                (0.5 - d).clamp(0.0, 1.0)
            };
            data[(py + pad) * mw + px + pad] = (cov * 255.0).round() as u8;
        }
    }

    // 2) 可分离盒式模糊三遍 ≈ 高斯。CSS 的 blur 半径对应 σ = blur/2，
    //    三遍盒式模糊逼近该高斯的盒宽 d ≈ 1.88σ（Wells 的经典近似）。
    let r = blur_radius(blur);
    if r > 0 {
        for _ in 0..3 {
            box_blur_h(&mut data, mw, mh, r);
            box_blur_v(&mut data, mw, mh, r);
        }
    }
    (mw, mh, data)
}

/// 横向滑动窗口均值（半径 r，窗口 2r+1）
fn box_blur_h(data: &mut [u8], w: usize, h: usize, r: usize) {
    if r == 0 || w == 0 {
        return;
    }
    let mut row = vec![0u8; w];
    let win = (2 * r + 1) as u32;
    for y in 0..h {
        let base = y * w;
        row.copy_from_slice(&data[base..base + w]);
        // 初始窗口：左端按边缘外为 0 处理（阴影外面本来就是空的）
        let mut sum: u32 = 0;
        for x in 0..=r.min(w - 1) {
            sum += row[x] as u32;
        }
        for x in 0..w {
            data[base + x] = (sum / win) as u8;
            // 滑窗：加入 x+r+1，移出 x-r
            let add = x + r + 1;
            if add < w {
                sum += row[add] as u32;
            }
            if x >= r {
                sum -= row[x - r] as u32;
            }
        }
    }
}

/// 纵向滑动窗口均值
fn box_blur_v(data: &mut [u8], w: usize, h: usize, r: usize) {
    if r == 0 || h == 0 {
        return;
    }
    let mut col = vec![0u8; h];
    let win = (2 * r + 1) as u32;
    for x in 0..w {
        for y in 0..h {
            col[y] = data[y * w + x];
        }
        let mut sum: u32 = 0;
        for y in 0..=r.min(h - 1) {
            sum += col[y] as u32;
        }
        for y in 0..h {
            data[y * w + x] = (sum / win) as u8;
            let add = y + r + 1;
            if add < h {
                sum += col[add] as u32;
            }
            if y >= r {
                sum -= col[y - r] as u32;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 掩膜中心必须是实心的，边缘要有一段渐变（模糊真的做了），
    /// 且四周留出的 pad 足够容纳模糊扩散。
    #[test]
    fn mask_is_solid_inside_and_fades_outside() {
        let (mw, mh, data) = build_mask(80, 40, 8.0, 12.0);
        let pad = pad_for(12.0);
        assert_eq!(mw, 80 + pad * 2);
        assert_eq!(mh, 40 + pad * 2);
        let at = |x: usize, y: usize| data[y * mw + x];
        // 正中间：完全不透明
        assert!(at(mw / 2, mh / 2) > 250, "中心应实心，实际 {}", at(mw / 2, mh / 2));
        // 形状边界外一点：应该衰减但仍有值（模糊扩散出来）
        let edge = at(pad / 2, mh / 2);
        assert!(edge > 0 && edge < 200, "边缘应是渐变，实际 {edge}");
        // 最外圈：基本为 0（pad 足够）
        assert!(at(0, 0) <= 2, "最外圈应接近 0，实际 {}", at(0, 0));
    }

    /// blur=0 时不该有渐变：边界一像素之内就从实心跳到空。
    #[test]
    fn zero_blur_keeps_hard_edge() {
        let (mw, _mh, data) = build_mask(20, 20, 0.0, 0.0);
        let pad = pad_for(0.0);
        let at = |x: usize, y: usize| data[y * mw + x];
        assert!(at(pad + 10, pad + 10) > 250);
        assert_eq!(at(pad - 1, pad + 10), 0, "blur=0 时形状外应为 0");
    }

    /// 模糊后总"墨量"应基本守恒（盒式模糊是均值滤波，不该凭空变暗/变亮）。
    /// 这一条能挡住「窗口宽度和除数不匹配」这类静默漏能量的错误。
    #[test]
    fn blur_preserves_total_coverage() {
        let (_, _, sharp) = build_mask(60, 60, 0.0, 0.0);
        let (_, _, blurred) = build_mask(60, 60, 0.0, 16.0);
        let sum = |v: &Vec<u8>| v.iter().map(|b| *b as u64).sum::<u64>();
        let (a, b) = (sum(&sharp), sum(&blurred));
        let ratio = b as f64 / a as f64;
        assert!(
            (0.9..=1.1).contains(&ratio),
            "模糊前后覆盖率总量应守恒，实际比值 {ratio:.3}"
        );
    }

    /// 同一几何尺寸必须命中缓存（拿到同一份 Arc），否则每帧都要重算模糊。
    #[test]
    fn mask_is_cached_by_geometry() {
        let a = get_mask(120.0, 60.0, 12.0, 16.0);
        let b = get_mask(120.0, 60.0, 12.0, 16.0);
        assert!(Arc::ptr_eq(&a, &b), "相同尺寸的阴影掩膜应复用缓存");
    }
}
