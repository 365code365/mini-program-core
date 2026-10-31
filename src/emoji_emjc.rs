//! iOS 的 Apple Color Emoji（`AppleColorEmoji-160px.ttc`）的 `sbix` 字形不是 PNG，而是苹果私有的 `emjc`：
//! 16 字节头（"emj1"、版本、宽、高、附加表长度，小端）+ lzfse 压缩的数据。解压后依次是
//! alpha[w×h]、每行一个滤波类型[h]、每像素三个操作数[w×h×3]、附加表[n]。按行滤波还原出三个有符号整数，
//! 再按一个可逆的线性变换换成 RGB（对 257 取模）。格式与算法照 cc4966/emjc-decoder（MIT 之前的公开分析，
//! https://qiita.com/496_/items/80cfc03ad2ed26c2ab52）重写，逐行对应 `emjc_decode_buffer`。
//!
//! 为什么要有：iOS 上不认 emjc，emoji 全画成豆腐块（farm001 商店里「🌱 种子」「💰 每季利润约」前面的方框）。
//! macOS 的 Apple Color Emoji 仍是 PNG，不走这里。
//!
//! lzfse 用系统的 libcompression（`compression_decode_buffer`，iOS 9+ / macOS 10.11+ 都有），不引第三方库；
//! 别的平台没有这种字体，`decode` 直接返回 None。

/// 解码一个 emjc 字形：返回 (宽, 高, 非预乘 RGBA)
pub fn decode(src: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    if src.len() < 16 || &src[0..4] != b"emj1" {
        return None;
    }
    let le = |i: usize| u16::from_le_bytes([src[i], src[i + 1]]) as usize;
    let (w, h, appendix) = (le(8), le(10), le(12));
    if w == 0 || h == 0 || w > 1024 || h > 1024 {
        return None;
    }
    let pixels = w * h;
    let need = pixels + h + pixels * 3 + appendix;
    let raw = lzfse(&src[16..], need)?;
    if raw.len() != need {
        return None;
    }
    Some((w as u32, h as u32, reconstruct(&raw, w, h, appendix)))
}

/// 解压后的数据 → RGBA（与 emjc_decode_buffer 同一个算法；它输出 BGRA，这里直接排成 RGBA）
fn reconstruct(raw: &[u8], w: usize, h: usize, appendix_len: usize) -> Vec<u8> {
    let pixels = w * h;
    let alpha = &raw[..pixels];
    let filters = &raw[pixels..pixels + h];
    let ops = &raw[pixels + h..pixels + h + pixels * 3];
    let appendix = &raw[pixels + h + pixels * 3..pixels + h + pixels * 3 + appendix_len];
    let colors = pixels * 3;
    let mut buf = vec![0i32; colors];
    // 附加表：高 6 位是跳过几个操作数，低 2 位 × 128 是那个操作数绝对值的增量
    let mut off = 0usize;
    for &a in appendix {
        off += (a / 4) as usize;
        if off >= colors {
            break;
        }
        buf[off] = 128 * (a % 4) as i32;
        off += 1;
    }
    // 操作数：最低位是符号，其余是绝对值（再加上附加表给的增量）
    let diff = |v: u8, extra: i32| if v & 1 == 1 { -((v >> 1) as i32) - extra } else { (v >> 1) as i32 + extra };
    // 滤波 4 的「左与上的平均」：向零取整的 (l + u + 1) / 2
    let avg = |l: i32, u: i32| { let v = l + u + 1; if v < 0 { -((-v) / 2) } else { v / 2 } };
    let mut out = vec![0u8; pixels * 4];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            for k in 0..3 {
                buf[i * 3 + k] = diff(ops[i * 3 + k], buf[i * 3 + k]);
            }
            let left = |b: &[i32], k: usize| b[(i - 1) * 3 + k];
            let up = |b: &[i32], k: usize| b[(i - w) * 3 + k];
            // 滤波：0 原值；1 取左或取上（变化大的那边，只看第一个分量）；2 取左；3 取上；4 左与上的平均。
            // 1 / 4 在第一行只能取左、在第一列只能取上
            let f = filters[y];
            let add: [i32; 3] = if f == 1 && x > 0 && y > 0 {
                let (l, u, lu) = (left(&buf, 0), up(&buf, 0), buf[(i - w - 1) * 3]);
                if (l - lu).abs() < (u - lu).abs() { [0, 1, 2].map(|k| up(&buf, k)) } else { [0, 1, 2].map(|k| left(&buf, k)) }
            } else if f == 4 && x > 0 && y > 0 {
                [0, 1, 2].map(|k| avg(left(&buf, k), up(&buf, k)))
            } else if matches!(f, 1 | 2 | 4) && x > 0 {
                [0, 1, 2].map(|k| left(&buf, k))
            } else if matches!(f, 1 | 3 | 4) && y > 0 {
                [0, 1, 2].map(|k| up(&buf, k))
            } else {
                [0; 3]
            };
            for k in 0..3 {
                buf[i * 3 + k] += add[k];
            }
            let (base, p, q) = (buf[i * 3], buf[i * 3 + 1], buf[i * 3 + 2]);
            // 半数的取整按符号分四种（C 的整数除法向零取整，Rust 的 / 也是）
            let (r, g, b) = match (p < 0, q < 0) {
                (true, true) => (base + p / 2 - (q + 1) / 2, base + q / 2, base - (p + 1) / 2 - (q + 1) / 2),
                (true, false) => (base + p / 2 - q / 2, base + (q + 1) / 2, base - (p + 1) / 2 - q / 2),
                (false, true) => (base + (p + 1) / 2 - (q + 1) / 2, base + q / 2, base - p / 2 - (q + 1) / 2),
                (false, false) => (base + (p + 1) / 2 - q / 2, base + (q + 1) / 2, base - p / 2 - q / 2),
            };
            let m = |v: i32| v.rem_euclid(257) as u8;
            out[i * 4..i * 4 + 4].copy_from_slice(&[m(r), m(g), m(b), alpha[i]]);
        }
    }
    out
}

#[cfg(target_vendor = "apple")]
fn lzfse(src: &[u8], expect: usize) -> Option<Vec<u8>> {
    #[link(name = "compression")]
    extern "C" {
        fn compression_decode_buffer(dst: *mut u8, dst_size: usize, src: *const u8, src_size: usize, scratch: *mut std::ffi::c_void, algorithm: i32) -> usize;
    }
    const COMPRESSION_LZFSE: i32 = 0x801;
    // 多给 1 字节：解出来正好填满时分不清「刚好」还是「被截断」
    let mut dst = vec![0u8; expect + 1];
    // SAFETY: 两块缓冲的指针与长度都来自切片；scratch 给空由库自己分配
    let n = unsafe { compression_decode_buffer(dst.as_mut_ptr(), dst.len(), src.as_ptr(), src.len(), std::ptr::null_mut(), COMPRESSION_LZFSE) };
    dst.truncate(n);
    Some(dst)
}

#[cfg(not(target_vendor = "apple"))]
fn lzfse(_src: &[u8], _expect: usize) -> Option<Vec<u8>> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 不经 lzfse 直接喂解压后的数据：滤波 0（原值）/ 2（取左）/ 3（取上）与颜色变换
    #[test]
    fn 滤波与颜色变换() {
        let (w, h) = (2usize, 2usize);
        // 操作数编码：偶数 = +v/2，奇数 = -(v>>1)
        let enc = |v: i32| if v < 0 { ((-v) as u8) << 1 | 1 } else { (v as u8) << 1 };
        let mut raw = vec![255u8, 128, 64, 0]; // alpha
        raw.extend_from_slice(&[0, 3]); // 第 0 行原值，第 1 行取上
        // 像素 (0,0)：base 100、p 0、q 0 → 灰 100；(1,0)：base 50，p 20，q -10
        raw.extend_from_slice(&[enc(100), enc(0), enc(0), enc(50), enc(20), enc(-10)]);
        // 第 1 行全 0 增量：等于上一行
        raw.extend_from_slice(&[0; 6]);
        let rgba = reconstruct(&raw, w, h, 0);
        assert_eq!(&rgba[0..4], &[100, 100, 100, 255]);
        // p=20 q=-10：r = 50 + 10 - (-9/2 = -4) = 64，g = 50 + (-5) = 45，b = 50 - 10 - (-4) = 44
        assert_eq!(&rgba[4..8], &[64, 45, 44, 128]);
        assert_eq!(&rgba[8..11], &rgba[0..3], "取上");
        assert_eq!(rgba[11], 64);
        assert_eq!(&rgba[12..15], &rgba[4..7]);
    }

    #[test]
    fn 头不对返回空() {
        assert!(decode(b"png ............").is_none());
        assert!(decode(b"emj1").is_none());
    }
}
