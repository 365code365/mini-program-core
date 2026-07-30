//! 按字节数封顶的 LRU 缓存，给解码后的图片用。
//!
//! 为什么必须封顶：解码后的图是 **RGBA 原图尺寸**，一张 1200×1200 的商品图就是
//! 5.5MB。以前两张缓存表都是无上限的 `HashMap`，一个长列表滚到底就把几十张图
//! 全留在内存里，换页也不释放（缓存是进程级的）。实测 tea-app 的图片页
//! 常驻内存能涨到几百 MB —— 在手机上就是被系统杀掉。
//!
//! 逐出策略是「按最近使用时间从旧到新逐出，直到回到预算内」。
//! 预算可以用 `MINI_IMAGE_CACHE_MB` 覆盖（原生集成时按机型调）。
//!
//! 注意：逐出只是把缓存条目丢掉，**不会影响已经拿到 `Arc` 的调用方**
//! （那一帧照常画完），下次要用时重新解码。

use std::collections::HashMap;

/// 默认预算：解码图 64MB。够装满一屏长列表，又不会让进程无限涨。
const DEFAULT_BUDGET_MB: usize = 64;

pub fn budget_bytes() -> usize {
    static BUDGET: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *BUDGET.get_or_init(|| {
        let mb = std::env::var("MINI_IMAGE_CACHE_MB")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(DEFAULT_BUDGET_MB);
        mb * 1024 * 1024
    })
}

struct Node<V> {
    value: V,
    bytes: usize,
    last_used: u64,
}

/// 按字节封顶的 LRU
pub struct ByteLru<V> {
    map: HashMap<String, Node<V>>,
    bytes: usize,
    tick: u64,
    budget: usize,
    evicted: u64,
}

impl<V> ByteLru<V> {
    pub fn new(budget: usize) -> Self {
        Self { map: HashMap::new(), bytes: 0, tick: 0, budget, evicted: 0 }
    }

    /// 取值并刷新最近使用时间
    pub fn get(&mut self, key: &str) -> Option<&V> {
        self.tick += 1;
        let tick = self.tick;
        let node = self.map.get_mut(key)?;
        node.last_used = tick;
        Some(&node.value)
    }

    /// 只看一眼，不刷新 LRU。
    ///
    /// 给 `probe_load`（`bindload`/`binderror` 的判据）用：它每帧对每个
    /// `<image>` 都要问一次，让它参与 LRU 排序没有意义。
    pub fn peek(&self, key: &str) -> Option<&V> {
        self.map.get(key).map(|n| &n.value)
    }

    pub fn insert(&mut self, key: String, value: V, bytes: usize) {
        self.tick += 1;
        if let Some(old) = self.map.remove(&key) {
            self.bytes = self.bytes.saturating_sub(old.bytes);
        }
        self.bytes += bytes;
        let tick = self.tick;
        self.map.insert(key, Node { value, bytes, last_used: tick });
        self.evict_if_needed();
    }

    fn evict_if_needed(&mut self) {
        if self.bytes <= self.budget {
            return;
        }
        // 按最近使用时间升序逐出。**不逐出刚插入的那一条**（tick 最大者），
        // 否则「一张图比整个预算还大」时会陷入插入即逐出的空转。
        let newest = self.tick;
        let mut order: Vec<(u64, String)> = self
            .map
            .iter()
            .filter(|(_, n)| n.last_used != newest)
            .map(|(k, n)| (n.last_used, k.clone()))
            .collect();
        order.sort_unstable_by_key(|(t, _)| *t);
        for (_, key) in order {
            if self.bytes <= self.budget {
                break;
            }
            if let Some(n) = self.map.remove(&key) {
                self.bytes = self.bytes.saturating_sub(n.bytes);
                self.evicted += 1;
            }
        }
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }


    pub fn evicted(&self) -> u64 {
        self.evicted
    }

    /// 清空（换小程序、内存告警时用）
    pub fn clear(&mut self) {
        self.map.clear();
        self.bytes = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lru(budget: usize) -> ByteLru<u32> {
        ByteLru::new(budget)
    }

    #[test]
    fn keeps_entries_within_budget() {
        let mut c = lru(100);
        c.insert("a".into(), 1, 40);
        c.insert("b".into(), 2, 40);
        assert_eq!(c.len(), 2);
        assert_eq!(c.bytes(), 80);
        assert_eq!(c.evicted(), 0);
    }

    #[test]
    fn evicts_least_recently_used_first() {
        let mut c = lru(100);
        c.insert("a".into(), 1, 40);
        c.insert("b".into(), 2, 40);
        // 用一下 a，让 b 变成最旧的
        assert_eq!(c.get("a").copied(), Some(1));
        c.insert("c".into(), 3, 40); // 总量 120 > 100，要逐出
        assert!(c.bytes() <= 100, "逐出后应回到预算内，实际 {}", c.bytes());
        assert_eq!(c.peek("b"), None, "最久未用的 b 应被逐出");
        assert_eq!(c.peek("a").copied(), Some(1), "刚用过的 a 要留下");
        assert_eq!(c.peek("c").copied(), Some(3), "新插入的必须留下");
        assert_eq!(c.evicted(), 1);
    }

    #[test]
    fn peek_does_not_refresh_lru() {
        let mut c = lru(100);
        c.insert("a".into(), 1, 40);
        c.insert("b".into(), 2, 40);
        // 只 peek a（不刷新），b 是后插入的所以更新 —— 逐出应落在 a 上
        assert_eq!(c.peek("a").copied(), Some(1));
        c.insert("c".into(), 3, 40);
        assert_eq!(c.peek("a"), None, "peek 不该把 a 救回来");
    }

    #[test]
    fn oversized_entry_does_not_spin() {
        // 单条就超预算时：留下它（否则刚插就被逐出，等于永远不缓存、每帧重解码）
        let mut c = lru(100);
        c.insert("big".into(), 1, 500);
        assert_eq!(c.peek("big").copied(), Some(1));
        assert_eq!(c.len(), 1);
        // 再插一条小的，大的应被逐出
        c.insert("small".into(), 2, 10);
        assert_eq!(c.peek("big"), None);
        assert_eq!(c.peek("small").copied(), Some(2));
    }

    #[test]
    fn reinsert_replaces_byte_accounting() {
        let mut c = lru(1000);
        c.insert("a".into(), 1, 100);
        c.insert("a".into(), 2, 300);
        assert_eq!(c.len(), 1);
        assert_eq!(c.bytes(), 300, "重复插入不能把字节数累加两遍");
    }

    #[test]
    fn clear_resets_bytes() {
        let mut c = lru(1000);
        c.insert("a".into(), 1, 100);
        c.clear();
        assert_eq!(c.bytes(), 0);
        assert_eq!(c.len(), 0);
    }

    #[test]
    fn budget_can_be_read() {
        assert!(budget_bytes() >= 1024 * 1024, "预算至少 1MB");
    }
}
