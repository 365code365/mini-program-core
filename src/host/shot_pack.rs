//! 页面截图（侧滑返回 / 返回转场垫在下面的那页，`PageShot`）的无损压缩。
//!
//! 每往下进一层页面就留一张整屏图（1080×2400 ≈ 10MB），栈 5 层就是 40MB 常驻 —— 而只有栈顶下面那一张
//! 会被用到（侧滑 / 返回转场只看最后一张），更下面的要等退回去才用。所以只留最后一张原样，
//! 其余的压起来，退栈时把新的最后一张解开（一次几毫秒，发生在退栈那一帧之前的导航里）。
//!
//! 页面截图大片是同色（背景、卡片底）和整行重复（留白），按行编码：
//! - 与上一行完全相同：一个字节；
//! - 否则按「游程」写：同一像素连续 ≥ 3 个写成 (长度, 像素)，其余原样成段写。
//! 逐像素无损（解出来与压之前逐字节相同，单测守着），实测普通页面压到原来的 5%~20%。
use super::edge_back::PageShot;

const SAME_ROW: u8 = 0;
const ROW: u8 = 1;
const RUN: u8 = 0;
const LIT: u8 = 1;

fn put_len(out: &mut Vec<u8>, mut n: usize) {
    while n >= 0x80 {
        out.push((n as u8) | 0x80);
        n >>= 7;
    }
    out.push(n as u8);
}

fn get_len(b: &[u8], i: &mut usize) -> Option<usize> {
    let mut n = 0usize;
    let mut shift = 0;
    loop {
        let v = *b.get(*i)?;
        *i += 1;
        n |= ((v & 0x7f) as usize) << shift;
        if v & 0x80 == 0 {
            return Some(n);
        }
        shift += 7;
        if shift > 35 {
            return None;
        }
    }
}

/// `w` 宽的像素压成字节串
pub fn pack(px: &[u32], w: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(px.len() / 8);
    if w == 0 {
        return out;
    }
    let mut prev: Option<&[u32]> = None;
    for row in px.chunks_exact(w) {
        if prev == Some(row) {
            out.push(SAME_ROW);
            continue;
        }
        out.push(ROW);
        let mut x = 0;
        let mut lit_start = 0;
        while x < w {
            let v = row[x];
            let mut run = 1;
            while x + run < w && row[x + run] == v {
                run += 1;
            }
            if run >= 3 {
                if lit_start < x {
                    out.push(LIT);
                    put_len(&mut out, x - lit_start);
                    for p in &row[lit_start..x] {
                        out.extend_from_slice(&p.to_le_bytes());
                    }
                }
                out.push(RUN);
                put_len(&mut out, run);
                out.extend_from_slice(&v.to_le_bytes());
                x += run;
                lit_start = x;
            } else {
                x += run;
            }
        }
        if lit_start < w {
            out.push(LIT);
            put_len(&mut out, w - lit_start);
            for p in &row[lit_start..w] {
                out.extend_from_slice(&p.to_le_bytes());
            }
        }
        prev = Some(row);
    }
    out
}

/// [`pack`] 的逆：解出 `w × h` 个像素。字节串坏了返回 None
pub fn unpack(b: &[u8], w: usize, h: usize) -> Option<Vec<u32>> {
    let mut out: Vec<u32> = Vec::with_capacity(w * h);
    let mut i = 0;
    for y in 0..h {
        match *b.get(i)? {
            SAME_ROW => {
                i += 1;
                if y == 0 {
                    return None;
                }
                let start = out.len() - w;
                out.extend_from_within(start..start + w);
            }
            ROW => {
                i += 1;
                let end = out.len() + w;
                while out.len() < end {
                    let tag = *b.get(i)?;
                    i += 1;
                    let n = get_len(b, &mut i)?;
                    if out.len() + n > end {
                        return None;
                    }
                    let word = |j: usize| -> Option<u32> { Some(u32::from_le_bytes(b.get(j..j + 4)?.try_into().ok()?)) };
                    match tag {
                        RUN => {
                            let v = word(i)?;
                            i += 4;
                            out.extend(std::iter::repeat(v).take(n));
                        }
                        LIT => {
                            for k in 0..n {
                                out.push(word(i + k * 4)?);
                            }
                            i += n * 4;
                        }
                        _ => return None,
                    }
                }
            }
            _ => return None,
        }
    }
    (i == b.len()).then_some(out)
}

impl PageShot {
    /// 压起来（像素放掉）。已经压过 / 空图不动
    pub fn pack(&mut self) {
        if self.pixels.is_empty() || self.packed.is_some() {
            return;
        }
        let bytes = pack(&self.pixels, self.width as usize);
        self.packed = Some(bytes.into_boxed_slice());
        self.pixels = Vec::new();
    }

    /// 解开（要显示之前）。解不开（不该发生）就当没有图：侧滑时露暗底，与截不到图时一样
    pub fn unpack(&mut self) {
        let Some(b) = self.packed.take() else { return };
        match unpack(&b, self.width as usize, self.height as usize) {
            Some(px) => self.pixels = px,
            None => {
                self.width = 0;
                self.height = 0;
            }
        }
    }

    /// 占了多少字节（原样的像素或压后的字节）
    pub fn bytes(&self) -> usize {
        self.pixels.capacity() * 4 + self.packed.as_ref().map_or(0, |b| b.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(px: Vec<u32>, w: usize) -> usize {
        let h = px.len() / w;
        let b = pack(&px, w);
        assert_eq!(unpack(&b, w, h).as_deref(), Some(&px[..]), "解出来与压之前不同");
        b.len()
    }

    #[test]
    fn 各种行都逐像素还原() {
        let w = 37;
        let mut px = Vec::new();
        for y in 0..50u32 {
            for x in 0..w as u32 {
                px.push(match y % 5 {
                    0 => 0xFFFFFF,                                   // 整行同色
                    1 => x.wrapping_mul(2_654_435_761) ^ y,          // 全是杂的
                    2 => if x % 7 < 3 { 0x112233 } else { x },       // 游程与杂的交替
                    3 => 0xFFFFFF,                                   // 与上上行同，与上一行不同
                    _ => if x < 2 { x } else { 0xABCDEF },           // 短头 + 长游程
                });
            }
            if y % 9 == 0 {
                let last: Vec<u32> = px[px.len() - w..].to_vec();
                px.extend(last); // 与上一行完全相同
            }
        }
        roundtrip(px, w);
        roundtrip(vec![5; 1], 1);
        roundtrip(vec![1, 2], 2);
    }

    #[test]
    fn 页面那样的图压得很小() {
        let (w, h) = (1080usize, 2400usize);
        let mut px = vec![0xF5F5F5u32; w * h];
        for card in 0..10 {
            for y in card * 220 + 20..card * 220 + 200 {
                for x in 30..w - 30 {
                    px[y * w + x] = if (x / 13 + y / 17) % 9 == 0 { (x * y) as u32 } else { 0xFFFFFF };
                }
            }
        }
        let n = roundtrip(px, w);
        assert!(n < w * h * 4 / 5, "压完 {n} 字节，没到原来的两成");
    }

    #[test]
    fn 坏字节串不崩() {
        assert!(unpack(&[ROW, LIT, 200], 4, 1).is_none());
        assert!(unpack(&[SAME_ROW], 4, 1).is_none());
        assert!(unpack(&[9], 4, 1).is_none());
    }
}
