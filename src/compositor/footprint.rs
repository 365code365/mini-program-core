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
    pub fn apply_lut(buf: &mut [u32], w: u32, rows: (u32, u32), lut: &[u8; 256], high: u32) {
        let a = (rows.0 as usize * w as usize).min(buf.len());
        let b = (rows.1 as usize * w as usize).clamp(a, buf.len());
        for p in buf[a..b].iter_mut() {
            let (r, g, b) = ((*p >> 16) & 0xFF, (*p >> 8) & 0xFF, *p & 0xFF);
            *p = high | ((lut[r as usize] as u32) << 16) | ((lut[g as usize] as u32) << 8) | lut[b as usize] as u32;
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
