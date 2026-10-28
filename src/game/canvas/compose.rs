//! 主画布 → 输出像素（game-doc/02 §5.4）。
//!
//! 移动端**不走这里缩放**：主画布按自己的分辨率交给 surface（Android `SurfaceHolder.setFixedSize` /
//! `ANativeWindow_setBuffersGeometry`，iOS `CAMetalLayer.drawableSize`），由系统合成器的硬件缩放器拉到全屏 ——
//! 1272×2772 的屏上每帧在 CPU 上做一遍双线性要 10ms 以上，硬件缩放是零成本。
//! 这里的软件缩放只给无头快照、桌面窗体与测试用，算法写死成整数定点，结果逐字节可复现。
//!
//! 透明：主画布清成透明时显示成黑底（微信真机的底色以取证为准，game-doc/06 §7），即 `rgb × a / 255`。

use crate::Color;

/// 主画布 1:1 转成不透明 RGBA（压到黑底上）
pub fn to_rgba_opaque(src: &[Color], out: &mut Vec<u8>) {
    out.clear();
    out.reserve(src.len() * 4);
    for c in src {
        let a = c.a as u32;
        // 整数：(v * a + 127) / 255，与浮点四舍五入一致
        let m = |v: u8| ((v as u32 * a + 127) / 255) as u8;
        out.extend_from_slice(&[m(c.r), m(c.g), m(c.b), 255]);
    }
}

/// 主画布直接写进宿主缓冲（`stride` 字节一行，RGBA8），压黑底。上屏热路径：不分配、不经中间缓冲。
///
/// `Color` 是 `#[repr(C)]` 的 RGBA 四字节（canvas/mod.rs 有断言守着），与 RGBA8 缓冲同一布局：整行 memcpy，
/// 再只修 a != 255 的那几个像素。游戏每帧整屏清成不透明底色是常态，这样一行只剩一次拷贝 + 一遍扫 alpha
pub fn write_opaque_rows(src: &[Color], w: u32, h: u32, dst: &mut [u8], stride: usize) {
    let row = w as usize * 4;
    if stride < row || dst.len() < stride * h.saturating_sub(1) as usize + row || src.len() < (w * h) as usize {
        return;
    }
    // SAFETY: Color 是 repr(C) 的 4 个 u8，没有填充，可以按字节看
    let bytes = unsafe { std::slice::from_raw_parts(src.as_ptr() as *const u8, src.len() * 4) };
    for y in 0..h as usize {
        let s = &bytes[y * row..(y + 1) * row];
        let d = &mut dst[y * stride..y * stride + row];
        // 只读源（可缓存的内存）、只写目标：目标是 gralloc 缓冲，常是不可缓存 / 写合并的，**读回它**极慢
        // （一加 PLY110 上先拷贝再回读 alpha，整帧 2ms 多）。所以先在源上判这一行是否全不透明：
        // 是 → 一次整行拷贝；否 → 逐像素算好再写
        if row_opaque(s) {
            d.copy_from_slice(s);
        } else {
            for (o, i) in d.chunks_exact_mut(4).zip(s.chunks_exact(4)) {
                let a = i[3] as u32;
                if a == 255 {
                    o.copy_from_slice(i);
                } else {
                    let m = |v: u8| ((v as u32 * a + 127) / 255) as u8;
                    o.copy_from_slice(&[m(i[0]), m(i[1]), m(i[2]), 255]);
                }
            }
        }
    }
}

/// 一行 RGBA 是否全不透明：两个像素一组按 u64 看 alpha（小端：第 3、7 字节）
#[inline]
fn row_opaque(s: &[u8]) -> bool {
    const ALPHA: u64 = 0xFF00_0000_FF00_0000;
    let mut pairs = s.chunks_exact(8);
    for q in pairs.by_ref() {
        if u64::from_le_bytes([q[0], q[1], q[2], q[3], q[4], q[5], q[6], q[7]]) & ALPHA != ALPHA {
            return false;
        }
    }
    pairs.remainder().chunks_exact(4).all(|p| p[3] == 255)
}

/// 双线性缩放到 `(dw, dh)`，压黑底，输出 RGBA。16.16 定点坐标、8 位权重，像素中心对齐（与浏览器 / GPU 采样同一约定）
pub fn scale_rgba_opaque(src: &[Color], sw: u32, sh: u32, dw: u32, dh: u32, out: &mut Vec<u8>) {
    out.clear();
    out.resize((dw as usize) * (dh as usize) * 4, 0);
    if sw == 0 || sh == 0 || dw == 0 || dh == 0 || src.len() < (sw * sh) as usize {
        return;
    }
    if (sw, sh) == (dw, dh) {
        let mut v = Vec::new();
        to_rgba_opaque(&src[..(sw * sh) as usize], &mut v);
        *out = v;
        return;
    }
    // 预乘黑底后的颜色：先压底再插值，等价于插值不透明图（边缘不会出现半透明发白）
    let px = |x: i64, y: i64| -> [u32; 3] {
        let x = x.clamp(0, sw as i64 - 1) as usize;
        let y = y.clamp(0, sh as i64 - 1) as usize;
        let c = src[y * sw as usize + x];
        let a = c.a as u32;
        [c.r as u32 * a / 255, c.g as u32 * a / 255, c.b as u32 * a / 255]
    };
    for oy in 0..dh as i64 {
        // 源坐标 = (o + 0.5) * s/d - 0.5，16.16
        let fy = ((oy << 16) + (1 << 15)) * sh as i64 / dh as i64 - (1 << 15);
        let (y0, wy) = (fy >> 16, ((fy & 0xffff) >> 8) as u32);
        for ox in 0..dw as i64 {
            let fx = ((ox << 16) + (1 << 15)) * sw as i64 / dw as i64 - (1 << 15);
            let (x0, wx) = (fx >> 16, ((fx & 0xffff) >> 8) as u32);
            let (p00, p10, p01, p11) = (px(x0, y0), px(x0 + 1, y0), px(x0, y0 + 1), px(x0 + 1, y0 + 1));
            let i = ((oy as usize) * dw as usize + ox as usize) * 4;
            for k in 0..3 {
                let top = p00[k] * (256 - wx) + p10[k] * wx;
                let bot = p01[k] * (256 - wx) + p11[k] * wx;
                out[i + k] = ((top * (256 - wy) + bot * wy + (1 << 15)) >> 16) as u8;
            }
            out[i + 3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, c: Color) -> Vec<Color> {
        vec![c; (w * h) as usize]
    }

    #[test]
    fn 透明压黑底_半透明按比例() {
        let mut o = Vec::new();
        to_rgba_opaque(&[Color::new(255, 0, 0, 0), Color::new(255, 255, 255, 128)], &mut o);
        assert_eq!(o, vec![0, 0, 0, 255, 128, 128, 128, 255]);
    }

    #[test]
    fn 直写与中间缓冲逐字节相同_带行跨度() {
        let src: Vec<Color> = (0..12u8).map(|i| Color::new(i * 20, 255 - i * 20, i, if i % 3 == 0 { 255 } else { i * 21 })).collect();
        let mut a = Vec::new();
        to_rgba_opaque(&src, &mut a);
        let (w, h, stride) = (4u32, 3u32, 20usize);
        let mut d = vec![7u8; stride * 3];
        write_opaque_rows(&src, w, h, &mut d, stride);
        for y in 0..3 {
            assert_eq!(&d[y * stride..y * stride + 16], &a[y * 16..y * 16 + 16]);
            assert_eq!(&d[y * stride + 16..y * stride + 20], &[7, 7, 7, 7], "行尾填充不能被写");
        }
    }

    #[test]
    fn 纯色缩放保持纯色_尺寸相同走拷贝() {
        let src = solid(10, 10, Color::new(10, 200, 30, 255));
        let mut o = Vec::new();
        scale_rgba_opaque(&src, 10, 10, 27, 13, &mut o);
        assert!(o.chunks(4).all(|p| p == [10, 200, 30, 255]));
        scale_rgba_opaque(&src, 10, 10, 10, 10, &mut o);
        assert_eq!(o.len(), 400);
    }

    #[test]
    fn 二倍放大_左右两半中间过渡() {
        // 2×1：左黑右白 → 4×1：中心对齐的双线性：0, 64, 191, 255
        let src = vec![Color::new(0, 0, 0, 255), Color::new(255, 255, 255, 255)];
        let mut o = Vec::new();
        scale_rgba_opaque(&src, 2, 1, 4, 1, &mut o);
        let r: Vec<u8> = o.chunks(4).map(|p| p[0]).collect();
        assert_eq!(r, vec![0, 64, 191, 255]);
    }
}
