//! 2D 画布 → 纹理的增量上传：`texSubImage2D(…, canvas)` 再传同一张画布到同一个纹理时，只传变了的那几行。
//!
//! 为什么：小游戏的 UI 常画在一张全屏 2D 画布上（萌宠小岛 1320×2868，15MB），每改一个数字整张重传。
//! iPhone 上一次 10~15ms，其中三分之二在 ANGLE 里搬数据、三分之一在 Metal 的 replaceRegion 里转成
//! tile 排布（Time Profiler），一个慢帧的一大块。画布像素大部分行其实没变。
//!
//! 做法：每个 TEXTURE_2D 纹理记住「上次整张传进来的是哪张画布、哪个位置与格式」以及那次的源像素副本；
//! 下次参数完全相同就逐行比（memcmp），只把首末变化行之间那一段转换、上传；一行没变就不调 GL。
//! 比的是**副本**而不是哈希：结果逐字节等于整张重传，没有碰撞的可能。
//!
//! 纹理被别的途径改过就得忘掉它：同一纹理上的其它 texImage / texSubImage / copyTex / 压缩纹理 / texStorage、
//! 删除（编号会复用）、挂到帧缓冲上（之后画进去的内容我们看不见 —— 挂过的纹理永不缓存）。
//! 副本占内存：单张 ≤ 32MB 才缓存，总量 ≤ 64MB，超了丢最早的。

use std::collections::{HashMap, HashSet, VecDeque};

const MAX_ONE: usize = 32 << 20;
const MAX_ALL: usize = 64 << 20;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Key {
    pub canvas: u32,
    pub level: i32,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub format: u32,
    pub ty: u32,
    pub flip: bool,
    pub premul: bool,
}

struct Entry {
    key: Key,
    /// 上次传上去的源像素（转换前、未翻转，RGBA8 直通 alpha）
    src: Vec<u8>,
}

/// 这次要传什么（源像素的行号，未翻转）
#[derive(Debug, PartialEq)]
pub enum Plan {
    /// 第一次 / 参数变了：整张
    Full,
    /// 只有 `[r0, r1)` 这几行变了
    Rows(usize, usize),
    /// 一行没变
    Skip,
}

#[derive(Default)]
pub struct UploadCache {
    entries: HashMap<u32, Entry>,
    order: VecDeque<u32>,
    attached: HashSet<u32>,
    bytes: usize,
}

impl UploadCache {
    /// 纹理被别的途径改过
    pub fn forget(&mut self, tex: u32) {
        if let Some(e) = self.entries.remove(&tex) {
            self.bytes -= e.src.len();
            self.order.retain(|t| *t != tex);
        }
    }

    /// 挂到帧缓冲上：之后可能被画进去，永不缓存（直到删除）
    pub fn attach(&mut self, tex: u32) {
        self.forget(tex);
        self.attached.insert(tex);
    }

    pub fn deleted(&mut self, tex: u32) {
        self.forget(tex);
        self.attached.remove(&tex);
    }

    /// 比对并记下这次的源像素。`src` 是整张画布（w×h×4）
    pub fn plan(&mut self, tex: u32, key: Key, src: &[u8]) -> Plan {
        if tex == 0 || self.attached.contains(&tex) || src.len() > MAX_ONE {
            self.forget(tex);
            return Plan::Full;
        }
        let row = key.w as usize * 4;
        if row == 0 || src.len() != row * key.h as usize {
            self.forget(tex);
            return Plan::Full;
        }
        if let Some(e) = self.entries.get_mut(&tex) {
            if e.key == key && e.src.len() == src.len() {
                let rows_eq = |y: usize| e.src[y * row..(y + 1) * row] == src[y * row..(y + 1) * row];
                let h = key.h as usize;
                let Some(r0) = (0..h).find(|&y| !rows_eq(y)) else { return Plan::Skip };
                let r1 = (r0..h).rev().find(|&y| !rows_eq(y)).map_or(r0 + 1, |y| y + 1);
                e.src[r0 * row..r1 * row].copy_from_slice(&src[r0 * row..r1 * row]);
                return Plan::Rows(r0, r1);
            }
        }
        self.forget(tex);
        while self.bytes + src.len() > MAX_ALL {
            let Some(old) = self.order.pop_front() else { break };
            if let Some(e) = self.entries.remove(&old) {
                self.bytes -= e.src.len();
            }
        }
        self.bytes += src.len();
        self.order.push_back(tex);
        self.entries.insert(tex, Entry { key, src: src.to_vec() });
        Plan::Full
    }
}

/// 源的 `[r0, r1)` 行在纹理里落在哪一行起（翻转时整段倒过来放到对称位置）
pub fn dest_row(key: &Key, r0: usize, r1: usize) -> i32 {
    if key.flip { key.y + (key.h as usize - r1) as i32 } else { key.y + r0 as i32 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> Key {
        Key { canvas: 1, level: 0, x: 0, y: 0, w: 2, h: 4, format: 0x1908, ty: 0x1401, flip: false, premul: false }
    }

    #[test]
    fn 只传变了的行_参数变了或被改过就整张() {
        let mut c = UploadCache::default();
        let mut px = vec![0u8; 2 * 4 * 4];
        assert_eq!(c.plan(7, key(), &px), Plan::Full);
        assert_eq!(c.plan(7, key(), &px), Plan::Skip);
        px[8] = 1; // 第 1 行
        px[3 * 8 + 4] = 9; // 第 3 行
        assert_eq!(c.plan(7, key(), &px), Plan::Rows(1, 4));
        assert_eq!(c.plan(7, key(), &px), Plan::Skip, "副本已经更新");
        let mut k2 = key();
        k2.canvas = 2;
        assert_eq!(c.plan(7, k2, &px), Plan::Full, "换了画布");
        c.forget(7);
        assert_eq!(c.plan(7, k2, &px), Plan::Full, "纹理被别处改过");
        c.attach(7);
        assert_eq!(c.plan(7, k2, &px), Plan::Full);
        assert_eq!(c.plan(7, k2, &px), Plan::Full, "挂过帧缓冲的永不缓存");
        c.deleted(7);
        c.plan(7, k2, &px);
        assert_eq!(c.plan(7, k2, &px), Plan::Skip, "删掉后编号复用，重新开始缓存");
    }

    #[test]
    fn 翻转时落到对称位置() {
        let mut k = key();
        k.y = 10;
        assert_eq!(dest_row(&k, 1, 3), 11);
        k.flip = true;
        // 源第 1、2 行翻过来是第 2、1 行（h=4）：从第 1 行开始放
        assert_eq!(dest_row(&k, 1, 3), 11);
        assert_eq!(dest_row(&k, 0, 1), 13);
    }
}
