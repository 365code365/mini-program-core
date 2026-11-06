//! 整屏级的逐行合成分给几个线程做：成组透明度贴回（`blend_pixels_opacity`）、竖直渐变铺满屏（`fill_rows`）。
//!
//! 为什么：播放页整页重画时，背景图的成组透明度（0.55）要把 300 万个像素逐个混回画布，上面那层
//! 竖直渐变遮罩又是 350 万个像素逐个混合。桌面 sample（单线程）里这两样加起来占整帧绘制的一半多，
//! 一加 PLY110 上 15~20ms。每一行只读源、只写目标的这一行，与单线程逐行做逐字节相同；
//! 帮手线程是变换贴回那几个常驻的（`renderer::blit_pool`），按「谁空了谁领下一条行带」分。
use super::*;

/// 一条行带多少行
const BAND_ROWS: usize = 32;
/// 少于这么多像素就不分线程（起帮手、领带子的开销比省下的多）
const PARALLEL_MIN_PIXELS: usize = 300_000;

/// 把画布 `[y0, y1)` 这几行切成行带，交给几个线程各自 `f(带子, 带子第一行的 y)`。
/// `pixels` 是整块画布的像素、`width` 是画布宽；`area` 是这次真要写的像素数（决定分不分线程）
pub(super) fn for_bands(pixels: &mut [Color], width: usize, (y0, y1): (usize, usize), area: usize, f: &(dyn Fn(&mut [Color], usize) + Sync)) {
    if y1 <= y0 {
        return;
    }
    let rows = &mut pixels[y0 * width..y1 * width];
    let threads = if area >= PARALLEL_MIN_PIXELS { crate::renderer::compose::blit_threads(area) } else { 1 };
    if threads <= 1 {
        f(rows, y0);
        return;
    }
    let bands = std::sync::Mutex::new(rows.chunks_mut(BAND_ROWS * width).enumerate());
    let work = || loop {
        let next = bands.lock().map(|mut it| it.next()).unwrap_or(None);
        let Some((i, band)) = next else { break };
        f(band, y0 + i * BAND_ROWS);
    };
    crate::renderer::blit_pool::run(threads, &work);
}

impl Canvas {
    /// 每一行一个颜色的整行合成（竖直渐变的无圆角那几行）：与逐行 [`Canvas::fill_span`]`(x0, x1, y, c)` 逐字节相同
    pub(crate) fn fill_rows(&mut self, x0: i32, x1: i32, rows: &[(i32, Color)]) {
        let (Some(&(first, _)), Some(&(last, _))) = (rows.first(), rows.last()) else { return };
        // 只接「连续、递增」的行（渐变那边就是这样给的）；别的形状逐行做
        let contiguous = rows.windows(2).all(|w| w[1].0 == w[0].0 + 1);
        let (bx0, bx1, by0, by1) = self.draw_bounds();
        let (start, end) = (bx0.max(x0), bx1.min(x1));
        let (ya, yb) = (first.max(by0), (last + 1).min(by1));
        if !contiguous || end <= start || yb <= ya {
            for &(y, c) in rows {
                self.fill_span(x0, x1, y, c);
            }
            return;
        }
        let width = self.width as usize;
        let (s, e) = (start as usize, end as usize);
        let area = (e - s) * (yb - ya) as usize;
        let run = |band: &mut [Color], y_first: usize| {
            for (k, line) in band.chunks_mut(width).enumerate() {
                let color = rows[y_first + k - first as usize].1;
                fill_line(&mut line[s..e], color);
            }
        };
        for_bands(&mut self.pixels, width, (ya as usize, yb as usize), area, &run);
    }
}

/// 一段像素铺一个颜色：与 [`Canvas::fill_span`] 裁剪之后的那段完全相同（同色底上复用上一个像素的结果）
#[inline]
pub(super) fn fill_line(span: &mut [Color], color: Color) {
    if color.a == 0 || span.is_empty() {
        return;
    }
    if color.a == 255 {
        span.fill(color);
        return;
    }
    let (mut last_dst, mut last_out) = (span[0], color.blend(&span[0]));
    for px in span.iter_mut() {
        if *px != last_dst {
            last_dst = *px;
            last_out = color.blend(px);
        }
        *px = last_out;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 分线程的整行合成与逐行单线程逐字节相同（竖直渐变 + 成组透明度；底下是花的，带裁剪区）
    #[test]
    fn 分线程整行合成与逐行做逐字节相同() {
        let (w, h) = (1270u32, 900u32);
        let base = |c: &mut Canvas| {
            for (i, p) in c.pixels_mut().iter_mut().enumerate() {
                *p = Color::new((i * 7 % 251) as u8, (i * 13 % 241) as u8, (i % 239) as u8, if i % 5 == 0 { 120 } else { 255 });
            }
        };
        let rows: Vec<(i32, Color)> = (10..880).map(|y| (y, Color::new(20, 18, 30, (140 + y % 100) as u8))).collect();
        let src: Vec<Color> = (0..(w * 800) as usize).map(|i| Color::new((i % 256) as u8, (i / 7 % 256) as u8, 90, if i % 9 == 0 { 0 } else { 255 })).collect();
        for clip in [None, Some(crate::Rect::new(100.0, 200.0, 900.0, 300.0))] {
            let (mut a, mut b) = (Canvas::new(w, h), Canvas::new(w, h));
            base(&mut a);
            base(&mut b);
            if let Some(r) = clip {
                a.save();
                a.clip_rect(r);
                b.save();
                b.clip_rect(r);
            }
            a.fill_rows(3, 1260, &rows);
            for &(y, c) in &rows {
                b.fill_span(3, 1260, y, c);
            }
            assert!(a.pixels() == b.pixels(), "fill_rows（裁剪 {clip:?}）与逐行 fill_span 不一致");
            a.blend_pixels_opacity(0, 50, &src, w as usize, 0.55);
            b.blend_pixels_opacity_serial(0, 50, &src, w as usize, 0.55);
            assert!(a.pixels() == b.pixels(), "分线程的成组透明度（裁剪 {clip:?}）与单线程不一致");
        }
    }
}
