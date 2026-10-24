//! 预览层的光栅化：按屏幕尺寸缩好的图（缓存）、把图按缩放 / 位移 / 不透明度贴进输出缓冲。
//!
//! 从前每一帧都对原图（长边 2048）逐像素浮点缩放、再开一块整屏的临时缓冲按 alpha 混回去：
//! 一加 PLY110 上一帧十几毫秒，左右滑就是一顿一顿的。现在：
//! - 每张图只缩一次（按屏幕 aspectFit 的尺寸，盒式滤波，比最近邻不闪），在后台线程缩，留在 [`FIT_CACHE`]；
//!   缩好之前那几帧拿原图最近邻凑合；
//! - 缩放 1、不透明（左右滑动的绝大多数帧）是整行拷贝；
//! - 开合 / 下拉那几帧按目标像素反查（最近邻，预先算好每列的源下标），混色用整数。
use std::cell::RefCell;
use std::sync::Arc;

/// 缩好的图（0x00RRGGBB，已压在黑底上）
pub(super) struct Fitted {
    pub w: u32,
    pub h: u32,
    pub px: Arc<Vec<u32>>,
}

/// 缓存几张：当前 + 左右各一张，再多留一张给来回滑
const FIT_CACHE: usize = 4;

thread_local! {
    static CACHE: RefCell<Vec<(String, u32, u32, Fitted)>> = const { RefCell::new(Vec::new()) };
}

/// `iw × ih` 按 aspectFit 放进 `w × h`，取整后的尺寸
pub(super) fn fit_size(iw: u32, ih: u32, w: u32, h: u32) -> (u32, u32) {
    if iw == 0 || ih == 0 {
        return (0, 0);
    }
    let s = (w as f32 / iw as f32).min(h as f32 / ih as f32);
    (((iw as f32 * s).round() as u32).clamp(1, w), ((ih as f32 * s).round() as u32).clamp(1, h))
}

/// 已经缩好的那份（不触发解码 / 缩放）
pub(super) fn cached(url: &str, w: u32, h: u32) -> Option<Fitted> {
    CACHE.with(|c| {
        c.borrow().iter().find(|e| e.0 == url && e.1 == w && e.2 == h).map(|e| Fitted { w: e.3.w, h: e.3.h, px: e.3.px.clone() })
    })
}

/// 缩好的那份。没有就交给后台线程去缩、先给 None（调用方拿原图凑合画这几帧，见 `mod.rs::render`）。
///
/// 为什么不在引擎线程上缩：长边 2048 的照片盒式滤波一次在一加 PLY110 上 30~45ms，
/// 打开 / 预取左右两张时那一帧就卡住了（量过：预览里左右滑屏上重复 45%，全是这几下）
pub(super) fn fitted(url: &str, w: u32, h: u32) -> Option<Fitted> {
    if url.is_empty() {
        return None;
    }
    adopt_done();
    if let Some(f) = cached(url, w, h) {
        return Some(f);
    }
    request(url, w, h);
    None
}

/// 后台缩好的收进缓存
fn adopt_done() {
    let done: Vec<_> = match jobs().lock() {
        Ok(mut j) => std::mem::take(&mut j.done),
        Err(_) => return,
    };
    if done.is_empty() {
        return;
    }
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        for (url, w, h, f) in done {
            c.retain(|e| !(e.0 == url && e.1 == w && e.2 == h));
            c.insert(0, (url, w, h, f));
        }
        c.truncate(FIT_CACHE);
    });
}

/// 交给后台缩（已经在缩 / 图还没解出来就什么也不做）
pub(super) fn request(url: &str, w: u32, h: u32) {
    adopt_done();
    if url.is_empty() || cached(url, w, h).is_some() {
        return;
    }
    let key = (url.to_string(), w, h);
    if jobs().lock().map(|j| j.pending.contains(&key)).unwrap_or(true) {
        return;
    }
    let Some((data, iw, ih)) = crate::renderer::components::image_rgba(url) else { return };
    let (fw, fh) = fit_size(iw, ih, w, h);
    if fw == 0 || data.len() < (iw * ih * 4) as usize {
        return;
    }
    if let Ok(mut j) = jobs().lock() {
        j.pending.push(key.clone());
    }
    std::thread::spawn(move || {
        let px = Arc::new(resample(&data, iw, ih, fw, fh));
        if let Ok(mut j) = jobs().lock() {
            j.pending.retain(|k| *k != key);
            j.done.push((key.0, key.1, key.2, Fitted { w: fw, h: fh, px }));
        }
    });
}

/// 后台缩图的进度（跨线程，所以不放线程局部）
#[derive(Default)]
struct Jobs {
    pending: Vec<(String, u32, u32)>,
    done: Vec<(String, u32, u32, Fitted)>,
}

fn jobs() -> &'static std::sync::Mutex<Jobs> {
    static J: std::sync::OnceLock<std::sync::Mutex<Jobs>> = std::sync::OnceLock::new();
    J.get_or_init(Default::default)
}

pub(super) fn clear_cache() {
    CACHE.with(|c| c.borrow_mut().clear());
}

/// RGBA → 0x00RRGGBB（压在黑底上），缩到 `fw × fh`：缩小用盒式滤波（源里落进这个像素的都平均），
/// 放大用双线性
fn resample(data: &[u8], iw: u32, ih: u32, fw: u32, fh: u32) -> Vec<u32> {
    let mut out = vec![0u32; (fw * fh) as usize];
    let at = |x: u32, y: u32| -> [u32; 3] {
        let i = ((y * iw + x) * 4) as usize;
        let a = data[i + 3] as u32;
        [data[i] as u32 * a / 255, data[i + 1] as u32 * a / 255, data[i + 2] as u32 * a / 255]
    };
    let (sx, sy) = (iw as f32 / fw as f32, ih as f32 / fh as f32);
    if sx >= 1.0 && sy >= 1.0 {
        // 每列 / 每行在源里的范围先算好
        let span = |i: u32, s: f32, max: u32| -> (u32, u32) {
            let a = ((i as f32 * s) as u32).min(max - 1);
            (a, (((i + 1) as f32 * s).ceil() as u32).clamp(a + 1, max))
        };
        let cols: Vec<(u32, u32)> = (0..fw).map(|x| span(x, sx, iw)).collect();
        for y in 0..fh {
            let (y0, y1) = span(y, sy, ih);
            for (x, &(x0, x1)) in cols.iter().enumerate() {
                let mut s = [0u32; 3];
                for yy in y0..y1 {
                    for xx in x0..x1 {
                        let p = at(xx, yy);
                        s[0] += p[0];
                        s[1] += p[1];
                        s[2] += p[2];
                    }
                }
                let n = (x1 - x0) * (y1 - y0);
                out[(y * fw) as usize + x] = ((s[0] / n) << 16) | ((s[1] / n) << 8) | (s[2] / n);
            }
        }
        return out;
    }
    for y in 0..fh {
        let fy = ((y as f32 + 0.5) * sy - 0.5).clamp(0.0, (ih - 1) as f32);
        let (y0, ty) = (fy as u32, fy.fract());
        let y1 = (y0 + 1).min(ih - 1);
        for x in 0..fw {
            let fx = ((x as f32 + 0.5) * sx - 0.5).clamp(0.0, (iw - 1) as f32);
            let (x0, tx) = (fx as u32, fx.fract());
            let x1 = (x0 + 1).min(iw - 1);
            let (a, b, c, d) = (at(x0, y0), at(x1, y0), at(x0, y1), at(x1, y1));
            let ch = |k: usize| {
                let top = a[k] as f32 * (1.0 - tx) + b[k] as f32 * tx;
                let bot = c[k] as f32 * (1.0 - tx) + d[k] as f32 * tx;
                (top * (1.0 - ty) + bot * ty + 0.5) as u32
            };
            out[(y * fw + x) as usize] = (ch(0) << 16) | (ch(1) << 8) | ch(2);
        }
    }
    out
}

/// 贴图的源：缩好的图，或视频的一帧（RGBA，每帧都不一样，不进缓存）
pub(super) enum Src<'a> {
    Px(&'a [u32], u32, u32),
    Rgba(&'a [u8], u32, u32),
}

impl Src<'_> {
    fn size(&self) -> (u32, u32) {
        match self {
            Src::Px(_, w, h) | Src::Rgba(_, w, h) => (*w, *h),
        }
    }
    #[inline]
    fn get(&self, x: u32, y: u32) -> u32 {
        match self {
            Src::Px(p, w, _) => p[(y * w + x) as usize],
            Src::Rgba(d, w, _) => {
                let i = ((y * w + x) * 4) as usize;
                ((d[i] as u32) << 16) | ((d[i + 1] as u32) << 8) | d[i + 2] as u32
            }
        }
    }
}

/// 两个 0x00RRGGBB 按 `k`（0..=256，256 = 全是 `s`）混
#[inline]
pub(super) fn mix(d: u32, s: u32, k: u32) -> u32 {
    let inv = 256 - k;
    let rb = (((d & 0xFF00FF) * inv + (s & 0xFF00FF) * k) >> 8) & 0xFF00FF;
    let g = (((d & 0x00FF00) * inv + (s & 0x00FF00) * k) >> 8) & 0x00FF00;
    rb | g
}

/// 整屏压暗到 `keep`（0..=256，0 = 全黑）
pub(super) fn darken(buf: &mut [u32], keep: u32) {
    if keep == 0 {
        buf.fill(0);
        return;
    }
    for p in buf.iter_mut() {
        *p = mix(0, *p, keep);
    }
}

/// 把 `src` 贴进 `(x, y, dw, dh)`（设备像素，浮点），不透明度 `k`（0..=256）
pub(super) fn blit(buf: &mut [u32], w: u32, h: u32, src: &Src, (x, y, dw, dh): (f32, f32, f32, f32), k: u32) {
    let (sw, sh) = src.size();
    if sw == 0 || sh == 0 || dw < 1.0 || dh < 1.0 || k == 0 {
        return;
    }
    let (x0, y0) = (x.round() as i32, y.round() as i32);
    let (x1, y1) = ((x + dw).round() as i32, (y + dh).round() as i32);
    let (cx0, cy0, cx1, cy1) = (x0.max(0), y0.max(0), x1.min(w as i32), y1.min(h as i32));
    if cx1 <= cx0 || cy1 <= cy0 {
        return;
    }
    let (tw, th) = ((x1 - x0) as u32, (y1 - y0) as u32);
    let row = w as usize;
    // 1:1、不透明的缩好的图：整行拷贝（左右滑动的帧都走这里）
    if let (Src::Px(px, ..), true, 256) = (src, tw == sw && th == sh, k) {
        let (a, b) = ((cx0 - x0) as usize, (cx1 - x0) as usize);
        for ty in cy0..cy1 {
            let sy = (ty - y0) as usize;
            let s = &px[sy * sw as usize + a..sy * sw as usize + b];
            buf[ty as usize * row + cx0 as usize..ty as usize * row + cx1 as usize].copy_from_slice(s);
        }
        return;
    }
    let cols: Vec<u32> = (cx0..cx1).map(|tx| (((tx - x0) as u64 * sw as u64) / tw as u64).min(sw as u64 - 1) as u32).collect();
    for ty in cy0..cy1 {
        let sy = ((((ty - y0) as u64) * sh as u64) / th as u64).min(sh as u64 - 1) as u32;
        let line = &mut buf[ty as usize * row + cx0 as usize..ty as usize * row + cx1 as usize];
        // 开合 / 下拉那几帧每帧两百多万像素：按源的种类与是否半透明各走一条不分支的内循环
        match (src, k >= 256) {
            (Src::Px(px, ..), true) => {
                let srow = &px[(sy * sw) as usize..((sy + 1) * sw) as usize];
                for (d, &sx) in line.iter_mut().zip(&cols) {
                    *d = srow[sx as usize];
                }
            }
            (Src::Px(px, ..), false) => {
                let srow = &px[(sy * sw) as usize..((sy + 1) * sw) as usize];
                for (d, &sx) in line.iter_mut().zip(&cols) {
                    *d = mix(*d, srow[sx as usize], k);
                }
            }
            _ => {
                for (d, &sx) in line.iter_mut().zip(&cols) {
                    let s = src.get(sx, sy);
                    *d = if k >= 256 { s } else { mix(*d, s, k) };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 缩小是盒式平均_放大是双线性() {
        // 2×1：红、蓝 → 缩成 1×1 是两者平均
        let data = [255, 0, 0, 255, 0, 0, 255, 255];
        assert_eq!(resample(&data, 2, 1, 1, 1), vec![(127 << 16) | 127]);
        // 1×1 放大成 2×2：每个像素都是原色
        assert_eq!(resample(&[10, 20, 30, 255], 1, 1, 2, 2), vec![(10 << 16) | (20 << 8) | 30; 4]);
    }

    #[test]
    fn 整行拷贝与逐像素反查结果一样() {
        let px: Vec<u32> = (0..12).collect();
        let (mut a, mut b) = (vec![0u32; 50], vec![0u32; 50]);
        blit(&mut a, 10, 5, &Src::Px(&px, 4, 3), (3.0, 1.0, 4.0, 3.0), 256);
        // 同一张图以 RGBA 给出，走逐像素那条路
        let rgba: Vec<u8> = px.iter().flat_map(|v| [((v >> 16) & 255) as u8, ((v >> 8) & 255) as u8, (v & 255) as u8, 255]).collect();
        blit(&mut b, 10, 5, &Src::Rgba(&rgba, 4, 3), (3.0, 1.0, 4.0, 3.0), 256);
        assert_eq!(a, b);
        assert_eq!(a[13], 0);
        assert_eq!(a[3 * 10 + 6], 11);
    }

    #[test]
    fn 混色两端() {
        assert_eq!(mix(0x102030, 0xFFFFFF, 0), 0x102030);
        assert_eq!(mix(0x102030, 0xA0B0C0, 256), 0xA0B0C0);
    }
}
