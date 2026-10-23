//! 覆盖层这一帧写了哪些像素：GPU 据此只传覆盖层那几行，其余行在着色器里算。
//!
//! Toast / Loading / Modal、操作菜单、picker 面板都是直接画进输出缓冲的（混合时读目标像素、
//! 带浮点），GPU 复刻不了逐字节，于是这些帧走「直通」：CPU 合成整帧、整块上传。
//! 一加 PLY110（Adreno 810）上 1080×2354 整帧上传 + 合成一帧 9~20ms，比 CPU 后端还慢。
//!
//! 但这些覆盖层的写法其实很规整：
//! - **遮罩**是对整个缓冲逐分量查表（`c → lut[c]`），与位置无关；
//! - 其余笔画（面板、文字、按钮）都落在一段**行区间**里。
//!
//! 所以行区间外的像素 = 查表(不带覆盖层的那一帧)，着色器自己算得出来（基础帧本来就逐字节一致，
//! 表由 CPU 算好传进去，着色器不做浮点）；只有行区间里的那几行要 CPU 合成、上传。
//! 操作菜单弹着时那一段是屏幕下方 1/3，入场途中更少。
//!
//! 记录是**画的时候顺手记的**（每个覆盖层的 `render` 返回自己的足迹），不是事后按几何推算：
//! 面板入场 / 退场、遮罩淡入都按墙上时钟走，事后再算一次位置已经变了。
//! 记不清的（图片预览、全屏视频、侧滑叠覆盖层）一律 [`Footprint::full`]：误判成整帧永远是对的。

/// 一帧里覆盖层的足迹（物理像素行）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Footprint {
    /// 说不清写了哪里：整帧直通
    pub full: bool,
    /// 覆盖层笔画所在的行区间 [y0, y1)；`None` 表示一笔也没画
    pub rows: Option<(u32, u32)>,
    /// 遮罩：行区间外每个分量 `c → lut[c]`；`None` 表示没有遮罩（几层遮罩已经复合成一张表）
    pub lut: Option<[u8; 256]>,
}

impl Default for Footprint {
    fn default() -> Self {
        Self::NONE
    }
}

impl Footprint {
    /// 什么也没画
    pub const NONE: Self = Self { full: false, rows: None, lut: None };

    pub fn full() -> Self {
        Self { full: true, ..Self::NONE }
    }

    /// 什么也没写（与不带覆盖层的那一帧逐字节相同）
    pub fn is_none(&self) -> bool {
        !self.full && self.rows.is_none() && self.lut.is_none()
    }

    /// 记一段笔画：物理像素的浮点行区间 [y0, y1)，向外各多取一行（抗锯齿边、取整方向都不用操心），
    /// 夹到 [0, h)
    pub fn touch(&mut self, y0: f32, y1: f32, h: u32) {
        let a = (y0.floor() as i64 - 1).clamp(0, h as i64) as u32;
        let b = (y1.ceil() as i64 + 1).clamp(0, h as i64) as u32;
        if b <= a {
            return;
        }
        self.rows = Some(match self.rows {
            Some((p, q)) => (p.min(a), q.max(b)),
            None => (a, b),
        });
    }

    /// 记一次整屏遮罩（之后的遮罩压在之前的结果上：表复合）
    pub fn dim(&mut self, lut: &[u8; 256]) {
        self.lut = Some(match self.lut {
            Some(prev) => std::array::from_fn(|c| lut[prev[c] as usize]),
            None => *lut,
        });
    }

    /// 后画的一层叠上来
    pub fn then(&mut self, next: Footprint) {
        if next.full {
            self.full = true;
        }
        if let Some(l) = next.lut {
            self.dim(&l);
        }
        if let Some((a, b)) = next.rows {
            self.rows = Some(match self.rows {
                Some((p, q)) => (p.min(a), q.max(b)),
                None => (a, b),
            });
        }
    }

    /// 按表压暗缓冲（行宽 `w`）的 `rows` 那几行（整屏遮罩传 (0, 高)；GPU 直通帧只合成覆盖层那一截，
    /// 见 `cpu::compose_rows`）。`high` 是写进高字节的值：picker 一向写 0xFF00_0000，其余写 0
    ///
    /// 表能写成 `(c · m + bias) >> 16`（[`LinearLut::fit`]；遮罩动画里的表实测全都能）就按乘法算：逐分量乘法
    /// 编译器能整段向量化，查表不能（每个分量一次随机读，只能一个一个来）。结果与查表逐字节相同（fit 逐项核对过）。
    /// M3 上 1078×2343 整屏：查表 1.7ms、乘法 0.64ms（同尺寸 memcpy 0.23ms）。一加 PLY110 的 CPU 后端上，
    /// 操作菜单弹出那 0.3s 每帧都整屏压一遍，查表那一版一帧合成 10~21ms，90Hz 下掉成 30~45 帧
    pub fn apply_lut(buf: &mut [u32], w: u32, rows: (u32, u32), lut: &[u8; 256], high: u32) {
        let a = (rows.0 as usize * w as usize).min(buf.len());
        let b = (rows.1 as usize * w as usize).clamp(a, buf.len());
        let px = &mut buf[a..b];
        if let Some(k) = LinearLut::fit(lut) {
            k.apply(px, high);
            return;
        }
        for p in px.iter_mut() {
            *p = by_table(*p, lut, high);
        }
    }

    /// [`apply_lut`](Self::apply_lut)，但从 `src` 读、写进 `dst`（两块同样大小的缓冲，只动 `rows` 那几行）：
    /// 操作菜单弹着时底图留着一份（`compositor::sheet_base`），每帧从它直接压暗到输出缓冲 —— 拼底图 + 原地压暗
    /// 两遍变成边读边压一遍
    pub fn apply_lut_from(src: &[u32], dst: &mut [u32], w: u32, rows: (u32, u32), lut: &[u8; 256], high: u32) {
        let n = src.len().min(dst.len());
        let a = (rows.0 as usize * w as usize).min(n);
        let b = (rows.1 as usize * w as usize).clamp(a, n);
        let (src, dst) = (&src[a..b], &mut dst[a..b]);
        if let Some(k) = LinearLut::fit(lut) {
            k.apply_from(src, dst, high);
            return;
        }
        for (d, s) in dst.iter_mut().zip(src) {
            *d = by_table(*s, lut, high);
        }
    }
}

#[inline(always)]
fn by_table(p: u32, lut: &[u8; 256], high: u32) -> u32 {
    let (r, g, b) = ((p >> 16) & 0xFF, (p >> 8) & 0xFF, p & 0xFF);
    high | ((lut[r as usize] as u32) << 16) | ((lut[g as usize] as u32) << 8) | lut[b as usize] as u32
}

/// 写成乘法的压暗表：`lut[c] == (c · m + bias) >> 16`（`m ≤ 65536`、`bias < 65536`，算式在 u32 里不溢出）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinearLut {
    m: u32,
    bias: u32,
}

impl LinearLut {
    /// 找一组 `(m, bias)` 让 256 项全部吻合；找不到（不是线性的表）返回 None，调用方照旧查表。
    ///
    /// 每一项 `t = lut[c]` 要求 `t·2¹⁶ ≤ c·m + bias < (t+1)·2¹⁶`。
    /// - 先试两个现成的 bias：截断式 `(c · keep) as u32` 是 0、四舍五入式 `(c · keep + 0.5) as u32` 是 32768，
    ///   这时 `m` 的取值是各项区间的交集，取最小的那个；
    /// - 不行再一般地找：f32 乘法的舍入偶尔把 `c · keep` 推过一个整数（比如 `c` 那一项进到了 `k`、`2c` 那一项
    ///   却没进到 `2k`），任何 bias = 0 的乘法都对不上，要一个小小的 bias 把两边都兜住。固定 `m` 时 bias 的
    ///   可行区间宽度是 `m` 的凹函数（线性函数的 min 减 max），三分查找它的最大值，≥ 0 就有解；
    ///
    /// 最后逐项核对一遍（只信核对，不信推导）
    pub fn fit(lut: &[u8; 256]) -> Option<Self> {
        if lut[0] != 0 {
            return None;
        }
        let verified = |k: Self| (0..256u32).all(|c| k.channel(c) == lut[c as usize] as u32).then_some(k);
        for bias in [0i64, 32768] {
            let (mut lo, mut hi) = (0i64, 65536i64);
            for c in 1..256i64 {
                let t = lut[c as usize] as i64;
                lo = lo.max((t * 65536 - bias + c - 1).div_euclid(c));
                hi = hi.min(((t + 1) * 65536 - 1 - bias).div_euclid(c));
            }
            if lo <= hi {
                if let Some(k) = verified(Self { m: lo as u32, bias: bias as u32 }) {
                    return Some(k);
                }
            }
        }
        // 固定 m：bias ∈ [max(0, maxᵢ(t·2¹⁶ − c·m)), min(65535, minᵢ((t+1)·2¹⁶ − 1 − c·m))]
        let span = |m: i64| {
            let (mut lo, mut hi) = (0i64, 65535i64);
            for c in 1..256i64 {
                let t = lut[c as usize] as i64;
                lo = lo.max(t * 65536 - c * m);
                hi = hi.min((t + 1) * 65536 - 1 - c * m);
            }
            (hi - lo, lo)
        };
        let (mut lo, mut hi) = (0i64, 65536i64);
        while hi - lo > 2 {
            let (m1, m2) = (lo + (hi - lo) / 3, hi - (hi - lo) / 3);
            if span(m1).0 < span(m2).0 {
                lo = m1 + 1;
            } else {
                hi = m2;
            }
        }
        let m = (lo..=hi).max_by_key(|&m| span(m).0)?;
        let (width, bias) = span(m);
        if width < 0 {
            return None;
        }
        verified(Self { m: m as u32, bias: bias as u32 })
    }

    #[inline(always)]
    fn channel(self, c: u32) -> u32 {
        (c * self.m + self.bias) >> 16
    }

    #[inline(always)]
    fn pixel(self, v: u32, high: u32) -> u32 {
        high | (self.channel((v >> 16) & 0xFF) << 16) | (self.channel((v >> 8) & 0xFF) << 8) | self.channel(v & 0xFF)
    }

    /// 每个像素三个分量各按表压暗，高字节写 `high`（与查表那一版同一个结果）
    pub fn apply(self, px: &mut [u32], high: u32) {
        for p in px.iter_mut() {
            *p = self.pixel(*p, high);
        }
    }

    /// 同 [`apply`](Self::apply)，从 `src` 读、写进 `dst`（取两者较短的长度）
    pub fn apply_from(self, src: &[u32], dst: &mut [u32], high: u32) {
        for (d, s) in dst.iter_mut().zip(src) {
            *d = self.pixel(*s, high);
        }
    }
}

/// 截断式压暗表：`(c as f32 * keep) as u32`（操作菜单、picker、侧滑的写法）
pub fn lut_trunc(keep: f32) -> [u8; 256] {
    std::array::from_fn(|c| (c as f32 * keep) as u32 as u8)
}

/// 四舍五入式压暗表：`(c as f32 * keep + 0.5) as u32`（Modal 的写法）
pub fn lut_round(keep: f32) -> [u8; 256] {
    std::array::from_fn(|c| ((c as f32) * keep + 0.5) as u32 as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 行区间外扩一行并夹到画面内() {
        let mut f = Footprint::NONE;
        f.touch(10.4, 20.2, 100);
        assert_eq!(f.rows, Some((9, 22)));
        f.touch(-5.0, 3.0, 100);
        assert_eq!(f.rows, Some((0, 22)));
        f.touch(95.0, 140.0, 100);
        assert_eq!(f.rows, Some((0, 100)));
        let mut g = Footprint::NONE;
        g.touch(120.0, 130.0, 100);
        assert!(g.is_none(), "完全在画面外：不算画了");
    }

    #[test]
    fn 两层遮罩复合成一张表() {
        let (a, b) = (lut_trunc(0.4), lut_round(0.7));
        let mut f = Footprint::NONE;
        f.dim(&a);
        f.dim(&b);
        let l = f.lut.unwrap();
        for c in 0..256 {
            assert_eq!(l[c], b[a[c] as usize]);
        }
    }

    /// 查表那一版（参照，照最早的写法逐分量查）
    fn table_ref(px: &[u32], lut: &[u8; 256], high: u32) -> Vec<u32> {
        px.iter()
            .map(|p| high | (lut[(p >> 16 & 0xFF) as usize] as u32) << 16 | (lut[(p >> 8 & 0xFF) as usize] as u32) << 8 | lut[(p & 0xFF) as usize] as u32)
            .collect()
    }

    /// 乘法版与查表版逐字节相同；遮罩淡入那一路上（keep 从 1 到 0.4）每一张表都写得成乘法
    #[test]
    fn 压暗表写成乘法与查表逐字节相同() {
        let px: Vec<u32> = (0..4096u32).map(|i| i.wrapping_mul(2_654_435_761) ^ (i << 24)).collect();
        let mut missed = Vec::new();
        for i in 0..40_000 {
            // 无理数步长：动画里每帧的 keep 由 vsync 时刻算出来，不落在整齐的小数上
            let keep = 1.0 - 0.6 * ((i as f64 * 0.618_033_988_749_895) % 1.0) as f32;
            for (name, lut) in [("trunc", lut_trunc(keep)), ("round", lut_round(keep))] {
                let Some(k) = LinearLut::fit(&lut) else {
                    missed.push((name, keep));
                    continue;
                };
                if i % 16 == 0 {
                    for high in [0, 0xFF00_0000] {
                        let want = table_ref(&px, &lut, high);
                        let mut fast = px.clone();
                        k.apply(&mut fast, high);
                        assert!(fast == want, "{name} keep {keep}：乘法与查表不一致");
                        let mut copied = vec![0u32; px.len()];
                        k.apply_from(&px, &mut copied, high);
                        assert!(copied == want, "{name} keep {keep}：边读边压与查表不一致");
                    }
                }
            }
        }
        assert!(missed.is_empty(), "这些表没写成乘法（会退回查表，慢）：{missed:?}");
        // 弹着不动时的那几张（操作菜单 0.6、picker 0.5 截断；Modal 0.6 四舍五入）：整段静止期间每帧都压
        for (name, lut) in [("操作菜单", lut_trunc(1.0 - 0.6)), ("picker", lut_trunc(1.0 - 0.5)), ("Modal", lut_round(0.4))] {
            assert!(LinearLut::fit(&lut).is_some(), "{name} 静止时的遮罩表没写成乘法");
        }
    }

    /// 写不成乘法的表必须认出来、退回查表：随便改一项的；还有 f32 舍入把整齐小数推得忽上忽下的
    /// （0.84f32 × 25 进到 21、× 75 却落在 62.999996，再 × 100 又进到 84，任何一条直线都穿不过去）
    #[test]
    fn 写不成乘法的表退回查表() {
        let mut odd = lut_trunc(0.5);
        odd[200] = 7;
        for lut in [odd, lut_trunc(0.84), lut_round(0.65)] {
            assert_eq!(LinearLut::fit(&lut), None);
            let px: Vec<u32> = (0..4096u32).map(|i| i.wrapping_mul(2_654_435_761)).collect();
            let mut buf = px.clone();
            Footprint::apply_lut(&mut buf, 64, (0, 64), &lut, 0);
            assert!(buf == table_ref(&px, &lut, 0));
            // 边读边压：只动给的那几行，其余行保留目标缓冲原值
            let mut dst = vec![0x0012_3456u32; px.len()];
            Footprint::apply_lut_from(&px, &mut dst, 64, (8, 40), &lut, 0);
            let want: Vec<u32> = table_ref(&px, &lut, 0).iter().enumerate().map(|(i, &p)| if (8 * 64..40 * 64).contains(&i) { p } else { 0x0012_3456 }).collect();
            assert!(dst == want);
        }
    }

    #[test]
    fn 表与逐像素算式逐字节相同() {
        for keep in [0.0f32, 0.123, 0.4, 0.5, 0.97, 1.0] {
            let (t, r) = (lut_trunc(keep), lut_round(keep));
            for c in 0..256u32 {
                assert_eq!(t[c as usize] as u32, (c as f32 * keep) as u32);
                assert_eq!(r[c as usize] as u32, ((c as f32) * keep + 0.5) as u32);
            }
        }
    }
}
