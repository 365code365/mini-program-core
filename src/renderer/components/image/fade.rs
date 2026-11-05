//! Skyline 的 `<image fade-in>`：图第一次画出来时从透明渐显到不透明。
//!
//! 官方只写了「是否渐显」（默认 false），没给时长与曲线：这里取 [`DURATION_MS`] 线性（待验，Flutter 的 FadeInImage 默认 700ms）。
//!
//! 怎么只重画那张图：渐显期间把它当成「一张会换帧的动图」—— [`frame`] 给出 0..=[`STEPS`] 的帧号，
//! `wxml_renderer/animated_image.rs` 的动图续帧照常工作：宿主每帧问一次，帧号变了只重画那块，页面其余部分不动。
//! 渐显完再留 [`LINGER_MS`] 报最后一帧（保证不透明的那一帧真的画上去），之后不再报，宿主停帧。
//!
//! 每帧代价：没有正在渐显的图时 [`frame`] 只读一个原子计数就返回（每个 image 节点每帧一次）；
//! 渐显中的图每次绘制拷一份乘了 alpha 的像素（只在这 300ms 里）。
//! 按 src 记：同一张图（同一 src）只渐显一次，再出现（换页回来、列表复用）直接不透明。
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// 渐显时长
pub const DURATION_MS: u128 = 300;
/// 帧号的级数（16 级透明度，300ms 里约 50ms 一级 —— 不按刷新率每帧重画）
pub const STEPS: u32 = 16;
/// 渐显完之后继续报最后一帧的时长
const LINGER_MS: u128 = 400;

/// 正在渐显（含 linger）的图数：为 0 时 [`frame`] 不加锁
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
/// src → (开始时刻, 已经结束)
static STARTS: once_cell::sync::Lazy<Mutex<HashMap<String, (Instant, bool)>>> = once_cell::sync::Lazy::new(Default::default);

/// 绘制时取透明度（第一次画到就开始计时）
pub fn alpha(src: &str) -> f32 {
    let Ok(mut m) = STARTS.lock() else { return 1.0 };
    if m.len() > 4096 {
        // 防异常页面无限增长：清掉已经结束的
        m.retain(|_, v| !v.1);
    }
    let (t0, _) = *m.entry(src.to_string()).or_insert_with(|| {
        ACTIVE.fetch_add(1, Ordering::Relaxed);
        (Instant::now(), false)
    });
    (t0.elapsed().as_millis() as f32 / DURATION_MS as f32).clamp(0.0, 1.0)
}

/// 渐显中这张图该显示的帧号（[`STEPS`] = 不透明）；没在渐显（或早已结束）返回 None
pub fn frame(src: &str) -> Option<u32> {
    if ACTIVE.load(Ordering::Relaxed) == 0 {
        return None;
    }
    let mut m = STARTS.lock().ok()?;
    let (t0, done) = m.get_mut(src)?;
    if *done {
        return None;
    }
    let ms = t0.elapsed().as_millis();
    if ms < DURATION_MS {
        return Some((ms * STEPS as u128 / DURATION_MS) as u32);
    }
    if ms < DURATION_MS + LINGER_MS {
        return Some(STEPS);
    }
    *done = true;
    ACTIVE.fetch_sub(1, Ordering::Relaxed);
    None
}

/// 像素乘上透明度（非预乘 RGBA）。`a` ≥ 1 不该调
pub fn faded(rgba: &[u8], a: f32) -> Vec<u8> {
    let k = (a.clamp(0.0, 1.0) * 256.0) as u32;
    let mut out = rgba.to_vec();
    for p in out.chunks_exact_mut(4) {
        p[3] = ((p[3] as u32 * k) >> 8) as u8;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 渐显_从透明到不透明_结束后不再报帧() {
        let src = "fade-test://a.png";
        assert_eq!(frame(src), None, "没画过不在渐显");
        let a0 = alpha(src);
        assert!(a0 < 0.2, "刚开始几乎透明：{a0}");
        assert!(frame(src).unwrap() < STEPS);
        std::thread::sleep(std::time::Duration::from_millis(DURATION_MS as u64 + 20));
        assert_eq!(alpha(src), 1.0);
        assert_eq!(frame(src), Some(STEPS), "渐显完还报最后一帧（让不透明那帧画上去）");
        std::thread::sleep(std::time::Duration::from_millis(LINGER_MS as u64 + 20));
        assert_eq!(frame(src), None, "之后不再报，宿主可以停帧");
        assert_eq!(alpha(src), 1.0, "同一张图不再渐显第二次");
        assert_eq!(faded(&[10, 20, 30, 200], 0.5), vec![10, 20, 30, 100]);
    }
}
