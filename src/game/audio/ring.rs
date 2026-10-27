//! 单生产者单消费者的定长环形队列（game-doc/02 §8.1）：引擎线程 ↔ 音频回调之间的命令与回报。
//!
//! 为什么不用 `std::sync::mpsc`：它的接收端在取走一块之后会**释放**那块内存（链表块），音频回调里不许分配也不许释放
//! （steering 30「热路径不分配」在音频线程是硬要求：系统分配器带锁，拿不到锁就是一次爆音）。这里槽位一次分配好，
//! push / pop 只做原子读写。满了 push 返回 Err（调用方决定丢掉还是下次再发），不阻塞。

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct Inner<T> {
    buf: Box<[UnsafeCell<MaybeUninit<T>>]>,
    /// 下一个要读的位置（只有消费者写）
    head: AtomicUsize,
    /// 下一个要写的位置（只有生产者写）
    tail: AtomicUsize,
}

// SAFETY: 一个槽位同一时刻只被一方访问：生产者只写 [tail, head + cap) 之间的空槽，消费者只读 [head, tail) 之间已写好的槽，
// 两个下标各自只由一方推进，用 Release / Acquire 保证「写完槽再推进下标」对另一方可见
unsafe impl<T: Send> Sync for Inner<T> {}

impl<T> Drop for Inner<T> {
    fn drop(&mut self) {
        let (mut h, t) = (*self.head.get_mut(), *self.tail.get_mut());
        while h != t {
            // SAFETY: [head, tail) 里的槽都已初始化，且没有别人再访问（正在析构）
            unsafe { self.buf[h % self.buf.len()].get_mut().assume_init_drop() };
            h = h.wrapping_add(1);
        }
    }
}

pub struct Producer<T>(Arc<Inner<T>>);
pub struct Consumer<T>(Arc<Inner<T>>);

/// 容量 `cap` 的一对端点
pub fn ring<T: Send>(cap: usize) -> (Producer<T>, Consumer<T>) {
    let cap = cap.max(2);
    let buf = (0..cap).map(|_| UnsafeCell::new(MaybeUninit::uninit())).collect();
    let inner = Arc::new(Inner { buf, head: AtomicUsize::new(0), tail: AtomicUsize::new(0) });
    (Producer(inner.clone()), Consumer(inner))
}

impl<T: Send> Producer<T> {
    /// 放一个；满了原样还回来
    pub fn push(&mut self, v: T) -> Result<(), T> {
        let i = &self.0;
        let t = i.tail.load(Ordering::Relaxed);
        if t.wrapping_sub(i.head.load(Ordering::Acquire)) >= i.buf.len() {
            return Err(v);
        }
        // SAFETY: 这个槽在 [head, head + cap) 之外的空闲区，消费者不会读它（见 Inner 的 Sync 说明）
        unsafe { (*i.buf[t % i.buf.len()].get()).write(v) };
        i.tail.store(t.wrapping_add(1), Ordering::Release);
        Ok(())
    }
}

impl<T: Send> Consumer<T> {
    pub fn pop(&mut self) -> Option<T> {
        let i = &self.0;
        let h = i.head.load(Ordering::Relaxed);
        if h == i.tail.load(Ordering::Acquire) {
            return None;
        }
        // SAFETY: h < tail，槽已由生产者写好并以 Release 发布；读走之后推进 head，生产者才会再写它
        let v = unsafe { (*i.buf[h % i.buf.len()].get()).assume_init_read() };
        i.head.store(h.wrapping_add(1), Ordering::Release);
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 先进先出_满了还回来() {
        let (mut p, mut c) = ring::<u32>(3);
        assert!(p.push(1).is_ok() && p.push(2).is_ok() && p.push(3).is_ok());
        assert_eq!(p.push(4), Err(4));
        assert_eq!((c.pop(), c.pop()), (Some(1), Some(2)));
        assert!(p.push(5).is_ok());
        assert_eq!((c.pop(), c.pop(), c.pop()), (Some(3), Some(5), None));
    }

    #[test]
    fn 跨线程一百万个不丢不乱() {
        let (mut p, mut c) = ring::<u64>(64);
        let t = std::thread::spawn(move || {
            for i in 0..1_000_000u64 {
                let mut v = i;
                while let Err(back) = p.push(v) {
                    v = back;
                    std::hint::spin_loop();
                }
            }
        });
        let mut next = 0u64;
        while next < 1_000_000 {
            if let Some(v) = c.pop() {
                assert_eq!(v, next);
                next += 1;
            }
        }
        t.join().unwrap();
    }

    #[test]
    fn 析构时释放未取走的元素() {
        let v = Arc::new(());
        {
            let (mut p, _c) = ring::<Arc<()>>(4);
            p.push(v.clone()).unwrap();
            p.push(v.clone()).unwrap();
            assert_eq!(Arc::strong_count(&v), 3);
        }
        assert_eq!(Arc::strong_count(&v), 1);
    }
}
