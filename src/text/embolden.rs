//! 没有粗体字面时的「伪粗体」：把字形的覆盖率位图向四周加粗一点（形态学膨胀，带小数半径）。
//!
//! 为什么不再「偏移一点再画一遍」：从前 `font-weight: bold` 在没有粗体字面的字体上把整段字往右挪
//! 0.7 个逻辑像素再画一遍（一加 PLY110 上 2px）。两份抗锯齿边缘按 alpha 叠在一起，竖笔左右两侧
//! 各一圈半透明的重影，横笔不变粗 —— 看着发虚、有锯齿（安卓机的系统中文字体 `SysSans-Hans` 是可变字体，
//! fontdue 读不出它的粗体实例，每个粗体标题都走这条路）。
//!
//! 现在与 Skia 的 fake bold 同一个思路：轮廓向外扩一圈（横竖一样粗），在覆盖率上做：
//! 每个像素取「半径 r 以内邻居覆盖率」的最大值，半径的小数部分按比例算最外一圈。横、竖两遍分开做。
//! 前进宽度不变（度量与断行不变），只是字形本身变粗。
use fontdue::Metrics;

/// 加粗量（总共加宽多少物理像素）：字号的 1/32，夹在 [0.5, 2] 之间（Skia fake bold 大字号那一档）。
/// 对照图（examples/faux_bold_compare.rs）里比过：1/20 时「缴」「醒」这种笔画密的字糊成一团，
/// 1/32 与真实粗体字面（Hiragino W6）的笔画粗细最接近
pub fn strength(size: f32) -> f32 {
    (size / 32.0).clamp(0.5, 2.0)
}

/// 覆盖率位图加粗。返回新的度量（位图变大、原点外移）与位图；前进宽度不变。
///
/// 做法同 FreeType 的 `FT_Bitmap_Embolden`：把位图往右（横向）、往上（竖向）各错开 0~n 像素叠加，
/// **饱和相加**而不是取最大 —— 笔画里面还是满的，抗锯齿边缘两份相加变实，不会多出一圈灰边。
/// 试过「取邻域最大值 + 小数半径按比例」（Skia 轮廓外扩的近似）：边缘外面多一圈半透明，看着发虚。
/// 小数部分：最后一份按比例叠。位图整体左移 / 下移一半，字形仍居中在原来的位置
pub fn embolden(m: &Metrics, bitmap: &[u8], size: f32) -> (Metrics, Vec<u8>) {
    let (w, h) = (m.width, m.height);
    if w == 0 || h == 0 {
        return (*m, bitmap.to_vec());
    }
    let s = strength(size);
    let (n, f) = (s.floor() as usize, s - s.floor());
    let grow = n + usize::from(f > 0.01);
    let (nw, nh) = (w + grow, h + grow);
    // 原图放在新画布的左下角（往右、往上叠）
    let mut src = vec![0u16; nw * nh];
    for y in 0..h {
        for x in 0..w {
            src[(y + grow) * nw + x] = bitmap[y * w + x] as u16;
        }
    }
    let weight = |k: usize| if k <= n { 1.0 } else { f };
    // 横向：第 x 列 = Σ 原图第 x-k 列（k = 0..grow），饱和
    let mut tmp = vec![0u16; nw * nh];
    for y in 0..nh {
        for x in 0..nw {
            let mut v = 0.0f32;
            for k in 0..=grow.min(x) {
                v += src[y * nw + x - k] as f32 * weight(k);
            }
            tmp[y * nw + x] = v.min(255.0) as u16;
        }
    }
    // 竖向：第 y 行 = Σ 第 y+k 行（往上叠），饱和
    let mut out = vec![0u8; nw * nh];
    for y in 0..nh {
        for x in 0..nw {
            let mut v = 0.0f32;
            for k in 0..=grow {
                if y + k < nh {
                    v += tmp[(y + k) * nw + x] as f32 * weight(k);
                }
            }
            out[y * nw + x] = v.min(255.0).round() as u8;
        }
    }
    let mut nm = *m;
    nm.width = nw;
    nm.height = nh;
    // 往右、往上各长了 grow：左移一半保持居中；ymin 是位图底边，往上长不改它，再下移一半
    nm.xmin -= (grow / 2) as i32;
    nm.ymin -= (grow / 2) as i32;
    (nm, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics(w: usize, h: usize) -> Metrics {
        Metrics { xmin: 2, ymin: -1, width: w, height: h, advance_width: 10.0, ..Default::default() }
    }

    #[test]
    fn 竖笔变宽_边缘不发灰_前进宽度不变() {
        // 5×5 里一条 1px 宽的竖线（第 2 列全满）
        let mut b = vec![0u8; 25];
        for y in 0..5 {
            b[y * 5 + 2] = 255;
        }
        let (m, out) = embolden(&metrics(5, 5), &b, 64.0); // 强度 2px
        assert_eq!((m.width, m.height, m.xmin, m.ymin, m.advance_width), (7, 7, 1, -2, 10.0));
        let row = |y: usize| out[y * 7..(y + 1) * 7].to_vec();
        assert_eq!(row(3), vec![0, 0, 255, 255, 255, 0, 0], "竖线该变成 3px 宽、全是实的");
    }

    #[test]
    fn 半透明边缘相加变实() {
        // 一行 [128, 255, 128]：加粗 1px 后中间一段全满，没有新的灰边
        let b = vec![128u8, 255, 128];
        let (m, out) = embolden(&metrics(3, 1), &b, 32.0); // 强度 1px
        assert_eq!((m.width, m.height), (4, 2));
        assert_eq!(&out[4..8], &[128, 255, 255, 128]);
    }
}
