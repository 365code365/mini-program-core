//! QuickJS 的分配器：按大小分档的空闲链表池（每个 JS 运行时一个，只在它自己的线程上用）。
//!
//! 为什么不用 QuickJS 默认的那套：默认分配器每次 malloc / free 之后都要调一次 `malloc_usable_size`
//! 维护自己的内存记账，真机（一加，Cocos 的 farm001）上光这一项就占游戏线程 3.5%，再加上 malloc / free
//! 本身（jemalloc 带锁）合计约 8%。JS 对象、属性表、字符串大多是几十字节、生命周期很短 ——
//! 同一个大小反复分配释放，放进按 16 字节分档的空闲链表里重用，绝大多数分配连系统分配器都不进。
//!
//! 布局：每块前面 16 字节的头记录「可用大小」（`usable_size` 直接读头，不再问系统）。
//! ≤ 512 字节的按 16 字节分 32 档进池，大的直接走系统分配器（头里同样记大小，释放时按大小区分）。
//! 每档最多存 [`MAX_FREE`] 块，多了的直接还给系统：一次性的大量临时对象不会让池子永远占着内存。
//! 运行时销毁时 QuickJS 先释放完所有对象，池子 Drop 时把空闲块全部还掉。
use rquickjs::allocator::Allocator;
use std::alloc::{alloc, alloc_zeroed, dealloc, realloc, Layout};

const HEADER: usize = 16;
const ALIGN: usize = 16;
const CLASS: usize = 16;
const SMALL_MAX: usize = 512;
const CLASSES: usize = SMALL_MAX / CLASS;
/// 每档空闲块上限：32 档 × 4096 × 平均 ~260 字节 ≈ 最多压着 30MB 级别的空闲块，实际远达不到
const MAX_FREE: usize = 4096;

pub struct PoolAllocator {
    free: [Vec<*mut u8>; CLASSES],
}

// SAFETY：空闲链表里的裸指针只属于这个分配器；QuickJS 的运行时同一时刻只在一个线程上用
// （跨线程时外面套着 Mutex，见 runtime/app.rs），分配器跟着运行时走，不会被并发调用
unsafe impl Send for PoolAllocator {}

impl Default for PoolAllocator {
    fn default() -> Self {
        Self { free: std::array::from_fn(|_| Vec::new()) }
    }
}

/// 请求大小 → 可用大小（小块向上取到档位，大块按 16 对齐）
#[inline]
fn usable_for(size: usize) -> usize {
    size.max(1).div_ceil(CLASS) * CLASS
}

#[inline]
fn layout(usable: usize) -> Layout {
    // SAFETY: usable + HEADER 已对齐到 16 且不为 0；ALIGN 是 2 的幂
    unsafe { Layout::from_size_align_unchecked(usable + HEADER, ALIGN) }
}

impl PoolAllocator {
    #[inline]
    fn class_of(usable: usize) -> Option<usize> {
        (usable <= SMALL_MAX).then(|| usable / CLASS - 1)
    }

    fn take(&mut self, size: usize, zeroed: bool) -> *mut u8 {
        let usable = usable_for(size);
        if let Some(c) = Self::class_of(usable) {
            if let Some(p) = self.free[c].pop() {
                if zeroed {
                    // SAFETY: p 是这一档的块，至少 usable 字节
                    unsafe { std::ptr::write_bytes(p, 0, usable) };
                }
                return p;
            }
        }
        // SAFETY: layout 非零大小
        let base = unsafe { if zeroed { alloc_zeroed(layout(usable)) } else { alloc(layout(usable)) } };
        if base.is_null() {
            return std::ptr::null_mut();
        }
        // SAFETY: base 至少 HEADER + usable 字节
        unsafe {
            base.cast::<usize>().write(usable);
            base.add(HEADER)
        }
    }

    /// SAFETY：p 必须是本分配器给出的块
    unsafe fn give_back(&mut self, p: *mut u8) {
        let usable = Self::usable_size(p);
        if let Some(c) = Self::class_of(usable) {
            if self.free[c].len() < MAX_FREE {
                self.free[c].push(p);
                return;
            }
        }
        dealloc(p.sub(HEADER), layout(usable));
    }
}

// SAFETY：返回的指针都对齐到 16（≥ QuickJS 要的 usize / u64 对齐），至少 size 字节；
// usable_size 读的是本分配器写下的头
unsafe impl Allocator for PoolAllocator {
    fn alloc(&mut self, size: usize) -> *mut u8 {
        self.take(size, false)
    }

    fn calloc(&mut self, count: usize, size: usize) -> *mut u8 {
        match count.checked_mul(size) {
            Some(n) => self.take(n, true),
            None => std::ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8) {
        if !ptr.is_null() {
            self.give_back(ptr);
        }
    }

    unsafe fn realloc(&mut self, ptr: *mut u8, new_size: usize) -> *mut u8 {
        if ptr.is_null() {
            return self.take(new_size, false);
        }
        // 与 libc 的 realloc(p, 0) 同义：释放，返回空（QuickJS 的默认分配器也这么处理）
        if new_size == 0 {
            self.give_back(ptr);
            return std::ptr::null_mut();
        }
        let old = Self::usable_size(ptr);
        let want = usable_for(new_size);
        if want == old {
            return ptr;
        }
        // 两头都是大块：交给系统 realloc（大数组 / 字符串增长时可能原地扩）
        if old > SMALL_MAX && want > SMALL_MAX {
            let base = realloc(ptr.sub(HEADER), layout(old), want + HEADER);
            if base.is_null() {
                return std::ptr::null_mut();
            }
            base.cast::<usize>().write(want);
            return base.add(HEADER);
        }
        let np = self.take(new_size, false);
        if np.is_null() {
            return np;
        }
        std::ptr::copy_nonoverlapping(ptr, np, old.min(want));
        self.give_back(ptr);
        np
    }

    unsafe fn usable_size(ptr: *mut u8) -> usize {
        ptr.sub(HEADER).cast::<usize>().read()
    }
}

impl Drop for PoolAllocator {
    fn drop(&mut self) {
        for (c, list) in self.free.iter_mut().enumerate() {
            let usable = (c + 1) * CLASS;
            for p in list.drain(..) {
                // SAFETY: 空闲链表里都是本分配器分出去、已经还回来的小块
                unsafe { dealloc(p.sub(HEADER), layout(usable)) };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 同档重用_大小与对齐() {
        let mut a = PoolAllocator::default();
        let p = a.alloc(20);
        assert_eq!(p as usize % 16, 0);
        assert_eq!(unsafe { PoolAllocator::usable_size(p) }, 32);
        unsafe { a.dealloc(p) };
        let q = a.alloc(30);
        assert_eq!(p, q, "同一档拿回刚还的那块");
        unsafe { a.dealloc(q) };
    }

    #[test]
    fn calloc_清零_realloc_保留内容() {
        let mut a = PoolAllocator::default();
        let p = a.alloc(64);
        unsafe { std::ptr::write_bytes(p, 0xAB, 64) };
        unsafe { a.dealloc(p) };
        let z = a.calloc(8, 8);
        assert!(unsafe { std::slice::from_raw_parts(z, 64) }.iter().all(|&b| b == 0), "从池里拿的块也要清零");
        unsafe { std::ptr::write_bytes(z, 7, 64) };
        let big = unsafe { a.realloc(z, 4000) };
        assert!(unsafe { std::slice::from_raw_parts(big, 64) }.iter().all(|&b| b == 7));
        assert_eq!(unsafe { PoolAllocator::usable_size(big) }, 4000);
        let small = unsafe { a.realloc(big, 10) };
        assert!(unsafe { std::slice::from_raw_parts(small, 10) }.iter().all(|&b| b == 7));
        unsafe { a.dealloc(small) };
    }

    /// 真跑一遍 QuickJS：分配密集的脚本结果正确、GC 之后内存记账归零附近
    #[test]
    fn 在_quickjs_里跑() {
        let rt = rquickjs::Runtime::new_with_alloc(PoolAllocator::default()).unwrap();
        let ctx = rquickjs::Context::full(&rt).unwrap();
        let n: i32 = ctx.with(|c| c.eval("let s=0; for (let i=0;i<20000;i++){ const o={a:i,b:[i,i+1],c:'x'+i}; s+=o.b[1]-o.a; } s").unwrap());
        assert_eq!(n, 20000);
        rt.run_gc();
    }
}
