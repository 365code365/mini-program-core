//! 「这段字在这个宽度下排几行」的记忆表（每个 `TextRenderer` 一份）。
//!
//! taffy 排一遍版要对同一个文本节点问好几次度量（max-content / min-content / 定宽各一次，
//! 第二遍修正再问），每次都是逐字查字形宽度、逐个断行单元累加。setData 之后整棵重建，
//! 页面上没变的那几百段字又全部重数一遍：一加真机上「节点与滚动」页一次重建 6~8ms，
//! 其中数行数占两成（`count_wrapped_text_lines`，逐字 `measure_char_weighted`）。
//!
//! 结果只取决于（字体、文字、可用宽、字号、字间距、粗细），而字体就是这个 `TextRenderer`
//! 本身（表挂在它身上，换字体就是换了一张表），所以可以原样记住：命中时与重算逐位相同。
//! 判等用完整的键（文字逐字节比），哈希只用来分桶；哈希撞了就覆盖重算，不会拿错结果。
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::sync::Mutex;

/// 记满这么多条就整表清空（键是短文本，一条几十字节；整页几百段字，够两三页轮换）
const CAP: usize = 4096;

/// 哈希值已经算好了，表里不再哈希一遍
#[derive(Default)]
struct PreHashed(u64);
impl Hasher for PreHashed {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, _: &[u8]) {
        unreachable!("只用 write_u64")
    }
    fn write_u64(&mut self, v: u64) {
        self.0 = v;
    }
}

struct Entry {
    text: Box<str>,
    /// 可用宽、字号、字间距的位模式与粗细
    params: (u32, u32, u32, bool),
    lines: usize,
}

#[derive(Default)]
pub(crate) struct WrapMemo {
    map: Mutex<HashMap<u64, Entry, BuildHasherDefault<PreHashed>>>,
}

impl WrapMemo {
    /// 记过就直接给，没记过就 `compute` 一次再记下
    pub(crate) fn lines(&self, text: &str, max_width: f32, size: f32, ls: f32, bold: bool, compute: impl FnOnce() -> usize) -> usize {
        let params = (max_width.to_bits(), size.to_bits(), ls.to_bits(), bold);
        // 固定种子的 SipHash（不取线程本地的随机种子）：同一进程内同键同值就够了
        let mut h = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut h);
        params.hash(&mut h);
        let key = h.finish();
        if let Ok(map) = self.map.lock() {
            if let Some(e) = map.get(&key).filter(|e| e.params == params && &*e.text == text) {
                return e.lines;
            }
        }
        // 算的时候不拿着锁（compute 里还要查字形）
        let lines = compute();
        if let Ok(mut map) = self.map.lock() {
            if map.len() >= CAP {
                map.clear();
            }
            map.insert(key, Entry { text: text.into(), params, lines });
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 同键直接给_换任何一项都重算() {
        let m = WrapMemo::default();
        let mut calls = 0;
        assert_eq!(m.lines("你好 world", 100.0, 14.0, 0.0, false, || { calls += 1; 2 }), 2);
        assert_eq!(m.lines("你好 world", 100.0, 14.0, 0.0, false, || { calls += 1; 9 }), 2);
        assert_eq!(calls, 1);
        // 文字、宽、字号、字间距、粗细，任何一项不同都是另一条
        assert_eq!(m.lines("你好 worle", 100.0, 14.0, 0.0, false, || 3), 3);
        assert_eq!(m.lines("你好 world", 100.5, 14.0, 0.0, false, || 4), 4);
        assert_eq!(m.lines("你好 world", 100.0, 15.0, 0.0, false, || 5), 5);
        assert_eq!(m.lines("你好 world", 100.0, 14.0, 1.0, false, || 6), 6);
        assert_eq!(m.lines("你好 world", 100.0, 14.0, 0.0, true, || 7), 7);
        assert_eq!(m.lines("你好 world", 100.0, 14.0, 0.0, false, || 0), 2, "原来那条还在");
    }

    #[test]
    fn 记满了清空重来() {
        let m = WrapMemo::default();
        for i in 0..CAP + 10 {
            m.lines(&i.to_string(), 1.0, 1.0, 0.0, false, || i);
        }
        assert!(m.map.lock().unwrap().len() <= CAP);
        assert_eq!(m.lines(&(CAP + 5).to_string(), 1.0, 1.0, 0.0, false, || 0), CAP + 5);
    }
}
