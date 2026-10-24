//! 一帧里「慢在哪」的归因（随 `MINI_DAMAGE_LOG` 打开，接在 🩹 行尾的 `〔慢：…〕`）。
//!
//! 🩹 行只说「画 61ms」，这 61ms 是几张图在解码、几百个字在第一次光栅化，还是大图在降采样，
//! 从前只能猜（下载来的小程序一滑就顿：每露出一截新内容，就有一批新图、新字第一次画）。
//! 这里在那几处「第一次才做、做一次很贵」的地方记账：
//! - `解码`：图片同步解码（读盘 + 解码，`image/load.rs`）；
//! - `降采样`：大图第一次按目标尺寸面积降采样（`canvas/images.rs` 的 mip）；
//! - `缩放`：缩放结果缓存未命中、重新双线性采样（同上）；
//! - `字形`：字形缓存未命中、第一次光栅化（`text.rs`）。
//!
//! 每类记次数、总耗时、最慢那一次是谁（图名 / 尺寸）。线程局部，关着时只读一个 `Cell<bool>`，不取时钟。
use std::cell::{Cell, RefCell};
use std::fmt::Write;
use std::time::Instant;

#[derive(Default)]
struct Cat {
    name: &'static str,
    n: u32,
    ms: f64,
    worst_ms: f64,
    worst: String,
}

thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
    static ACC: RefCell<Vec<Cat>> = const { RefCell::new(Vec::new()) };
}

/// 本线程开 / 关（SDK 引擎建好时按 `MINI_DAMAGE_LOG` 设）
pub fn set_enabled(on: bool) {
    ON.with(|c| c.set(on));
}

#[inline]
pub fn enabled() -> bool {
    ON.with(Cell::get)
}

/// 开着时给一个起点（关着给 None，调用方什么也不做）
#[inline]
pub fn start() -> Option<Instant> {
    enabled().then(Instant::now)
}

/// 记一笔：`cat` 类，从 `t` 到现在；`what` 只在它是这一类里最慢的一次时才求值
pub fn record(cat: &'static str, t: Option<Instant>, what: impl FnOnce() -> String) {
    let Some(t) = t else { return };
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    ACC.with(|a| {
        let mut a = a.borrow_mut();
        let i = match a.iter().position(|c| c.name == cat) {
            Some(i) => i,
            None => {
                a.push(Cat { name: cat, ..Default::default() });
                a.len() - 1
            }
        };
        let c = &mut a[i];
        c.n += 1;
        c.ms += ms;
        if ms > c.worst_ms {
            c.worst_ms = ms;
            c.worst = what();
        }
    });
}

/// 取走这一帧记下的（同时清空）。合计不到 `min_ms` 给 None（便宜的帧不加长日志）
pub fn take(min_ms: f64) -> Option<String> {
    let cats = ACC.with(|a| std::mem::take(&mut *a.borrow_mut()));
    let total: f64 = cats.iter().map(|c| c.ms).sum();
    if cats.is_empty() || total < min_ms {
        return None;
    }
    let mut cats = cats;
    cats.sort_by(|a, b| b.ms.total_cmp(&a.ms));
    let mut out = String::from("  〔慢：");
    for (k, c) in cats.iter().enumerate() {
        if k > 0 {
            out.push_str(" · ");
        }
        let _ = write!(out, "{} {}次 {:.1}ms", c.name, c.n, c.ms);
        if c.n > 1 || !c.worst.is_empty() {
            let _ = write!(out, "（最慢 {:.1}", c.worst_ms);
            if !c.worst.is_empty() {
                let _ = write!(out, " {}", c.worst);
            }
            out.push('）');
        }
    }
    out.push_str(" 〕");
    Some(out)
}

/// 路径的最后一段（日志里不要整条沙盒路径）
pub fn short(src: &str) -> &str {
    let s = src.split(['?', '#']).next().unwrap_or(src);
    s.rsplit('/').next().unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 关着不记_开着按类汇总并带最慢的那次() {
        set_enabled(false);
        record("解码", start(), || "a.jpg".into());
        assert_eq!(take(0.0), None);
        set_enabled(true);
        record("字形", start(), String::new);
        record("字形", start(), String::new);
        record("解码", Some(Instant::now() - std::time::Duration::from_millis(5)), || "b.jpg 10×10".into());
        let s = take(0.0).unwrap();
        assert!(s.starts_with("  〔慢：解码 1次"), "{s}");
        assert!(s.contains("b.jpg 10×10") && s.contains("字形 2次"), "{s}");
        assert_eq!(take(0.0), None, "取走即清空");
        set_enabled(false);
    }
}
