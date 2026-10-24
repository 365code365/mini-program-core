//! 一块缓冲**真正驻留在物理内存里**的字节（`mincore`）。
//!
//! 画布按内容高 calloc 出来：分配了 40MB 不等于占了 40MB，没写过的页是系统的零页、不进 RSS。
//! 内存分项（host/engine_memory.rs）要的是驻留量，不是容量。非 unix 平台退回容量。

/// `s` 里有多少字节所在的页是驻留的（按整页算，头尾不满一页的也算整页）
pub fn resident<T>(s: &[T]) -> usize {
    let len = std::mem::size_of_val(s);
    if len == 0 {
        return 0;
    }
    #[cfg(unix)]
    {
        // SAFETY: sysconf 无副作用
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(4096) as usize;
        let start = s.as_ptr() as usize & !(page - 1);
        let end = (s.as_ptr() as usize + len + page - 1) & !(page - 1);
        let pages = (end - start) / page;
        #[cfg(any(target_os = "linux", target_os = "android"))]
        if let Some(n) = pagemap_exclusive(start / page, pages) {
            return n * page;
        }
        let mut vec = vec![0u8; pages];
        // SAFETY: [start, end) 覆盖的是 `s` 所在的、已映射的页；vec 每页一个字节
        let r = unsafe { libc::mincore(start as *mut libc::c_void, end - start, vec.as_mut_ptr() as *mut _) };
        if r != 0 {
            return len;
        }
        vec.iter().filter(|b| **b & 1 != 0).count() * page
    }
    #[cfg(not(unix))]
    {
        len
    }
}

/// Linux / Android：`/proc/self/pagemap` 里「在内存里且只映射给本进程」的页数。
///
/// 为什么不直接用 mincore：只**读过**、没写过的匿名页映射的是全系统共用的零页，mincore 照样报驻留，
/// 而它不占内存（smaps 的 Rss 也不算它）。覆盖层每帧要整张扫一遍找非零行（render.rs `row_spans`），
/// 于是还掉的空行马上又「驻留」了 —— 真机上覆盖层显示 8.5MB，其实几乎不占。
/// 零页没有 `PM_MMAP_EXCLUSIVE`（bit 56，内核 4.2+），自己写过的私有页有。读不了 pagemap 就退回 mincore
#[cfg(any(target_os = "linux", target_os = "android"))]
fn pagemap_exclusive(first: usize, pages: usize) -> Option<usize> {
    use std::os::unix::fs::FileExt;
    const PRESENT: u64 = 1 << 63;
    const EXCLUSIVE: u64 = 1 << 56;
    static SUPPORTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let read = |first: usize, pages: usize| -> Option<Vec<u64>> {
        let f = std::fs::File::open("/proc/self/pagemap").ok()?;
        let mut buf = vec![0u8; pages * 8];
        f.read_exact_at(&mut buf, (first * 8) as u64).ok()?;
        Some(buf.chunks_exact(8).map(|e| u64::from_le_bytes(e.try_into().unwrap())).collect())
    };
    // 内核报不报 exclusive 这一位（太老 / 被限制时全是 0，数就不可信）：拿一页自己刚写过的探一次
    let ok = *SUPPORTED.get_or_init(|| {
        // SAFETY: sysconf 无副作用
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(4096) as usize;
        let mut probe = vec![1u8; 3 * page];
        let p = std::hint::black_box(&mut probe).as_mut_ptr() as usize;
        let mid = (p + page - 1) / page;
        read(mid, 1).is_some_and(|v| v[0] & PRESENT != 0 && v[0] & EXCLUSIVE != 0)
    });
    if !ok {
        return None;
    }
    let v = read(first, pages)?;
    Some(v.iter().filter(|e| **e & PRESENT != 0 && **e & EXCLUSIVE != 0).count())
}

/// 把 `s` 里**整页**落在范围内的那些页还给系统（头尾不满一页的不动）。
///
/// 调用方保证这段内容之后不会在重写之前被读：Linux / Android 上它们变成零页（`MADV_DONTNEED`），
/// macOS / iOS 上内容可能被丢掉也可能还在（`MADV_FREE_REUSABLE`）—— 两种都只适用于「内容不再算数」
/// 或「本来就全是 0」的那段。返回还掉了多少字节
pub fn release<T: Copy>(s: &mut [T]) -> usize {
    let len = std::mem::size_of_val(s);
    #[cfg(unix)]
    {
        // SAFETY: sysconf 无副作用
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(4096) as usize;
        let start = (s.as_mut_ptr() as usize + page - 1) & !(page - 1);
        let end = (s.as_mut_ptr() as usize + len) & !(page - 1);
        if end <= start {
            return 0;
        }
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let advice = libc::MADV_FREE_REUSABLE;
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        let advice = libc::MADV_DONTNEED;
        // SAFETY: [start, end) 是 `s` 内部、按页对齐的一段，独占借用（&mut）保证没有别人在读写；
        // `T: Copy` 的任何字节（含全 0）都是合法值
        let r = unsafe { libc::madvise(start as *mut libc::c_void, end - start, advice) };
        if r == 0 { end - start } else { 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = len;
        0
    }
}
